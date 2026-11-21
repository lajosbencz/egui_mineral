use crate::Material;
use crate::MaterialExt;
use crate::MaterialUiExt as _;
use egui::{Pos2, Rangef, Rect, Response, Sense, Stroke, Vec2, pos2, vec2};
use egui_mineral::kit::strip;
use egui_mineral::layout::Fraction;
use egui_mineral::semantic::KeyedSalt;
use egui_mineral::status::StatusLead;
use egui_mineral::widget::{Availability, MineralUi};
use egui_mineral::{
    action::{ActionValue, NoAction, Widen},
    i18n::Direction,
    select::ItemKey,
    semantic::{self, Role, SemanticNode, StableId},
};

use crate::communication::{CircularProgressIndicator, LinearProgressIndicator, PlainTooltip};
use crate::spec::{ColorRole, TypeScale, tokens::divider};
use crate::text_inputs::text_field::elided_galley;
use crate::widget::{
    common::{self, Interaction},
    parts,
};

// Non-MD3 desktop status bar on the 4dp grid
const HEIGHT: f32 = 24.0;
const PAD_X: f32 = 8.0;
const ICON: f32 = 16.0;
const LINEAR: f32 = 48.0;
const LINEAR_HEIGHT: f32 = 4.0;
const ICON_GAP: f32 = 4.0;
const CONTAINER: ColorRole = ColorRole::SurfaceContainer;
const CONTENT: ColorRole = ColorRole::OnSurfaceVariant;
const TEXT: TypeScale = TypeScale::BodySmall;

fn height(density: crate::adaptive::Density) -> f32 {
    density.dense_to(HEIGHT, TEXT.line_height().max(ICON) + crate::adaptive::CONTENT_PAD)
}

pub use egui_mineral::status::{StatusItem, StatusProgress};
use egui_mineral::tone::Tone;

// Linear indicators are fixed at 48dp wide.
fn lead_width(lead: StatusLead) -> f32 {
    match lead {
        StatusLead::Progress(StatusProgress::Linear(_)) => LINEAR,
        _ => ICON,
    }
}

/// Result of [`StatusBar::show`].
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct StatusBarResponse<A = NoAction> {
    /// Bar response.
    pub response: Response,
    /// Value of the action item clicked this frame.
    pub clicked: Option<A>,
}

/// Desktop status bar: items at the reading start and end of a full-width strip; non-MD3. Show it
/// last in a bottom panel. Start items that do not fit are dropped from the end of the start group, unless [`StatusItem::elide`].
#[derive(Clone, Debug)]
#[must_use = "call .show(..)"]
pub struct StatusBar<A = NoAction> {
    salt: KeyedSalt,
    label: String,
    // Item, whether it sits at the end, and the value of an action item.
    items: Vec<(StatusItem, bool, Option<A>)>,
    tone: Tone,
    grip: bool,
}

impl StatusBar {
    /// `label` names the bar for assistive technology. Item agent keys `<id_salt>-<index>-status-item`.
    pub fn new(id_salt: impl Into<KeyedSalt>, label: impl Into<String>) -> Self {
        Self {
            salt: id_salt.into(),
            label: label.into(),
            items: Vec::new(),
            tone: Tone::Default,
            grip: false,
        }
    }
}

impl<A: ActionValue> StatusBar<A> {
    crate::widget::setters! {
        /// Replaces the `<id_salt>` key prefix.
        agent_key(salt)
    }

    /// Colors of the whole bar, e.g. a debugging or offline state; items with their own tone keep it.
    /// Default [`Tone::Default`].
    pub fn tone(mut self, tone: Tone) -> Self {
        self.tone = tone;
        self
    }

    /// Window resize grip in the bottom corner at the reading end; a drag resizes the viewport. Default off.
    pub fn grip(mut self, grip: bool) -> Self {
        self.grip = grip;
        self
    }

    /// Item at the reading start, after the previous ones.
    pub fn item(mut self, item: impl Into<StatusItem>) -> Self {
        self.items.push((item.into(), false, None));
        self
    }

    /// Item at the reading end, before the previous trailing ones.
    pub fn trailing(mut self, item: impl Into<StatusItem>) -> Self {
        self.items.push((item.into(), true, None));
        self
    }

    /// [`item`](Self::item) as a focusable button reporting `value` when clicked.
    pub fn action<B: ItemKey>(self, value: B, item: impl Into<StatusItem>) -> StatusBar<B>
    where
        A: Widen<B>,
    {
        let mut next = self.retype();
        next.items.push((item.into(), false, Some(value)));
        next
    }

