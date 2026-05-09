use crate::error::{OnewayError, Span};
use crate::parser::ast::*;

/// Check an Oneway module for sort-order violations.
/// Returns a (possibly empty) list of errors.
pub fn check(module: &Module) -> Vec<OnewayError> {
    let mut errors = Vec::new();
    check_module(module, &mut errors);
    errors
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn check_error(message: impl Into<String>, span: Span) -> OnewayError {
    OnewayError::CheckError {
        message: message.into(),
        span,
    }
}

/// Returns the "name" used for sorting an item.
fn item_name(item: &Item) -> &str {
    match item {
        Item::Use(u) => {
            // For use items we compare by joined path, but this helper returns
            // a single &str. We handle use-path sorting separately; here we
            // return the first segment as a fallback (only used for category
            // ordering error messages).
            u.path.first().map(|s| s.as_str()).unwrap_or("")
        }
        Item::Struct(s) => &s.name,
        Item::Enum(e) => &e.name,
        Item::Contract(c) => &c.name,
        Item::Function(f) => &f.name,
        Item::Newtype(n) => &n.name,
    }
}

/// Returns the sort-category for a top-level item.
///   0 = use imports
///   1 = type definitions (Contract, Enum, Newtype, Struct)
///   2 = function definitions
fn item_category(item: &Item) -> u8 {
    match item {
        Item::Use(_) => 0,
        Item::Contract(_) | Item::Enum(_) | Item::Newtype(_) | Item::Struct(_) => 1,
        Item::Function(_) => 2,
    }
}

fn item_span(item: &Item) -> Span {
    match item {
        Item::Use(u) => u.span,
        Item::Struct(s) => s.span,
        Item::Enum(e) => e.span,
        Item::Contract(c) => c.span,
        Item::Function(f) => f.span,
        Item::Newtype(n) => n.span,
    }
}

fn category_label(cat: u8) -> &'static str {
    match cat {
        0 => "use import",
        1 => "type definition",
        2 => "function",
        _ => "item",
    }
}

/// Build the dot-joined path string used for sorting `use` items.
fn use_sort_key(u: &UseItem) -> String {
    u.path.join(".")
}

/// Extract a sort key from a `TypeExpr` (used for delegate sorting).
fn type_expr_sort_key(te: &TypeExpr) -> String {
    match te {
        TypeExpr::Named(n) => n.clone(),
        TypeExpr::Generic { name, .. } => name.clone(),
        TypeExpr::Function { .. } => "<fn>".to_string(),
        TypeExpr::Union(_) => "<union>".to_string(),
    }
}

/// Convert a pattern to a string key for sort comparison.
/// Wildcards are handled specially (they always sort last).
fn pattern_sort_key(pattern: &Pattern) -> String {
    match pattern {
        // Wildcard is handled by the caller — we give it a marker that sorts
        // after everything in practice, but the caller checks explicitly.
        Pattern::Wildcard => "\u{FFFF}".to_string(),
        Pattern::Literal(expr) => match expr.as_ref() {
            Expr::IntLiteral(n) => n.to_string(),
            Expr::FloatLiteral(f) => f.to_string(),
            Expr::StringLiteral(s) => format!("\"{}\"", s),
            Expr::BoolLiteral(b) => b.to_string(),
            _ => "<expr>".to_string(),
        },
        Pattern::Binding(name) => name.clone(),
        Pattern::EnumVariant {
            type_name: Some(t),
            variant,
            ..
        } => format!("{}.{}", t, variant),
        Pattern::EnumVariant {
            type_name: None,
            variant,
            ..
        } => variant.clone(),
        Pattern::Struct { type_name, .. } => type_name.clone(),
    }
}

fn is_wildcard(pattern: &Pattern) -> bool {
    matches!(pattern, Pattern::Wildcard)
}

/// Human-readable representation of a pattern for error messages.
fn pattern_display(pattern: &Pattern) -> String {
    match pattern {
        Pattern::Wildcard => "_".to_string(),
        _ => pattern_sort_key(pattern),
    }
}

