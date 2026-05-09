use crate::error::Span;
use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    // Literals
    IntLiteral(i64),
    FloatLiteral(f64),
    StringLiteral(String),
    BoolLiteral(bool),

    // Identifier (covers both variable names and type names)
    Identifier(String),

    // Keywords
    Contract,    // contract
    Enum,        // enum
    Fn,          // fn
    Match,       // match
    Mut,         // mut
    Pub,         // pub
    Struct,      // struct
    Use,         // use
    TypeKeyword, // type
    SelfType,    // Self
    Delegates,   // delegates

    // Operators
    Plus,         // +
    Minus,        // -
    Star,         // *
    Slash,        // /
    Percent,      // %
    EqualEqual,   // ==
    BangEqual,    // !=
    Less,         // <
    Greater,      // >
    LessEqual,    // <=
    GreaterEqual, // >=
    AndAnd,       // &&
    OrOr,         // ||
    Pipe,         // |
    Bang,         // !
    Equal,        // =
    Arrow,        // ->
    FatArrow,     // =>
    Dot,          // .
    QuestionMark, // ?

    // Delimiters
    LeftParen,    // (
    RightParen,   // )
    LeftBrace,    // {
    RightBrace,   // }
    LeftBracket,  // [
    RightBracket, // ]
    Colon,        // :
    Comma,        // ,

    // Special
    Newline,
    Eof,
}

#[derive(Debug, Clone)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
    pub lexeme: String,
}

impl fmt::Display for TokenKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TokenKind::IntLiteral(v) => write!(f, "Int({})", v),
            TokenKind::FloatLiteral(v) => write!(f, "Float({})", v),
            TokenKind::StringLiteral(v) => write!(f, "String(\"{}\")", v),
            TokenKind::BoolLiteral(v) => write!(f, "Bool({})", v),
            TokenKind::Identifier(v) => write!(f, "Identifier({})", v),
            TokenKind::Contract => write!(f, "contract"),
            TokenKind::Enum => write!(f, "enum"),
            TokenKind::Fn => write!(f, "fn"),
            TokenKind::Match => write!(f, "match"),
            TokenKind::Mut => write!(f, "mut"),
            TokenKind::Pub => write!(f, "pub"),
            TokenKind::Struct => write!(f, "struct"),
            TokenKind::Use => write!(f, "use"),
            TokenKind::TypeKeyword => write!(f, "type"),
            TokenKind::SelfType => write!(f, "Self"),
            TokenKind::Delegates => write!(f, "delegates"),
            TokenKind::Plus => write!(f, "+"),
            TokenKind::Minus => write!(f, "-"),
            TokenKind::Star => write!(f, "*"),
            TokenKind::Slash => write!(f, "/"),
            TokenKind::Percent => write!(f, "%"),
            TokenKind::EqualEqual => write!(f, "=="),
            TokenKind::BangEqual => write!(f, "!="),
            TokenKind::Less => write!(f, "<"),
            TokenKind::Greater => write!(f, ">"),
            TokenKind::LessEqual => write!(f, "<="),
            TokenKind::GreaterEqual => write!(f, ">="),
            TokenKind::AndAnd => write!(f, "&&"),
            TokenKind::OrOr => write!(f, "||"),
            TokenKind::Pipe => write!(f, "|"),
            TokenKind::Bang => write!(f, "!"),
            TokenKind::Equal => write!(f, "="),
            TokenKind::Arrow => write!(f, "->"),
            TokenKind::FatArrow => write!(f, "=>"),
            TokenKind::Dot => write!(f, "."),
            TokenKind::QuestionMark => write!(f, "?"),
            TokenKind::LeftParen => write!(f, "("),
            TokenKind::RightParen => write!(f, ")"),
            TokenKind::LeftBrace => write!(f, "{{"),
            TokenKind::RightBrace => write!(f, "}}"),
            TokenKind::LeftBracket => write!(f, "["),
            TokenKind::RightBracket => write!(f, "]"),
            TokenKind::Colon => write!(f, ":"),
            TokenKind::Comma => write!(f, ","),
            TokenKind::Newline => write!(f, "Newline"),
            TokenKind::Eof => write!(f, "Eof"),
        }
    }
}

impl TokenKind {
    /// Maps a keyword string to its corresponding `TokenKind`, or returns `None`
    /// if the string is not a recognized keyword.
    pub fn keyword(s: &str) -> Option<TokenKind> {
        match s {
            "contract" => Some(TokenKind::Contract),
            "enum" => Some(TokenKind::Enum),
            "fn" => Some(TokenKind::Fn),
            "match" => Some(TokenKind::Match),
            "mut" => Some(TokenKind::Mut),
            "pub" => Some(TokenKind::Pub),
            "struct" => Some(TokenKind::Struct),
            "use" => Some(TokenKind::Use),
            "Self" => Some(TokenKind::SelfType),
            "delegates" => Some(TokenKind::Delegates),
            "type" => Some(TokenKind::TypeKeyword),
            "true" => Some(TokenKind::BoolLiteral(true)),
            "false" => Some(TokenKind::BoolLiteral(false)),
            _ => None,
        }
    }
}
