//! Material Design 3 stylekit for `egui_mineral`: color, typography and motion tokens, a theme engine, [`spec`] data
//! and components built on the core behavior models. Pass a [`Material`] to
//! [`Runtime::new`](egui_mineral::runtime::Runtime::new) and import [`prelude`]. Each native window can run its own
//! runtime, with its own theme, tokens, locale and density.
//!
//! Cargo features; `icu`, `persist-fs` and `bundled-icons` are default. `unstable-*` features are outside the semver guarantee: any
//! release may change them.
//! - `icu`: ICU4X locale formatting (`egui_mineral/icu`); without it [`AsciiFormat`](egui_mineral::i18n::AsciiFormat).
//! - `persist-fs`: file-backed persistence (`egui_mineral/persist-fs`).
//! - `bundled-icons`: Material Symbols Rounded fonts (filled and outlined, 3 MB); without it
//!   [`Material::icon_fonts`] is required.
//! - `unstable-catalog`: [`catalog`], each component's category and spec origin ([`catalog::CATALOG`]).
//! - `unstable-spec`: public [`spec::tokens`] and [`spec::defaults`] (the generated Compose Material3 values, which
//!   change with every re-pin of the Compose sources).
//! - `theme-file`: TOML theme files and live reload ([`theme::load_toml`], [`theme::watch`]); native targets only.
//!
//! Entry points:
//! - Anything that yields a [`WidgetResponse`](egui_mineral::widget::WidgetResponse) is a
//!   [`MineralWidget`](egui_mineral::widget::MineralWidget) (leaf or
//!   [`Composite`](egui_mineral::widget::Composite)) and is shown with `ui.mineral(..)` on a
//!   [`MineralUi`](egui_mineral::widget::MineralUi).
//! - Containers, groups and overlays have `show(..)` returning their own `*Response`: in-layout ones take
//!   `&mut MineralUi`, floating and modal ones take the [`Runtime`](egui_mineral::runtime::Runtime), anchored
//!   popups take the anchor `&Response`.
//!   Open state is passed at show time (`show(.., open: &mut bool, ..)`, `show_modal(.., open, ..)`).
//!
//! ```
//! use egui_mineral_material::prelude::*;
//!
//! let ctx = egui::Context::default();
//! let rt = Runtime::new(&ctx, Material::new(egui::Theme::Light))?;
//! let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
//!     rt.pass(ui, |ui| {
//!         if ui.mineral(PushButton::new("save", "Save")).clicked() {
//!             // save
//!         }
//!     });
//! });
//! output.textures_delta.clear();
//! # Ok::<(), egui_mineral::runtime::InitError>(())
//! ```
// Feature-gated items: docs.rs (all features) links when the feature is off.
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
pub mod extras;
/// Font and icon installation.
pub mod fonts;
mod material;
mod motion;
pub mod navigation;
pub mod prelude;
pub mod scroll;
pub mod selection;
pub mod spec;
#[cfg(test)]
mod test_support;
pub mod text_inputs;
/// Theme engine: seeded or custom tokens per brightness and contrast; TOML files with feature `theme-file`.
pub mod theme;
mod tokens;
/// MD3 type scale.
pub mod typography;
mod widget;

pub use egui_material_icons::{MaterialIcon, icons};
pub use egui_mineral;
pub use egui_mineral::egui;
pub use material::{Material, MaterialExt, MaterialUiExt, Rt};
pub use widget::{
    leading::{Leading, LeadingPaint},
    vocab::{IconAction, SelectionResponse, Size},
};

pub(crate) fn tr(i18n: &egui_mineral::i18n::I18n, id: &str) -> String {
    i18n.tr(id, &[])
}

pub(crate) fn tr_or(i18n: &egui_mineral::i18n::I18n, over: Option<&str>, id: &str) -> String {
    over.map_or_else(|| tr(i18n, id), str::to_owned)
}