// ---------------------------------------------------------------------------
// Module-level checking
// ---------------------------------------------------------------------------

fn check_module(module: &Module, errors: &mut Vec<OnewayError>) {
    check_item_ordering(&module.items, errors);

    // Check internals of each item.
    for item in &module.items {
        match item {
            Item::Struct(s) => check_struct(s, errors),
            Item::Enum(e) => check_enum(e, errors),
            Item::Contract(c) => check_contract(c, errors),
            Item::Function(f) => check_function(f, errors),
            Item::Use(_) | Item::Newtype(_) => {}
        }
    }
}

fn check_item_ordering(items: &[Item], errors: &mut Vec<OnewayError>) {
    if items.is_empty() {
        return;
    }

    // 1. Check that categories never go backwards.
    for i in 1..items.len() {
        let prev_cat = item_category(&items[i - 1]);
        let curr_cat = item_category(&items[i]);
        if curr_cat < prev_cat {
            let prev_label = category_label(prev_cat);
            let curr_label = category_label(curr_cat);
            let curr_name = item_name(&items[i]);
            let prev_name = item_name(&items[i - 1]);
            errors.push(check_error(
                format!(
                    "item '{}' ({}) must come before '{}' ({}) — expected order: use imports, then types, then functions",
                    curr_name, curr_label, prev_name, prev_label
                ),
                item_span(&items[i]),
            ));
        }
    }

    // 2. Within each category, check alphabetical ordering.
    // Use items: sorted by their joined path.
    check_use_ordering(items, errors);
    // Type definitions: sorted by name.
    check_within_category(items, 1, "type definitions", errors);
    // Function definitions: sorted by name.
    check_within_category(items, 2, "function definitions", errors);
}

fn check_use_ordering(items: &[Item], errors: &mut Vec<OnewayError>) {
    let uses: Vec<&UseItem> = items
        .iter()
        .filter_map(|i| match i {
            Item::Use(u) => Some(u),
            _ => None,
        })
        .collect();

    for i in 1..uses.len() {
        let prev_key = use_sort_key(uses[i - 1]);
        let curr_key = use_sort_key(uses[i]);
        if curr_key < prev_key {
            errors.push(check_error(
                format!(
                    "use imports not sorted: '{}' must come after '{}'",
                    curr_key, prev_key
                ),
                uses[i].span,
            ));
        }
    }
}

fn check_within_category(
    items: &[Item],
    category: u8,
    context: &str,
    errors: &mut Vec<OnewayError>,
) {
    let in_cat: Vec<&Item> = items
        .iter()
        .filter(|i| item_category(i) == category)
        .collect();

    for i in 1..in_cat.len() {
        let prev_name = item_name(in_cat[i - 1]);
        let curr_name = item_name(in_cat[i]);
        if curr_name < prev_name {
            errors.push(check_error(
                format!(
                    "{} not sorted: '{}' must come after '{}'",
                    context, curr_name, prev_name
                ),
                item_span(in_cat[i]),
            ));
        }
    }
}

// ---------------------------------------------------------------------------
// Struct checking
// ---------------------------------------------------------------------------

fn check_struct(s: &StructDef, errors: &mut Vec<OnewayError>) {
    // Check delegates are sorted by type name.
    for i in 1..s.delegates.len() {
        let prev_key = type_expr_sort_key(&s.delegates[i - 1]);
        let curr_key = type_expr_sort_key(&s.delegates[i]);
        if curr_key < prev_key {
            errors.push(check_error(
                format!(
                    "struct delegates in '{}' not sorted: '{}' must come after '{}'",
                    s.name, curr_key, prev_key
                ),
                s.span,
            ));
        }
    }

    // Check fields are sorted by type name.
    for i in 1..s.fields.len() {
        let prev_key = type_expr_sort_key(&s.fields[i - 1].type_expr);
        let curr_key = type_expr_sort_key(&s.fields[i].type_expr);
        if curr_key < prev_key {
            errors.push(check_error(
                format!(
                    "struct '{}' fields not sorted: type '{}' must come after '{}'",
                    s.name, curr_key, prev_key
                ),
                s.fields[i].span,
            ));
        }
    }
}

