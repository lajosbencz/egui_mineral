use egui_mineral_fluent::{
    FluentExt as _,
    extras::{SplitPane, StatusBar},
    spec::ColorToken,
    spec::tokens::geometry,
};

use crate::support::render;

#[test]
fn split_pane_divider_is_a_thin_neutral_line_centered_in_the_handle() {
    let p = render(|ui| {
        SplitPane::new("sp", "Panes").show(ui, |ui| ui.label("A"), |ui| ui.label("B"));
    });
    let handle =
        p.rt.tree(egui::ViewportId::ROOT)
            .expect("semantics collected")
            .find_key("sp-splitter")
            .expect("splitter")
            .rect;
    let lines = p.segments();
    let color = p.rt.tokens().color(ColorToken::ColorNeutralStroke2);
    let divider: Vec<_> = lines.iter().filter(|(_, s)| s.color == color).collect();
    assert_eq!(divider.len(), 1, "{lines:?}");
    let ([a, b], stroke) = divider[0];
    assert_eq!(stroke.width, geometry::STROKE_WIDTH_THIN);
    assert_eq!(a.x, b.x, "vertical line between side by side panes");
    assert!(handle.x_range().contains(a.x), "{handle:?} {a:?}");
    assert_eq!(b.y - a.y, handle.height());
}

#[test]
fn status_bar_is_a_full_width_row_on_neutral_background_3() {
    let p = render(|ui| {
        StatusBar::new("status", "Status").item("Ready").show(ui);
    });
    let bar = p.one(p.rt.tokens().color(ColorToken::ColorNeutralBackground3));
    assert_eq!(bar.width(), crate::support::SCREEN.x);
    assert_eq!(bar.height(), 24.0);
}
