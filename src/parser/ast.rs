use crate::error::Span;

// ---------------------------------------------------------------------------
// Module (top-level compilation unit)
// ---------------------------------------------------------------------------

/// A complete source file / module – the root of every AST.
#[derive(Debug, Clone)]
pub struct Module {
    pub items: Vec<Item>,
}

// ---------------------------------------------------------------------------
// Top-level items
// ---------------------------------------------------------------------------

/// A top-level declaration inside a module.
#[derive(Debug, Clone)]
pub enum Item {
    Use(UseItem),
    Struct(StructDef),
    Enum(EnumDef),
    Contract(ContractDef),
    Function(FunctionDef),
    Newtype(NewtypeDef),
}

/// A `use` import: `use io` or `use net.http`.
#[derive(Debug, Clone)]
pub struct UseItem {
    /// The dot-separated path segments, e.g. `["net", "http"]`.
    pub path: Vec<String>,
    pub span: Span,
}

// ---------------------------------------------------------------------------
// Struct
// ---------------------------------------------------------------------------

/// A struct definition: `pub struct Person { name: String, age: Int }`.
#[derive(Debug, Clone)]
pub struct StructDef {
    pub public: bool,
    pub name: String,
    pub fields: Vec<Field>,
    pub delegates: Vec<TypeExpr>,
    pub span: Span,
}

/// A single field inside a struct definition.
#[derive(Debug, Clone)]
pub struct Field {
    pub type_expr: TypeExpr,
    pub span: Span,
}

// ---------------------------------------------------------------------------
// Enum
// ---------------------------------------------------------------------------

/// An enum definition: `pub enum Color { Red, Green, Blue }`.
#[derive(Debug, Clone)]
pub struct EnumDef {
    pub public: bool,
    pub name: String,
    pub variants: Vec<Variant>,
    pub span: Span,
}

/// A single variant inside an enum definition.
/// The optional `data` field holds the associated data type, e.g. `Circle(Float)`.
#[derive(Debug, Clone)]
pub struct Variant {
    pub name: String,
    pub data: Option<TypeExpr>,
    pub span: Span,
}

// ---------------------------------------------------------------------------
// Contract (interface / trait)
// ---------------------------------------------------------------------------

/// A contract definition: `pub contract Printable { fn to_string(Self) -> String }`.
#[derive(Debug, Clone)]
pub struct ContractDef {
    pub public: bool,
    pub name: String,
    pub functions: Vec<ContractFunction>,
    pub span: Span,
}

/// A function signature declared inside a contract (no body).
#[derive(Debug, Clone)]
pub struct ContractFunction {
    pub name: String,
    pub params: Vec<TypeExpr>,
    pub return_type: Option<TypeExpr>,
    pub span: Span,
}

// ---------------------------------------------------------------------------
// Function
// ---------------------------------------------------------------------------

/// A top-level function definition.
#[derive(Debug, Clone)]
pub struct FunctionDef {
    pub public: bool,
    pub name: String,
    pub params: Vec<Param>,
    pub return_type: Option<TypeExpr>,
    pub body: Expr,
    pub span: Span,
}

/// A function parameter.
#[derive(Debug, Clone)]
pub struct Param {
    pub type_expr: TypeExpr,
    pub span: Span,
}

/// A newtype definition: `type TaskId = Int`.
#[derive(Debug, Clone)]
pub struct NewtypeDef {
    pub public: bool,
    pub name: String,
    pub inner_type: TypeExpr,
    pub span: Span,
}

// ---------------------------------------------------------------------------
// Type expressions
// ---------------------------------------------------------------------------

/// A type annotation that appears in signatures, fields, etc.
#[derive(Debug, Clone)]
pub enum TypeExpr {
    /// Simple named type: `Int`, `String`, `Person`.
    Named(String),

    /// Generic type application: `List[Int]`, `Map[String, Int]`.
    Generic { name: String, params: Vec<TypeExpr> },

