use rustc_ast::ast;
use rustc_data_structures::fx::{FxHashMap, FxHashSet};
use rustc_errors::Applicability;
use rustc_hir as hir;
use rustc_hir::def::{DefKind, Res};
use rustc_lint::{LateContext, LateLintPass, LintContext};
use rustc_session::{declare_tool_lint, impl_lint_pass};
use rustc_span::Span;

use crate::utils::span_lint_and_sugg;

declare_tool_lint!(
    pub ruma::USE_ORDER,
    Warn,
    "Use a consistend ordering for imports.",
    report_in_external_macro: true
);

#[derive(Default)]
pub struct UseOrder {
    imports: Vec<(String, Span)>,
}

impl_lint_pass!(UseOrder => [USE_ORDER]);

impl<'tcx> LateLintPass<'tcx> for UseOrder {
    fn check_item(&mut self, cx: &LateContext<'_>, item: &hir::Item<'_>) {
        if let hir::ItemKind::Use(path, _kind) = &item.kind {
            if let Some(root) = path
                .segments
                .iter()
                .next()
                .map(|seg| (seg.ident.to_string(), path.span))
            {
                println!("{:?}", root);
                self.imports.push(root);
            }
        }
    }

    fn check_crate_post(&mut self, cx: &LateContext<'_>, _krate: &hir::Crate<'_>) {
        for (imp, span) in &self.imports {
            span_lint_and_sugg(
                cx,
                USE_ORDER,
                *span,
                "hello",
                "help",
                "sugg".to_string(),
                Applicability::MachineApplicable,
            )
        }
    }
}
