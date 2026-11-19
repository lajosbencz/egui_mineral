use crate::Material;
use crate::MaterialExt;
use egui::{Color32, Pos2, Rangef, Rect, Response, Sense, Stroke, Vec2, pos2, vec2};
use egui_mineral::icon::Icon;
use egui_mineral::kit::strip;
use egui_mineral::semantic::KeyedSalt;
use egui_mineral::widget::{Availability, MineralUi};
use egui_mineral::{
    i18n::Direction,
    semantic::{self, Role, SemanticNode, StableId},
};

use crate::communication::{CircularProgressIndicator, LinearProgressIndicator, PlainTooltip};
use crate::spec::{ColorRole, TypeScale, tokens::divider};
use crate::text_inputs::text_field::{self, Elide};
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

/// Progress indicator of a [`StatusItem`]: the fraction done, `None` for indeterminate.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub enum StatusProgress {
    /// Circular indicator.
    Circular(Option<f32>),
    /// Linear indicator, fixed at 48dp wide.
    Linear(Option<f32>),
}

#[derive(Clone, Copy, Debug)]
enum Lead {
    Icon(Icon),
    Progress(StatusProgress),
}

impl Lead {
    fn width(self) -> f32 {
        match self {
            Self::Icon(_) | Self::Progress(StatusProgress::Circular(_)) => ICON,
            Self::Progress(StatusProgress::Linear(_)) => LINEAR,
        }
    }
}

/// Color treatment of a [`StatusItem`]; MD3 has no warning role, so there is none.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum StatusTone {
    /// Bar container and content colors.
    #[default]
    Default,
    /// Error container and on-error-container content.
    Error,
    /// App-chosen colors, e.g. a warning tone.
    Custom {
        /// Strip fill.
        container: Color32,
        /// Text, icon and progress color.
        content: Color32,
    },
}

impl StatusTone {
    fn content(self, t: &crate::theme::Tokens) -> Color32 {
        match self {
            Self::Default => t.role(CONTENT),
            Self::Error => t.role(ColorRole::OnErrorContainer),
            Self::Custom { content, .. } => content,
        }
    }

    fn container(self, t: &crate::theme::Tokens) -> Option<Color32> {
        match self {
            Self::Default => None,
            Self::Error => Some(t.role(ColorRole::ErrorContainer)),
            Self::Custom { container, .. } => Some(container),
        }
    }
}

/// Text, with an optional icon or progress indicator, in a [`StatusBar`].
#[derive(Clone, Debug)]
#[must_use]
pub struct StatusItem {
    text: String,
    lead: Option<Lead>,
    tooltip: Option<String>,
    clickable: bool,
    tone: StatusTone,
    elide: Option<Elide>,
}

impl StatusItem {
    /// `text` is shown and is the accessible name.
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            lead: None,
            tooltip: None,
            clickable: false,
            tone: StatusTone::Default,
            elide: None,
        }
    }

    /// Icon before the text; replaces a [`progress`](Self::progress) indicator.
    pub fn icon(mut self, icon: impl Into<Icon>) -> Self {
        self.lead = Some(Lead::Icon(icon.into()));
        self
    }

    /// Progress indicator before the text; replaces an [`icon`](Self::icon). The indicator is named by the item
    /// text, agent key `<id_salt>-<index>-progress`.
    pub fn progress(mut self, progress: StatusProgress) -> Self {
        self.lead = Some(Lead::Progress(progress));
        self
    }

    /// [`PlainTooltip`] on hover; also the item's accessible description.
    pub fn tooltip(mut self, tooltip: impl Into<String>) -> Self {
        self.tooltip = Some(tooltip.into());
        self
    }

    /// Focusable button, reported in [`StatusBarResponse::clicked`]; default `false`.
    pub fn clickable(mut self, clickable: bool) -> Self {
        self.clickable = clickable;
        self
    }
}

impl StatusItem {
    /// [`StatusTone`] of the item's strip; default [`StatusTone::Default`].
    pub fn tone(mut self, tone: StatusTone) -> Self {
        self.tone = tone;
        self
    }

    /// Start items only: the one that overflows shrinks to the room left, `elide` choosing the part replaced by
    /// an ellipsis, rather than vanishing with the following items. Default: it vanishes.
    pub fn elide(mut self, elide: Elide) -> Self {
        self.elide = Some(elide);
        self
    }
}