    /// Function type: `fn(Int) -> String`.
    Function {
        params: Vec<Box<TypeExpr>>,
        return_type: Box<TypeExpr>,
    },

    /// Union type: `ErrorA | ErrorB | ErrorC`.
    Union(Vec<TypeExpr>),
}

// ---------------------------------------------------------------------------
// Expressions
// ---------------------------------------------------------------------------

/// An expression – the core building block of Oneway programs.
#[derive(Debug, Clone)]
pub enum Expr {
    /// Integer literal: `42`.
    IntLiteral(i64),

    /// Float literal: `3.14`.
    FloatLiteral(f64),

    /// String literal: `"hello"`.
    StringLiteral(String),

    /// Bool literal: `true`, `false`.
    BoolLiteral(bool),

    /// Variable / identifier reference: `foo`, `person`.
    Identifier(String),

    /// String interpolation: `"Hello, {name}!"`.
    StringInterpolation(Vec<StringPart>),

    /// Binary operation: `a + b`.
    BinaryOp {
        left: Box<Expr>,
        op: BinOp,
        right: Box<Expr>,
    },

    /// Unary operation: `!x`, `-x`.
    UnaryOp { op: UnOp, operand: Box<Expr> },

    /// Dot access / UFCS: `expr.field` or `expr.method()`.
    DotAccess { object: Box<Expr>, field: String },

    /// Function call: `f(arg)` or `f()`.
    Call {
        function: Box<Expr>,
        argument: Option<Box<Expr>>,
    },

    /// Struct literal: `Person { 30, "Alice" }`.
    StructLiteral {
        type_name: String,
        fields: Vec<Expr>,
    },

    /// Match expression.
    Match {
        subject: Option<Box<Expr>>,
        arms: Vec<MatchArm>,
    },

    /// Binding: `name = expr`.
    Binding { name: String, value: Box<Expr> },

    /// Block of expressions: `{ expr1 \n expr2 }`.
    Block(Vec<Expr>),

    /// Error propagation: `expr?`.
    Try(Box<Expr>),
}

// ---------------------------------------------------------------------------
// Match
// ---------------------------------------------------------------------------

/// A single arm in a `match` expression.
#[derive(Debug, Clone)]
pub struct MatchArm {
    pub pattern: Pattern,
    pub guard: Option<Box<Expr>>,
    pub body: Expr,
}

/// A pattern used on the left-hand side of a match arm.
#[derive(Debug, Clone)]
pub enum Pattern {
    /// Wildcard pattern: `_`.
    Wildcard,

    /// Literal value: `0`, `1`, `"hello"`, `true`.
    Literal(Box<Expr>),

    /// Binding pattern: captures the matched value into a name.
    Binding(String),

    /// Struct destructuring: `Person { pat, pat }`.
    Struct {
        type_name: String,
        fields: Vec<Pattern>,
    },

    /// Enum variant pattern: `Color.Red` or `Shape.Circle(r)`.
    EnumVariant {
        type_name: Option<String>,
        variant: String,
        data: Option<Box<Pattern>>,
    },
}

// ---------------------------------------------------------------------------
// Operators
// ---------------------------------------------------------------------------

/// Binary operators.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BinOp {
    Add,          // +
    Subtract,     // -
    Multiply,     // *
    Divide,       // /
    Modulo,       // %
    Equal,        // ==
    NotEqual,     // !=
    Less,         // <
    Greater,      // >
    LessEqual,    // <=
    GreaterEqual, // >=
    And,          // &&
    Or,           // ||
}

/// Unary operators.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum UnOp {
    Negate, // -
    Not,    // !
}

// ---------------------------------------------------------------------------
// String interpolation helpers
// ---------------------------------------------------------------------------

/// A fragment inside a string interpolation expression.
#[derive(Debug, Clone)]
pub enum StringPart {
    /// A literal text segment.
    Literal(String),
    /// An interpolated expression: `{expr}`.
    Expr(Box<Expr>),
}