// ---------------------------------------------------------------------------
// Enum checking
// ---------------------------------------------------------------------------

fn check_enum(e: &EnumDef, errors: &mut Vec<OnewayError>) {
    for i in 1..e.variants.len() {
        let prev_name = &e.variants[i - 1].name;
        let curr_name = &e.variants[i].name;
        if curr_name < prev_name {
            errors.push(check_error(
                format!(
                    "enum variants in '{}' not sorted: '{}' must come after '{}'",
                    e.name, curr_name, prev_name
                ),
                e.variants[i].span,
            ));
        }
    }
}

// ---------------------------------------------------------------------------
// Contract checking
// ---------------------------------------------------------------------------

fn check_contract(c: &ContractDef, errors: &mut Vec<OnewayError>) {
    for i in 1..c.functions.len() {
        let prev_name = &c.functions[i - 1].name;
        let curr_name = &c.functions[i].name;
        if curr_name < prev_name {
            errors.push(check_error(
                format!(
                    "contract functions in '{}' not sorted: '{}' must come after '{}'",
                    c.name, curr_name, prev_name
                ),
                c.functions[i].span,
            ));
        }
    }
}

// ---------------------------------------------------------------------------
// Function checking (recurse into body)
// ---------------------------------------------------------------------------

fn check_function(f: &FunctionDef, errors: &mut Vec<OnewayError>) {
    check_expr(&f.body, errors);
}

// ---------------------------------------------------------------------------
// Match arm checking
// ---------------------------------------------------------------------------

fn check_match_arms(arms: &[MatchArm], errors: &mut Vec<OnewayError>) {
    for i in 1..arms.len() {
        let prev = &arms[i - 1];
        let curr = &arms[i];

        let prev_is_wildcard = is_wildcard(&prev.pattern);
        let curr_is_wildcard = is_wildcard(&curr.pattern);

        if prev_is_wildcard && !curr_is_wildcard {
            // Wildcard appeared before a non-wildcard — error.
            errors.push(check_error(
                format!(
                    "match arms not sorted: '{}' must come before '_' (wildcard must be last)",
                    pattern_display(&curr.pattern)
                ),
                Span::default(),
            ));
        } else if !prev_is_wildcard && !curr_is_wildcard {
            let prev_key = pattern_sort_key(&prev.pattern);
            let curr_key = pattern_sort_key(&curr.pattern);
            if curr_key < prev_key {
                errors.push(check_error(
                    format!(
                        "match arms not sorted: '{}' must come after '{}'",
                        curr_key, prev_key
                    ),
                    Span::default(),
                ));
            }
        }
        // Both wildcards or curr is wildcard and prev isn't → fine.
    }

    // Also recurse into guard and body expressions of each arm.
    for arm in arms {
        if let Some(guard) = &arm.guard {
            check_expr(guard, errors);
        }
        check_expr(&arm.body, errors);
    }
}

// ---------------------------------------------------------------------------
// Recursive expression walker
// ---------------------------------------------------------------------------

