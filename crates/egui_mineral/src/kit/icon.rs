//! Icon geometry shared by the stylekits.

use egui::{Color32, Context, Pos2, emath::Rot2, epaint::TextShape, vec2};

use crate::icon::Glyph;

/// Shape of `glyph` centered on `center` by its ink, not its line box, rotated by `angle` radians around the center.
/// The stylekit adds it to its painter.
pub fn shape(ctx: &Context, center: Pos2, glyph: Glyph, size: f32, color: Color32, angle: f32) -> TextShape {
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
