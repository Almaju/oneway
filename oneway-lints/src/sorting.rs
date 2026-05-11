use rustc_ast::ast;
use rustc_lint::{EarlyContext, EarlyLintPass, LintContext};
use rustc_session::{declare_lint, impl_lint_pass};
use rustc_span::Span;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Returns `Some((index, prev_name, curr_name))` for the first pair of
/// adjacent names that are out of alphabetical order.
fn first_unsorted(names: &[String]) -> Option<(usize, String, String)> {
    names.windows(2).enumerate().find_map(|(i, w)| {
        if w[0] > w[1] {
            Some((i + 1, w[0].clone(), w[1].clone()))
        } else {
            None
        }
    })
}

/// Emit a lint diagnostic at the given span.
fn emit_lint(cx: &EarlyContext<'_>, lint: &'static rustc_lint::Lint, span: Span, msg: String) {
    cx.opt_span_lint(lint, Some(span), |diag| {
        diag.primary_message(msg);
    });
}

/// Build a sort key from a `UseTree` by joining its path segments.
fn use_tree_sort_key(tree: &ast::UseTree) -> String {
    tree.prefix
        .segments
        .iter()
        .map(|s| s.ident.name.to_string())
        .collect::<Vec<_>>()
        .join("::")
}

// ---------------------------------------------------------------------------
// 1. UNSORTED_STRUCT_FIELDS
// ---------------------------------------------------------------------------

declare_lint! {
    /// **Deny** — struct fields must be in alphabetical order.
    pub UNSORTED_STRUCT_FIELDS,
    Deny,
    "struct fields must be in alphabetical order"
}

pub struct UnsortedStructFields;
impl_lint_pass!(UnsortedStructFields => [UNSORTED_STRUCT_FIELDS]);