impl<S: Into<String>> From<S> for StatusItem {
    fn from(text: S) -> Self {
        Self::new(text)
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

/// Desktop status bar: items at the reading start and end of a full-width strip; non-MD3. Show it
/// last in a bottom panel. Start items that do not fit are dropped from the end of the start group, unless [`StatusItem::elide`].
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

    crate::widget::setters! {
        /// Replaces the `<id_salt>` key prefix.
        agent_key(salt)
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
    pub fn show(self, ui: &mut MineralUi<'_, Material>) -> StatusBarResponse {
        let rt = ui.rt();
        let t = rt.tokens();
        let dir = rt.i18n().direction();
        let id = ui.make_persistent_id(self.salt.id_salt());
        let w = ui.available_width();
        let (rect, _) = ui.allocate_exact_size(vec2(w, HEIGHT), Sense::hover());
        let response = ui.interact(rect, id, Sense::hover());
        let p = ui.painter();
        p.rect_filled(rect, 0, t.role(CONTAINER));
        p.hline(
            rect.x_range(),
            rect.top(),
            Stroke::new(divider::THICKNESS, t.role(divider::COLOR)),
        );

        let content_of = |it: &StatusItem| it.tone.content(&t);
        let mut galleys: Vec<_> = self
            .items
            .iter()
            .map(|(it, _)| parts::type_galley(rt.core(), ui.painter(), &it.text, TEXT, content_of(it), None))
            .collect();
        let chrome = |i: usize| 2.0 * PAD_X + self.items[i].0.lead.map_or(0.0, |l| l.width() + ICON_GAP);
        let width = |i: usize, galleys: &[std::sync::Arc<egui::Galley>]| chrome(i) + galleys[i].size().x;
        let measured: Vec<strip::Item> = (0..self.items.len())
            .map(|i| strip::Item {
                width: width(i, &galleys),
                chrome: chrome(i),
                at_end: self.items[i].1,
                elide: self.items[i].0.elide.is_some(),
            })
            .collect();
        let slots: Vec<Option<Rect>> = strip::place(rect.width(), &measured)
            .into_iter()
            .zip(&self.items)
            .enumerate()
            .map(|(i, (placed, (item, at_end)))| {
                let p = placed?;
                if let Some((room, elide)) = p.elided_room.zip(item.elide) {
                    let color = content_of(item);
                    galleys[i] = text_field::elided_galley(ui.painter(), &item.text, TEXT, color, room, elide);
                }
                Some(slot(rect, dir, *at_end, p.offset, p.width))
            })
            .collect();

        let mut clicked = None;
        for (i, ((item, _), galley)) in self.items.iter().zip(galleys).enumerate() {
            let Some(r) = slots[i] else { continue };
            let content = content_of(item);
            if let Some(container) = item.tone.container(&t) {
                ui.painter().rect_filled(r, 0, container);
            }
            let sense = if item.clickable { Sense::click() } else { Sense::hover() };
            let resp = ui.interact(r, id.with(i), sense);
            if item.clickable {
                if resp.clicked() {
                    clicked = Some(i);
                }
                let state = Interaction::of(&resp, Availability::Enabled);
                common::state_focus(ui.painter(), r, 0, content, state, &t);
            }
            let inner = Rect::from_x_y_ranges(r.left() + PAD_X..=r.right() - PAD_X, r.y_range());
            let mut x = dir.start_x(inner, Rangef::point(0.0));
            if let Some(lead) = item.lead {
                let c = pos2(dir.start_x(inner, Rangef::point(lead.width() / 2.0)), r.center().y);
                match lead {
                    Lead::Icon(icon) => common::icon(ui.painter(), c, icon, ICON, content),
                    Lead::Progress(StatusProgress::Circular(value)) => self.progress(ui, i, &item.text, value, c),
                    Lead::Progress(StatusProgress::Linear(value)) => self.linear(ui, i, &item.text, value, c),
                }
                x = dir.start_x(inner, Rangef::point(lead.width() + ICON_GAP));
            }
            let gx = if dir == Direction::Rtl { x - galley.size().x } else { x };
            ui.painter()
                .galley(pos2(gx, r.center().y - galley.size().y / 2.0), galley, content);
            if let Some(tip) = &item.tooltip {
                PlainTooltip::new(tip).show(rt, &resp);
            }
            let role = if item.clickable { Role::Button } else { Role::Label };
            semantic::emit(ui.ctx(), Some(resp.id), r, || {
                SemanticNode::new(StableId::of(&self.salt, (i, "status-item")), role, &item.text)
            });
        }
        semantic::emit(ui.ctx(), Some(response.id), rect, || {
            SemanticNode::new(StableId::of(&self.salt, "status-bar"), Role::Status, &self.label)
        });
        StatusBarResponse { response, clicked }
    }
}

impl StatusBar {
    fn progress(&self, ui: &mut MineralUi<'_, Material>, i: usize, text: &str, value: Option<f32>, center: Pos2) {
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

impl StatusBar {
    fn linear(&self, ui: &mut MineralUi<'_, Material>, i: usize, text: &str, value: Option<f32>, center: Pos2) {
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

// `w` wide at logical offset `from` of the start (or end) edge.
fn slot(bar: Rect, dir: Direction, at_end: bool, from: f32, w: f32) -> Rect {
    let d = if at_end { dir.reversed() } else { dir };
    let x = d.start_x(bar, Rangef::new(from, from + w));
    Rect::from_x_y_ranges(x..=x + w, bar.y_range())
}
