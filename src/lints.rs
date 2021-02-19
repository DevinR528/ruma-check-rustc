mod use_order;

use rustc_lint::LintId;
use rustc_session::Session;

use crate::config::Conf;

pub fn register_plugins(store: &mut rustc_lint::LintStore, sess: &Session, conf: &Conf) {
    store.register_lints(&[]);

    store.register_late_pass(|| Box::new(use_order::UseOrder::default()));

    store.register_group(
        true,
        "ruma",
        Some("ruma"),
        vec![LintId::of(use_order::USE_ORDER)],
    );
}
