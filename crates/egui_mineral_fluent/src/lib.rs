//! [Fluent 2](https://fluent2.microsoft.design) stylekit for [egui_mineral](https://crates.io/crates/egui_mineral)
//! and [egui](https://github.com/emilk/egui), ported from Fluent UI React v9: alias color tokens per theme and
//! brand, the type ramp, geometry, shadows and motion, a theme engine and components built on the core behavior
//! models. See every component in the [live showcase](https://lajosbencz.github.io/egui_mineral/fluent/).
//!
//! Components by category: [`actions`], [`inputs`], [`feedback`], [`surfaces`], [`navigation`], [`data`].
//!
//! Targets egui 0.37 on native and wasm. Pre-1.0: any minor release may break the API.
//!
//! # Use
//!
//! ```sh
//! cargo add egui_mineral_fluent
//! cargo add eframe@0.37
//! ```
//!
//! Pass a [`Fluent`] to [`Runtime::new`](egui_mineral::runtime::Runtime::new) once, import the [`prelude`] and
//! show widgets in each pass:
//!
//! ```
//! use std::sync::Arc;
//!
//! use egui_mineral_fluent::{egui, prelude::*};
//!
//! struct App {
//!     rt: Arc<Rt>,
//!     clicks: u32,
//! }
//!
//! impl App {
//!     fn new(ctx: &egui::Context) -> Result<Self, egui_mineral_fluent::egui_mineral::runtime::InitError> {
//!         let rt = Runtime::new(ctx, Fluent::default())?;
//!         Ok(Self { rt, clicks: 0 })
//!     }
//!
//!     /// Call from `eframe::App::ui`.
//!     fn ui(&mut self, ui: &mut egui::Ui) {
//!         let clicks = &mut self.clicks;
//!         self.rt.pass(ui, |ui| {
//!             let label = format!("Clicked {clicks} times");
//!             if ui.mineral(PushButton::new("counter", label).appearance(ButtonAppearance::Primary)).clicked() {
//!                 *clicks += 1;
//!             }
//!         });
//!     }
//! }
//! # let ctx = egui::Context::default();
//! # let mut app = App::new(&ctx)?;
//! # let mut output = ctx.run_ui(egui::RawInput::default(), |ui| app.ui(ui));
//! # output.textures_delta.clear();
//! # Ok::<(), egui_mineral_fluent::egui_mineral::runtime::InitError>(())
//! ```
//!
//! Each native window can run its own runtime, with its own theme, brand and locale.
//!
//! # Entry points
//!
//! - Anything that yields a [`WidgetResponse`](egui_mineral::widget::WidgetResponse) is a
//!   [`MineralWidget`](egui_mineral::widget::MineralWidget) and is shown with `ui.mineral(..)` on a
//!   [`MineralUi`](egui_mineral::widget::MineralUi).
//! - Groups have `show(..)` returning their own `*Response` ([`actions::SplitButton`]).
//!
//! # Cargo features
//!
//! | Feature | Default | Adds |
//! |---|---|---|
//! | `icu` | yes | ICU4X locale formatting (`egui_mineral/icu`); without it [`AsciiFormat`](egui_mineral::i18n::AsciiFormat) |
//! | `persist-json` | | JSON persistence codec (`egui_mineral/persist-json`) |
//! | `persist-toml` | yes | TOML persistence codec (`egui_mineral/persist-toml`) |
//! | `persist-fs` | yes | Native file-backed storage (`egui_mineral/persist-fs`) |
//! | `bundled-icons` | yes | Fluent UI System Icons resizable font (1.6 MB); without it use [`Fluent::with_icon_font`] |
//! | `unstable-catalog` | | [`catalog`]: each component's category and React v9 styles file ([`catalog::CATALOG`]) |
//! | `unstable-spec` | | Public [`spec::tokens`] and [`spec::defaults`]: the generated Fluent UI React v9 values, changed by every re-pin of the sources |
//!
//! `unstable-*` features are outside the semver guarantee: any release may change them.
// Feature-gated items: docs.rs (all features) links when the feature is off.
#![cfg_attr(
    not(feature = "unstable-catalog"),
    doc = "",
    doc = "[`catalog`]: https://docs.rs/egui_mineral_fluent/latest/egui_mineral_fluent/catalog/index.html",
    doc = "[`catalog::CATALOG`]: https://docs.rs/egui_mineral_fluent/latest/egui_mineral_fluent/catalog/constant.CATALOG.html"
)]
#![cfg_attr(
    not(feature = "unstable-spec"),
    doc = "",
    doc = "[`spec::tokens`]: https://docs.rs/egui_mineral_fluent/latest/egui_mineral_fluent/spec/tokens/index.html",
    doc = "[`spec::defaults`]: https://docs.rs/egui_mineral_fluent/latest/egui_mineral_fluent/spec/defaults/index.html"
)]
#![cfg_attr(docsrs, feature(doc_cfg))]
// docs/rules.md rule 10; tests may panic.
#![cfg_attr(
    not(test),
    deny(clippy::panic, clippy::unwrap_used, clippy::expect_used, clippy::unreachable)
)]

pub mod actions;
/// Component provenance: category and React v9 styles file.
#[cfg(feature = "unstable-catalog")]
pub mod catalog;
pub mod data;
pub mod feedback;
mod fluent;
/// Font and icon installation.
pub mod fonts;
mod icon;
mod icon_set;
pub mod icons;
pub mod inputs;
pub mod navigation;
mod policy;
pub mod prelude;
pub mod scroll;
pub mod spec;
pub mod surfaces;
/// Theme engine: web light, web dark and high contrast tokens against a brand ramp.
pub mod theme;
mod tokens;
/// Fluent type ramp.
pub mod typography;
mod widget;

pub use egui_mineral;
pub use egui_mineral::egui;
pub use fluent::{Fluent, FluentExt, Rt};
pub use egui_mineral::icon::Icon;
pub use icon::FluentIcon;
pub use icon_set::IconSet;
pub use spec::{BrandRamp, BrandStep, ColorToken, Shadow};
pub use widget::vocab::{IconPosition, Size};

pub(crate) use egui_mineral::kit::i18n::{tr, tr_or};
