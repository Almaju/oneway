use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};

use oneway::checker;
use oneway::error::{OnewayError, Span};
use oneway::lexer::Scanner;
use oneway::parser::ast::*;
use oneway::parser::Parser;

// ---------------------------------------------------------------------------
// Symbol index — maps names to their definition locations
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
struct SymbolInfo {
    name: String,
    kind: SymbolKind,
    detail: String,
    span: Span,
    uri: Url,
}

#[derive(Debug, Default)]
struct DocumentState {
    source: String,
    symbols: HashMap<String, SymbolInfo>,
    symbol_list: Vec<SymbolInfo>,
}

fn collect_symbols(module: &Module, uri: &Url) -> DocumentState {
    let mut state = DocumentState::default();

    for item in &module.items {
        let info = match item {
            Item::Struct(s) => SymbolInfo {
                name: s.name.clone(),
                kind: SymbolKind::STRUCT,
                detail: format!("struct {} ({} fields)", s.name, s.fields.len()),
                span: s.span,
                uri: uri.clone(),
            },
            Item::Enum(e) => SymbolInfo {
                name: e.name.clone(),
                kind: SymbolKind::ENUM,
                detail: format!("enum {} ({} variants)", e.name, e.variants.len()),
                span: e.span,
                uri: uri.clone(),
            },
            Item::Function(f) => {
                let params: Vec<String> = f
                    .params
                    .iter()
                    .map(|p| format_type_expr(&p.type_expr))
                    .collect();
                let ret = f
                    .return_type
                    .as_ref()
                    .map(|t| format!(" -> {}", format_type_expr(t)))
                    .unwrap_or_default();
                SymbolInfo {
                    name: f.name.clone(),
                    kind: SymbolKind::FUNCTION,
                    detail: format!("fn {}({}){}", f.name, params.join(", "), ret),
                    span: f.span,
                    uri: uri.clone(),
                }
            }
            Item::Newtype(n) => SymbolInfo {
                name: n.name.clone(),
                kind: SymbolKind::TYPE_PARAMETER,
                detail: format!("type {} = {}", n.name, format_type_expr(&n.inner_type)),
                span: n.span,
                uri: uri.clone(),
            },
            Item::Contract(c) => SymbolInfo {
                name: c.name.clone(),
                kind: SymbolKind::INTERFACE,
                detail: format!("contract {} ({} functions)", c.name, c.functions.len()),
                span: c.span,
                uri: uri.clone(),
            },
            Item::Use(u) => SymbolInfo {
                name: u.path.join("."),
                kind: SymbolKind::MODULE,
                detail: format!("use {}", u.path.join(".")),
                span: u.span,
                uri: uri.clone(),
            },
        };

        state.symbols.insert(info.name.clone(), info.clone());
        state.symbol_list.push(info);

        // Also index enum variants
        if let Item::Enum(e) = item {
            for v in &e.variants {
                let vinfo = SymbolInfo {
                    name: v.name.clone(),
                    kind: SymbolKind::ENUM_MEMBER,
                    detail: format!("{}.{}", e.name, v.name),
                    span: v.span,
                    uri: uri.clone(),
                };
                state.symbols.insert(v.name.clone(), vinfo.clone());
                state.symbol_list.push(vinfo);
            }
        }
    }

    state
}

fn format_type_expr(te: &TypeExpr) -> String {
    match te {
        TypeExpr::Named(n) => n.clone(),
        TypeExpr::Generic { name, params } => {
            let ps: Vec<String> = params.iter().map(format_type_expr).collect();
            format!("{}<{}>", name, ps.join(", "))
        }
        TypeExpr::Function {
            params,
            return_type,
        } => {
            let ps: Vec<String> = params.iter().map(|p| format_type_expr(p)).collect();
            format!("fn({}) -> {}", ps.join(", "), format_type_expr(return_type))
        }
        TypeExpr::Union(types) => {
            let ts: Vec<String> = types.iter().map(format_type_expr).collect();
            ts.join(" | ")
        }
    }
}

fn span_to_range(span: &Span) -> Range {
    let line = if span.line > 0 { span.line - 1 } else { 0 };
    let col = if span.column > 0 { span.column - 1 } else { 0 };
    let end_col = col + (span.end.saturating_sub(span.start) as u32).max(1);
    Range {
        start: Position {
            line,
            character: col,
        },
        end: Position {
            line,
            character: end_col,
        },
    }
}

/// Find the word at a given position in the source text.
fn word_at_position(source: &str, position: &Position) -> Option<String> {
    let lines: Vec<&str> = source.lines().collect();
    let line_idx = position.line as usize;
    if line_idx >= lines.len() {
        return None;
    }
    let line = lines[line_idx];
    let col = position.character as usize;
    if col > line.len() {
        return None;
    }

    let chars: Vec<char> = line.chars().collect();

    // Find word boundaries
    let mut start = col;
    while start > 0 && is_ident_char(chars[start - 1]) {
        start -= 1;
    }
    let mut end = col;
    while end < chars.len() && is_ident_char(chars[end]) {
        end += 1;
    }

    if start == end {
        return None;
    }

    Some(chars[start..end].iter().collect())
}

fn is_ident_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

// ---------------------------------------------------------------------------
// LSP server
// ---------------------------------------------------------------------------

#[derive(Debug)]
struct OnewayLsp {
    client: Client,
    documents: Arc<RwLock<HashMap<Url, DocumentState>>>,
}

