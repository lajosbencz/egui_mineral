//! [Material Design 3](https://m3.material.io) stylekit for [egui_mineral](https://crates.io/crates/egui_mineral)
//! and [egui](https://github.com/emilk/egui): color, typography and motion tokens, a theme engine and components
//! built on the core behavior models. See every component in the
//! [live showcase](https://lajosbencz.github.io/egui_mineral/material/).
//!
//! Components by MD3 category: [`actions`], [`communication`], [`containment`], [`navigation`], [`selection`],
//! [`text_inputs`], plus [`extras`] (feature `extras`).
//!
//! Targets egui 0.37 on native and wasm. Pre-1.0: any minor release may break the API.
//!
//! # Use
//!
//! ```sh
//! cargo add egui_mineral_material
//! cargo add eframe@0.37
//! ```
//!
//! Pass a [`Material`] to [`Runtime::new`](egui_mineral::runtime::Runtime::new) once, import the [`prelude`] and
//! show widgets in each pass:
//!
//! ```
//! use std::sync::Arc;
//!
//! use egui_mineral_material::{egui, prelude::*};
//!
//! struct App {
//!     rt: Arc<Rt>,
//!     clicks: u32,
//! }
//!
//! impl App {
//!     fn new(ctx: &egui::Context) -> Result<Self, InitError> {
//!         let rt = Runtime::new(ctx, Material::default())?;
//!         Ok(Self { rt, clicks: 0 })
//!     }
//!
//!     /// Call from `eframe::App::ui`.
//!     fn ui(&mut self, ui: &mut egui::Ui) {
//!         // Disjoint field borrows; the `Arc` is not cloned per frame.
//!         let rt = &self.rt;
//!         let clicks = &mut self.clicks;
//!         rt.pass(ui, |ui| {
//!             if ui.mineral(PushButton::new("counter", format!("Clicked {clicks} times"))).clicked() {
//!                 *clicks += 1;
//!             }
//!         });
//!     }
//! }
//! # let ctx = egui::Context::default();
//! # let mut app = App::new(&ctx)?;
//! # let mut output = ctx.run_ui(egui::RawInput::default(), |ui| app.ui(ui));
//! # output.textures_delta.clear();
//! # Ok::<(), InitError>(())
//! ```
//!
//! Each native window can run its own runtime, with its own theme, tokens, locale and density.
//!
//! # Entry points
//!
//! - Anything that yields a [`WidgetResponse`](egui_mineral::widget::WidgetResponse) is a
//!   [`MineralWidget`](egui_mineral::widget::MineralWidget) (leaf or [`Composite`](egui_mineral::widget::Composite))
//!   and is shown with `ui.mineral(..)` on a [`MineralUi`](egui_mineral::widget::MineralUi).
//! - Containers, groups and overlays have `show(..)` returning their own `*Response`: in-layout ones take
//!   `&mut MineralUi`, floating and modal ones take the [`Runtime`](egui_mineral::runtime::Runtime), anchored
//!   popups take the anchor `&Response`. Open state is passed at show time (`show(.., open: &mut bool, ..)`,
//!   `show_modal(.., open, ..)`).
//!
//! # Cargo features
//!
//! | Feature | Default | Adds |
//! |---|---|---|
//! | `extras` | yes | The [`extras`] module (components without an MD3 counterpart), [`prelude`] re-exports of them and `ValueSlider::value_field` |
//! | `icu` | yes | ICU4X locale formatting (`egui_mineral/icu`); without it [`AsciiFormat`](egui_mineral::i18n::AsciiFormat) |
//! | `persist-json` | | JSON persistence codec (`egui_mineral/persist-json`) |
//! | `persist-toml` | yes | TOML persistence codec (`egui_mineral/persist-toml`) |
//! | `persist-fs` | yes | Native file-backed storage (`egui_mineral/persist-fs`) |
//! | `bundled-icons` | yes | Material Symbols Rounded fonts, filled and outlined ([3 MB](https://github.com/lajosbencz/egui_mineral#release-size)); without it `Material::new` and `Default` are absent: use [`Material::from_fonts`] |
//! | `theme-file` | | TOML theme files and live reload ([`theme::load_toml`], [`theme::watch`]); native only |
//! | `unstable-catalog` | | [`catalog`]: each component's category and spec origin ([`catalog::CATALOG`]) |
//! | `unstable-spec` | | Public [`spec::tokens`] and [`spec::defaults`]: the generated Compose Material3 values, changed by every re-pin of the Compose sources |
//!
//! `unstable-*` features are outside the semver guarantee: any release may change them.
// Feature-gated items: docs.rs (all features) links when the feature is off.
#![cfg_attr(
    not(feature = "extras"),
    doc = "",
    doc = "[`extras`]: https://docs.rs/egui_mineral_material/latest/egui_mineral_material/extras/index.html"
)]
#![cfg_attr(
    not(feature = "unstable-catalog"),
    doc = "",
    doc = "[`catalog`]: https://docs.rs/egui_mineral_material/latest/egui_mineral_material/catalog/index.html",
    doc = "[`catalog::CATALOG`]: https://docs.rs/egui_mineral_material/latest/egui_mineral_material/catalog/constant.CATALOG.html"
)]
#![cfg_attr(
    not(feature = "unstable-spec"),
    doc = "",
    doc = "[`spec::tokens`]: https://docs.rs/egui_mineral_material/latest/egui_mineral_material/spec/tokens/index.html",
    doc = "[`spec::defaults`]: https://docs.rs/egui_mineral_material/latest/egui_mineral_material/spec/defaults/index.html"
)]
#![cfg_attr(
    not(all(feature = "theme-file", not(target_arch = "wasm32"))),
    doc = "",
    doc = "[`theme::load_toml`]: https://docs.rs/egui_mineral_material/latest/egui_mineral_material/theme/fn.load_toml.html",
    doc = "[`theme::watch`]: https://docs.rs/egui_mineral_material/latest/egui_mineral_material/theme/fn.watch.html"
)]
#![cfg_attr(docsrs, feature(doc_cfg))]
// docs/rules.md rule 10; tests may panic.
#![cfg_attr(
    not(test),
    deny(clippy::panic, clippy::unwrap_used, clippy::expect_used, clippy::unreachable)
)]

