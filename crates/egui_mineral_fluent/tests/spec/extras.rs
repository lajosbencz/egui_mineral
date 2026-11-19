use egui_mineral_fluent::{ColorToken, FluentExt as _, extras::SplitPane, spec::tokens::geometry};

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
