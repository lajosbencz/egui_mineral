use egui::{Align2, FontId, Sense, Stroke, StrokeKind, vec2};
use egui_mineral_fluent::{
    ColorToken, FluentExt,
    data::Text,
    theme::{Elevation, Radius, ShadowTint},
    typography::TypeRamp,
};

use super::heading;
use crate::Fui;

const SWATCH: egui::Vec2 = vec2(150.0, 36.0);
const TILE: egui::Vec2 = vec2(96.0, 64.0);

pub(crate) fn show(ui: &mut Fui<'_>) {
    let t = ui.tokens();
    heading(ui, "h-ramp", "Type ramp");
    for (i, ramp) in TypeRamp::ALL.iter().copied().enumerate() {
        ui.mineral(
            Text::new(("ramp", i), format!("{ramp:?}"))
                .ramp(ramp)
                .wrap(false)
                .truncate(true),
        );
    }
    heading(ui, "h-shadows", "Shadows");
    let rt = ui.rt();
    ui.horizontal_wrapped(|ui| {
        let ui = &mut rt.ui(ui);
        ui.spacing_mut().item_spacing = vec2(24.0, 24.0);
        for &level in Elevation::ALL {
            let (rect, _) = ui.allocate_exact_size(TILE, Sense::hover());
            let r = Radius::Medium.on(rect);
            for layer in t.shadow(level, ShadowTint::Neutral).layers() {
                ui.painter().add(layer.as_shape(rect, r));
            }
            ui.painter()
                .rect_filled(rect, r, t.color(ColorToken::ColorNeutralBackground1));
            label(
                ui,
                rect,
                &format!("{level:?}"),
                t.color(ColorToken::ColorNeutralForeground1),
            );
        }
    });
    heading(ui, "h-radii", "Corner radii");
    let rt = ui.rt();
    ui.horizontal_wrapped(|ui| {
        let ui = &mut rt.ui(ui);
        for &radius in Radius::ALL {
            let (rect, _) = ui.allocate_exact_size(TILE, Sense::hover());
            ui.painter().rect(
                rect,
                radius.on(rect),
                t.color(ColorToken::ColorBrandBackground2),
                Stroke::new(1.0, t.color(ColorToken::ColorBrandStroke1)),
                StrokeKind::Inside,
            );
            label(
                ui,
                rect,
                &format!("{radius:?}"),
                t.color(ColorToken::ColorNeutralForeground1),
            );
        }
    });
    heading(ui, "h-colors", "Alias colors");
    let rt = ui.rt();
    ui.horizontal_wrapped(|ui| {
        let ui = &mut rt.ui(ui);
        ui.spacing_mut().item_spacing = vec2(4.0, 4.0);
        for &token in ColorToken::ALL {
            let (rect, _) = ui.allocate_exact_size(SWATCH, Sense::hover());
            if !ui.is_rect_visible(rect) {
                continue;
            }
            let color = t.color(token);
            ui.painter().rect(
                rect,
                2.0,
                color,
                Stroke::new(1.0, t.color(ColorToken::ColorNeutralStroke2)),
                StrokeKind::Inside,
            );
            let name = format!("{token:?}");
            let ink = if color.intensity() > 0.5 || color.a() < 128 {
                t.color(ColorToken::ColorNeutralForeground1Static)
            } else {
                t.color(ColorToken::ColorNeutralForegroundInverted)
            };
            label(ui, rect, name.trim_start_matches("Color"), ink);
        }
    });
}

fn label(ui: &Fui<'_>, rect: egui::Rect, text: &str, color: egui::Color32) {
    ui.painter().with_clip_rect(rect).text(
        rect.center(),
        Align2::CENTER_CENTER,
        text,
        FontId::proportional(10.0),
        color,
    );
}
