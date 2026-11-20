//! Shapes and text layout jobs shared by stylekits; the core builds them, kits add them with their painter.

use egui::{
    Color32, Context, Pos2, Rect, Response, Shape, TextFormat, Ui, emath::Rot2, epaint::TextShape, text::LayoutJob,
    vec2,
};

use crate::icon::Glyph;

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

/// Glyph drawn centered on a point by its ink, not its line box.
#[derive(Clone, Copy, Debug, PartialEq)]
#[must_use = "build a shape with .shape(..)"]
pub struct GlyphShape {
    glyph: Glyph,
    size: f32,
    color: Color32,
    angle: f32,
}

impl GlyphShape {
    /// `glyph` at `size` points in `color`, unrotated.
    pub const fn new(glyph: Glyph, size: f32, color: Color32) -> Self {
        Self {
            glyph,
            size,
            color,
            angle: 0.0,
        }
    }

    /// Rotation around the center, radians; default 0.
    #[track_caller]
    pub fn angle(mut self, angle: f32) -> Self {
        self.angle = crate::kit::sanitize::finite(angle, "GlyphShape::angle").unwrap_or(0.0);
        self
    }

    /// Text shape centered on `center`; the stylekit adds it to its painter.
    pub fn shape(self, ctx: &Context, center: Pos2) -> TextShape {
        let Self {
            glyph,
            size,
            color,
            angle,
        } = self;
        let galley = ctx.fonts_mut(|f| f.layout(glyph.ch().to_string(), glyph.font_id(size), color, f32::INFINITY));
        let mid_y = galley
            .rows
            .iter()
            .flat_map(|row| {
                row.glyphs
                    .iter()
                    .map(move |g| row.pos.y + g.pos.y + g.uv_rect.offset.y + g.uv_rect.size.y / 2.0)
            })
            .next()
            .unwrap_or_else(|| galley.size().y / 2.0);
        let anchor = vec2(galley.size().x / 2.0, mid_y);
        TextShape::new(center - Rot2::from_angle(angle) * anchor, galley, color).with_angle(angle)
    }
}

/// Job of `text` in `format` with every ASCII digit in a cell as wide as the font's widest digit,
/// flush to the cell's end, so a number keeps its width while its digits change. Stands in for
/// OpenType `tnum`, which epaint cannot request.
pub fn tabular_figures(ctx: &Context, text: &str, format: &TextFormat) -> LayoutJob {
    let widths = ctx
        .fonts_mut(|f| ['0', '1', '2', '3', '4', '5', '6', '7', '8', '9'].map(|d| f.glyph_width(&format.font_id, d)));
    let cell = widths.iter().copied().fold(0.0, f32::max);
    let mut job = LayoutJob::default();
    let mut buf = [0; 4];
    for c in text.chars() {
        let pad = c
            .to_digit(10)
            .filter(|_| c.is_ascii_digit())
            .and_then(|d| widths.get(d as usize))
            .map_or(0.0, |w| cell - w);
        job.append(c.encode_utf8(&mut buf), pad, format.clone());
    }
    job
}

#[cfg(test)]
mod tests {
    use egui::{FontId, TextFormat, text::LayoutJob};

    #[test]
    fn digits_share_one_advance() {
        let ctx = crate::test_support::ctx();
        let format = TextFormat::simple(FontId::proportional(14.0), egui::Color32::WHITE);
        let mut widths = Vec::new();
        crate::test_support::run(&ctx, egui::RawInput::default(), |ui| {
            let width = |job| ui.fonts_mut(|f| f.layout_job(job)).size().x;
            widths = ["f 111", "f 000", "f 478"]
                .map(|s| width(super::tabular_figures(ui.ctx(), s, &format)))
                .to_vec();
            let plain = LayoutJob::single_section("f 000".to_owned(), format.clone());
            assert!(widths[0] >= width(plain) && widths[0] > 0.0);
        });
        assert!(widths.iter().all(|w| (w - widths[0]).abs() < 0.01), "{widths:?}");
    }
}
