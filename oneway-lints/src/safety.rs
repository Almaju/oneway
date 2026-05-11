use rustc_lint::EarlyLintPass;
use rustc_session::{declare_lint, impl_lint_pass};

// ---------------------------------------------------------------------------
// NO_UNWRAP
// ---------------------------------------------------------------------------

declare_lint! {
    /// **Deny** — never use `.unwrap()` or `.expect()` in non-test code.
    pub NO_UNWRAP,
    Deny,
    "never use .unwrap() or .expect() in non-test code"
}

pub struct NoUnwrap;
impl_lint_pass!(NoUnwrap => [NO_UNWRAP]);
impl EarlyLintPass for NoUnwrap {
    // TODO: implement check_expr (detect method calls named "unwrap" / "expect")
}

// ---------------------------------------------------------------------------
// NO_PANIC
// ---------------------------------------------------------------------------

declare_lint! {
    /// **Deny** — never use `panic!` / `todo!` / `unimplemented!` /
    /// `unreachable!` in non-test code.
    pub NO_PANIC,
    Deny,
    "never use panic!/todo!/unimplemented!/unreachable! in non-test code"
}

pub struct NoPanic;
impl_lint_pass!(NoPanic => [NO_PANIC]);
impl EarlyLintPass for NoPanic {
    // TODO: implement check_mac (detect panic-family macro invocations)
}
