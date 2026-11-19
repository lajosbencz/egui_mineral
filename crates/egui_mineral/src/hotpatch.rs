//! Dioxus `subsecond` wiring behind `unstable-hotpatch`.

use std::sync::{Arc, Once};

use egui::Context;

// dioxus_devtools::connect_subsecond opens one devserver socket per process.
pub(crate) fn connect(ctx: &Context) {
    static CONNECT: Once = Once::new();
    CONNECT.call_once(dioxus_devtools::connect_subsecond);
    let ctx = ctx.clone();
    #[expect(clippy::disallowed_methods, reason = "one repaint per applied patch")]
    subsecond::register_handler(Arc::new(move || ctx.request_repaint()));
}
