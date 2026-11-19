//! Shapes shared by stylekits; the core builds them, kits add them with their painter.

use egui::{Rect, Response, Shape, Ui, vec2};

/// Close glyph of an embedded window title bar: hover/press disc, cross, focus ring. `visual` is the
/// icon-sized box, `response` the larger hit target; colors come from `ui.style()`.
pub fn window_close(ui: &Ui, visual: Rect, response: &Response) -> Vec<Shape> {
    let visuals = ui.style().interact(response);
    let c = visual.center();
    let r = visual.width() / 2.0;
    let mut shapes = Vec::with_capacity(4);
    if response.hovered() || response.is_pointer_button_down_on() {
        shapes.push(Shape::circle_filled(c, r, visuals.weak_bg_fill));
    }
    let arm = visual.width() / 4.0;
    for d in [vec2(arm, arm), vec2(arm, -arm)] {
        shapes.push(Shape::line_segment([c - d, c + d], visuals.fg_stroke));
    }
    if response.has_focus() {
        shapes.push(Shape::circle_stroke(c, r + 2.0, ui.visuals().selection.stroke));
    }
    shapes
}