impl OnewayLsp {
    fn new(client: Client) -> Self {
        Self {
            client,
            documents: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    async fn on_change(&self, uri: Url, text: String) {
        let diagnostics = self.run_diagnostics(&text);

        // Parse and index symbols
        let mut scanner = Scanner::new(&text);
        if let Ok(tokens) = scanner.scan_tokens() {
            let mut parser = Parser::new(tokens);
            if let Ok(module) = parser.parse() {
                let mut state = collect_symbols(&module, &uri);
                state.source = text;
                self.documents.write().await.insert(uri.clone(), state);
            } else {
                // Store source even on parse failure for word-at-position
                let mut state = DocumentState::default();
                state.source = text;
                self.documents.write().await.insert(uri.clone(), state);
            }
        } else {
            let mut state = DocumentState::default();
            state.source = text;
            self.documents.write().await.insert(uri.clone(), state);
        }

        self.client
            .publish_diagnostics(uri, diagnostics, None)
            .await;
    }

    fn run_diagnostics(&self, text: &str) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        let mut scanner = Scanner::new(text);
        let tokens = match scanner.scan_tokens() {
            Ok(tokens) => tokens,
            Err(err) => {
                diagnostics.push(oneway_error_to_diagnostic(&err));
                return diagnostics;
            }
        };

        let mut parser = Parser::new(tokens);
        let module = match parser.parse() {
            Ok(module) => module,
            Err(err) => {
                diagnostics.push(oneway_error_to_diagnostic(&err));
                return diagnostics;
            }
        };

        let check_errors = checker::check(&module);
        for err in &check_errors {
            diagnostics.push(oneway_error_to_diagnostic(err));
        }

        diagnostics
    }
}

fn oneway_error_to_diagnostic(err: &OnewayError) -> Diagnostic {
    let span = err.span();
    let message = err.message().to_string();
    let severity = match err {
        OnewayError::CheckError { .. } => DiagnosticSeverity::WARNING,
        _ => DiagnosticSeverity::ERROR,
    };
    Diagnostic {
        range: span_to_range(span),
        severity: Some(severity),
        source: Some("oneway".to_string()),
        message,
        ..Default::default()
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for OnewayLsp {
    async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                definition_provider: Some(OneOf::Left(true)),
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                document_symbol_provider: Some(OneOf::Left(true)),
                ..Default::default()
            },
            server_info: Some(ServerInfo {
                name: "oneway-lsp".to_string(),
                version: Some("0.2.0".to_string()),
            }),
            ..Default::default()
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "Oneway LSP initialized")
            .await;
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        self.on_change(params.text_document.uri, params.text_document.text)
            .await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        if let Some(change) = params.content_changes.into_iter().last() {
            self.on_change(params.text_document.uri, change.text).await;
        }
    }

    async fn did_save(&self, params: DidSaveTextDocumentParams) {
        if let Some(text) = params.text {
            self.on_change(params.text_document.uri, text).await;
        }
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        self.documents
            .write()
            .await
            .remove(&params.text_document.uri);
        self.client
            .publish_diagnostics(params.text_document.uri, vec![], None)
            .await;
    }

    // -- Go to Definition ---------------------------------------------------
    async fn goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> Result<Option<GotoDefinitionResponse>> {
        let uri = params.text_document_position_params.text_document.uri;
        let pos = params.text_document_position_params.position;

        let docs = self.documents.read().await;
        let Some(state) = docs.get(&uri) else {
            return Ok(None);
        };

        let Some(word) = word_at_position(&state.source, &pos) else {
            return Ok(None);
        };

        // Look up in the current document's symbols
        if let Some(sym) = state.symbols.get(&word) {
            return Ok(Some(GotoDefinitionResponse::Scalar(Location {
                uri: sym.uri.clone(),
                range: span_to_range(&sym.span),
            })));
        }

        Ok(None)
    }

    // -- Hover --------------------------------------------------------------
    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        let uri = params.text_document_position_params.text_document.uri;
        let pos = params.text_document_position_params.position;

        let docs = self.documents.read().await;
        let Some(state) = docs.get(&uri) else {
            return Ok(None);
        };

        let Some(word) = word_at_position(&state.source, &pos) else {
            return Ok(None);
        };

        if let Some(sym) = state.symbols.get(&word) {
            return Ok(Some(Hover {
                contents: HoverContents::Markup(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value: format!("```oneway\n{}\n```", sym.detail),
                }),
                range: None,
            }));
        }

        Ok(None)
    }

    // -- Document Symbols (outline) -----------------------------------------
    async fn document_symbol(
        &self,
        params: DocumentSymbolParams,
    ) -> Result<Option<DocumentSymbolResponse>> {
        let uri = params.text_document.uri;

        let docs = self.documents.read().await;
        let Some(state) = docs.get(&uri) else {
            return Ok(None);
        };

        #[allow(deprecated)]
        let symbols: Vec<SymbolInformation> = state
            .symbol_list
            .iter()
            .map(|sym| SymbolInformation {
                name: sym.name.clone(),
                kind: sym.kind,
                tags: None,
                deprecated: None,
                location: Location {
                    uri: sym.uri.clone(),
                    range: span_to_range(&sym.span),
                },
                container_name: None,
            })
            .collect();

        Ok(Some(DocumentSymbolResponse::Flat(symbols)))
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }
}

#[tokio::main]
async fn main() {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::new(OnewayLsp::new);
    Server::new(stdin, stdout, socket).serve(service).await;
}
