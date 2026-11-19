use std::sync::Arc;

use egui::{Color32, Galley, Rangef, Rect, Response, Sense, Stroke, UiBuilder, pos2, text::LayoutJob, vec2};
use egui_mineral::{
    i18n::Direction,
    kit::strip,
    semantic::{self, KeyedSalt, Role, SemanticNode, StableId},
    widget::{Availability, Interaction, MineralUi, SizeExt as _},
};

use crate::ColorToken::{
    ColorNeutralBackground3, ColorNeutralForeground2, ColorNeutralStroke2, ColorPaletteRedBackground1,
    ColorPaletteRedForeground1, ColorSubtleBackgroundHover, ColorSubtleBackgroundPressed,
};
use crate::{
    Fluent, FluentExt, Icon,
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

/// Progress indicator of a [`StatusItem`]: the fraction done, `None` for indeterminate.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub enum StatusProgress {
    /// Extra tiny [`Spinner`].
    Spinner,
    /// [`ProgressBar`] fixed at 48 wide.
    Bar(Option<f32>),
}

#[derive(Clone, Copy, Debug)]
enum Lead {
    Icon(Icon),
    Progress(StatusProgress),
}

impl Lead {
    fn width(self) -> f32 {
        match self {
            Self::Icon(_) | Self::Progress(StatusProgress::Spinner) => ICON,
            Self::Progress(StatusProgress::Bar(_)) => BAR,
        }
    }
}

/// Color treatment of a [`StatusItem`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum StatusTone {
    /// Bar container and content colors.
    #[default]
    Default,
    /// Red palette background and foreground.
    Error,
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
    colors: Option<(Color32, Color32)>,
    elide: bool,
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
            colors: None,
            elide: false,
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

    /// [`Tooltip`] on hover; also the item's accessible description.
    pub fn tooltip(mut self, tooltip: impl Into<String>) -> Self {
        self.tooltip = Some(tooltip.into());
        self
    }

    /// Focusable button, reported in [`StatusBarResponse::clicked`]; default `false`.
    pub fn clickable(mut self, clickable: bool) -> Self {
        self.clickable = clickable;
        self
    }

    /// [`StatusTone`] of the item's strip; default [`StatusTone::Default`].
    pub fn tone(mut self, tone: StatusTone) -> Self {
        self.tone = tone;
        self
    }

    /// App-chosen strip and text colors, overriding the [`tone`](Self::tone).
    pub fn colors(mut self, container: Color32, content: Color32) -> Self {
        self.colors = Some((container, content));
        self
    }

    /// Start items only: the one that overflows ends in an ellipsis at the room left rather than
    /// vanishing with the following items; default `false`.
    pub fn elide(mut self, elide: bool) -> Self {
        self.elide = elide;
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
            .map(|(it, _)| galley(&it.text, content_of(it), None))
            .collect();
        let chrome = |i: usize| 2.0 * PAD_X + self.items[i].0.lead.map_or(0.0, |l| l.width() + ICON_GAP);
        let measured: Vec<strip::Item> = self
            .items
            .iter()
            .zip(&galleys)
            .enumerate()
            .map(|(i, ((it, at_end), g))| strip::Item {
                width: chrome(i) + g.size().x,
                chrome: chrome(i),
                at_end: *at_end,
                elide: it.elide,
            })
            .collect();
        let slots: Vec<Option<Rect>> = strip::place(rect.width(), &measured)
            .into_iter()
            .zip(&self.items)
            .enumerate()
            .map(|(i, (placed, (item, at_end)))| {
                let placed = placed?;
                if let Some(room) = placed.elided_room {
                    galleys[i] = galley(&item.text, content_of(item), Some(room));
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
        match (item.colors, item.tone) {
            (Some((container, _)), _) => {
                ui.painter().rect_filled(r, 0, container);
            }
            (None, StatusTone::Error) => {
                ui.painter().rect_filled(r, 0, t.color(ColorPaletteRedBackground1));
            }
            (None, StatusTone::Default) => {}
        }
        let sense = if item.clickable { Sense::click() } else { Sense::hover() };
        let resp = ui.interact(r, id.with(i), sense);
        if item.clickable {
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
        if let Some(lead) = item.lead {
            let c = pos2(dir.start_x(inner, Rangef::point(lead.width() / 2.0)), r.center().y);
            match lead {
                Lead::Icon(icon) => paint::icon(ui.painter(), c, icon, false, ICON, content),
                Lead::Progress(progress) => self.progress(ui, i, &item.text, progress, c),
            }
            x = dir.start_x(inner, Rangef::point(lead.width() + ICON_GAP));
        }
        let gx = if dir == Direction::Rtl { x - text.size().x } else { x };
        ui.painter()
            .galley(pos2(gx, r.center().y - text.size().y / 2.0), text, content);
        if let Some(tip) = &item.tooltip {
            Tooltip::new(self.salt.child((i, "tooltip")), tip).show(rt, &resp);
        }
        let role = if item.clickable { Role::Button } else { Role::Label };
        semantic::emit(ui.ctx(), Some(resp.id), r, || {
            SemanticNode::new(StableId::of(&self.salt, (i, "status-item")), role, &item.text)
        });
        item.clickable && resp.clicked()
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
            StatusProgress::Spinner => {
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
            StatusProgress::Bar(value) => {
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
    match (item.colors, item.tone) {
        (Some((_, content)), _) => content,
        (None, StatusTone::Default) => t.color(ColorNeutralForeground2),
        (None, StatusTone::Error) => t.color(ColorPaletteRedForeground1),
    }
}
