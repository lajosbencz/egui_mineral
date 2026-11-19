use crate::Material;
use crate::MaterialExt;
use egui_mineral::runtime::Core;
use egui_mineral::widget::MineralUi;
use std::sync::Arc;

use egui::{
    Color32, CornerRadius, FontId, Galley, Id, Painter, Pos2, Rangef, Rect, Response, Sense, Stroke, TextFormat, Ui,
    Vec2, pos2, text::LayoutJob, vec2,
};
use egui_mineral::widget::{Availability, Interaction};
use egui_mineral::{
    kit,
    shortcut::{self, Mnemonic},
    widget::WidgetFrame,
};

use crate::tokens::Tokens;
use crate::widget::{common, indication};
use crate::{Icon, spec::TypeScale};

pub(crate) fn label_job(
    core: &Core,
    text: &str,
    font: FontId,
    color: Color32,
    mnemonic: Option<Mnemonic>,
) -> LayoutJob {
    format_job(core, text, &TextFormat::simple(font, color), mnemonic)
}

pub(crate) fn format_job(core: &Core, text: &str, plain: &TextFormat, mnemonic: Option<Mnemonic>) -> LayoutJob {
    shortcut::mnemonic_job(core, text, plain, mnemonic, Stroke::new(1.0, plain.color))
}

pub(crate) fn label_galley(
    core: &Core,
    p: &Painter,
    text: &str,
    font: FontId,
    color: Color32,
    mnemonic: Option<Mnemonic>,
) -> Arc<Galley> {
    p.layout_job(label_job(core, text, font, color, mnemonic))
}

pub(crate) fn type_galley(
    core: &Core,
    p: &Painter,
    text: &str,
    scale: TypeScale,
    color: Color32,
    mnemonic: Option<Mnemonic>,
) -> Arc<Galley> {
    p.layout_job(format_job(core, text, &scale.format(p.ctx(), color), mnemonic))
}

#[derive(Clone, Copy)]
pub(crate) struct Press {
    pub clicked: bool,
    pub interaction: Interaction,
}

pub(crate) fn press(ui: &MineralUi<'_, Material>, frame: &WidgetFrame<'_>, availability: Availability) -> Press {
    kit::input::hover_cursor(ui.core(), &frame.response, availability);
    Press {
        clicked: frame.response.clicked(),
        interaction: Interaction::of(&frame.response, availability),
    }
}

pub(crate) fn flip(checked: &mut Option<&mut bool>, selected: &mut Option<bool>) -> bool {
    let Some(c) = checked.as_deref_mut() else {
        return false;
    };
    *c = !*c;
    *selected = Some(*c);
    true
}

pub(crate) fn action_slot(dir: egui_mineral::i18n::Direction, bar: Rect, from_start: f32) -> Rect {
    let size = crate::adaptive::MIN_INTERACTIVE_DP;
    let x = dir.start_x(bar, Rangef::new(from_start, from_start + size));
    Rect::from_min_size(pos2(x, bar.center().y - size / 2.0), vec2(size, size))
}

pub(crate) fn close_button(ui: &mut MineralUi<'_, Material>, salt: egui_mineral::semantic::KeyedSalt) -> bool {
    let label = crate::tr(ui.i18n(), "mineral-close");
    ui.mineral(crate::actions::IconButton::new(salt, ui.icons().close, label))
        .clicked()
}

pub(crate) fn small_icon_metrics() -> (Vec2, f32) {
    crate::actions::icon_button::metrics(crate::Size::Small, crate::actions::IconButtonWidth::Default)
}

pub(crate) struct IconPart {
    pub icon: Icon,
    pub icon_size: f32,
    pub layer: Vec2,
    pub radius: CornerRadius,
    pub color: Color32,
    pub availability: Availability,
}

impl IconPart {
    pub(crate) fn small(icon: Icon, color: Color32, availability: Availability) -> Self {
        let (layer, icon_size) = small_icon_metrics();
        Self {
            icon,
            icon_size,
            layer,
            radius: CornerRadius::from(layer.min_elem() / 2.0),
            color,
            availability,
        }
    }
}

pub(crate) fn paint_icon_part(
    ui: &MineralUi<'_, Material>,
    response: &Response,
    center: Pos2,
    part: &IconPart,
    t: &Tokens,
) {
    let layer = Rect::from_center_size(center, part.layer);
    let i = Interaction::of(response, part.availability);
    indication::paint(ui, response, layer, part.radius, part.color, i, t);
    common::icon(ui.painter(), center, part.icon, part.icon_size, part.color);
}

pub(crate) fn icon_part(
    ui: &mut MineralUi<'_, Material>,
    target: Rect,
    id: Id,
    sense: Sense,
    part: &IconPart,
) -> Response {
    let rt = ui.rt();
    let r = kit::input::interact(&rt.ui(ui), target, id, sense, part.availability);
    paint_icon_part(ui, &r, target.center(), part, &rt.tokens());
    r
}

pub(crate) fn icon_rotated(p: &Painter, center: Pos2, icon: Icon, size: f32, color: egui::Color32, angle: f32) {
    p.add(crate::widget::common::icon_shape(p, center, icon, size, color, angle));
}

#[cfg(feature = "extras")]
pub(crate) fn row<R>(
    ui: &mut MineralUi<'_, Material>,
    dir: egui_mineral::i18n::Direction,
    height: f32,
    add: impl FnOnce(&mut MineralUi<'_, Material>) -> R,
) -> egui::InnerResponse<R> {
    let size = egui::vec2(ui.available_size_before_wrap().x, height);
    let rt = ui.rt();
    ui.allocate_ui_with_layout(size, dir.row(), |ui| add(&mut rt.ui(ui)))
}

pub(crate) fn type_wrapped(
    p: &Painter,
    text: &str,
    scale: TypeScale,
    color: Color32,
    max_width: f32,
    max_rows: usize,
) -> Arc<Galley> {
    let mut job = LayoutJob::single_section(text.to_owned(), scale.format(p.ctx(), color));
    job.wrap = egui::epaint::text::TextWrapping {
        max_width: max_width.max(0.0),
        max_rows: if max_rows == 0 { usize::MAX } else { max_rows },
        break_anywhere: false,
        overflow_character: Some('\u{2026}'),
    };
    p.layout_job(job)
}

pub(crate) fn type_label(ui: &mut Ui, text: &str, scale: TypeScale, color: Color32, max_rows: usize) -> Response {
    let g = type_wrapped(ui.painter(), text, scale, color, ui.available_width(), max_rows);
    let (rect, response) = ui.allocate_exact_size(g.size(), egui::Sense::hover());
    ui.painter().galley(rect.min, g, Color32::PLACEHOLDER);
    response
}

// Compose FirstBaseline
pub(crate) fn first_baseline(g: &Galley) -> f32 {
    g.rows.first().map_or(0.0, row_baseline)
}

// Compose LastBaseline
pub(crate) fn last_baseline(g: &Galley) -> f32 {
    g.rows.last().map_or(0.0, row_baseline)
}

fn row_baseline(row: &egui::epaint::text::PlacedRow) -> f32 {
    row.pos.y + row.glyphs.first().map_or(row.height(), |g| g.pos.y)
}
