use crate::error::{OnewayError, Result, Span};
use crate::lexer::token::{Token, TokenKind};

/// The scanner (lexer) for the Oneway language.
///
/// Converts a source string into a sequence of `Token`s, tracking line and
/// column information as it goes.
pub struct Scanner<'src> {
    source: &'src str,
    chars: Vec<char>,
    tokens: Vec<Token>,

    /// Byte offset where the current token started.
    start: usize,
    /// Current byte offset into `chars`.
    current: usize,

    /// 1-based line number of `start`.
    start_line: u32,
    /// 1-based column number of `start`.
    start_column: u32,

    /// 1-based line number of `current`.
    line: u32,
    /// 1-based column of `current`.
    column: u32,

    /// Whether the last token we emitted was a `Newline` (used to coalesce
    /// consecutive newlines into a single token).
    last_was_newline: bool,
}

impl<'src> Scanner<'src> {
    /// Create a new scanner for the given source text.
    pub fn new(source: &'src str) -> Self {
        Self {
            source,
            chars: source.chars().collect(),
            tokens: Vec::new(),
            start: 0,
            current: 0,
            start_line: 1,
            start_column: 1,
            line: 1,
            column: 1,
            last_was_newline: true, // suppress leading newlines
        }
    }

    // ── public API ──────────────────────────────────────────────────────

    /// Scan the entire source and return the resulting token stream.
    /// The stream always ends with an `Eof` token.
    pub fn scan_tokens(&mut self) -> Result<Vec<Token>> {
        while !self.is_at_end() {
            self.skip_whitespace();
            if self.is_at_end() {
                break;
            }
            self.start = self.current;
            self.start_line = self.line;
            self.start_column = self.column;
            self.scan_token()?;
        }

        // Emit Eof
        self.tokens.push(Token {
            kind: TokenKind::Eof,
            span: Span::new(self.current, self.current, self.line, self.column),
            lexeme: String::new(),
        });

        Ok(self.tokens.clone())
    }

    // ── helpers ─────────────────────────────────────────────────────────

    fn is_at_end(&self) -> bool {
        self.current >= self.chars.len()
    }

    /// Peek at the current character without consuming it.
    fn peek(&self) -> char {
        if self.is_at_end() {
            '\0'
        } else {
            self.chars[self.current]
        }
    }

    /// Peek at the character one position ahead of `current`.
    fn peek_next(&self) -> char {
        if self.current + 1 >= self.chars.len() {
            '\0'
        } else {
            self.chars[self.current + 1]
        }
    }

    /// Consume the current character and advance.
    fn advance(&mut self) -> char {
        let c = self.chars[self.current];
        self.current += 1;
        if c == '\n' {
            self.line += 1;
            self.column = 1;
        } else {
            self.column += 1;
        }
        c
    }

    /// Consume the current character only if it matches `expected`.
    fn match_char(&mut self, expected: char) -> bool {
        if self.is_at_end() || self.chars[self.current] != expected {
            false
        } else {
            self.advance();
            true
        }
    }

    /// Return the lexeme for the current token (from `start` to `current`).
    fn current_lexeme(&self) -> String {
        self.chars[self.start..self.current].iter().collect()
    }

    /// Build a `Span` that covers the current token.
    fn current_span(&self) -> Span {
        Span::new(self.start, self.current, self.start_line, self.start_column)
    }

    /// Push a token with the given kind, using the current span and lexeme.
    fn add_token(&mut self, kind: TokenKind) {
        let is_newline = kind == TokenKind::Newline;

        // Coalesce consecutive newlines.
        if is_newline && self.last_was_newline {
            return;
        }

        self.last_was_newline = is_newline;

        self.tokens.push(Token {
            kind,
            span: self.current_span(),
            lexeme: self.current_lexeme(),
        });
    }

    /// Create a `LexError` at the current span.
    fn lex_error(&self, message: impl Into<String>) -> OnewayError {
        OnewayError::LexError {
            message: message.into(),
            span: self.current_span(),
        }
    }

