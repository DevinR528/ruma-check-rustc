use rustc_errors::{Applicability, DiagnosticBuilder};
use rustc_hir::HirId;
use rustc_lint::{LateContext, Lint, LintContext};
use rustc_span::source_map::{MultiSpan, Span};

use std::env;

fn docs_link(diag: &mut DiagnosticBuilder<'_>, lint: &'static Lint) {
    if env::var("CLIPPY_DISABLE_DOCS_LINKS").is_err() {
        diag.help(&format!(
            "for further information visit https://rust-lang.github.io/rust-clippy/{}/index.html#{}",
            &option_env!("RUST_RELEASE_NUM").map_or("master".to_string(), |n| {
                // extract just major + minor version and ignore patch versions
                format!("rust-{}", n.rsplitn(2, '.').nth(1).unwrap())
            }),
            lint.name_lower().replacen("clippy::", "", 1)
        ));
    }
}

/// Emit a basic lint message with a `msg` and a `span`.
///
/// This is the most primitive of our lint emission methods and can
/// be a good way to get a new lint started.
///
/// Usually it's nicer to provide more context for lint messages.
/// Be sure the output is understandable when you use this method.
///
/// # Example
///
/// ```ignore
/// error: usage of mem::forget on Drop type
///   --> $DIR/mem_forget.rs:17:5
///    |
/// 17 |     std::mem::forget(seven);
///    |     ^^^^^^^^^^^^^^^^^^^^^^^
/// ```
pub fn span_lint<T: LintContext>(cx: &T, lint: &'static Lint, sp: impl Into<MultiSpan>, msg: &str) {
    cx.struct_span_lint(lint, sp, |diag| {
        let mut diag = diag.build(msg);
        docs_link(&mut diag, lint);
        diag.emit();
    });
}

/// Same as `span_lint` but with an extra `help` message.
///
/// Use this if you want to provide some general help but
/// can't provide a specific machine applicable suggestion.
///
/// The `help` message can be optionally attached to a `Span`.
///
/// If you change the signature, remember to update the internal lint `CollapsibleCalls`
///
/// # Example
///
/// ```ignore
/// error: constant division of 0.0 with 0.0 will always result in NaN
///   --> $DIR/zero_div_zero.rs:6:25
///    |
/// 6  |     let other_f64_nan = 0.0f64 / 0.0;
///    |                         ^^^^^^^^^^^^
///    |
///    = help: Consider using `f64::NAN` if you would like a constant representing NaN
/// ```
pub fn span_lint_and_help<'a, T: LintContext>(
    cx: &'a T,
    lint: &'static Lint,
    span: Span,
    msg: &str,
    help_span: Option<Span>,
    help: &str,
) {
    cx.struct_span_lint(lint, span, |diag| {
        let mut diag = diag.build(msg);
        if let Some(help_span) = help_span {
            diag.span_help(help_span, help);
        } else {
            diag.help(help);
        }
        docs_link(&mut diag, lint);
        diag.emit();
    });
}

/// Like `span_lint` but allows to add notes, help and suggestions using a closure.
///
/// If you need to customize your lint output a lot, use this function.
/// If you change the signature, remember to update the internal lint `CollapsibleCalls`
pub fn span_lint_and_then<'a, T: LintContext, F>(
    cx: &'a T,
    lint: &'static Lint,
    sp: Span,
    msg: &str,
    f: F,
) where
    F: for<'b> FnOnce(&mut DiagnosticBuilder<'b>),
{
    cx.struct_span_lint(lint, sp, |diag| {
        let mut diag = diag.build(msg);
        f(&mut diag);
        docs_link(&mut diag, lint);
        diag.emit();
    });
}

/// Add a span lint with a suggestion on how to fix it.
///
/// These suggestions can be parsed by rustfix to allow it to automatically fix your code.
/// In the example below, `help` is `"try"` and `sugg` is the suggested replacement `".any(|x| x >
/// 2)"`.
///
/// If you change the signature, remember to update the internal lint `CollapsibleCalls`
///
/// # Example
///
/// ```ignore
/// error: This `.fold` can be more succinctly expressed as `.any`
/// --> $DIR/methods.rs:390:13
///     |
/// 390 |     let _ = (0..3).fold(false, |acc, x| acc || x > 2);
///     |                   ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ help: try: `.any(|x| x > 2)`
///     |
///     = note: `-D fold-any` implied by `-D warnings`
/// ```
#[cfg_attr(feature = "internal-lints", allow(clippy::collapsible_span_lint_calls))]
pub fn span_lint_and_sugg<'a, T: LintContext>(
    cx: &'a T,
    lint: &'static Lint,
    sp: Span,
    msg: &str,
    help: &str,
    sugg: String,
    applicability: Applicability,
) {
    span_lint_and_then(cx, lint, sp, msg, |diag| {
        diag.span_suggestion(sp, help, sugg, applicability);
    });
}
