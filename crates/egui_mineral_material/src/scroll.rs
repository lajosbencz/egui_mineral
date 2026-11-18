//! Scroll bar treatment. The page container keeps a solid bar in its own gutter, so it never covers
//! content; every other scroll area (installed as the default `spacing.scroll`) gets a thin floating
//! bar inset from its container edge, so nested bars never sit on the page bar's edge.
//! Handles use the widget foreground (the track is `extreme_bg_color`, same as the idle widget fill).
//! Bar metrics follow Compose Multiplatform desktop `defaultScrollbarStyle` (8dp thick, 16dp min handle).

use crate::Material;
use egui::{ScrollArea, UiBuilder, scroll_area::ScrollAreaOutput, style::ScrollStyle};
use egui_mineral::semantic::KeyedSalt;
use egui_mineral::widget::MineralUi;

const THICKNESS: f32 = 8.0;
const MIN_HANDLE: f32 = 16.0;
const NESTED_IDLE: f32 = 2.0;

pub(crate) fn page_style() -> ScrollStyle {
    ScrollStyle {
        bar_width: THICKNESS,
        handle_min_length: MIN_HANDLE,
        bar_inner_margin: crate::adaptive::grid(2),
        bar_outer_margin: crate::adaptive::grid(1),
        foreground_color: true,
        ..ScrollStyle::solid()
    }
}

pub(crate) fn nested_style() -> ScrollStyle {
    ScrollStyle {
        bar_width: THICKNESS,
        floating_width: NESTED_IDLE,
        handle_min_length: MIN_HANDLE,
        bar_outer_margin: crate::adaptive::grid(1),
        foreground_color: true,
        ..ScrollStyle::floating()
    }
}

/// Vertical page scroll container filling the available space with a solid page bar; `add` runs
/// with floating nested bars, so scroll areas inside it stay distinct from the page bar. `id_salt` also
/// scopes every id inside, so per-position state (auto ids) never leaks between pages.
pub fn page<R>(
    ui: &mut MineralUi<'_, Material>,
    id_salt: impl Into<KeyedSalt>,
    add: impl FnOnce(&mut MineralUi<'_, Material>) -> R,
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
