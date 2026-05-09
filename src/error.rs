use std::fmt;

/// Represents a location span in source code.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Span {
    pub start: usize,
    pub end: usize,
    pub line: u32,
    pub column: u32,
}

impl Span {
    pub fn new(start: usize, end: usize, line: u32, column: u32) -> Self {
        Self {
            start,
            end,
            line,
            column,
        }
    }
}

/// The set of all error kinds produced by the Oneway compiler.
#[derive(Debug, Clone)]
pub enum OnewayError {
    /// An error produced during lexical analysis.
    LexError { message: String, span: Span },
    /// An error produced during parsing.
    ParseError { message: String, span: Span },
    /// An error produced during type/sort checking.
    CheckError { message: String, span: Span },
    /// An error produced during code generation.
    CodegenError { message: String, span: Span },
}

impl OnewayError {
    /// Returns a reference to the span associated with this error.
    pub fn span(&self) -> &Span {
        match self {
            OnewayError::LexError { span, .. } => span,
            OnewayError::ParseError { span, .. } => span,
            OnewayError::CheckError { span, .. } => span,
            OnewayError::CodegenError { span, .. } => span,
        }
    }

    /// Returns a reference to the message associated with this error.
    pub fn message(&self) -> &str {
        match self {
            OnewayError::LexError { message, .. } => message,
            OnewayError::ParseError { message, .. } => message,
            OnewayError::CheckError { message, .. } => message,
            OnewayError::CodegenError { message, .. } => message,
        }
    }

    /// Returns the name of the compiler phase that produced this error.
    fn phase(&self) -> &'static str {
        match self {
            OnewayError::LexError { .. } => "lex error",
            OnewayError::ParseError { .. } => "parse error",
            OnewayError::CheckError { .. } => "check error",
            OnewayError::CodegenError { .. } => "codegen error",
        }
    }
}

impl fmt::Display for OnewayError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let span = self.span();
        write!(
            f,
            "{} at {}:{}: {}",
            self.phase(),
            span.line,
            span.column,
            self.message()
        )
    }
}

impl std::error::Error for OnewayError {}

/// A convenience `Result` type that uses `OnewayError` as the error variant.
pub type Result<T> = std::result::Result<T, OnewayError>;
