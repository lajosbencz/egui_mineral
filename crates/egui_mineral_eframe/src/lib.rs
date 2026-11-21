//! [eframe](https://crates.io/crates/eframe) adapter for [egui_mineral](https://crates.io/crates/egui_mineral).
//!
//! [`EframeApp`] owns the [`Runtime`] and runs one [`Runtime::pass`] per frame; the app implements
//! [`MineralApp`] and draws through the [`MineralUi`] it is handed, reaching the runtime with `ui.rt()`.
//!
//! ```no_run
//! # #[cfg(not(target_arch = "wasm32"))] {
//! use egui_mineral::runtime::{Runtime, Stylekit};
//! use egui_mineral::widget::MineralUi;
//! use egui_mineral_eframe::{EframeApp, MineralApp};
//!
//! struct Counter(u32);
//!
//! impl<K: Stylekit> MineralApp<K> for Counter {
//!     fn ui(&mut self, ui: &mut MineralUi<'_, K>, _frame: &mut eframe::Frame) {
//!         if ui.button(format!("Clicked {} times", self.0)).clicked() {
//!             self.0 += 1;
//!         }
//!     }
//! }
//!
//! fn run<K: Stylekit>(kit: K) -> eframe::Result {
//!     eframe::run_native(
//!         "counter",
//!         eframe::NativeOptions::default(),
//!         Box::new(|cc| Ok(Box::new(EframeApp::new(Runtime::new(&cc.egui_ctx, kit)?, Counter(0))))),
//!     )
//! }
//! # }
//! ```
//!
//! Features forward to eframe: renderers `glow` (default) and `wgpu`, at least one required; Linux windowing
//! `wayland` and `x11` (both default), at least one required there. Other eframe features (accesskit,
//! persistence, ..) are enabled on the app's own `eframe` dependency.

#![cfg_attr(docsrs, feature(doc_cfg))]
// docs/rules.md rule 10; tests may panic.
#![cfg_attr(
    not(test),
    deny(clippy::panic, clippy::unwrap_used, clippy::expect_used, clippy::unreachable)
)]

use std::sync::Arc;

use egui_mineral::egui::{Context, RawInput, Ui};
use egui_mineral::runtime::{Runtime, Stylekit};
use egui_mineral::widget::MineralUi;

pub use eframe;

/// Run by [`EframeApp`] once per frame.
///
/// eframe's `on_exit` is not forwarded: its signature depends on eframe's `glow` feature, which any crate in
/// the graph may enable.
pub trait MineralApp<K: Stylekit> {
    /// Draws the frame inside the runtime's pass; `frame` is eframe's per-frame data (`frame.info()`: CPU
    /// time, web location).
    fn ui(&mut self, ui: &mut MineralUi<'_, K>, frame: &mut eframe::Frame);

    /// After the pass; for tools observing it, e.g. an inspector in its own viewport.
    fn after_pass(&mut self, _rt: &Runtime<K>) {}

    /// As [`eframe::App::save`].
    fn save(&mut self, _storage: &mut dyn eframe::Storage) {}

    /// As [`eframe::App::raw_input_hook`]; e.g. sets `raw_input.system_theme` where the platform reports none.
    fn raw_input_hook(&mut self, _ctx: &Context, _raw_input: &mut RawInput) {}
}

/// [`eframe::App`] owning the [`Runtime`] and running `A` inside its pass.
pub struct EframeApp<K, A> {
    rt: Arc<Runtime<K>>,
    app: A,
}

impl<K, A> std::fmt::Debug for EframeApp<K, A> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EframeApp").finish_non_exhaustive()
    }
}

impl<K: Stylekit, A: MineralApp<K>> EframeApp<K, A> {
    /// `app` run by `rt`.
    pub const fn new(rt: Arc<Runtime<K>>, app: A) -> Self {
        Self { rt, app }
    }

    /// Runtime driving the app's pass.
    pub const fn rt(&self) -> &Arc<Runtime<K>> {
        &self.rt
    }

    /// Wrapped app.
    pub const fn app(&self) -> &A {
        &self.app
    }

    /// Wrapped app, mutably.
    pub const fn app_mut(&mut self) -> &mut A {
        &mut self.app
    }
}

impl<K: Stylekit, A: MineralApp<K>> eframe::App for EframeApp<K, A> {
    fn ui(&mut self, ui: &mut Ui, frame: &mut eframe::Frame) {
        let Self { rt, app } = self;
        rt.pass(ui, |ui| app.ui(ui, frame));
        app.after_pass(rt);
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        self.app.save(storage);
    }

    fn raw_input_hook(&mut self, ctx: &Context, raw_input: &mut RawInput) {
        self.app.raw_input_hook(ctx, raw_input);
    }
}
