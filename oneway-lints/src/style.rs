use rustc_lint::EarlyLintPass;
use rustc_session::{declare_lint, impl_lint_pass};

// ---------------------------------------------------------------------------
// NO_GLOB_IMPORTS
// ---------------------------------------------------------------------------

declare_lint! {
    /// **Deny** — no wildcard imports. Every imported symbol must be named
    /// explicitly.
    pub NO_GLOB_IMPORTS,
    Deny,
    "no wildcard imports — name every imported symbol"
}

pub struct NoGlobImports;
impl_lint_pass!(NoGlobImports => [NO_GLOB_IMPORTS]);
impl EarlyLintPass for NoGlobImports {
    // TODO: implement check_item (detect UseTreeKind::Glob)
}