    /// Skip spaces and tabs (but NOT newlines — those are significant).
    fn skip_whitespace(&mut self) {
        while !self.is_at_end() {
            match self.peek() {
                ' ' | '\t' | '\r' => {
                    self.advance();
                }
                _ => break,
            }
        }
    }

    // ── main dispatch ───────────────────────────────────────────────────

    fn scan_token(&mut self) -> Result<()> {
        let c = self.advance();

        match c {
            // ----- newlines -----
            '\n' => {
                self.add_token(TokenKind::Newline);
            }

            // ----- single-character tokens -----
            '+' => self.add_token(TokenKind::Plus),
            '*' => self.add_token(TokenKind::Star),
            '%' => self.add_token(TokenKind::Percent),
            '.' => self.add_token(TokenKind::Dot),
            '(' => self.add_token(TokenKind::LeftParen),
            ')' => self.add_token(TokenKind::RightParen),
            '{' => self.add_token(TokenKind::LeftBrace),
            '}' => self.add_token(TokenKind::RightBrace),
            '[' => self.add_token(TokenKind::LeftBracket),
            ']' => self.add_token(TokenKind::RightBracket),
            ':' => self.add_token(TokenKind::Colon),
            ',' => self.add_token(TokenKind::Comma),
            '?' => self.add_token(TokenKind::QuestionMark),

            // ----- one-or-two character tokens -----
            '=' => {
                if self.match_char('=') {
                    self.add_token(TokenKind::EqualEqual);
                } else if self.match_char('>') {
                    self.add_token(TokenKind::FatArrow);
                } else {
                    self.add_token(TokenKind::Equal);
                }
            }
            '!' => {
                if self.match_char('=') {
                    self.add_token(TokenKind::BangEqual);
                } else {
                    self.add_token(TokenKind::Bang);
                }
            }
            '<' => {
                if self.match_char('=') {
                    self.add_token(TokenKind::LessEqual);
                } else {
                    self.add_token(TokenKind::Less);
                }
            }
            '>' => {
                if self.match_char('=') {
                    self.add_token(TokenKind::GreaterEqual);
                } else {
                    self.add_token(TokenKind::Greater);
                }
            }
            '-' => {
                if self.match_char('>') {
                    self.add_token(TokenKind::Arrow);
                } else {
                    self.add_token(TokenKind::Minus);
                }
            }
            '&' => {
                if self.match_char('&') {
                    self.add_token(TokenKind::AndAnd);
                } else {
                    return Err(self.lex_error("unexpected character '&'; did you mean '&&'?"));
                }
            }
            '|' => {
                if self.match_char('|') {
                    self.add_token(TokenKind::OrOr);
                } else {
                    self.add_token(TokenKind::Pipe);
                }
            }

            // ----- slash / comments -----
            '/' => {
                if self.match_char('/') {
                    return Err(self.lex_error("comments are not allowed in Oneway — if the code needs explaining, refactor it"));
                } else {
                    self.add_token(TokenKind::Slash);
                }
            }

            // ----- string literals -----
            '"' => self.scan_string()?,

            // ----- number literals -----
            c if c.is_ascii_digit() => self.scan_number(),

            // ----- identifiers / keywords -----
            c if c.is_alphabetic() || c == '_' => self.scan_identifier(),

            _ => {
                return Err(self.lex_error(format!("unexpected character '{}'", c)));
            }
        }

        Ok(())
    }

    // ── string literals ─────────────────────────────────────────────────

    fn scan_string(&mut self) -> Result<()> {
        let mut value = String::new();

        while !self.is_at_end() && self.peek() != '"' {
            let c = self.peek();

            if c == '\n' {
                // Allow multiline strings — just keep tracking lines.
                value.push('\n');
                self.advance();
                continue;
            }

            if c == '\\' {
                // Escape sequence
                self.advance(); // consume the backslash
                if self.is_at_end() {
                    return Err(self.lex_error("unterminated escape sequence in string"));
                }
                let escaped = self.advance();
                match escaped {
                    '\\' => value.push('\\'),
                    '"' => value.push('"'),
                    'n' => value.push('\n'),
                    't' => value.push('\t'),
                    '{' => value.push('{'),
                    '}' => value.push('}'),
                    _ => {
                        return Err(
                            self.lex_error(format!("unknown escape sequence '\\{}'", escaped))
                        );
                    }
                }
            } else {
                value.push(c);
                self.advance();
            }
        }

        if self.is_at_end() {
            return Err(self.lex_error("unterminated string literal"));
        }

        // Consume the closing quote.
        self.advance();

        self.add_token(TokenKind::StringLiteral(value));
        Ok(())
    }

