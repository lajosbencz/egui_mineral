use egui::Key;
use egui_mineral_fluent::extras::SplitPane;

use crate::support::{FluentHarness, harness};

fn pane(disabled: bool) -> impl FnMut(&mut crate::support::Fui<'_>, &mut f32) {
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