    /// [`trailing`](Self::trailing) as a focusable button reporting `value` when clicked.
    pub fn trailing_action<B: ItemKey>(self, value: B, item: impl Into<StatusItem>) -> StatusBar<B>
    where
        A: Widen<B>,
    {
        let mut next = self.retype();
        next.items.push((item.into(), true, Some(value)));
        next
    }

    fn retype<B>(self) -> StatusBar<B>
    where
        A: Widen<B>,
    {
        StatusBar {
            salt: self.salt,
            label: self.label,
            items: self
                .items
                .into_iter()
                .map(|(item, at_end, v)| (item, at_end, v.map(Widen::widen)))
                .collect(),
            tone: self.tone,
            grip: self.grip,
        }
    }

    /// Allocates the full available width.
    pub fn show(self, ui: &mut MineralUi<'_, Material>) -> StatusBarResponse<A> {
        let rt = ui.rt();
        let t = rt.tokens();
        let dir = rt.i18n().direction();
        let id = ui.make_persistent_id(self.salt.id_salt());
        let size = vec2(ui.available_width(), height(ui.density()));
        let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
        let response = ui.interact(rect, id, Sense::hover());
        let (bar_container, bar_content) = t.tone(self.tone).unwrap_or((t.role(CONTAINER), t.role(CONTENT)));
        let p = ui.painter();
        p.rect_filled(rect, 0, bar_container);
        p.hline(
            rect.x_range(),
            rect.top(),
            Stroke::new(divider::THICKNESS, t.role(divider::COLOR)),
        );

        let content_of = |it: &StatusItem| t.tone(it.tone_kind()).map_or(bar_content, |c| c.1);
        let grip_w = if self.grip { rect.height() } else { 0.0 };
        let grip = slot(rect, dir, true, 0.0, grip_w);
        let rect = slot(rect, dir, false, 0.0, rect.width() - grip_w);
        let mut galleys: Vec<_> = self
            .items
            .iter()
            .map(|(it, ..)| text_galley(ui, it, content_of(it)))
            .collect();
        let chrome = |i: usize| 2.0 * PAD_X + self.items[i].0.lead().map_or(0.0, |l| lead_width(l) + ICON_GAP);
        let width = |i: usize, galleys: &[std::sync::Arc<egui::Galley>]| chrome(i) + galleys[i].size().x;
        let measured: Vec<strip::Item> = (0..self.items.len())
            .map(|i| {
                strip::Item::new(width(i, &galleys))
                    .chrome(chrome(i))
                    .at_end(self.items[i].1)
                    .elide(self.items[i].0.elision().is_some())
            })
            .collect();
        let slots: Vec<Option<Rect>> = strip::place(rect.width(), &measured)
            .into_iter()
            .zip(&self.items)
            .enumerate()
            .map(|(i, (placed, (item, at_end, _)))| {
                let p = placed?;
                if let Some(room) = p.elided_room {
                    let elide = item.elision().unwrap_or_default();
                    galleys[i] = elided_galley(ui.painter(), item.text(), TEXT, content_of(item), room, elide);
                }
                Some(slot(rect, dir, *at_end, p.offset, p.width))
            })
            .collect();

        let mut clicked = None;
        for (i, galley) in galleys.into_iter().enumerate() {
            let Some(r) = slots[i] else { continue };
            let content = content_of(&self.items[i].0);
            if self.paint_item(ui, id, (i, r), galley, content) {
                clicked.clone_from(&self.items[i].2);
            }
        }
        if self.grip {
            egui_mineral::status::resize_grip(ui, grip, id.with("grip"));
            for c in egui_mineral::status::grip_dots(grip.shrink(crate::adaptive::grid(1)), dir) {
                ui.painter()
                    .circle_filled(c, egui_mineral::status::GRIP_DOT, bar_content);
            }
        }
        semantic::emit(ui.ctx(), Some(response.id), response.rect, || {
            SemanticNode::new(StableId::of(&self.salt, "status-bar"), Role::Status, &self.label)
        });
        StatusBarResponse { response, clicked }
    }
}

