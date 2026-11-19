//! Scroll bar treatment. The page container keeps a solid bar in its own gutter, so it never covers
//! content; every other scroll area (the style's default `spacing.scroll`) gets a thin floating bar
//! inset from its container edge, so nested bars never sit on the page bar's edge.
//! Metrics are Fluent spacing tokens; the handle uses the widget foreground.

use crate::{Fluent, spec::tokens::geometry as g};
use egui::{ScrollArea, UiBuilder, scroll_area::ScrollAreaOutput, style::ScrollStyle};
use egui_mineral::semantic::KeyedSalt;
use egui_mineral::widget::MineralUi;

pub(crate) fn page_style() -> ScrollStyle {
    ScrollStyle {
        bar_width: g::SPACING_HORIZONTAL_S,
        handle_min_length: g::SPACING_HORIZONTAL_L,
        bar_inner_margin: g::SPACING_HORIZONTAL_XS,
        bar_outer_margin: g::SPACING_HORIZONTAL_XXS,
        foreground_color: true,
        ..ScrollStyle::solid()
    }
}

pub(crate) fn nested_style() -> ScrollStyle {
    ScrollStyle {
        bar_width: g::SPACING_HORIZONTAL_S,
        floating_width: g::SPACING_HORIZONTAL_XXS,
        handle_min_length: g::SPACING_HORIZONTAL_L,
        bar_outer_margin: g::SPACING_HORIZONTAL_XXS,
        foreground_color: true,
        ..ScrollStyle::floating()
    }
}

/// Vertical page scroll container filling the available space with a solid page bar; `add` runs
/// with floating nested bars. `id_salt` also scopes every id inside, so per-position state never
/// leaks between pages.
pub fn page<R>(
    ui: &mut MineralUi<'_, Fluent>,
    id_salt: impl Into<KeyedSalt>,
    add: impl FnOnce(&mut MineralUi<'_, Fluent>) -> R,
) -> ScrollAreaOutput<R> {
    let rt = ui.rt();
    let id_salt = id_salt.into().id_salt();
    ui.scope_builder(UiBuilder::new().id_salt(id_salt), |ui| {
        ui.spacing_mut().scroll = page_style();
        ScrollArea::vertical()
            .id_salt(id_salt)
            .auto_shrink([false; 2])
            .show(ui, |ui| {
                ui.spacing_mut().scroll = nested_style();
                add(&mut rt.ui(ui))
            })
    })
    .inner
}