impl EarlyLintPass for UnsortedStructFields {
    fn check_item(&mut self, cx: &EarlyContext<'_>, item: &ast::Item) {
        // ItemKind::Struct is Struct(Ident, Generics, VariantData)
        if let ast::ItemKind::Struct(_, _, ref vdata) = item.kind {
            let fields = vdata.fields();
            let names: Vec<String> = fields
                .iter()
                .filter_map(|f| f.ident.map(|id| id.name.to_string()))
                .collect();
            // Only check structs where every field is named (skip tuple structs)
            if names.len() != fields.len() || names.len() < 2 {
                return;
            }
            if let Some((idx, prev, curr)) = first_unsorted(&names) {
                emit_lint(
                    cx,
                    UNSORTED_STRUCT_FIELDS,
                    fields[idx].span,
                    format!(
                        "struct field `{curr}` should come before `{prev}` (alphabetical order required)"
                    ),
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// 2. UNSORTED_ENUM_VARIANTS
// ---------------------------------------------------------------------------

declare_lint! {
    /// **Deny** — enum variants must be in alphabetical order.
    pub UNSORTED_ENUM_VARIANTS,
    Deny,
    "enum variants must be in alphabetical order"
}

pub struct UnsortedEnumVariants;
impl_lint_pass!(UnsortedEnumVariants => [UNSORTED_ENUM_VARIANTS]);

impl EarlyLintPass for UnsortedEnumVariants {
    fn check_item(&mut self, cx: &EarlyContext<'_>, item: &ast::Item) {
        // ItemKind::Enum is Enum(Ident, Generics, EnumDef)
        if let ast::ItemKind::Enum(_, _, ref enum_def) = item.kind {
            let names: Vec<String> = enum_def
                .variants
                .iter()
                .map(|v| v.ident.name.to_string())
                .collect();
            if names.len() < 2 {
                return;
            }
            if let Some((idx, prev, curr)) = first_unsorted(&names) {
                emit_lint(
                    cx,
                    UNSORTED_ENUM_VARIANTS,
                    enum_def.variants[idx].span,
                    format!(
                        "enum variant `{curr}` should come before `{prev}` (alphabetical order required)"
                    ),
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// 3. UNSORTED_MATCH_ARMS
// ---------------------------------------------------------------------------

declare_lint! {
    /// **Deny** — match arms must be sorted by pattern text. Wildcard `_` must
    /// always be last.
    pub UNSORTED_MATCH_ARMS,
    Deny,
    "match arms must be sorted by pattern text; wildcard `_` must be last"
}

pub struct UnsortedMatchArms;
impl_lint_pass!(UnsortedMatchArms => [UNSORTED_MATCH_ARMS]);

impl EarlyLintPass for UnsortedMatchArms {
    fn check_expr(&mut self, cx: &EarlyContext<'_>, expr: &ast::Expr) {
        // ExprKind::Match may have 2 or 3 fields depending on the nightly
        // version; the trailing `..` handles both.
        if let ast::ExprKind::Match(_, ref arms, ..) = expr.kind {
            if arms.len() < 2 {
                return;
            }

            let source_map = cx.sess().source_map();

            // Collect (pattern_text, is_wildcard, span) for each arm.
            let mut arm_keys: Vec<(String, bool, Span)> = Vec::new();
            for arm in arms.iter() {
                let is_wild = matches!(arm.pat.kind, ast::PatKind::Wild);
                let snippet = source_map
                    .span_to_snippet(arm.pat.span)
                    .unwrap_or_else(|_| "_".into());
                arm_keys.push((snippet, is_wild, arm.pat.span));
            }

            // 1. Wildcards must be last.
            let mut seen_wild = false;
            for (snippet, is_wild, span) in &arm_keys {
                if seen_wild && !is_wild {
                    emit_lint(
                        cx,
                        UNSORTED_MATCH_ARMS,
                        *span,
                        format!(
                            "match arm `{snippet}` appears after wildcard `_`; wildcard must be last"
                        ),
                    );
                    return;
                }
                if *is_wild {
                    seen_wild = true;
                }
            }

            // 2. Non-wildcard arms must be alphabetically sorted.
            let non_wild: Vec<&(String, bool, Span)> =
                arm_keys.iter().filter(|(_, w, _)| !w).collect();
            let names: Vec<String> = non_wild.iter().map(|(s, _, _)| s.clone()).collect();
            if let Some((idx, prev, curr)) = first_unsorted(&names) {
                emit_lint(
                    cx,
                    UNSORTED_MATCH_ARMS,
                    non_wild[idx].2,
                    format!(
                        "match arm `{curr}` should come before `{prev}` (alphabetical order required)"
                    ),
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// 4. UNSORTED_IMPORTS
// ---------------------------------------------------------------------------

declare_lint! {
    /// **Deny** — `use` statements must be in alphabetical order within each
    /// group, and items inside braced use trees must also be sorted.
    pub UNSORTED_IMPORTS,
    Deny,
    "use statements must be in alphabetical order"
}

pub struct UnsortedImports;
impl_lint_pass!(UnsortedImports => [UNSORTED_IMPORTS]);

/// Check consecutive `use` items in a list of items and return diagnostics
/// for any that are out of order.  Generic over the smart-pointer type
/// wrapping `ast::Item` (works with `Box<Item>`, `P<Item>`, etc.).
fn check_consecutive_uses<T: std::ops::Deref<Target = ast::Item>>(
    items: &[T],
) -> Vec<(Span, String)> {
    let mut prev: Option<(String, Span)> = None;
    let mut diagnostics = Vec::new();

    for item in items.iter() {
        if let ast::ItemKind::Use(ref use_tree) = item.kind {
            let key = use_tree_sort_key(use_tree);
            if let Some((ref prev_key, _)) = prev {
                if key < *prev_key {
                    diagnostics.push((
                        item.span,
                        format!(
                            "import `{key}` should come before `{prev_key}` (alphabetical order required)"
                        ),
                    ));
                }
            }
            prev = Some((key, item.span));
        } else {
            // A non-use item resets the consecutive group.
            prev = None;
        }
    }
    diagnostics
}

/// Recursively check that children of braced use trees are sorted
/// (e.g. `use std::{io, fmt}` should be `use std::{fmt, io}`).
fn check_use_tree_children(cx: &EarlyContext<'_>, tree: &ast::UseTree) {
    // UseTreeKind::Nested is a struct variant: Nested { items, span }.
    if let ast::UseTreeKind::Nested { ref items, .. } = tree.kind {
        let names: Vec<String> = items.iter().map(|(t, _)| use_tree_sort_key(t)).collect();
        if names.len() >= 2 {
            if let Some((idx, prev, curr)) = first_unsorted(&names) {
                emit_lint(
                    cx,
                    UNSORTED_IMPORTS,
                    items[idx].0.span,
                    format!(
                        "import `{curr}` should come before `{prev}` in use group (alphabetical order required)"
                    ),
                );
            }
        }
        // Recurse into each nested subtree.
        for (subtree, _) in items.iter() {
            check_use_tree_children(cx, subtree);
        }
    }
}

impl EarlyLintPass for UnsortedImports {
    fn check_crate(&mut self, cx: &EarlyContext<'_>, krate: &ast::Crate) {
        // Check ordering of consecutive `use` statements at the crate root.
        for (span, msg) in check_consecutive_uses(&krate.items) {
            emit_lint(cx, UNSORTED_IMPORTS, span, msg);
        }
    }

    fn check_item(&mut self, cx: &EarlyContext<'_>, item: &ast::Item) {
        // Check nested use-tree groups (e.g. `use std::{B, A}`).
        if let ast::ItemKind::Use(ref use_tree) = item.kind {
            check_use_tree_children(cx, use_tree);
        }

        // Check consecutive use statements inside `mod` blocks.
        // ItemKind::Mod is now Mod(Safety, Ident, ModKind).
        if let ast::ItemKind::Mod(_, _, ast::ModKind::Loaded(ref items, ..)) = item.kind {
            for (span, msg) in check_consecutive_uses(items) {
                emit_lint(cx, UNSORTED_IMPORTS, span, msg);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// 5. UNSORTED_IMPL_METHODS
// ---------------------------------------------------------------------------

declare_lint! {
    /// **Deny** — methods within an `impl` block must be alphabetically sorted.
    pub UNSORTED_IMPL_METHODS,
    Deny,
    "methods within an impl block must be alphabetically sorted"
}

pub struct UnsortedImplMethods;
impl_lint_pass!(UnsortedImplMethods => [UNSORTED_IMPL_METHODS]);

/// Try to extract the function/method name from an `AssocItemKind`.
/// Returns `Some(name)` for function items, `None` for others.
fn assoc_fn_name(kind: &ast::AssocItemKind) -> Option<String> {
    // In this nightly, Item no longer has an `ident` field — the name is
    // embedded in the kind-specific inner struct.  For functions, the `Fn`
    // struct (or `AssocItemKind::Fn` tuple) should carry an `ident`.
    if let ast::AssocItemKind::Fn(ref fn_box) = kind {
        // ast::Fn may have an `ident` field in this nightly.
        Some(fn_box.ident.name.to_string())
    } else {
        None
    }
}

impl EarlyLintPass for UnsortedImplMethods {
    fn check_item(&mut self, cx: &EarlyContext<'_>, item: &ast::Item) {
        if let ast::ItemKind::Impl(ref impl_block) = item.kind {
            let methods: Vec<(String, Span)> = impl_block
                .items
                .iter()
                .filter_map(|assoc| assoc_fn_name(&assoc.kind).map(|name| (name, assoc.span)))
                .collect();

            if methods.len() < 2 {
                return;
            }

            let names: Vec<String> = methods.iter().map(|(n, _)| n.clone()).collect();
            if let Some((idx, prev, curr)) = first_unsorted(&names) {
                emit_lint(
                    cx,
                    UNSORTED_IMPL_METHODS,
                    methods[idx].1,
                    format!(
                        "impl method `{curr}` should come before `{prev}` (alphabetical order required)"
                    ),
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// 6. UNSORTED_DERIVES
// ---------------------------------------------------------------------------

declare_lint! {
    /// **Deny** — `#[derive(...)]` attributes must list traits in alphabetical
    /// order.
    pub UNSORTED_DERIVES,
    Deny,
    "#[derive(...)] traits must be in alphabetical order"
}

pub struct UnsortedDerives;
impl_lint_pass!(UnsortedDerives => [UNSORTED_DERIVES]);

/// Extract trait names from a `#[derive(Trait1, Trait2, ...)]` source snippet.
fn extract_derive_names(snippet: &str) -> Vec<String> {
    let s = snippet.trim();
    if let Some(start) = s.find('(') {
        if let Some(end) = s.rfind(')') {
            let inner = &s[start + 1..end];
            return inner
                .split(',')
                .map(|t| t.trim().to_string())
                .filter(|t| !t.is_empty())
                .collect();
        }
    }
    Vec::new()
}

impl EarlyLintPass for UnsortedDerives {
    fn check_item(&mut self, cx: &EarlyContext<'_>, item: &ast::Item) {
        for attr in &item.attrs {
            if attr.has_name(rustc_span::symbol::sym::derive) {
                let source_map = cx.sess().source_map();
                if let Ok(snippet) = source_map.span_to_snippet(attr.span) {
                    let names = extract_derive_names(&snippet);
                    if names.len() < 2 {
                        continue;
                    }
                    if let Some((_idx, prev, curr)) = first_unsorted(&names) {
                        emit_lint(
                            cx,
                            UNSORTED_DERIVES,
                            attr.span,
                            format!(
                                "derive trait `{curr}` should come before `{prev}` (alphabetical order required)"
                            ),
                        );
                    }
                }
            }
        }
    }
}
