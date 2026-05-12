use rustc_lint::EarlyLintPass;
use rustc_session::{declare_lint, impl_lint_pass};

// ---------------------------------------------------------------------------
// NO_LOOP
// ---------------------------------------------------------------------------

declare_lint! {
    /// **Deny** — don't use `loop`, `while`, or `for`. Use iterators instead.
    pub NO_LOOP,
    Deny,
    "don't use loop/while/for — use iterators"
}

pub struct NoLoop;
impl_lint_pass!(NoLoop => [NO_LOOP]);
impl EarlyLintPass for NoLoop {
    // TODO: implement check_expr (detect ExprKind::Loop, While, ForLoop)
}

// ---------------------------------------------------------------------------
// NO_IF_ELSE
// ---------------------------------------------------------------------------

declare_lint! {
    /// **Warn** — prefer `match` over `if`/`else` chains.
    pub NO_IF_ELSE,
    Warn,
    "prefer match over if/else chains"
}

pub struct NoIfElse;
impl_lint_pass!(NoIfElse => [NO_IF_ELSE]);
impl EarlyLintPass for NoIfElse {
    // TODO: implement check_expr (detect ExprKind::If with else branch)
}

