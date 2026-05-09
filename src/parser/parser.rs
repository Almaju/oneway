use crate::error::{OnewayError, Result, Span};
use crate::lexer::token::{Token, TokenKind};
use crate::parser::ast::*;

/// A recursive-descent parser for the Oneway language.
///
/// Consumes a flat `Vec<Token>` produced by the lexer and builds a typed AST.
pub struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    // -----------------------------------------------------------------------
    // Construction
    // -----------------------------------------------------------------------

    pub fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, pos: 0 }
    }

    // -----------------------------------------------------------------------
    // Public entry point
    // -----------------------------------------------------------------------

    /// Parse the entire token stream into a [`Module`].
    pub fn parse(&mut self) -> Result<Module> {
        let mut items = Vec::new();
        self.skip_newlines();
        while !self.is_at_end() {
            items.push(self.parse_item()?);
            self.skip_newlines();
        }
        Ok(Module { items })
    }

    // -----------------------------------------------------------------------
    // Token helpers
    // -----------------------------------------------------------------------

    /// Look at the current token without consuming it.
    fn peek(&self) -> &Token {
        self.tokens.get(self.pos).unwrap_or_else(|| {
            self.tokens
                .last()
                .expect("token stream must contain at least an Eof token")
        })
    }

    /// Look at the token `offset` positions ahead of the current position.
    fn peek_ahead(&self, offset: usize) -> &Token {
        self.tokens.get(self.pos + offset).unwrap_or_else(|| {
            self.tokens
                .last()
                .expect("token stream must contain at least an Eof token")
        })
    }

    /// Consume the current token and return it.
    fn advance(&mut self) -> Token {
        let tok = self.tokens.get(self.pos).cloned().unwrap_or_else(|| {
            self.tokens
                .last()
                .cloned()
                .expect("token stream must contain at least an Eof token")
        });
        if self.pos < self.tokens.len() {
            self.pos += 1;
        }
        tok
    }

    /// Check whether the current token's kind matches `kind` exactly
    /// (works for simple keyword / punctuation tokens without embedded data).
    fn check(&self, kind: &TokenKind) -> bool {
        &self.peek().kind == kind
    }

    /// Compare discriminants only – useful for data-carrying variants such as
    /// `Identifier(_)` where we don't care about the inner value.
    fn check_discriminant(&self, kind: &TokenKind) -> bool {
        std::mem::discriminant(&self.peek().kind) == std::mem::discriminant(kind)
    }

    /// If the current token matches `kind`, consume it and return `true`.
    fn match_token(&mut self, kind: &TokenKind) -> bool {
        if self.check(kind) {
            self.advance();
            true
        } else {
            false
        }
    }

    /// Consume the current token if it matches `kind`; otherwise emit a parse
    /// error with `msg`.
    fn expect(&mut self, kind: &TokenKind, msg: &str) -> Result<Token> {
        if self.check(kind) {
            Ok(self.advance())
        } else {
            Err(self.error(msg))
        }
    }

    /// Like [`expect`] but compares only discriminants (for data-carrying
    /// variants).
    fn expect_discriminant(&mut self, kind: &TokenKind, msg: &str) -> Result<Token> {
        if self.check_discriminant(kind) {
            Ok(self.advance())
        } else {
            Err(self.error(msg))
        }
    }

    /// Skip over any consecutive `Newline` tokens.
    fn skip_newlines(&mut self) {
        while self.check(&TokenKind::Newline) {
            self.advance();
        }
    }

    /// `true` when we have reached `Eof`.
    fn is_at_end(&self) -> bool {
        self.peek().kind == TokenKind::Eof
    }

    /// Build a `ParseError` located at the current token.
    fn error(&self, msg: &str) -> OnewayError {
        let tok = self.peek();
        OnewayError::ParseError {
            message: format!("{} (found `{}`)", msg, tok.lexeme),
            span: tok.span,
        }
    }

    /// Build a `ParseError` located at the given span.
    fn error_at(&self, msg: &str, span: Span) -> OnewayError {
        OnewayError::ParseError {
            message: msg.to_string(),
            span,
        }
    }

    // -----------------------------------------------------------------------
    // Items
    // -----------------------------------------------------------------------

    fn parse_item(&mut self) -> Result<Item> {
        self.skip_newlines();

        // Peek for optional `pub` qualifier.
        let public = self.check(&TokenKind::Pub);

        // Determine which item kind follows.  When `pub` is present the actual
        // keyword is one token further along.
        let keyword = if public {
            &self.peek_ahead(1).kind
        } else {
            &self.peek().kind
        };

        match keyword {
            TokenKind::Use => {
                if public {
                    return Err(self.error("`use` items cannot be declared `pub`"));
                }
                self.parse_use_item().map(Item::Use)
            }
            TokenKind::Struct => self.parse_struct_def().map(Item::Struct),
            TokenKind::Enum => self.parse_enum_def().map(Item::Enum),
            TokenKind::Contract => self.parse_contract_def().map(Item::Contract),
            TokenKind::Fn => self.parse_function_def().map(Item::Function),
            TokenKind::TypeKeyword => self.parse_newtype_def().map(Item::Newtype),
            _ => Err(self.error(
                "expected a top-level item (`use`, `struct`, `enum`, `contract`, `fn`, or `type`)",
            )),
        }
    }

    // -- use ----------------------------------------------------------------

    fn parse_use_item(&mut self) -> Result<UseItem> {
        let start = self.peek().span;
        self.expect(&TokenKind::Use, "expected `use`")?;

        let mut path = Vec::new();
        let first = self.expect_discriminant(
            &TokenKind::Identifier(String::new()),
            "expected module name after `use`",
        )?;
        if let TokenKind::Identifier(name) = first.kind {
            path.push(name);
        }

        while self.check(&TokenKind::Dot) {
            self.advance(); // consume `.`
            let seg = self.expect_discriminant(
                &TokenKind::Identifier(String::new()),
                "expected identifier after `.` in use path",
            )?;
            if let TokenKind::Identifier(name) = seg.kind {
                path.push(name);
            }
        }

        Ok(UseItem { path, span: start })
    }

    // -- struct -------------------------------------------------------------

    fn parse_struct_def(&mut self) -> Result<StructDef> {
        let span = self.peek().span;
        let public = self.match_token(&TokenKind::Pub);
        self.expect(&TokenKind::Struct, "expected `struct`")?;

        let name_tok = self.expect_discriminant(
            &TokenKind::Identifier(String::new()),
            "expected struct name",
        )?;
        let name = match name_tok.kind {
            TokenKind::Identifier(n) => n,
            _ => unreachable!(),
        };

        self.skip_newlines();
        self.expect(&TokenKind::LeftBrace, "expected `{` after struct name")?;
        self.skip_newlines();

        // Collect `delegates` declarations before fields.
        let mut delegates = Vec::new();
        while self.check(&TokenKind::Delegates) {
            self.advance(); // consume `delegates`
            delegates.push(self.parse_type_expr()?);
            self.skip_newlines();
            if self.check(&TokenKind::Comma) {
                self.advance();
                self.skip_newlines();
            }
        }

        let mut fields = Vec::new();
        while !self.check(&TokenKind::RightBrace) && !self.is_at_end() {
            fields.push(self.parse_field()?);
            self.skip_newlines();
            // Allow trailing comma.
            if self.check(&TokenKind::Comma) {
                self.advance();
                self.skip_newlines();
            }
        }

        self.expect(&TokenKind::RightBrace, "expected `}` to close struct")?;

        Ok(StructDef {
            public,
            name,
            fields,
            delegates,
            span,
        })
    }

    fn parse_field(&mut self) -> Result<Field> {
        let span = self.peek().span;
        let type_expr = self.parse_type_expr()?;
        Ok(Field { type_expr, span })
    }

    // -- enum ---------------------------------------------------------------

    fn parse_enum_def(&mut self) -> Result<EnumDef> {
        let span = self.peek().span;
        let public = self.match_token(&TokenKind::Pub);
        self.expect(&TokenKind::Enum, "expected `enum`")?;

        let name_tok =
            self.expect_discriminant(&TokenKind::Identifier(String::new()), "expected enum name")?;
        let name = match name_tok.kind {
            TokenKind::Identifier(n) => n,
            _ => unreachable!(),
        };

        self.skip_newlines();
        self.expect(&TokenKind::LeftBrace, "expected `{` after enum name")?;
        self.skip_newlines();

        let mut variants = Vec::new();
        while !self.check(&TokenKind::RightBrace) && !self.is_at_end() {
            variants.push(self.parse_variant()?);
            self.skip_newlines();
            if self.check(&TokenKind::Comma) {
                self.advance();
                self.skip_newlines();
            }
        }

        self.expect(&TokenKind::RightBrace, "expected `}` to close enum")?;

        Ok(EnumDef {
            public,
            name,
            variants,
            span,
        })
    }

    fn parse_variant(&mut self) -> Result<Variant> {
        let span = self.peek().span;
        let name_tok = self.expect_discriminant(
            &TokenKind::Identifier(String::new()),
            "expected variant name",
        )?;
        let name = match name_tok.kind {
            TokenKind::Identifier(n) => n,
            _ => unreachable!(),
        };

        let data = if self.check(&TokenKind::LeftParen) {
            self.advance(); // consume `(`
            let ty = self.parse_type_expr()?;
            self.expect(
                &TokenKind::RightParen,
                "expected `)` after variant data type",
            )?;
            Some(ty)
        } else {
            None
        };

        Ok(Variant { name, data, span })
    }

    // -- contract -----------------------------------------------------------

    fn parse_contract_def(&mut self) -> Result<ContractDef> {
        let span = self.peek().span;
        let public = self.match_token(&TokenKind::Pub);
        self.expect(&TokenKind::Contract, "expected `contract`")?;

        let name_tok = self.expect_discriminant(
            &TokenKind::Identifier(String::new()),
            "expected contract name",
        )?;
        let name = match name_tok.kind {
            TokenKind::Identifier(n) => n,
            _ => unreachable!(),
        };

        self.skip_newlines();
        self.expect(&TokenKind::LeftBrace, "expected `{` after contract name")?;
        self.skip_newlines();

        let mut functions = Vec::new();
        while !self.check(&TokenKind::RightBrace) && !self.is_at_end() {
            functions.push(self.parse_contract_function()?);
            self.skip_newlines();
            if self.check(&TokenKind::Comma) {
                self.advance();
                self.skip_newlines();
            }
        }

        self.expect(&TokenKind::RightBrace, "expected `}` to close contract")?;

        Ok(ContractDef {
            public,
            name,
            functions,
            span,
        })
    }

    fn parse_contract_function(&mut self) -> Result<ContractFunction> {
        let span = self.peek().span;
        self.expect(&TokenKind::Fn, "expected `fn` in contract body")?;

        let name_tok = self.expect_discriminant(
            &TokenKind::Identifier(String::new()),
            "expected function name",
        )?;
        let name = match name_tok.kind {
            TokenKind::Identifier(n) => n,
            _ => unreachable!(),
        };

        self.expect(&TokenKind::LeftParen, "expected `(` in contract function")?;
        let mut params = Vec::new();
        if !self.check(&TokenKind::RightParen) {
            params.push(self.parse_type_expr_allow_self()?);
            if self.check(&TokenKind::Comma) {
                self.advance();
                params.push(self.parse_type_expr_allow_self()?);
            }
        }
        self.expect(&TokenKind::RightParen, "expected `)` in contract function")?;

        let return_type = if self.check(&TokenKind::Arrow) {
            self.advance();
            Some(self.parse_type_expr_allow_self()?)
        } else {
            None
        };

        Ok(ContractFunction {
            name,
            params,
            return_type,
            span,
        })
    }

    // -- function -----------------------------------------------------------

    fn parse_function_def(&mut self) -> Result<FunctionDef> {
        let span = self.peek().span;
        let public = self.match_token(&TokenKind::Pub);
        self.expect(&TokenKind::Fn, "expected `fn`")?;

        let name_tok = self.expect_discriminant(
            &TokenKind::Identifier(String::new()),
            "expected function name",
        )?;
        let name = match name_tok.kind {
            TokenKind::Identifier(n) => n,
            _ => unreachable!(),
        };

        self.expect(&TokenKind::LeftParen, "expected `(` after function name")?;
        let mut params = Vec::new();
        if !self.check(&TokenKind::RightParen) {
            let param_span = self.peek().span;
            let type_expr = self.parse_type_expr()?;
            params.push(Param {
                type_expr,
                span: param_span,
            });

            // Optional second parameter
            if self.check(&TokenKind::Comma) {
                self.advance();
                let param_span2 = self.peek().span;
                let type_expr2 = self.parse_type_expr()?;
                params.push(Param {
                    type_expr: type_expr2,
                    span: param_span2,
                });
            }
        }
        self.expect(&TokenKind::RightParen, "expected `)` after parameters")?;

        let return_type = if self.check(&TokenKind::Arrow) {
            self.advance();
            Some(self.parse_type_expr()?)
        } else {
            None
        };

        self.skip_newlines();
        let body = self.parse_block()?;

        Ok(FunctionDef {
            public,
            name,
            params,
            return_type,
            body,
            span,
        })
    }

    fn parse_newtype_def(&mut self) -> Result<NewtypeDef> {
        let span = self.peek().span;
        let public = self.match_token(&TokenKind::Pub);
        self.expect(&TokenKind::TypeKeyword, "expected `type`")?;

        let name_tok = self.expect_discriminant(
            &TokenKind::Identifier(String::new()),
            "expected type name after `type`",
        )?;
        let name = match name_tok.kind {
            TokenKind::Identifier(n) => n,
            _ => unreachable!(),
        };

        self.expect(&TokenKind::Equal, "expected `=` after type name")?;
        let inner_type = self.parse_type_expr()?;

        Ok(NewtypeDef {
            public,
            name,
            inner_type,
            span,
        })
    }

    // -----------------------------------------------------------------------
    // Type expressions
    // -----------------------------------------------------------------------

    fn parse_type_expr(&mut self) -> Result<TypeExpr> {
        let first = self.parse_single_type_expr()?;

        // Check for union: Type | Type | ...
        if self.check(&TokenKind::Pipe) {
            let mut types = vec![first];
            while self.check(&TokenKind::Pipe) {
                self.advance(); // consume `|`
                types.push(self.parse_single_type_expr()?);
            }
            return Ok(TypeExpr::Union(types));
        }

        Ok(first)
    }

    fn parse_single_type_expr(&mut self) -> Result<TypeExpr> {
        // Function type: fn(T) -> U
        if self.check(&TokenKind::Fn) {
            return self.parse_fn_type();
        }

        let name_tok =
            self.expect_discriminant(&TokenKind::Identifier(String::new()), "expected type name")?;
        let name = match name_tok.kind {
            TokenKind::Identifier(n) => n,
            // Also accept `Self` as a type name.
            _ => unreachable!(),
        };

        // Generic: Name<T, U, ...>
        if self.check(&TokenKind::Less) {
            self.advance(); // consume `<`
            let mut params = vec![self.parse_type_expr()?];
            while self.check(&TokenKind::Comma) {
                self.advance();
                params.push(self.parse_type_expr()?);
            }
            self.expect(
                &TokenKind::Greater,
                "expected `>` to close generic parameters",
            )?;
            return Ok(TypeExpr::Generic { name, params });
        }

        Ok(TypeExpr::Named(name))
    }

    /// Also allow `Self` keyword as a type name.
    fn parse_type_expr_allow_self(&mut self) -> Result<TypeExpr> {
        let first = self.parse_single_type_expr_allow_self()?;

        if self.check(&TokenKind::Pipe) {
            let mut types = vec![first];
            while self.check(&TokenKind::Pipe) {
                self.advance();
                types.push(self.parse_single_type_expr_allow_self()?);
            }
            return Ok(TypeExpr::Union(types));
        }

        Ok(first)
    }

    fn parse_single_type_expr_allow_self(&mut self) -> Result<TypeExpr> {
        if self.check(&TokenKind::SelfType) {
            let tok = self.advance();
            let name = tok.lexeme;
            if self.check(&TokenKind::Less) {
                self.advance();
                let mut params = vec![self.parse_type_expr()?];
                while self.check(&TokenKind::Comma) {
                    self.advance();
                    params.push(self.parse_type_expr()?);
                }
                self.expect(&TokenKind::Greater, "expected `>`")?;
                return Ok(TypeExpr::Generic { name, params });
            }
            return Ok(TypeExpr::Named(name));
        }
        self.parse_single_type_expr()
    }

    fn parse_fn_type(&mut self) -> Result<TypeExpr> {
        self.expect(&TokenKind::Fn, "expected `fn`")?;
        self.expect(&TokenKind::LeftParen, "expected `(` in function type")?;
        let mut params = Vec::new();
        if !self.check(&TokenKind::RightParen) {
            params.push(Box::new(self.parse_type_expr_allow_self()?));
            if self.check(&TokenKind::Comma) {
                self.advance();
                params.push(Box::new(self.parse_type_expr_allow_self()?));
            }
        }
        self.expect(&TokenKind::RightParen, "expected `)` in function type")?;
        self.expect(&TokenKind::Arrow, "expected `->` in function type")?;
        let return_type = Box::new(self.parse_type_expr_allow_self()?);
        Ok(TypeExpr::Function {
            params,
            return_type,
        })
    }

    // -----------------------------------------------------------------------
    // Blocks
    // -----------------------------------------------------------------------

    /// Parse `{ expr* }`.  Returns `Expr::Block(...)`.
    fn parse_block(&mut self) -> Result<Expr> {
        self.expect(&TokenKind::LeftBrace, "expected `{` to start block")?;
        self.skip_newlines();

        let mut exprs = Vec::new();
        while !self.check(&TokenKind::RightBrace) && !self.is_at_end() {
            exprs.push(self.parse_expr()?);
            self.skip_newlines();
        }

        self.expect(&TokenKind::RightBrace, "expected `}` to close block")?;

        Ok(Expr::Block(exprs))
    }

    // -----------------------------------------------------------------------
    // Expressions  (recursive descent, ordered by increasing precedence)
    // -----------------------------------------------------------------------

    /// Top-level expression entry point.
    fn parse_expr(&mut self) -> Result<Expr> {
        self.parse_binding()
    }

    // -- binding  (lowest precedence) ----------------------------------------

    /// `ident = expr`  or fall through to `||`.
    fn parse_binding(&mut self) -> Result<Expr> {
        // Look-ahead: if we see Identifier followed by `=` (but NOT `==`),
        // it is a binding.
        if self.check_discriminant(&TokenKind::Identifier(String::new())) {
            if let Some(next) = self.tokens.get(self.pos + 1) {
                if next.kind == TokenKind::Equal {
                    let name_tok = self.advance();
                    let name = match name_tok.kind {
                        TokenKind::Identifier(n) => n,
                        _ => unreachable!(),
                    };
                    self.advance(); // consume `=`
                    let value = self.parse_expr()?;
                    return Ok(Expr::Binding {
                        name,
                        value: Box::new(value),
                    });
                }
            }
        }

        self.parse_or()
    }

    // -- or -----------------------------------------------------------------

    fn parse_or(&mut self) -> Result<Expr> {
        let mut left = self.parse_and()?;
        while self.check(&TokenKind::OrOr) {
            self.advance();
            let right = self.parse_and()?;
            left = Expr::BinaryOp {
                left: Box::new(left),
                op: BinOp::Or,
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    // -- and ----------------------------------------------------------------

    fn parse_and(&mut self) -> Result<Expr> {
        let mut left = self.parse_equality()?;
        while self.check(&TokenKind::AndAnd) {
            self.advance();
            let right = self.parse_equality()?;
            left = Expr::BinaryOp {
                left: Box::new(left),
                op: BinOp::And,
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    // -- equality -----------------------------------------------------------

    fn parse_equality(&mut self) -> Result<Expr> {
        let mut left = self.parse_comparison()?;
        loop {
            let op = match &self.peek().kind {
                TokenKind::EqualEqual => BinOp::Equal,
                TokenKind::BangEqual => BinOp::NotEqual,
                _ => break,
            };
            self.advance();
            let right = self.parse_comparison()?;
            left = Expr::BinaryOp {
                left: Box::new(left),
                op,
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    // -- comparison ---------------------------------------------------------

    fn parse_comparison(&mut self) -> Result<Expr> {
        let mut left = self.parse_addition()?;
        loop {
            let op = match &self.peek().kind {
                TokenKind::Less => BinOp::Less,
                TokenKind::Greater => BinOp::Greater,
                TokenKind::LessEqual => BinOp::LessEqual,
                TokenKind::GreaterEqual => BinOp::GreaterEqual,
                _ => break,
            };
            self.advance();
            let right = self.parse_addition()?;
            left = Expr::BinaryOp {
                left: Box::new(left),
                op,
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    // -- addition / subtraction ---------------------------------------------

    fn parse_addition(&mut self) -> Result<Expr> {
        let mut left = self.parse_multiplication()?;
        loop {
            let op = match &self.peek().kind {
                TokenKind::Plus => BinOp::Add,
                TokenKind::Minus => BinOp::Subtract,
                _ => break,
            };
            self.advance();
            let right = self.parse_multiplication()?;
            left = Expr::BinaryOp {
                left: Box::new(left),
                op,
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    // -- multiplication / division / modulo ---------------------------------

    fn parse_multiplication(&mut self) -> Result<Expr> {
        let mut left = self.parse_unary()?;
        loop {
            let op = match &self.peek().kind {
                TokenKind::Star => BinOp::Multiply,
                TokenKind::Slash => BinOp::Divide,
                TokenKind::Percent => BinOp::Modulo,
                _ => break,
            };
            self.advance();
            let right = self.parse_unary()?;
            left = Expr::BinaryOp {
                left: Box::new(left),
                op,
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    // -- unary --------------------------------------------------------------

    fn parse_unary(&mut self) -> Result<Expr> {
        match &self.peek().kind {
            TokenKind::Bang => {
                self.advance();
                let operand = self.parse_unary()?;
                Ok(Expr::UnaryOp {
                    op: UnOp::Not,
                    operand: Box::new(operand),
                })
            }
            TokenKind::Minus => {
                self.advance();
                let operand = self.parse_unary()?;
                Ok(Expr::UnaryOp {
                    op: UnOp::Negate,
                    operand: Box::new(operand),
                })
            }
            _ => self.parse_postfix(),
        }
    }

    // -- postfix (dot access, call, try `?`) --------------------------------

    fn parse_postfix(&mut self) -> Result<Expr> {
        let mut expr = self.parse_primary()?;

        loop {
            match &self.peek().kind {
                TokenKind::Dot => {
                    self.advance(); // consume `.`
                    let field_tok = self.expect_discriminant(
                        &TokenKind::Identifier(String::new()),
                        "expected field or method name after `.`",
                    )?;
                    let field = match field_tok.kind {
                        TokenKind::Identifier(n) => n,
                        _ => unreachable!(),
                    };
                    expr = Expr::DotAccess {
                        object: Box::new(expr),
                        field,
                    };
                }
                TokenKind::LeftParen => {
                    self.advance(); // consume `(`
                    let argument = if !self.check(&TokenKind::RightParen) {
                        Some(Box::new(self.parse_expr()?))
                    } else {
                        None
                    };
                    self.expect(
                        &TokenKind::RightParen,
                        "expected `)` after function argument",
                    )?;
                    expr = Expr::Call {
                        function: Box::new(expr),
                        argument,
                    };
                }
                TokenKind::QuestionMark => {
                    self.advance();
                    expr = Expr::Try(Box::new(expr));
                }
                _ => break,
            }
        }

        Ok(expr)
    }

    // -- primary ------------------------------------------------------------

    fn parse_primary(&mut self) -> Result<Expr> {
        let tok = self.peek().clone();

        match &tok.kind {
            // --- literals ---
            TokenKind::IntLiteral(v) => {
                let v = *v;
                self.advance();
                Ok(Expr::IntLiteral(v))
            }
            TokenKind::FloatLiteral(v) => {
                let v = *v;
                self.advance();
                Ok(Expr::FloatLiteral(v))
            }
            TokenKind::StringLiteral(s) => {
                let s = s.clone();
                self.advance();
                // Check for string interpolation: contains `{` … `}` pairs.
                if s.contains('{') && s.contains('}') {
                    let parts = self.parse_string_interpolation_parts(&s, tok.span)?;
                    if parts.len() == 1 {
                        if let StringPart::Literal(text) = &parts[0] {
                            return Ok(Expr::StringLiteral(text.clone()));
                        }
                    }
                    Ok(Expr::StringInterpolation(parts))
                } else {
                    Ok(Expr::StringLiteral(s))
                }
            }
            TokenKind::BoolLiteral(v) => {
                let v = *v;
                self.advance();
                Ok(Expr::BoolLiteral(v))
            }

            // --- parenthesized expression ---
            TokenKind::LeftParen => {
                self.advance(); // consume `(`
                let inner = self.parse_expr()?;
                self.expect(&TokenKind::RightParen, "expected `)` after expression")?;
                Ok(inner)
            }

            // --- block ---
            TokenKind::LeftBrace => self.parse_block(),

            // --- match ---
            TokenKind::Match => self.parse_match_expr(),

            // --- identifier (variable, struct literal, or enum variant via postfix) ---
            TokenKind::Identifier(name) => {
                let name = name.clone();

                // If the identifier starts with an uppercase letter and is
                // followed by `{`, treat as a struct literal.
                let is_uppercase = name.starts_with(|c: char| c.is_ascii_uppercase());
                if is_uppercase && self.peek_ahead(1).kind == TokenKind::LeftBrace {
                    return self.parse_struct_literal();
                }

                // Otherwise it is a plain identifier reference.
                self.advance();
                Ok(Expr::Identifier(name))
            }

            _ => Err(self.error("expected expression")),
        }
    }

    // -- struct literal -----------------------------------------------------

    /// Parse `TypeName { field: expr, ... }`.
    /// The cursor must be on the Identifier token.
    fn parse_struct_literal(&mut self) -> Result<Expr> {
        let name_tok = self.advance();
        let type_name = match name_tok.kind {
            TokenKind::Identifier(n) => n,
            _ => unreachable!(),
        };

        self.expect(&TokenKind::LeftBrace, "expected `{` in struct literal")?;
        self.skip_newlines();

        let mut fields = Vec::new();
        while !self.check(&TokenKind::RightBrace) && !self.is_at_end() {
            fields.push(self.parse_expr()?);
            self.skip_newlines();
            if self.check(&TokenKind::Comma) {
                self.advance();
                self.skip_newlines();
            }
        }

        self.expect(
            &TokenKind::RightBrace,
            "expected `}` to close struct literal",
        )?;

        Ok(Expr::StructLiteral { type_name, fields })
    }

    // -- match expression ---------------------------------------------------

    fn parse_match_expr(&mut self) -> Result<Expr> {
        self.expect(&TokenKind::Match, "expected `match`")?;

        // Optional subject expression – present unless the next token is `{`.
        let subject = if !self.check(&TokenKind::LeftBrace) {
            Some(Box::new(self.parse_expr()?))
        } else {
            None
        };

        self.skip_newlines();
        self.expect(&TokenKind::LeftBrace, "expected `{` after match subject")?;
        self.skip_newlines();

        let mut arms = Vec::new();
        while !self.check(&TokenKind::RightBrace) && !self.is_at_end() {
            arms.push(self.parse_match_arm()?);
            self.skip_newlines();
            // optional comma between arms
            if self.check(&TokenKind::Comma) {
                self.advance();
                self.skip_newlines();
            }
        }

        self.expect(&TokenKind::RightBrace, "expected `}` to close match")?;

        Ok(Expr::Match { subject, arms })
    }

    fn parse_match_arm(&mut self) -> Result<MatchArm> {
        let pattern = self.parse_pattern()?;

        // Optional guard is not yet surfaced in the grammar; reserved.
        let guard = None;

        self.expect(&TokenKind::FatArrow, "expected `=>` in match arm")?;

        let body = self.parse_expr()?;

        Ok(MatchArm {
            pattern,
            guard,
            body,
        })
    }

    // -----------------------------------------------------------------------
    // Patterns
    // -----------------------------------------------------------------------

    fn parse_pattern(&mut self) -> Result<Pattern> {
        let tok = self.peek().clone();

        match &tok.kind {
            // Wildcard: `_`
            TokenKind::Identifier(name) if name == "_" => {
                self.advance();
                Ok(Pattern::Wildcard)
            }

            // Literal patterns
            TokenKind::IntLiteral(v) => {
                let v = *v;
                self.advance();
                Ok(Pattern::Literal(Box::new(Expr::IntLiteral(v))))
            }
            TokenKind::FloatLiteral(v) => {
                let v = *v;
                self.advance();
                Ok(Pattern::Literal(Box::new(Expr::FloatLiteral(v))))
            }
            TokenKind::StringLiteral(s) => {
                let s = s.clone();
                self.advance();
                Ok(Pattern::Literal(Box::new(Expr::StringLiteral(s))))
            }
            TokenKind::BoolLiteral(v) => {
                let v = *v;
                self.advance();
                Ok(Pattern::Literal(Box::new(Expr::BoolLiteral(v))))
            }

            // Negative integer literal: `-42`
            TokenKind::Minus => {
                self.advance(); // consume `-`
                if let TokenKind::IntLiteral(v) = &self.peek().kind {
                    let v = *v;
                    self.advance();
                    Ok(Pattern::Literal(Box::new(Expr::UnaryOp {
                        op: UnOp::Negate,
                        operand: Box::new(Expr::IntLiteral(v)),
                    })))
                } else if let TokenKind::FloatLiteral(v) = &self.peek().kind {
                    let v = *v;
                    self.advance();
                    Ok(Pattern::Literal(Box::new(Expr::UnaryOp {
                        op: UnOp::Negate,
                        operand: Box::new(Expr::FloatLiteral(v)),
                    })))
                } else {
                    Err(self.error("expected numeric literal after `-` in pattern"))
                }
            }

            // Identifier-based patterns
            TokenKind::Identifier(name) => {
                let name = name.clone();
                let is_upper = name.starts_with(|c: char| c.is_ascii_uppercase());

                // Peek ahead: if `Ident.Ident` then enum variant pattern.
                if self.peek_ahead(1).kind == TokenKind::Dot {
                    if let TokenKind::Identifier(_) = &self.peek_ahead(2).kind {
                        return self.parse_enum_variant_pattern();
                    }
                }

                // Uppercase followed by `{` → struct destructuring pattern.
                if is_upper && self.peek_ahead(1).kind == TokenKind::LeftBrace {
                    return self.parse_struct_pattern();
                }

                // Plain binding (lowercase) or bare enum variant (uppercase).
                self.advance();
                Ok(Pattern::Binding(name))
            }

            _ => Err(self.error("expected pattern")),
        }
    }

    /// Parse `TypeName.Variant` or `TypeName.Variant(inner)`.
    fn parse_enum_variant_pattern(&mut self) -> Result<Pattern> {
        let type_tok = self.advance();
        let type_name = match type_tok.kind {
            TokenKind::Identifier(n) => n,
            _ => unreachable!(),
        };
        self.expect(&TokenKind::Dot, "expected `.` in enum variant pattern")?;

        let variant_tok = self.expect_discriminant(
            &TokenKind::Identifier(String::new()),
            "expected variant name after `.`",
        )?;
        let variant = match variant_tok.kind {
            TokenKind::Identifier(n) => n,
            _ => unreachable!(),
        };

        let data = if self.check(&TokenKind::LeftParen) {
            self.advance(); // consume `(`
            let inner = self.parse_pattern()?;
            self.expect(
                &TokenKind::RightParen,
                "expected `)` after variant data pattern",
            )?;
            Some(Box::new(inner))
        } else {
            None
        };

        Ok(Pattern::EnumVariant {
            type_name: Some(type_name),
            variant,
            data,
        })
    }

    /// Parse `TypeName { pat, pat, ... }`.
    fn parse_struct_pattern(&mut self) -> Result<Pattern> {
        let type_tok = self.advance();
        let type_name = match type_tok.kind {
            TokenKind::Identifier(n) => n,
            _ => unreachable!(),
        };

        self.expect(&TokenKind::LeftBrace, "expected `{` in struct pattern")?;
        self.skip_newlines();

        let mut fields = Vec::new();
        while !self.check(&TokenKind::RightBrace) && !self.is_at_end() {
            fields.push(self.parse_pattern()?);
            self.skip_newlines();
            if self.check(&TokenKind::Comma) {
                self.advance();
                self.skip_newlines();
            }
        }

        self.expect(
            &TokenKind::RightBrace,
            "expected `}` to close struct pattern",
        )?;

        Ok(Pattern::Struct { type_name, fields })
    }

    // -----------------------------------------------------------------------
    // String interpolation helpers
    // -----------------------------------------------------------------------

    /// Parse the *contents* of a string that may contain `{expr}` interpolation
    /// segments.  The lexer delivers the whole string as a single
    /// `StringLiteral`, so we do a lightweight re-parse here.
    fn parse_string_interpolation_parts(&self, raw: &str, span: Span) -> Result<Vec<StringPart>> {
        let mut parts: Vec<StringPart> = Vec::new();
        let mut buf = String::new();
        let mut chars = raw.chars().peekable();

        while let Some(ch) = chars.next() {
            if ch == '{' {
                // Flush any accumulated literal text.
                if !buf.is_empty() {
                    parts.push(StringPart::Literal(std::mem::take(&mut buf)));
                }
                // Collect everything up to the matching `}`.
                let mut expr_src = String::new();
                let mut depth = 1u32;
                for inner in chars.by_ref() {
                    if inner == '{' {
                        depth += 1;
                    } else if inner == '}' {
                        depth -= 1;
                        if depth == 0 {
                            break;
                        }
                    }
                    expr_src.push(inner);
                }
                if depth != 0 {
                    return Err(
                        self.error_at("unterminated interpolation expression in string", span)
                    );
                }
                // For now we only support simple identifier interpolation.
                let trimmed = expr_src.trim();
                if trimmed.is_empty() {
                    return Err(self.error_at("empty interpolation expression in string", span));
                }
                parts.push(StringPart::Expr(Box::new(Expr::Identifier(
                    trimmed.to_string(),
                ))));
            } else if ch == '}' {
                // Stray `}` without matching `{` — treat as literal.
                buf.push(ch);
            } else {
                buf.push(ch);
            }
        }

        if !buf.is_empty() {
            parts.push(StringPart::Literal(buf));
        }

        Ok(parts)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::Span;
    use crate::lexer::token::{Token, TokenKind};

    /// Helper to build a token quickly.
    fn tok(kind: TokenKind, lexeme: &str) -> Token {
        Token {
            kind,
            span: Span::default(),
            lexeme: lexeme.to_string(),
        }
    }

    fn eof() -> Token {
        tok(TokenKind::Eof, "")
    }

    fn nl() -> Token {
        tok(TokenKind::Newline, "\n")
    }

    fn ident(name: &str) -> Token {
        tok(TokenKind::Identifier(name.to_string()), name)
    }

    #[test]
    fn parse_empty_module() {
        let tokens = vec![eof()];
        let mut p = Parser::new(tokens);
        let module = p.parse().unwrap();
        assert!(module.items.is_empty());
    }

    #[test]
    fn parse_use_item_simple() {
        let tokens = vec![tok(TokenKind::Use, "use"), ident("io"), eof()];
        let mut p = Parser::new(tokens);
        let module = p.parse().unwrap();
        assert_eq!(module.items.len(), 1);
        match &module.items[0] {
            Item::Use(u) => assert_eq!(u.path, vec!["io"]),
            _ => panic!("expected Use item"),
        }
    }

    #[test]
    fn parse_use_item_dotted() {
        let tokens = vec![
            tok(TokenKind::Use, "use"),
            ident("net"),
            tok(TokenKind::Dot, "."),
            ident("http"),
            eof(),
        ];
        let mut p = Parser::new(tokens);
        let module = p.parse().unwrap();
        match &module.items[0] {
            Item::Use(u) => assert_eq!(u.path, vec!["net", "http"]),
            _ => panic!("expected Use item"),
        }
    }

    #[test]
    fn parse_struct_def() {
        // pub struct Person { Int, String }
        let tokens = vec![
            tok(TokenKind::Pub, "pub"),
            tok(TokenKind::Struct, "struct"),
            ident("Person"),
            tok(TokenKind::LeftBrace, "{"),
            ident("Int"),
            tok(TokenKind::Comma, ","),
            ident("String"),
            tok(TokenKind::RightBrace, "}"),
            eof(),
        ];
        let mut p = Parser::new(tokens);
        let module = p.parse().unwrap();
        match &module.items[0] {
            Item::Struct(s) => {
                assert!(s.public);
                assert_eq!(s.name, "Person");
                assert_eq!(s.fields.len(), 2);
                assert!(matches!(&s.fields[0].type_expr, TypeExpr::Named(n) if n == "Int"));
                assert!(matches!(&s.fields[1].type_expr, TypeExpr::Named(n) if n == "String"));
            }
            _ => panic!("expected Struct item"),
        }
    }

    #[test]
    fn parse_enum_def() {
        // enum Shape { Circle(Float), Rectangle }
        let tokens = vec![
            tok(TokenKind::Enum, "enum"),
            ident("Shape"),
            tok(TokenKind::LeftBrace, "{"),
            ident("Circle"),
            tok(TokenKind::LeftParen, "("),
            ident("Float"),
            tok(TokenKind::RightParen, ")"),
            tok(TokenKind::Comma, ","),
            ident("Rectangle"),
            tok(TokenKind::RightBrace, "}"),
            eof(),
        ];
        let mut p = Parser::new(tokens);
        let module = p.parse().unwrap();
        match &module.items[0] {
            Item::Enum(e) => {
                assert_eq!(e.name, "Shape");
                assert_eq!(e.variants.len(), 2);
                assert!(e.variants[0].data.is_some());
                assert!(e.variants[1].data.is_none());
            }
            _ => panic!("expected Enum item"),
        }
    }

    #[test]
    fn parse_function_with_body() {
        // fn add(Int) -> Int { 42 }
        let tokens = vec![
            tok(TokenKind::Fn, "fn"),
            ident("add"),
            tok(TokenKind::LeftParen, "("),
            ident("Int"),
            tok(TokenKind::RightParen, ")"),
            tok(TokenKind::Arrow, "->"),
            ident("Int"),
            tok(TokenKind::LeftBrace, "{"),
            tok(TokenKind::IntLiteral(42), "42"),
            tok(TokenKind::RightBrace, "}"),
            eof(),
        ];
        let mut p = Parser::new(tokens);
        let module = p.parse().unwrap();
        match &module.items[0] {
            Item::Function(f) => {
                assert_eq!(f.name, "add");
                assert_eq!(f.params.len(), 1);
                assert!(f.return_type.is_some());
                match &f.body {
                    Expr::Block(exprs) => {
                        assert_eq!(exprs.len(), 1);
                        match &exprs[0] {
                            Expr::IntLiteral(42) => {}
                            other => panic!("expected IntLiteral(42), got {:?}", other),
                        }
                    }
                    _ => panic!("expected Block body"),
                }
            }
            _ => panic!("expected Function item"),
        }
    }

    #[test]
    fn parse_binary_expr_precedence() {
        // 1 + 2 * 3  →  Add(1, Mul(2, 3))
        let tokens = vec![
            tok(TokenKind::IntLiteral(1), "1"),
            tok(TokenKind::Plus, "+"),
            tok(TokenKind::IntLiteral(2), "2"),
            tok(TokenKind::Star, "*"),
            tok(TokenKind::IntLiteral(3), "3"),
            eof(),
        ];
        let mut p = Parser::new(tokens);
        let expr = p.parse_expr().unwrap();
        match &expr {
            Expr::BinaryOp { op, left, right } => {
                assert_eq!(*op, BinOp::Add);
                assert!(matches!(left.as_ref(), Expr::IntLiteral(1)));
                match right.as_ref() {
                    Expr::BinaryOp { op, .. } => assert_eq!(*op, BinOp::Multiply),
                    other => panic!("expected BinaryOp(Multiply), got {:?}", other),
                }
            }
            _ => panic!("expected BinaryOp"),
        }
    }

    #[test]
    fn parse_dot_chain_and_call() {
        // a.b.c()
        let tokens = vec![
            ident("a"),
            tok(TokenKind::Dot, "."),
            ident("b"),
            tok(TokenKind::Dot, "."),
            ident("c"),
            tok(TokenKind::LeftParen, "("),
            tok(TokenKind::RightParen, ")"),
            eof(),
        ];
        let mut p = Parser::new(tokens);
        let expr = p.parse_expr().unwrap();
        // Should be Call(DotAccess(DotAccess(a, b), c), None)
        match &expr {
            Expr::Call { function, argument } => {
                assert!(argument.is_none());
                match function.as_ref() {
                    Expr::DotAccess { object, field } => {
                        assert_eq!(field, "c");
                        match object.as_ref() {
                            Expr::DotAccess { object, field } => {
                                assert_eq!(field, "b");
                                assert!(matches!(object.as_ref(), Expr::Identifier(n) if n == "a"));
                            }
                            _ => panic!("expected DotAccess"),
                        }
                    }
                    _ => panic!("expected DotAccess"),
                }
            }
            _ => panic!("expected Call"),
        }
    }

    #[test]
    fn parse_match_expr() {
        // match x { 0 => true, _ => false }
        let tokens = vec![
            tok(TokenKind::Match, "match"),
            ident("x"),
            tok(TokenKind::LeftBrace, "{"),
            tok(TokenKind::IntLiteral(0), "0"),
            tok(TokenKind::FatArrow, "=>"),
            tok(TokenKind::BoolLiteral(true), "true"),
            tok(TokenKind::Comma, ","),
            ident("_"),
            tok(TokenKind::FatArrow, "=>"),
            tok(TokenKind::BoolLiteral(false), "false"),
            tok(TokenKind::RightBrace, "}"),
            eof(),
        ];
        let mut p = Parser::new(tokens);
        let expr = p.parse_expr().unwrap();
        match &expr {
            Expr::Match { subject, arms } => {
                assert!(subject.is_some());
                assert_eq!(arms.len(), 2);
                assert!(matches!(&arms[0].pattern, Pattern::Literal(_)));
                assert!(matches!(&arms[1].pattern, Pattern::Wildcard));
            }
            _ => panic!("expected Match"),
        }
    }

    #[test]
    fn parse_struct_literal_expr() {
        // Person { 30, "Alice" }
        let tokens = vec![
            ident("Person"),
            tok(TokenKind::LeftBrace, "{"),
            tok(TokenKind::IntLiteral(30), "30"),
            tok(TokenKind::Comma, ","),
            tok(TokenKind::StringLiteral("Alice".into()), "\"Alice\""),
            tok(TokenKind::RightBrace, "}"),
            eof(),
        ];
        let mut p = Parser::new(tokens);
        let expr = p.parse_expr().unwrap();
        match &expr {
            Expr::StructLiteral { type_name, fields } => {
                assert_eq!(type_name, "Person");
                assert_eq!(fields.len(), 2);
            }
            _ => panic!("expected StructLiteral"),
        }
    }

    #[test]
    fn parse_try_expr() {
        // f()?
        let tokens = vec![
            ident("f"),
            tok(TokenKind::LeftParen, "("),
            tok(TokenKind::RightParen, ")"),
            tok(TokenKind::QuestionMark, "?"),
            eof(),
        ];
        let mut p = Parser::new(tokens);
        let expr = p.parse_expr().unwrap();
        assert!(matches!(expr, Expr::Try(_)));
    }

    #[test]
    fn parse_binding() {
        // x = 42
        let tokens = vec![
            ident("x"),
            tok(TokenKind::Equal, "="),
            tok(TokenKind::IntLiteral(42), "42"),
            eof(),
        ];
        let mut p = Parser::new(tokens);
        let expr = p.parse_expr().unwrap();
        match &expr {
            Expr::Binding { name, value } => {
                assert_eq!(name, "x");
                assert!(matches!(value.as_ref(), Expr::IntLiteral(42)));
            }
            _ => panic!("expected Binding"),
        }
    }

    #[test]
    fn parse_contract_def() {
        // pub contract Printable { fn to_string(Self) -> String }
        let tokens = vec![
            tok(TokenKind::Pub, "pub"),
            tok(TokenKind::Contract, "contract"),
            ident("Printable"),
            tok(TokenKind::LeftBrace, "{"),
            tok(TokenKind::Fn, "fn"),
            ident("to_string"),
            tok(TokenKind::LeftParen, "("),
            tok(TokenKind::SelfType, "Self"),
            tok(TokenKind::RightParen, ")"),
            tok(TokenKind::Arrow, "->"),
            ident("String"),
            tok(TokenKind::RightBrace, "}"),
            eof(),
        ];
        let mut p = Parser::new(tokens);
        let module = p.parse().unwrap();
        match &module.items[0] {
            Item::Contract(c) => {
                assert!(c.public);
                assert_eq!(c.name, "Printable");
                assert_eq!(c.functions.len(), 1);
                assert_eq!(c.functions[0].name, "to_string");
                assert_eq!(c.functions[0].params.len(), 1);
                assert!(c.functions[0].return_type.is_some());
            }
            _ => panic!("expected Contract item"),
        }
    }

    #[test]
    fn parse_generic_type() {
        // fn id(List<Int>) -> List<Int> { x }
        let tokens = vec![
            tok(TokenKind::Fn, "fn"),
            ident("id"),
            tok(TokenKind::LeftParen, "("),
            ident("List"),
            tok(TokenKind::Less, "<"),
            ident("Int"),
            tok(TokenKind::Greater, ">"),
            tok(TokenKind::RightParen, ")"),
            tok(TokenKind::Arrow, "->"),
            ident("List"),
            tok(TokenKind::Less, "<"),
            ident("Int"),
            tok(TokenKind::Greater, ">"),
            tok(TokenKind::LeftBrace, "{"),
            ident("x"),
            tok(TokenKind::RightBrace, "}"),
            eof(),
        ];
        let mut p = Parser::new(tokens);
        let module = p.parse().unwrap();
        match &module.items[0] {
            Item::Function(f) => match &f.params[0].type_expr {
                TypeExpr::Generic { name, params } => {
                    assert_eq!(name, "List");
                    assert_eq!(params.len(), 1);
                }
                other => panic!("expected Generic type, got {:?}", other),
            },
            _ => panic!("expected Function item"),
        }
    }

    #[test]
    fn parse_enum_variant_pattern() {
        // match x { Shape.Circle(r) => r, Shape.Rectangle => 0 }
        let tokens = vec![
            tok(TokenKind::Match, "match"),
            ident("x"),
            tok(TokenKind::LeftBrace, "{"),
            ident("Shape"),
            tok(TokenKind::Dot, "."),
            ident("Circle"),
            tok(TokenKind::LeftParen, "("),
            ident("r"),
            tok(TokenKind::RightParen, ")"),
            tok(TokenKind::FatArrow, "=>"),
            ident("r"),
            tok(TokenKind::Comma, ","),
            ident("Shape"),
            tok(TokenKind::Dot, "."),
            ident("Rectangle"),
            tok(TokenKind::FatArrow, "=>"),
            tok(TokenKind::IntLiteral(0), "0"),
            tok(TokenKind::RightBrace, "}"),
            eof(),
        ];
        let mut p = Parser::new(tokens);
        let expr = p.parse_expr().unwrap();
        match &expr {
            Expr::Match { arms, .. } => {
                assert_eq!(arms.len(), 2);
                match &arms[0].pattern {
                    Pattern::EnumVariant {
                        type_name,
                        variant,
                        data,
                    } => {
                        assert_eq!(type_name.as_deref(), Some("Shape"));
                        assert_eq!(variant, "Circle");
                        assert!(data.is_some());
                    }
                    other => panic!("expected EnumVariant, got {:?}", other),
                }
            }
            _ => panic!("expected Match"),
        }
    }

    #[test]
    fn parse_newlines_between_items() {
        let tokens = vec![
            tok(TokenKind::Use, "use"),
            ident("io"),
            nl(),
            nl(),
            tok(TokenKind::Use, "use"),
            ident("net"),
            nl(),
            eof(),
        ];
        let mut p = Parser::new(tokens);
        let module = p.parse().unwrap();
        assert_eq!(module.items.len(), 2);
    }

    #[test]
    fn parse_block_with_newlines() {
        // { 1 \n 2 \n 3 }
        let tokens = vec![
            tok(TokenKind::LeftBrace, "{"),
            nl(),
            tok(TokenKind::IntLiteral(1), "1"),
            nl(),
            tok(TokenKind::IntLiteral(2), "2"),
            nl(),
            tok(TokenKind::IntLiteral(3), "3"),
            nl(),
            tok(TokenKind::RightBrace, "}"),
            eof(),
        ];
        let mut p = Parser::new(tokens);
        let expr = p.parse_block().unwrap();
        match &expr {
            Expr::Block(exprs) => assert_eq!(exprs.len(), 3),
            _ => panic!("expected Block"),
        }
    }

    #[test]
    fn parse_unary_negate() {
        // -42
        let tokens = vec![
            tok(TokenKind::Minus, "-"),
            tok(TokenKind::IntLiteral(42), "42"),
            eof(),
        ];
        let mut p = Parser::new(tokens);
        let expr = p.parse_expr().unwrap();
        match &expr {
            Expr::UnaryOp { op, operand } => {
                assert_eq!(*op, UnOp::Negate);
                assert!(matches!(operand.as_ref(), Expr::IntLiteral(42)));
            }
            _ => panic!("expected UnaryOp"),
        }
    }

    #[test]
    fn parse_union_type() {
        let tokens = vec![
            tok(TokenKind::Fn, "fn"),
            ident("handle"),
            tok(TokenKind::LeftParen, "("),
            ident("Request"),
            tok(TokenKind::RightParen, ")"),
            tok(TokenKind::Arrow, "->"),
            ident("Result"),
            tok(TokenKind::Less, "<"),
            ident("Response"),
            tok(TokenKind::Comma, ","),
            ident("NotFound"),
            tok(TokenKind::Pipe, "|"),
            ident("ServerError"),
            tok(TokenKind::Greater, ">"),
            tok(TokenKind::LeftBrace, "{"),
            nl(),
            ident("request"),
            tok(TokenKind::Dot, "."),
            ident("process"),
            tok(TokenKind::LeftParen, "("),
            tok(TokenKind::RightParen, ")"),
            nl(),
            tok(TokenKind::RightBrace, "}"),
            nl(),
            eof(),
        ];
        let mut parser = Parser::new(tokens);
        let module = parser.parse().expect("should parse union type");
        assert_eq!(module.items.len(), 1);
        if let Item::Function(f) = &module.items[0] {
            // The return type should be Generic "Result" with params [Named("Response"), Union([Named("NotFound"), Named("ServerError")])]
            if let Some(TypeExpr::Generic { name, params }) = &f.return_type {
                assert_eq!(name, "Result");
                assert_eq!(params.len(), 2);
                assert!(matches!(&params[1], TypeExpr::Union(types) if types.len() == 2));
            } else {
                panic!("expected generic return type");
            }
        } else {
            panic!("expected function");
        }
    }

    #[test]
    fn parse_two_param_function() {
        // fn addBalance(Wallet, Amount) -> Wallet { wallet }
        let tokens = vec![
            tok(TokenKind::Fn, "fn"),
            ident("addBalance"),
            tok(TokenKind::LeftParen, "("),
            ident("Wallet"),
            tok(TokenKind::Comma, ","),
            ident("Amount"),
            tok(TokenKind::RightParen, ")"),
            tok(TokenKind::Arrow, "->"),
            ident("Wallet"),
            tok(TokenKind::LeftBrace, "{"),
            ident("wallet"),
            tok(TokenKind::RightBrace, "}"),
            eof(),
        ];
        let mut parser = Parser::new(tokens);
        let module = parser.parse().expect("should parse 2-param function");
        assert_eq!(module.items.len(), 1);
        if let Item::Function(f) = &module.items[0] {
            assert_eq!(f.name, "addBalance");
            assert_eq!(f.params.len(), 2);
            assert!(matches!(&f.params[0].type_expr, TypeExpr::Named(n) if n == "Wallet"));
            assert!(matches!(&f.params[1].type_expr, TypeExpr::Named(n) if n == "Amount"));
        } else {
            panic!("expected Function");
        }
    }

    #[test]
    fn parse_newtype_def_test() {
        // type TaskId = Int
        let tokens = vec![
            tok(TokenKind::TypeKeyword, "type"),
            ident("TaskId"),
            tok(TokenKind::Equal, "="),
            ident("Int"),
            eof(),
        ];
        let mut parser = Parser::new(tokens);
        let module = parser.parse().expect("should parse newtype");
        assert_eq!(module.items.len(), 1);
        if let Item::Newtype(nt) = &module.items[0] {
            assert_eq!(nt.name, "TaskId");
            assert!(!nt.public);
            assert!(matches!(&nt.inner_type, TypeExpr::Named(n) if n == "Int"));
        } else {
            panic!("expected Newtype");
        }
    }

    #[test]
    fn parse_pub_newtype() {
        // pub type Balance = Int
        let tokens = vec![
            tok(TokenKind::Pub, "pub"),
            tok(TokenKind::TypeKeyword, "type"),
            ident("Balance"),
            tok(TokenKind::Equal, "="),
            ident("Int"),
            eof(),
        ];
        let mut parser = Parser::new(tokens);
        let module = parser.parse().expect("should parse pub newtype");
        if let Item::Newtype(nt) = &module.items[0] {
            assert!(nt.public);
            assert_eq!(nt.name, "Balance");
        } else {
            panic!("expected Newtype");
        }
    }

    #[test]
    fn parse_zero_param_function() {
        // fn now() -> Timestamp { x }
        let tokens = vec![
            tok(TokenKind::Fn, "fn"),
            ident("now"),
            tok(TokenKind::LeftParen, "("),
            tok(TokenKind::RightParen, ")"),
            tok(TokenKind::Arrow, "->"),
            ident("Timestamp"),
            tok(TokenKind::LeftBrace, "{"),
            ident("x"),
            tok(TokenKind::RightBrace, "}"),
            eof(),
        ];
        let mut parser = Parser::new(tokens);
        let module = parser.parse().expect("should parse 0-param function");
        if let Item::Function(f) = &module.items[0] {
            assert_eq!(f.params.len(), 0);
        } else {
            panic!("expected Function");
        }
    }

    #[test]
    fn parse_fn_type_expr() {
        // contract C { fn apply(fn(Int) -> String) -> String }
        let tokens = vec![
            tok(TokenKind::Contract, "contract"),
            ident("C"),
            tok(TokenKind::LeftBrace, "{"),
            tok(TokenKind::Fn, "fn"),
            ident("apply"),
            tok(TokenKind::LeftParen, "("),
            tok(TokenKind::Fn, "fn"),
            tok(TokenKind::LeftParen, "("),
            ident("Int"),
            tok(TokenKind::RightParen, ")"),
            tok(TokenKind::Arrow, "->"),
            ident("String"),
            tok(TokenKind::RightParen, ")"),
            tok(TokenKind::Arrow, "->"),
            ident("String"),
            tok(TokenKind::RightBrace, "}"),
            eof(),
        ];
        let mut p = Parser::new(tokens);
        let module = p.parse().unwrap();
        match &module.items[0] {
            Item::Contract(c) => {
                let f = &c.functions[0];
                assert_eq!(f.params.len(), 1);
                match &f.params[0] {
                    TypeExpr::Function {
                        params,
                        return_type,
                    } => {
                        assert_eq!(params.len(), 1);
                        assert!(
                            matches!(return_type.as_ref(), TypeExpr::Named(n) if n == "String")
                        );
                    }
                    other => panic!("expected function type param, got {:?}", other),
                }
            }
            _ => panic!("expected Contract"),
        }
    }
}
