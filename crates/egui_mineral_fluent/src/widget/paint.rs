use std::sync::Arc;

use egui::{Color32, CornerRadius, FontId, Galley, Painter, Pos2, Rect, Stroke, TextFormat, pos2, vec2};
use egui_mineral::{
    runtime::Core,
    shortcut::{self, Mnemonic},
};

use crate::{FluentIcon, theme::Elevation, theme::Tokens};

pub(crate) fn galley(
    core: &Core,
    p: &Painter,
    text: &str,
    format: &TextFormat,
    mnemonic: Option<Mnemonic>,
) -> Arc<Galley> {
    p.layout_job(shortcut::mnemonic_job(
        core,
        text,
        format,
        mnemonic,
        Stroke::new(1.0, format.color),
    ))
}

// Glyph centered on `center` by its ink, not its line box.
pub(crate) fn icon(p: &Painter, center: Pos2, icon: FluentIcon, filled: bool, size: f32, color: Color32) {
    let g = p.layout_no_wrap(
        icon.glyph(filled).to_string(),
        FontId::new(size, crate::fonts::icon_family()),
        color,
    );
    let mid_y = g
        .rows
        .iter()
        .flat_map(|row| {
            row.glyphs
                .iter()
                .map(move |gl| row.pos.y + gl.pos.y + gl.uv_rect.offset.y + gl.uv_rect.size.y / 2.0)
        })
        .next()
        .unwrap_or_else(|| g.size().y / 2.0);
    p.galley(center - vec2(g.size().x / 2.0, mid_y), g, color);
}

pub(crate) fn shadow(p: &Painter, rect: Rect, r: CornerRadius, level: Elevation, brand: bool, t: &Tokens) {
    for layer in t.shadow(level, brand) {
        p.add(layer.as_shape(rect, r));
    }
}

// Horizontal three-stop gradient; `colors` are premultiplied.
pub(crate) fn gradient_h(p: &Painter, rect: Rect, colors: [Color32; 3]) {
    let mut mesh = egui::Mesh::default();
    for (x, c) in [rect.left(), rect.center().x, rect.right()].into_iter().zip(colors) {
        mesh.colored_vertex(pos2(x, rect.top()), c);
        mesh.colored_vertex(pos2(x, rect.bottom()), c);
    }
    for i in 0..2_u32 {
        let b = i * 2;
        mesh.add_triangle(b, b + 1, b + 2);
        mesh.add_triangle(b + 1, b + 3, b + 2);
    }
    p.add(egui::Shape::mesh(mesh));
}