impl<A> StatusBar<A> {
    // Paints item `i` at `r`; whether it was clicked.
    fn paint_item(
        &self,
        ui: &mut MineralUi<'_, Material>,
        id: egui::Id,
        (i, r): (usize, Rect),
        galley: std::sync::Arc<egui::Galley>,
        content: egui::Color32,
    ) -> bool {
        let rt = ui.rt();
        let t = rt.tokens();
        let dir = rt.i18n().direction();
        let (item, _, value) = &self.items[i];
        match item.tone_kind() {
            Tone::Custom { container, .. } => {
                ui.painter().rect_filled(r, 0, container);
            }
            Tone::Error => {
                ui.painter().rect_filled(r, 0, t.role(ColorRole::ErrorContainer));
            }
            _ => {}
        }
        let clickable = value.is_some();
        let sense = if clickable { Sense::click() } else { Sense::hover() };
        let resp = ui.interact(r, id.with(i), sense);
        if clickable {
            let state = Interaction::of(&resp, Availability::Enabled);
            common::state_focus(ui.painter(), r, 0, content, state, &t);
        }
        let inner = Rect::from_x_y_ranges(r.left() + PAD_X..=r.right() - PAD_X, r.y_range());
        let mut x = dir.start_x(inner, Rangef::point(0.0));
        if let Some(lead) = item.lead() {
            let c = pos2(dir.start_x(inner, Rangef::point(lead_width(lead) / 2.0)), r.center().y);
            let text = item.text();
            match lead {
                StatusLead::Icon(icon) => common::icon(ui.painter(), c, icon, ICON, content),
                StatusLead::Progress(StatusProgress::Circular(value)) => self.progress(ui, i, text, value, c),
                StatusLead::Progress(StatusProgress::Linear(value)) => self.linear(ui, i, text, value, c),
                _ => {}
            }
            x = dir.start_x(inner, Rangef::point(lead_width(lead) + ICON_GAP));
        }
        let gx = if dir == Direction::Rtl { x - galley.size().x } else { x };
        ui.painter()
            .galley(pos2(gx, r.center().y - galley.size().y / 2.0), galley, content);
        if let Some(tip) = item.tooltip_text() {
            PlainTooltip::new(tip).show(rt, &resp);
        }
        let role = if clickable { Role::Button } else { Role::Label };
        semantic::emit(ui.ctx(), Some(resp.id), r, || {
            SemanticNode::new(StableId::of(&self.salt, (i, "status-item")), role, item.text())
        });
        clickable && resp.clicked()
    }
}

impl<A> StatusBar<A> {
    fn progress(&self, ui: &mut MineralUi<'_, Material>, i: usize, text: &str, value: Option<Fraction>, center: Pos2) {
        let salt = self.salt.child((i, "progress"));
        match value {
            Some(v) => CircularProgressIndicator::determinate(salt, v),
            None => CircularProgressIndicator::indeterminate(salt),
        }
        .label(text)
        .agent_key(StableId::of(&self.salt, (i, "progress")))
        .show_in(ui, Rect::from_center_size(center, Vec2::splat(ICON)));
    }
}

impl<A> StatusBar<A> {
    fn linear(&self, ui: &mut MineralUi<'_, Material>, i: usize, text: &str, value: Option<Fraction>, center: Pos2) {
        let salt = self.salt.child((i, "progress"));
        match value {
            Some(v) => LinearProgressIndicator::determinate(salt, v),
            None => LinearProgressIndicator::indeterminate(salt),
        }
        .label(text)
        .agent_key(StableId::of(&self.salt, (i, "progress")))
        .show_in(ui, Rect::from_center_size(center, vec2(LINEAR, LINEAR_HEIGHT)));
    }
}

fn text_galley(ui: &MineralUi<'_, Material>, item: &StatusItem, color: egui::Color32) -> std::sync::Arc<egui::Galley> {
    if item.has_tabular_figures() {
        let format = TEXT.format(ui.ctx(), color);
        let job = egui_mineral::kit::paint::tabular_figures(ui.ctx(), item.text(), &format);
        return ui.painter().layout_job(job);
    }
    parts::type_galley(ui.core(), ui.painter(), item.text(), TEXT, color, None)
}

// `w` wide at logical offset `from` of the start (or end) edge.
fn slot(bar: Rect, dir: Direction, at_end: bool, from: f32, w: f32) -> Rect {
    let d = if at_end { dir.reversed() } else { dir };
    let x = d.start_x(bar, Rangef::new(from, from + w));
    Rect::from_x_y_ranges(x..=x + w, bar.y_range())
}
