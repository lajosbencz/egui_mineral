//! Scroll containers shared by stylekits.

use egui::{ScrollArea, UiBuilder, scroll_area::ScrollAreaOutput, style::ScrollStyle};

use crate::{semantic::KeyedSalt, widget::MineralUi};

/// Bar styles of a [`page`] container.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PageBars {
    /// Bar of the page itself.
    pub page: ScrollStyle,
    /// Bars of the scroll areas inside the page.
    pub nested: ScrollStyle,
}

/// Vertical page scroll container filling the available space with the `page` bar; `add` runs with the `nested`
/// bar style, so scroll areas inside stay distinct from the page bar. `id_salt` also scopes every id inside, so
/// per-position state (auto ids) never leaks between pages.
pub fn page<K, R>(
    ui: &mut MineralUi<'_, K>,
    id_salt: impl Into<KeyedSalt>,
    bars: PageBars,
    add: impl FnOnce(&mut MineralUi<'_, K>) -> R,
) -> ScrollAreaOutput<R> {
    let PageBars { page, nested } = bars;
    let rt = ui.rt();
    let id_salt = id_salt.into().id_salt();
    ui.scope_builder(UiBuilder::new().id_salt(id_salt), |ui| {
        ui.spacing_mut().scroll = page;
        ScrollArea::vertical()
            .id_salt(id_salt)
            .auto_shrink([false; 2])
            .show(ui, |ui| {
                ui.spacing_mut().scroll = nested;
                add(&mut rt.ui(ui))
            })
    })
    .inner
}