    // ── number literals ─────────────────────────────────────────────────

    fn scan_number(&mut self) {
        // Consume the integer part (the first digit was already consumed by
        // `advance()` in `scan_token`).
        while !self.is_at_end() && self.peek().is_ascii_digit() {
            self.advance();
        }

        // Look for a fractional part.
        let is_float = if self.peek() == '.' && self.peek_next().is_ascii_digit() {
            self.advance(); // consume '.'
            while !self.is_at_end() && self.peek().is_ascii_digit() {
                self.advance();
            }
            true
        } else {
            false
        };

        let lexeme = self.current_lexeme();

        if is_float {
            let value: f64 = lexeme.parse().expect("invalid float literal after lexing");
            self.add_token(TokenKind::FloatLiteral(value));
        } else {
            let value: i64 = lexeme.parse().expect("invalid int literal after lexing");
            self.add_token(TokenKind::IntLiteral(value));
        }
    }

    // ── identifiers / keywords ──────────────────────────────────────────

    fn scan_identifier(&mut self) {
        while !self.is_at_end() && (self.peek().is_alphanumeric() || self.peek() == '_') {
            self.advance();
        }

        let text = self.current_lexeme();

        let kind = TokenKind::keyword(&text).unwrap_or_else(|| TokenKind::Identifier(text));

        self.add_token(kind);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: lex the source and return just the token kinds.
    fn kinds(source: &str) -> Vec<TokenKind> {
        let mut s = Scanner::new(source);
        s.scan_tokens()
            .expect("unexpected lex error")
            .into_iter()
            .map(|t| t.kind)
            .collect()
    }

    #[test]
    fn empty_source() {
        assert_eq!(kinds(""), vec![TokenKind::Eof]);
    }

    #[test]
    fn single_char_tokens() {
        let k = kinds("+ - * / % . ( ) { } [ ] : , ?");
        // whitespace is skipped; no newlines in the input
        assert_eq!(
            k,
            vec![
                TokenKind::Plus,
                TokenKind::Minus,
                TokenKind::Star,
                TokenKind::Slash,
                TokenKind::Percent,
                TokenKind::Dot,
                TokenKind::LeftParen,
                TokenKind::RightParen,
                TokenKind::LeftBrace,
                TokenKind::RightBrace,
                TokenKind::LeftBracket,
                TokenKind::RightBracket,
                TokenKind::Colon,
                TokenKind::Comma,
                TokenKind::QuestionMark,
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn two_char_tokens() {
        let k = kinds("== != <= >= && || -> =>");
        assert_eq!(
            k,
            vec![
                TokenKind::EqualEqual,
                TokenKind::BangEqual,
                TokenKind::LessEqual,
                TokenKind::GreaterEqual,
                TokenKind::AndAnd,
                TokenKind::OrOr,
                TokenKind::Arrow,
                TokenKind::FatArrow,
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn single_vs_double_char() {
        let k = kinds("= ! < >");
        assert_eq!(
            k,
            vec![
                TokenKind::Equal,
                TokenKind::Bang,
                TokenKind::Less,
                TokenKind::Greater,
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn keywords() {
        let k = kinds("contract enum fn match mut pub struct use Self true false");
        assert_eq!(
            k,
            vec![
                TokenKind::Contract,
                TokenKind::Enum,
                TokenKind::Fn,
                TokenKind::Match,
                TokenKind::Mut,
                TokenKind::Pub,
                TokenKind::Struct,
                TokenKind::Use,
                TokenKind::SelfType,
                TokenKind::BoolLiteral(true),
                TokenKind::BoolLiteral(false),
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn identifiers() {
        let k = kinds("foo _bar baz123");
        assert_eq!(
            k,
            vec![
                TokenKind::Identifier("foo".to_string()),
                TokenKind::Identifier("_bar".to_string()),
                TokenKind::Identifier("baz123".to_string()),
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn integer_literals() {
        let k = kinds("0 42 1000");
        assert_eq!(
            k,
            vec![
                TokenKind::IntLiteral(0),
                TokenKind::IntLiteral(42),
                TokenKind::IntLiteral(1000),
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn float_literals() {
        let k = kinds("3.14 0.5");
        assert_eq!(
            k,
            vec![
                TokenKind::FloatLiteral(3.14),
                TokenKind::FloatLiteral(0.5),
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn string_literal_simple() {
        let k = kinds(r#""hello""#);
        assert_eq!(
            k,
            vec![
                TokenKind::StringLiteral("hello".to_string()),
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn string_literal_escapes() {
        let k = kinds(r#""hello\nworld\t\"end\\""#);
        assert_eq!(
            k,
            vec![
                TokenKind::StringLiteral("hello\nworld\t\"end\\".to_string()),
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn string_unterminated() {
        let mut s = Scanner::new(r#""oops"#);
        assert!(s.scan_tokens().is_err());
    }

    #[test]
    fn comments_rejected() {
        let mut scanner = Scanner::new("// this is a comment");
        assert!(scanner.scan_tokens().is_err());
    }

    #[test]
    fn consecutive_newlines_coalesced() {
        let k = kinds("foo\n\n\nbar");
        assert_eq!(
            k,
            vec![
                TokenKind::Identifier("foo".to_string()),
                TokenKind::Newline,
                TokenKind::Identifier("bar".to_string()),
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn leading_newlines_suppressed() {
        let k = kinds("\n\nfoo");
        assert_eq!(
            k,
            vec![TokenKind::Identifier("foo".to_string()), TokenKind::Eof,]
        );
    }

    #[test]
    fn unexpected_character() {
        let mut s = Scanner::new("@");
        assert!(s.scan_tokens().is_err());
    }

    #[test]
    fn span_tracking() {
        let mut s = Scanner::new("ab\ncd");
        let tokens = s.scan_tokens().unwrap();
        // "ab" should be at line 1, column 1
        assert_eq!(tokens[0].span.line, 1);
        assert_eq!(tokens[0].span.column, 1);
        // Newline at line 1, column 3
        assert_eq!(tokens[1].span.line, 1);
        assert_eq!(tokens[1].span.column, 3);
        // "cd" should be at line 2, column 1
        assert_eq!(tokens[2].span.line, 2);
        assert_eq!(tokens[2].span.column, 1);
    }

    #[test]
    fn method_chain() {
        let k = kinds("foo.bar().baz");
        assert_eq!(
            k,
            vec![
                TokenKind::Identifier("foo".to_string()),
                TokenKind::Dot,
                TokenKind::Identifier("bar".to_string()),
                TokenKind::LeftParen,
                TokenKind::RightParen,
                TokenKind::Dot,
                TokenKind::Identifier("baz".to_string()),
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn arrow_vs_minus() {
        let k = kinds("a - b -> c");
        assert_eq!(
            k,
            vec![
                TokenKind::Identifier("a".to_string()),
                TokenKind::Minus,
                TokenKind::Identifier("b".to_string()),
                TokenKind::Arrow,
                TokenKind::Identifier("c".to_string()),
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn fat_arrow_vs_equal() {
        let k = kinds("a = b => c == d");
        assert_eq!(
            k,
            vec![
                TokenKind::Identifier("a".to_string()),
                TokenKind::Equal,
                TokenKind::Identifier("b".to_string()),
                TokenKind::FatArrow,
                TokenKind::Identifier("c".to_string()),
                TokenKind::EqualEqual,
                TokenKind::Identifier("d".to_string()),
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn type_keyword() {
        let k = kinds("type TaskId = Int");
        assert_eq!(
            k,
            vec![
                TokenKind::TypeKeyword,
                TokenKind::Identifier("TaskId".into()),
                TokenKind::Equal,
                TokenKind::Identifier("Int".into()),
                TokenKind::Eof,
            ]
        );
    }
}
