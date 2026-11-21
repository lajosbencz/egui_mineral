//! Fluent 2 specs as data.
//!
//! - [`tokens`] (feature `unstable-spec`): Fluent UI React v9 design tokens from `@fluentui/tokens`, e.g.
//!   [`tokens::geometry::BORDER_RADIUS_MEDIUM`] or [`tokens::motion::DURATION_NORMAL`].
//! - [`defaults`] (feature `unstable-spec`): values Fluent UI React v9 hard-codes in its component
//!   `use*Styles.styles.ts` files instead of reading a token.
//! - Alias colors: [`ColorToken`], resolved per theme against a [`BrandRamp`] ([`tokens::brand::BRAND_WEB`] is the
//!   Fluent web brand).
//!
//! Units and types: lengths as `f32` points, durations as [`std::time::Duration`], colors as
//! [`Color32`](egui::Color32), easings as [`CubicBezier`](egui_mineral::motion::CubicBezier), shadows as [`Shadow`].
//!
//! [`tokens`] comes from `tools/fluent_tokens` at microsoft/fluentui commit [`tokens::SOURCE_COMMIT`]; do not edit
//! it. Regenerate with `just spec`: it fetches the sources pinned in `tools/fluent_tokens/PIN`, including the
//! component style files `defaults` cites, then runs the offline, deterministic generator;
//! `tools/fluent_tokens/fetch.sh <sha>` re-pins. The sources are MIT (Microsoft Corporation); see `NOTICE`.
#![cfg_attr(
    not(feature = "unstable-spec"),
    doc = "",
    doc = "[`tokens`]: https://docs.rs/egui_mineral_fluent/latest/egui_mineral_fluent/spec/tokens/index.html",
    doc = "[`defaults`]: https://docs.rs/egui_mineral_fluent/latest/egui_mineral_fluent/spec/defaults/index.html",
    doc = "[`tokens::SOURCE_COMMIT`]: https://docs.rs/egui_mineral_fluent/latest/egui_mineral_fluent/spec/tokens/constant.SOURCE_COMMIT.html",
    doc = "[`tokens::geometry::BORDER_RADIUS_MEDIUM`]: https://docs.rs/egui_mineral_fluent/latest/egui_mineral_fluent/spec/tokens/geometry/constant.BORDER_RADIUS_MEDIUM.html",
    doc = "[`tokens::motion::DURATION_NORMAL`]: https://docs.rs/egui_mineral_fluent/latest/egui_mineral_fluent/spec/tokens/motion/constant.DURATION_NORMAL.html",
    doc = "[`tokens::brand::BRAND_WEB`]: https://docs.rs/egui_mineral_fluent/latest/egui_mineral_fluent/spec/tokens/brand/constant.BRAND_WEB.html"
)]

/// **Unstable:** outside the semver guarantee; any release may change it.
#[cfg(feature = "unstable-spec")]
pub mod defaults;
#[cfg(not(feature = "unstable-spec"))]
#[expect(
    unreachable_pub,
    reason = "whole module tree; public with the `unstable-spec` feature"
)]
pub(crate) mod defaults;
/// **Unstable:** outside the semver guarantee; any release may change it.
#[cfg(feature = "unstable-spec")]
pub mod tokens;
#[cfg(not(feature = "unstable-spec"))]
#[expect(
    dead_code,
    unreachable_pub,
    reason = "whole module tree; public with the `unstable-spec` feature"
)]
pub(crate) mod tokens;

pub use tokens::{
    brand::{BrandRamp, BrandStep},
    color::ColorToken,
    shadow::{Shadow, ShadowLayer},
};