pub mod actions;
/// Window size classes, density and minimum interactive size.
pub mod adaptive;
/// Component provenance: category and spec origin.
#[cfg(feature = "unstable-catalog")]
pub mod catalog;
/// MD3 color schemes from a seed (material-color-utilities).
pub mod color;
pub mod communication;
pub mod containment;
#[cfg(feature = "extras")]
pub mod extras;
/// Font and icon installation.
pub mod fonts;
mod icon_set;
/// Material Symbols constants, generated by `tools/material_symbols`.
pub mod icons;
mod material;
mod material_icon;
mod motion;
pub mod navigation;
pub mod prelude;
pub mod scroll;
pub mod selection;
pub mod spec;
#[cfg(all(test, feature = "bundled-icons"))]
mod test_support;
pub mod text_inputs;
/// Theme engine: seeded or custom tokens per brightness and contrast; TOML files with feature `theme-file`.
pub mod theme;
mod tokens;
/// MD3 type scale.
pub mod typography;
mod widget;

pub use egui_mineral;
pub use egui_mineral::egui;
pub use egui_mineral::icon::Icon;
pub use icon_set::IconSet;
pub use material::{Material, MaterialExt, MaterialUi, MaterialUiExt, Rt};
pub use material_icon::MaterialIcon;
pub use widget::{
    leading::{Leading, LeadingPaint},
    vocab::{IconAction, SelectionResponse, Size, TextAction},
};

pub(crate) use egui_mineral::kit::i18n::{tr, tr_or};
