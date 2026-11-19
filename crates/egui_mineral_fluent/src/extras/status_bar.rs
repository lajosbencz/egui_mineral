use std::sync::Arc;

use egui::{Color32, Galley, Rangef, Rect, Response, Sense, Stroke, UiBuilder, pos2, text::LayoutJob, vec2};
use egui_mineral::{
    i18n::Direction,
    kit::strip,
    semantic::{self, KeyedSalt, Role, SemanticNode, StableId},
    status::StatusLead,
    widget::{Availability, Interaction, MineralUi, SizeExt as _},
};

use crate::ColorToken::{
    ColorNeutralBackground3, ColorNeutralForeground2, ColorNeutralStroke2, ColorPaletteRedBackground1,
    ColorPaletteRedForeground1, ColorSubtleBackgroundHover, ColorSubtleBackgroundPressed,
};
use crate::{
    Fluent, FluentExt,
    feedback::{ProgressBar, Spinner, SpinnerSize},
    spec::tokens::geometry as g,
    surfaces::Tooltip,
    typography::TypeRamp,
    widget::{focus as ring, paint},
};

// Not in Fluent UI React v9: a desktop status bar on the 4pt grid.
const HEIGHT: f32 = g::SPACING_HORIZONTAL_XXL;
const PAD_X: f32 = g::SPACING_HORIZONTAL_S;
const ICON: f32 = 16.0;
const BAR: f32 = 48.0;
const ICON_GAP: f32 = g::SPACING_HORIZONTAL_XS;
const TEXT: TypeRamp = TypeRamp::Caption1;

pub use egui_mineral::status::{StatusItem, StatusProgress, StatusTone};

// Linear indicators are fixed at 48 wide.
fn lead_width(lead: StatusLead) -> f32 {
    match lead {
        StatusLead::Progress(StatusProgress::Linear(_)) => BAR,
        _ => ICON,
    }
}

/// Result of [`StatusBar::show`].
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct StatusBarResponse {
    /// Bar response.
    pub response: Response,
    /// Index, in insertion order, of the clickable item clicked this frame.
    pub clicked: Option<usize>,
}

/// Desktop status bar: items at the reading start and end of a full-width strip; not in Fluent UI React v9. Show it
/// last in a bottom panel. Start items that do not fit are dropped from the end of the start group, unless
/// [`StatusItem::elide`].
#[derive(Clone, Debug)]
#[must_use = "call .show(..)"]
pub struct StatusBar {
    salt: KeyedSalt,
    label: String,
    // Item and whether it sits at the end.
    items: Vec<(StatusItem, bool)>,
}

impl StatusBar {
    /// `label` names the bar for assistive technology. Item agent keys `<id_salt>-<index>-status-item`.
    pub fn new(id_salt: impl Into<KeyedSalt>, label: impl Into<String>) -> Self {
        Self {
            salt: id_salt.into(),
            label: label.into(),
            items: Vec::new(),
        }
    }

    /// Replaces the `<id_salt>` key prefix.
    pub fn agent_key(mut self, salt: impl Into<KeyedSalt>) -> Self {
        self.salt = salt.into();
        self
    }

    /// Item at the reading start, after the previous ones.
    pub fn item(mut self, item: impl Into<StatusItem>) -> Self {
        self.items.push((item.into(), false));
        self
    }

    /// Item at the reading end, before the previous trailing ones.
    pub fn trailing(mut self, item: impl Into<StatusItem>) -> Self {
        self.items.push((item.into(), true));
        self
    }

    /// Allocates the full available width.
    pub fn show(self, ui: &mut MineralUi<'_, Fluent>) -> StatusBarResponse {
        let rt = ui.rt();
        let t = ui.tokens();
        let dir = rt.i18n().direction();
        let id = ui.make_persistent_id(self.salt.id_salt());
        let w = ui.available_width();
        let (rect, _) = ui.allocate_exact_size(vec2(w, HEIGHT), Sense::hover());
        let response = ui.interact(rect, id, Sense::hover());
        let p = ui.painter();
        p.rect_filled(rect, 0, t.color(ColorNeutralBackground3));
        p.hline(
            rect.x_range(),
            rect.top(),
            Stroke::new(g::STROKE_WIDTH_THIN, t.color(ColorNeutralStroke2)),
        );

        let content_of = |it: &StatusItem| content_color(it, &t);
        let galley = |text: &str, color: Color32, room: Option<f32>| -> Arc<Galley> {
            let mut job = LayoutJob::single_section(text.to_owned(), TEXT.format(ui.ctx(), color));
            if let Some(room) = room {
                job.wrap = egui::text::TextWrapping::truncate_at_width(room);
            }
            ui.painter().layout_job(job)
        };
        let mut galleys: Vec<Arc<Galley>> = self
            .items
            .iter()
            .map(|(it, _)| galley(it.text(), content_of(it), None))
            .collect();
        let chrome = |i: usize| 2.0 * PAD_X + self.items[i].0.lead().map_or(0.0, |l| lead_width(l) + ICON_GAP);
        let measured: Vec<strip::Item> = self
            .items
            .iter()
            .zip(&galleys)
            .enumerate()
            .map(|(i, ((it, at_end), g))| {
                strip::Item::new(chrome(i) + g.size().x)
                    .chrome(chrome(i))
                    .at_end(*at_end)
                    .elide(it.elision().is_some())
            })
            .collect();
        let slots: Vec<Option<Rect>> = strip::place(rect.width(), &measured)
            .into_iter()
            .zip(&self.items)
            .enumerate()
            .map(|(i, (placed, (item, at_end)))| {
                let placed = placed?;
                if let Some(room) = placed.elided_room {
                    galleys[i] = galley(item.text(), content_of(item), Some(room));
                }
                Some(slot(rect, dir, *at_end, placed.offset, placed.width))
            })
            .collect();

        let mut clicked = None;
        for (i, text) in galleys.into_iter().enumerate() {
            let Some(r) = slots[i] else { continue };
            if self.paint_item(ui, id, i, r, text) {
                clicked = Some(i);
            }
        }
        semantic::emit(ui.ctx(), Some(response.id), rect, || {
            SemanticNode::new(StableId::of(&self.salt, "status-bar"), Role::Status, &self.label)
        });
        StatusBarResponse { response, clicked }
    }

