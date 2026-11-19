use egui::Key;
use egui_mineral_fluent::extras::{SplitPane, StatusBar, StatusItem, StatusProgress};

use crate::support::{FluentHarness, Fui, harness};

fn pane(disabled: bool) -> impl FnMut(&mut Fui<'_>, &mut f32) {
    move |ui, fraction| {
        *fraction = SplitPane::new("sp", "Panes")
            .disabled(disabled)
            .show(ui, |ui| ui.label("A"), |ui| ui.label("B"))
            .fraction;
    }
}

#[test]
fn split_pane_divider_moves_by_key() {
    let mut h = harness(0.0, pane(false));
    h.run();
    let before = h.state().value;
    let handle = h.widget("sp-splitter");
    h.ctx.memory_mut(|m| m.request_focus(handle));
    h.press(Key::ArrowRight);
    assert!(h.state().value > before, "{} > {before}", h.state().value);
}

#[test]
fn disabled_split_pane_ignores_keys() {
    let mut h = harness(0.0, pane(true));
    h.run();
    let before = h.state().value;
    let handle = h.widget("sp-splitter");
    h.ctx.memory_mut(|m| m.request_focus(handle));
    h.press(Key::ArrowRight);
    assert_eq!(h.state().value, before);
}

fn narrow<R>(ui: &mut Fui<'_>, width: f32, add: impl FnOnce(&mut Fui<'_>) -> R) -> R {
    let rt = ui.rt();
    ui.allocate_ui(egui::vec2(width, 100.0), |ui| add(&mut rt.ui(ui))).inner
}

// Headless text has no advance, so the lead indicators and padding carry the widths.
fn status_bar(ui: &mut Fui<'_>, width: f32) -> Option<usize> {
    narrow(ui, width, |ui| {
        StatusBar::new("status", "Status")
            .item(StatusItem::new("Indexing").progress(StatusProgress::Spinner))
            .trailing(
                StatusItem::new("Ln 4, Col 2")
                    .progress(StatusProgress::Bar(None))
                    .clickable(true),
            )
            .item(
                StatusItem::new("Encoding")
                    .progress(StatusProgress::Bar(Some(0.2)))
                    .clickable(true),
            )
            .item(StatusItem::new("Dropped").clickable(true))
            .show(ui)
            .clicked
    })
}

#[test]
fn status_bar_reports_clicks_and_drops_what_does_not_fit() {
    let mut h = harness(None, |ui, clicked| {
        *clicked = clicked.or(status_bar(ui, 150.0));
    });
    h.run();
    let bar = h.rect("status-status-bar");
    let (start, end) = (h.rect("status-0-status-item"), h.rect("status-1-status-item"));
    assert!((start.left() - bar.left()).abs() < 0.5 && (end.right() - bar.right()).abs() < 0.5);
    assert!(
        start.contains_rect(h.rect("status-0-progress")),
        "progress inside its item"
    );
    assert_eq!(h.node("status-0-progress").name, "Indexing", "named by the item text");
    assert!(
        h.tree().find_key("status-3-status-item").is_none(),
        "overflow is dropped"
    );
    h.click("status-1-status-item");
    assert_eq!(h.state().value, Some(1));
}

#[test]
fn status_item_bar_progress_exposes_its_value() {
    let mut h = harness((), |ui, ()| {
        narrow(ui, 300.0, |ui| {
            StatusBar::new("status", "Status")
                .item(StatusItem::new("Upload").progress(StatusProgress::Bar(Some(0.5))))
                .show(ui);
        });
    });
    h.run();
    let progress = h.node("status-0-progress");
    assert_eq!(progress.state.range.map(|r| r.value), Some(0.5));
    assert!(h.rect("status-0-status-item").contains_rect(progress.rect));
}
