use crate::tokens::ShadowTint;
use std::sync::Arc;

use egui::{Color32, CornerRadius, Galley, Painter, Pos2, Rect, Stroke, TextFormat, pos2};
use egui_mineral::{
    icon::IconStyle,
    runtime::Core,
    shortcut::{self, Mnemonic},
};

use crate::{Icon, theme::Elevation, theme::Tokens};

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

pub(crate) fn icon(p: &Painter, center: Pos2, icon: Icon, filled: bool, size: f32, color: Color32) {
    p.add(egui_mineral::kit::icon::shape(
        p.ctx(),
        center,
        icon.glyph(IconStyle::filled_if(filled)),
        size,
        color,
        0.0,
    ));
}

pub(crate) fn shadow(p: &Painter, rect: Rect, r: CornerRadius, level: Elevation, t: &Tokens) {
    for layer in t.shadow(level, ShadowTint::Neutral).layers() {
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