fn check_expr(expr: &Expr, errors: &mut Vec<OnewayError>) {
    match expr {
        Expr::Match { arms, .. } => {
            check_match_arms(arms, errors);
            // Also check the subject expression if present.
            if let Expr::Match {
                subject: Some(subj),
                ..
            } = expr
            {
                check_expr(subj, errors);
            }
        }
        Expr::BinaryOp { left, right, .. } => {
            check_expr(left, errors);
            check_expr(right, errors);
        }
        Expr::UnaryOp { operand, .. } => {
            check_expr(operand, errors);
        }
        Expr::DotAccess { object, .. } => {
            check_expr(object, errors);
        }
        Expr::Call {
            function, argument, ..
        } => {
            check_expr(function, errors);
            if let Some(arg) = argument {
                check_expr(arg, errors);
            }
        }
        Expr::StructLiteral { fields, .. } => {
            for fv in fields {
                check_expr(fv, errors);
            }
        }
        Expr::Binding { value, .. } => {
            check_expr(value, errors);
        }
        Expr::Block(exprs) => {
            for e in exprs {
                check_expr(e, errors);
            }
        }
        Expr::Try(inner) => {
            check_expr(inner, errors);
        }
        Expr::StringInterpolation(parts) => {
            for part in parts {
                if let StringPart::Expr(e) = part {
                    check_expr(e, errors);
                }
            }
        }
        // Leaf expressions — nothing to recurse into.
        Expr::IntLiteral(_)
        | Expr::FloatLiteral(_)
        | Expr::StringLiteral(_)
        | Expr::BoolLiteral(_)
        | Expr::Identifier(_) => {}
    }
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::Span;

    fn s() -> Span {
        Span::default()
    }

    fn use_item(path: &[&str]) -> Item {
        Item::Use(UseItem {
            path: path.iter().map(|s| s.to_string()).collect(),
            span: s(),
        })
    }

    fn struct_item(name: &str, field_types: &[&str]) -> Item {
        Item::Struct(StructDef {
            public: false,
            name: name.to_string(),
            fields: field_types
                .iter()
                .map(|t| Field {
                    type_expr: TypeExpr::Named(t.to_string()),
                    span: s(),
                })
                .collect(),
            delegates: vec![],
            span: s(),
        })
    }

    fn enum_item(name: &str, variants: &[&str]) -> Item {
        Item::Enum(EnumDef {
            public: false,
            name: name.to_string(),
            variants: variants
                .iter()
                .map(|v| Variant {
                    name: v.to_string(),
                    data: None,
                    span: s(),
                })
                .collect(),
            span: s(),
        })
    }

    fn fn_item(name: &str) -> Item {
        Item::Function(FunctionDef {
            public: false,
            name: name.to_string(),
            params: vec![],
            return_type: None,
            body: Expr::Block(vec![]),
            span: s(),
        })
    }

    fn newtype_item(name: &str) -> Item {
        Item::Newtype(NewtypeDef {
            public: false,
            name: name.to_string(),
            inner_type: TypeExpr::Named("Int".into()),
            span: s(),
        })
    }

    fn contract_item(name: &str, fns: &[&str]) -> Item {
        Item::Contract(ContractDef {
            public: false,
            name: name.to_string(),
            functions: fns
                .iter()
                .map(|f| ContractFunction {
                    name: f.to_string(),
                    params: vec![],
                    return_type: None,
                    span: s(),
                })
                .collect(),
            span: s(),
        })
    }

    // -----------------------------------------------------------------------
    // Use ordering
    // -----------------------------------------------------------------------

    #[test]
    fn sorted_uses_ok() {
        let module = Module {
            items: vec![use_item(&["io"]), use_item(&["math"])],
        };
        assert!(check(&module).is_empty());
    }

    #[test]
    fn unsorted_uses_error() {
        let module = Module {
            items: vec![use_item(&["math"]), use_item(&["io"])],
        };
        let errors = check(&module);
        assert_eq!(errors.len(), 1);
    }

    #[test]
    fn use_dotted_path_sorted() {
        let module = Module {
            items: vec![
                use_item(&["io"]),
                use_item(&["math"]),
                use_item(&["net", "http"]),
            ],
        };
        assert!(check(&module).is_empty());
    }

    #[test]
    fn use_dotted_path_unsorted() {
        let module = Module {
            items: vec![use_item(&["net", "http"]), use_item(&["math"])],
        };
        let errors = check(&module);
        assert_eq!(errors.len(), 1);
    }

    // -----------------------------------------------------------------------
    // Struct fields
    // -----------------------------------------------------------------------

    #[test]
    fn sorted_struct_fields_ok() {
        let module = Module {
            items: vec![struct_item("Person", &["Age", "Name"])],
        };
        assert!(check(&module).is_empty());
    }

    #[test]
    fn unsorted_struct_fields_error() {
        let module = Module {
            items: vec![struct_item("Person", &["Name", "Age"])],
        };
        assert_eq!(check(&module).len(), 1);
    }

    // -----------------------------------------------------------------------
    // Enum variants
    // -----------------------------------------------------------------------

    #[test]
    fn sorted_enum_variants_ok() {
        let module = Module {
            items: vec![enum_item("Color", &["Blue", "Green", "Red"])],
        };
        assert!(check(&module).is_empty());
    }

    #[test]
    fn unsorted_enum_variants_error() {
        let module = Module {
            items: vec![enum_item("Color", &["Red", "Blue"])],
        };
        assert_eq!(check(&module).len(), 1);
    }

    // -----------------------------------------------------------------------
    // Contract functions
    // -----------------------------------------------------------------------

    #[test]
    fn sorted_contract_functions_ok() {
        let module = Module {
            items: vec![contract_item("Printable", &["format", "to_string"])],
        };
        assert!(check(&module).is_empty());
    }

    #[test]
    fn unsorted_contract_functions_error() {
        let module = Module {
            items: vec![contract_item("Printable", &["to_string", "format"])],
        };
        assert_eq!(check(&module).len(), 1);
    }

    // -----------------------------------------------------------------------
    // Function ordering
    // -----------------------------------------------------------------------

    #[test]
    fn sorted_functions_ok() {
        let module = Module {
            items: vec![fn_item("add"), fn_item("main")],
        };
        assert!(check(&module).is_empty());
    }

    #[test]
    fn unsorted_functions_error() {
        let module = Module {
            items: vec![fn_item("main"), fn_item("add")],
        };
        assert_eq!(check(&module).len(), 1);
    }

    // -----------------------------------------------------------------------
    // Category ordering
    // -----------------------------------------------------------------------

    #[test]
    fn correct_category_order() {
        let module = Module {
            items: vec![
                use_item(&["io"]),
                struct_item("Person", &["Name"]),
                fn_item("main"),
            ],
        };
        assert!(check(&module).is_empty());
    }

    #[test]
    fn wrong_category_order() {
        let module = Module {
            items: vec![fn_item("main"), struct_item("Person", &["Name"])],
        };
        assert!(!check(&module).is_empty());
    }

    #[test]
    fn use_after_function_error() {
        let module = Module {
            items: vec![fn_item("main"), use_item(&["io"])],
        };
        assert!(!check(&module).is_empty());
    }

    // -----------------------------------------------------------------------
    // Mixed types sorted
    // -----------------------------------------------------------------------

    #[test]
    fn mixed_types_sorted() {
        let module = Module {
            items: vec![
                enum_item("Color", &["Blue", "Red"]),
                newtype_item("TaskId"),
                struct_item("User", &["Name"]),
            ],
        };
        assert!(check(&module).is_empty());
    }

    #[test]
    fn mixed_types_unsorted() {
        let module = Module {
            items: vec![
                struct_item("User", &["Name"]),
                enum_item("Color", &["Blue", "Red"]),
            ],
        };
        assert!(!check(&module).is_empty());
    }

    // -----------------------------------------------------------------------
    // Match arms
    // -----------------------------------------------------------------------

    #[test]
    fn match_arms_sorted() {
        let module = Module {
            items: vec![Item::Function(FunctionDef {
                public: false,
                name: "f".into(),
                params: vec![],
                return_type: None,
                body: Expr::Match {
                    subject: Some(Box::new(Expr::Identifier("x".into()))),
                    arms: vec![
                        MatchArm {
                            pattern: Pattern::Literal(Box::new(Expr::IntLiteral(0))),
                            guard: None,
                            body: Expr::IntLiteral(0),
                        },
                        MatchArm {
                            pattern: Pattern::Literal(Box::new(Expr::IntLiteral(1))),
                            guard: None,
                            body: Expr::IntLiteral(1),
                        },
                        MatchArm {
                            pattern: Pattern::Binding("n".into()),
                            guard: None,
                            body: Expr::IntLiteral(2),
                        },
                        MatchArm {
                            pattern: Pattern::Wildcard,
                            guard: None,
                            body: Expr::IntLiteral(3),
                        },
                    ],
                },
                span: s(),
            })],
        };
        assert!(check(&module).is_empty());
    }

    #[test]
    fn match_arms_unsorted() {
        let module = Module {
            items: vec![Item::Function(FunctionDef {
                public: false,
                name: "f".into(),
                params: vec![],
                return_type: None,
                body: Expr::Match {
                    subject: Some(Box::new(Expr::Identifier("x".into()))),
                    arms: vec![
                        MatchArm {
                            pattern: Pattern::Literal(Box::new(Expr::IntLiteral(1))),
                            guard: None,
                            body: Expr::IntLiteral(1),
                        },
                        MatchArm {
                            pattern: Pattern::Literal(Box::new(Expr::IntLiteral(0))),
                            guard: None,
                            body: Expr::IntLiteral(0),
                        },
                    ],
                },
                span: s(),
            })],
        };
        assert!(!check(&module).is_empty());
    }

    #[test]
    fn wildcard_not_last_error() {
        let module = Module {
            items: vec![Item::Function(FunctionDef {
                public: false,
                name: "f".into(),
                params: vec![],
                return_type: None,
                body: Expr::Match {
                    subject: Some(Box::new(Expr::Identifier("x".into()))),
                    arms: vec![
                        MatchArm {
                            pattern: Pattern::Wildcard,
                            guard: None,
                            body: Expr::IntLiteral(0),
                        },
                        MatchArm {
                            pattern: Pattern::Literal(Box::new(Expr::IntLiteral(1))),
                            guard: None,
                            body: Expr::IntLiteral(1),
                        },
                    ],
                },
                span: s(),
            })],
        };
        assert!(!check(&module).is_empty());
    }

    #[test]
    fn match_enum_variants_sorted() {
        let module = Module {
            items: vec![Item::Function(FunctionDef {
                public: false,
                name: "f".into(),
                params: vec![],
                return_type: None,
                body: Expr::Match {
                    subject: Some(Box::new(Expr::Identifier("x".into()))),
                    arms: vec![
                        MatchArm {
                            pattern: Pattern::EnumVariant {
                                type_name: Some("Color".into()),
                                variant: "Blue".into(),
                                data: None,
                            },
                            guard: None,
                            body: Expr::IntLiteral(0),
                        },
                        MatchArm {
                            pattern: Pattern::EnumVariant {
                                type_name: Some("Color".into()),
                                variant: "Red".into(),
                                data: None,
                            },
                            guard: None,
                            body: Expr::IntLiteral(1),
                        },
                    ],
                },
                span: s(),
            })],
        };
        assert!(check(&module).is_empty());
    }

    #[test]
    fn match_enum_variants_unsorted() {
        let module = Module {
            items: vec![Item::Function(FunctionDef {
                public: false,
                name: "f".into(),
                params: vec![],
                return_type: None,
                body: Expr::Match {
                    subject: Some(Box::new(Expr::Identifier("x".into()))),
                    arms: vec![
                        MatchArm {
                            pattern: Pattern::EnumVariant {
                                type_name: Some("Color".into()),
                                variant: "Red".into(),
                                data: None,
                            },
                            guard: None,
                            body: Expr::IntLiteral(0),
                        },
                        MatchArm {
                            pattern: Pattern::EnumVariant {
                                type_name: Some("Color".into()),
                                variant: "Blue".into(),
                                data: None,
                            },
                            guard: None,
                            body: Expr::IntLiteral(1),
                        },
                    ],
                },
                span: s(),
            })],
        };
        assert!(!check(&module).is_empty());
    }

    // -----------------------------------------------------------------------
    // Nested match (recursive walk)
    // -----------------------------------------------------------------------

    #[test]
    fn nested_match_checked() {
        // A function with a block containing a match — the checker should
        // recurse through the block.
        let inner_match = Expr::Match {
            subject: Some(Box::new(Expr::Identifier("y".into()))),
            arms: vec![
                MatchArm {
                    pattern: Pattern::Literal(Box::new(Expr::IntLiteral(5))),
                    guard: None,
                    body: Expr::IntLiteral(5),
                },
                MatchArm {
                    pattern: Pattern::Literal(Box::new(Expr::IntLiteral(3))),
                    guard: None,
                    body: Expr::IntLiteral(3),
                },
            ],
        };

        let module = Module {
            items: vec![Item::Function(FunctionDef {
                public: false,
                name: "f".into(),
                params: vec![],
                return_type: None,
                body: Expr::Block(vec![inner_match]),
                span: s(),
            })],
        };
        assert!(!check(&module).is_empty());
    }

    // -----------------------------------------------------------------------
    // Empty module
    // -----------------------------------------------------------------------

    #[test]
    fn empty_module_ok() {
        let module = Module { items: vec![] };
        assert!(check(&module).is_empty());
    }

    // -----------------------------------------------------------------------
    // Struct delegates sorted
    // -----------------------------------------------------------------------

    #[test]
    fn struct_delegates_sorted_ok() {
        let module = Module {
            items: vec![Item::Struct(StructDef {
                public: false,
                name: "MyStruct".into(),
                fields: vec![],
                delegates: vec![
                    TypeExpr::Named("Display".into()),
                    TypeExpr::Named("Printable".into()),
                ],
                span: s(),
            })],
        };
        assert!(check(&module).is_empty());
    }

    #[test]
    fn struct_delegates_unsorted_error() {
        let module = Module {
            items: vec![Item::Struct(StructDef {
                public: false,
                name: "MyStruct".into(),
                fields: vec![],
                delegates: vec![
                    TypeExpr::Named("Printable".into()),
                    TypeExpr::Named("Display".into()),
                ],
                span: s(),
            })],
        };
        assert_eq!(check(&module).len(), 1);
    }

    // -----------------------------------------------------------------------
    // Multiple errors at once
    // -----------------------------------------------------------------------

    #[test]
    fn multiple_errors() {
        let module = Module {
            items: vec![
                // Unsorted functions (category ok, but names wrong)
                fn_item("zebra"),
                fn_item("alpha"),
            ],
        };
        let errors = check(&module);
        assert!(!errors.is_empty());
    }

    // -----------------------------------------------------------------------
    // Pattern sort key tests
    // -----------------------------------------------------------------------

    #[test]
    fn pattern_sort_key_literals() {
        assert_eq!(
            pattern_sort_key(&Pattern::Literal(Box::new(Expr::IntLiteral(42)))),
            "42"
        );
        assert_eq!(
            pattern_sort_key(&Pattern::Literal(Box::new(Expr::FloatLiteral(3.14)))),
            "3.14"
        );
        assert_eq!(
            pattern_sort_key(&Pattern::Literal(Box::new(Expr::StringLiteral(
                "hello".into()
            )))),
            "\"hello\""
        );
        assert_eq!(
            pattern_sort_key(&Pattern::Literal(Box::new(Expr::BoolLiteral(true)))),
            "true"
        );
    }

    #[test]
    fn pattern_sort_key_binding() {
        assert_eq!(pattern_sort_key(&Pattern::Binding("x".into())), "x");
    }

    #[test]
    fn pattern_sort_key_enum_variant_with_type() {
        assert_eq!(
            pattern_sort_key(&Pattern::EnumVariant {
                type_name: Some("Color".into()),
                variant: "Red".into(),
                data: None,
            }),
            "Color.Red"
        );
    }

    #[test]
    fn pattern_sort_key_enum_variant_without_type() {
        assert_eq!(
            pattern_sort_key(&Pattern::EnumVariant {
                type_name: None,
                variant: "Red".into(),
                data: None,
            }),
            "Red"
        );
    }

    #[test]
    fn pattern_sort_key_struct() {
        assert_eq!(
            pattern_sort_key(&Pattern::Struct {
                type_name: "Person".into(),
                fields: vec![],
            }),
            "Person"
        );
    }

    // -----------------------------------------------------------------------
    // Helper function tests
    // -----------------------------------------------------------------------

    #[test]
    fn item_category_values() {
        assert_eq!(item_category(&use_item(&["io"])), 0);
        assert_eq!(item_category(&struct_item("A", &[])), 1);
        assert_eq!(item_category(&enum_item("A", &[])), 1);
        assert_eq!(item_category(&newtype_item("A")), 1);
        assert_eq!(item_category(&contract_item("A", &[])), 1);
        assert_eq!(item_category(&fn_item("a")), 2);
    }

    #[test]
    fn type_expr_sort_key_named() {
        assert_eq!(type_expr_sort_key(&TypeExpr::Named("Int".into())), "Int");
    }

    #[test]
    fn type_expr_sort_key_generic() {
        assert_eq!(
            type_expr_sort_key(&TypeExpr::Generic {
                name: "List".into(),
                params: vec![TypeExpr::Named("Int".into())],
            }),
            "List"
        );
    }

    // -----------------------------------------------------------------------
    // Expression recursion tests
    // -----------------------------------------------------------------------

    #[test]
    fn match_in_binary_op_checked() {
        let match_expr = Expr::Match {
            subject: Some(Box::new(Expr::Identifier("x".into()))),
            arms: vec![
                MatchArm {
                    pattern: Pattern::Literal(Box::new(Expr::IntLiteral(2))),
                    guard: None,
                    body: Expr::IntLiteral(2),
                },
                MatchArm {
                    pattern: Pattern::Literal(Box::new(Expr::IntLiteral(1))),
                    guard: None,
                    body: Expr::IntLiteral(1),
                },
            ],
        };

        let module = Module {
            items: vec![Item::Function(FunctionDef {
                public: false,
                name: "f".into(),
                params: vec![],
                return_type: None,
                body: Expr::BinaryOp {
                    left: Box::new(match_expr),
                    op: BinOp::Add,
                    right: Box::new(Expr::IntLiteral(1)),
                },
                span: s(),
            })],
        };
        assert!(!check(&module).is_empty());
    }

    #[test]
    fn match_in_call_argument_checked() {
        let match_expr = Expr::Match {
            subject: None,
            arms: vec![
                MatchArm {
                    pattern: Pattern::Literal(Box::new(Expr::BoolLiteral(true))),
                    guard: None,
                    body: Expr::IntLiteral(1),
                },
                MatchArm {
                    pattern: Pattern::Literal(Box::new(Expr::BoolLiteral(false))),
                    guard: None,
                    body: Expr::IntLiteral(0),
                },
            ],
        };

        let module = Module {
            items: vec![Item::Function(FunctionDef {
                public: false,
                name: "f".into(),
                params: vec![],
                return_type: None,
                body: Expr::Call {
                    function: Box::new(Expr::Identifier("g".into())),
                    argument: Some(Box::new(match_expr)),
                },
                span: s(),
            })],
        };
        // "true" > "false" alphabetically, so this is unsorted
        assert!(!check(&module).is_empty());
    }

    #[test]
    fn match_in_try_checked() {
        let match_expr = Expr::Match {
            subject: Some(Box::new(Expr::Identifier("x".into()))),
            arms: vec![
                MatchArm {
                    pattern: Pattern::Literal(Box::new(Expr::IntLiteral(9))),
                    guard: None,
                    body: Expr::IntLiteral(9),
                },
                MatchArm {
                    pattern: Pattern::Literal(Box::new(Expr::IntLiteral(1))),
                    guard: None,
                    body: Expr::IntLiteral(1),
                },
            ],
        };

        let module = Module {
            items: vec![Item::Function(FunctionDef {
                public: false,
                name: "f".into(),
                params: vec![],
                return_type: None,
                body: Expr::Try(Box::new(match_expr)),
                span: s(),
            })],
        };
        assert!(!check(&module).is_empty());
    }
}