    // Paints item `i` at `r`; whether it was clicked.
    fn paint_item(&self, ui: &mut MineralUi<'_, Fluent>, id: egui::Id, i: usize, r: Rect, text: Arc<Galley>) -> bool {
        let rt = ui.rt();
        let t = ui.tokens();
        let dir = rt.i18n().direction();
        let item = &self.items[i].0;
        let content = content_color(item, &t);
        match item.tone_kind() {
            StatusTone::Custom { container, .. } => {
                ui.painter().rect_filled(r, 0, container);
            }
            StatusTone::Error => {
                ui.painter().rect_filled(r, 0, t.color(ColorPaletteRedBackground1));
            }
            _ => {}
        }
        let sense = if item.is_clickable() {
            Sense::click()
        } else {
            Sense::hover()
        };
        let resp = ui.interact(r, id.with(i), sense);
        if item.is_clickable() {
            let state = Interaction::of(&resp, Availability::Enabled);
            if state.pressed {
                ui.painter().rect_filled(r, 0, t.color(ColorSubtleBackgroundPressed));
            } else if state.hovered {
                ui.painter().rect_filled(r, 0, t.color(ColorSubtleBackgroundHover));
            }
            if state.focused {
                ring::outline(ui.painter(), r.shrink(g::STROKE_WIDTH_THICK), 0.into(), &t);
            }
        }
        let inner = Rect::from_x_y_ranges(r.left() + PAD_X..=r.right() - PAD_X, r.y_range());
        let mut x = dir.start_x(inner, Rangef::point(0.0));
        if let Some(lead) = item.lead() {
            let c = pos2(dir.start_x(inner, Rangef::point(lead_width(lead) / 2.0)), r.center().y);
            match lead {
                StatusLead::Icon(icon) => paint::icon(ui.painter(), c, icon, false, ICON, content),
                StatusLead::Progress(progress) => self.progress(ui, i, item.text(), progress, c),
                _ => {}
            }
            x = dir.start_x(inner, Rangef::point(lead_width(lead) + ICON_GAP));
        }
        let gx = if dir == Direction::Rtl { x - text.size().x } else { x };
        ui.painter()
            .galley(pos2(gx, r.center().y - text.size().y / 2.0), text, content);
        if let Some(tip) = item.tooltip_text() {
            Tooltip::new(self.salt.child((i, "tooltip")), tip).show(rt, &resp);
        }
        let role = if item.is_clickable() { Role::Button } else { Role::Label };
        semantic::emit(ui.ctx(), Some(resp.id), r, || {
            SemanticNode::new(StableId::of(&self.salt, (i, "status-item")), role, item.text())
        });
        item.is_clickable() && resp.clicked()
    }

    fn progress(
        &self,
        ui: &mut MineralUi<'_, Fluent>,
        i: usize,
        text: &str,
        progress: StatusProgress,
        center: egui::Pos2,
    ) {
        let rt = ui.rt();
        let salt = self.salt.child((i, "progress"));
        let key = StableId::of(&self.salt, (i, "progress"));
        match progress {
            // Fluent's Spinner has no determinate form.
            StatusProgress::Circular(_) => {
                let rect = Rect::from_center_size(center, vec2(ICON, ICON));
                let mut child = ui.new_child(UiBuilder::new().max_rect(rect));
                rt.ui(&mut child).mineral(
                    Spinner::new(salt)
                        .size(SpinnerSize::ExtraTiny)
                        .label(text)
                        .label_visible(false)
                        .agent_key(key),
                );
            }
            StatusProgress::Linear(value) => {
                let rect = Rect::from_center_size(center, vec2(BAR, ICON));
                let bar = match value {
                    Some(v) => ProgressBar::determinate(salt, v),
                    None => ProgressBar::indeterminate(salt),
                };
                let mut child = ui.new_child(UiBuilder::new().max_rect(rect));
                child.add_space((ICON - crate::spec::defaults::progress_bar::THICKNESS_MEDIUM) / 2.0);
                rt.ui(&mut child)
                    .mineral(bar.label(text).agent_key(key).preferred_width(BAR));
            }
            _ => {}
        }
    }
}

// `w` wide at logical offset `from` of the start (or end) edge.
fn slot(bar: Rect, dir: Direction, at_end: bool, from: f32, w: f32) -> Rect {
    let d = if at_end { dir.reversed() } else { dir };
    let x = d.start_x(bar, Rangef::new(from, from + w));
    Rect::from_x_y_ranges(x..=x + w, bar.y_range())
}

fn content_color(item: &StatusItem, t: &crate::theme::Tokens) -> Color32 {
    match item.tone_kind() {
        StatusTone::Custom { content, .. } => content,
        StatusTone::Error => t.color(ColorPaletteRedForeground1),
        _ => t.color(ColorNeutralForeground2),
    }
}
