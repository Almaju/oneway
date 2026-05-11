use rustc_lint::EarlyLintPass;
use rustc_session::{declare_lint, impl_lint_pass};

// ---------------------------------------------------------------------------
// TOO_MANY_PARAMS
// ---------------------------------------------------------------------------

declare_lint! {
    /// **Deny** — functions must have at most 2 parameters (self + one input).
    pub TOO_MANY_PARAMS,
    Deny,
    "functions must have at most 2 parameters"
}

pub struct TooManyParams;
impl_lint_pass!(TooManyParams => [TOO_MANY_PARAMS]);
impl EarlyLintPass for TooManyParams {
    // TODO: implement check_fn
}

// ---------------------------------------------------------------------------
// NO_NESTED_FUNCTIONS
// ---------------------------------------------------------------------------

declare_lint! {
    /// **Warn** — don't define functions inside other functions.
    pub NO_NESTED_FUNCTIONS,
    Warn,
    "don't define functions inside other functions"
}

pub struct NoNestedFunctions;
impl_lint_pass!(NoNestedFunctions => [NO_NESTED_FUNCTIONS]);
impl EarlyLintPass for NoNestedFunctions {
    // TODO: implement check_fn / check_item
}

// ---------------------------------------------------------------------------
// ONE_CONSTRUCTOR_NAME
// ---------------------------------------------------------------------------

declare_lint! {
    /// **Deny** — constructors must be named `new`.
    pub ONE_CONSTRUCTOR_NAME,
    Deny,
    "constructors must be named `new`"
}

pub struct OneConstructorName;
impl_lint_pass!(OneConstructorName => [ONE_CONSTRUCTOR_NAME]);
impl EarlyLintPass for OneConstructorName {
    // TODO: implement check_item (look for impl blocks with methods returning Self)
}
