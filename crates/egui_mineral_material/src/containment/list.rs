#[cfg(feature = "extras")]
use crate::IconSet;
use crate::Material;
use crate::MaterialExt;
use crate::MaterialUiExt as _;
use crate::adaptive::Density;
use egui_mineral::i18n::I18n;
use egui_mineral::semantic::AgentKey;
use egui_mineral::semantic::KeyedSalt;
use egui_mineral::widget::MineralUi;
use std::sync::Arc;

use egui::{Color32, Key, Pos2, Rangef, Rect, Sense, Ui, pos2, vec2};
use egui_mineral::widget::Availability;
use egui_mineral::{
    i18n::Direction,
    kit,
    layout::{Constraints, Measure},
    semantic::{self, Action, Role, SemanticNode},
    widget::{MineralWidget, WidgetFrame, WidgetResponse},
};

use super::list_slot::{self, ListControl, ListSlot, RowControl};
use crate::widget::{
    common::{self, Interaction},
    indication, parts,
    row_parts::{self, RowColors, RowState},
};
use crate::{
    Icon,
    spec::{defaults, tokens::list},
    tokens::Tokens,
};
use egui_mineral::layout::Edge;

const REVEAL_W: f32 = 72.0;

/// Action revealed by swiping a [`ListItem`] (Compose `SwipeToDismissBox`).
#[derive(Clone, Debug)]
#[must_use]
pub struct SwipeAction {
    icon: Icon,
    label: String,
    removes_row: bool,
    destructive: bool,
}

impl SwipeAction {
    /// `label` is the accessible name of the revealed action.
    pub fn new(icon: impl Into<Icon>, label: impl Into<String>) -> Self {
        Self {
            icon: icon.into(),
            label: label.into(),
            removes_row: false,
            destructive: false,
        }
    }
    /// Swiping past the threshold or flinging removes the row (Compose `SwipeToDismissBox`); Delete triggers it.
    pub fn removes_row(mut self, removes_row: bool) -> Self {
        self.removes_row = removes_row;
        self
    }
    /// Error container colors instead of tertiary container.
    pub fn destructive(mut self, destructive: bool) -> Self {
        self.destructive = destructive;
        self
    }
}

#[doc(hidden)]
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct ListItemState {
    offset: f32,
    rest: f32,
    claim: egui_mineral::input::PanClaim,
}

/// Line count of a [`ListItem`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ListLines {
    /// Headline only.
    One,
    /// Headline plus one secondary line.
    Two,
    /// Headline plus two secondary lines.
    Three,
}

impl ListLines {
    // dp at default density
    pub(crate) const fn height(self) -> f32 {
        match self {
            Self::One => list::ITEM_ONE_LINE_CONTAINER_HEIGHT,
            Self::Two => list::ITEM_TWO_LINE_CONTAINER_HEIGHT,
            Self::Three => list::ITEM_THREE_LINE_CONTAINER_HEIGHT,
        }
    }
}

/// MD3 list item. Height depends only on [`ListLines`] (explicit, or one line plus one per
/// supporting/overline text), never on content width. Read activation from
/// [`ListItemOutput::activated`].
///
/// [`ListControl`]s in the leading and trailing [`ListSlot`]s are controls of their own: a click on
/// them toggles without activating the row, and Space on the focused row toggles (the leading
/// control first). A row that is not [`clickable`](Self::clickable) toggles from anywhere, like an
/// MD3 selection list item. Each control is an AccessKit/agent node that accepts Click:
/// `<key>-<kind>` leading, `<key>-trailing-<kind>` trailing (kind `checkbox`, `radio`, `switch`).
#[derive(Clone, Debug)]
#[must_use = "show with ui.mineral(..)"]
pub struct ListItem {
    salt: KeyedSalt,
    headline: String,
    supporting: Option<String>,
    overline: Option<String>,
    lines: Option<ListLines>,
    leading: ListSlot,
    trailing: ListSlot,
    selected: bool,
    clickable: bool,
    disabled: bool,
    dragged: bool,
    key: AgentKey,
    swipe_start: Option<SwipeAction>,
    swipe_end: Option<SwipeAction>,
}

/// Events of [`ListItem`], read from `WidgetResponse::output`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct ListItemOutput {
    /// Click or tap that did not close or complete a swipe, nor toggle a control.
    pub activated: bool,
    /// Side whose swipe action fired this frame.
    pub swiped: Option<Edge>,
    /// Leading [`ListControl`] toggled (see [`ListItem`] for the inputs); the app applies the new state.
    pub leading_toggled: bool,
    /// Trailing [`ListControl`] toggled (see [`ListItem`] for the inputs); the app applies the new state.
    pub trailing_toggled: bool,
}

impl ListItem {
    /// `headline` is the first line and accessible name; agent key `<id_salt>-list-item`.
    pub fn new(id_salt: impl Into<KeyedSalt>, headline: impl Into<String>) -> Self {
        let id_salt: KeyedSalt = id_salt.into();
        Self {
            key: AgentKey::Kind("list-item"),
            salt: id_salt,
            headline: headline.into(),
            supporting: None,
            overline: None,
            lines: None,
            leading: ListSlot::new(),
            trailing: ListSlot::new(),
            selected: false,
            clickable: false,
            disabled: false,
            dragged: false,
            swipe_start: None,
            swipe_end: None,
        }
    }

    /// Action revealed by swiping from the leading edge.
    pub fn swipe_start(mut self, swipe_start: SwipeAction) -> Self {
        self.swipe_start = Some(swipe_start);
        self
    }

    /// Action revealed by swiping from the trailing edge.
    pub fn swipe_end(mut self, swipe_end: SwipeAction) -> Self {
        self.swipe_end = Some(swipe_end);
        self
    }

    fn swipe(&self, side: Edge) -> Option<&SwipeAction> {
        match side {
            Edge::Start => self.swipe_start.as_ref(),
            Edge::End => self.swipe_end.as_ref(),
        }
    }

    fn swipeable(&self) -> bool {
        !self.disabled && (self.swipe_start.is_some() || self.swipe_end.is_some())
    }

    /// Secondary line below the headline.
    pub fn supporting(mut self, supporting: impl Into<String>) -> Self {
        self.supporting = Some(supporting.into());
        self
    }

    /// Line above the headline.
    pub fn overline(mut self, overline: impl Into<String>) -> Self {
        self.overline = Some(overline.into());
        self
    }

    /// Unset counts the text lines present.
    pub fn lines(mut self, lines: ListLines) -> Self {
        self.lines = Some(lines);
        self
    }

    /// Leading slot: control at the start edge, then the visual.
    pub fn leading(mut self, leading: impl Into<ListSlot>) -> Self {
        self.leading = leading.into();
        self
    }

    /// Trailing slot: the visual, then the control at the end edge.
    pub fn trailing(mut self, trailing: impl Into<ListSlot>) -> Self {
        self.trailing = trailing.into();
        self
    }

    /// Selected container color and `selected` state.
    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    /// Senses clicks and reports [`ListItemOutput::activated`].
    pub fn clickable(mut self, clickable: bool) -> Self {
        self.clickable = clickable;
        self
    }

    crate::widget::setters! {
        /// Greys out the row and ignores input.
        disabled
    }

    crate::widget::setters! {
        /// Replaces the `<id_salt>-list-item` agent key.
        agent_key(key)
    }

    // Drag indicator visual of a reorderable row; keeps an app trailing control.
    #[cfg(feature = "extras")]
    pub(crate) fn drag_indicator(mut self, icons: &IconSet) -> Self {
        self.trailing = self.trailing.icon(icons.drag_indicator);
        self
    }

    #[cfg(feature = "extras")]
    pub(crate) fn dragged(mut self, dragged: bool) -> Self {
        self.dragged = dragged;
        self
    }

    pub(crate) fn resolved_lines(&self) -> ListLines {
        self.lines.unwrap_or(
            match usize::from(self.supporting.is_some()) + usize::from(self.overline.is_some()) {
                0 => ListLines::One,
                1 => ListLines::Two,
                _ => ListLines::Three,
            },
        )
    }

    pub(crate) fn height(&self, density: Density) -> f32 {
        row_parts::height(density, self.resolved_lines())
    }

    fn interactive(&self) -> bool {
        self.clickable && !self.disabled
    }

    fn toggles(&self) -> bool {
        self.leading.control_ref().is_some() || self.trailing.control_ref().is_some()
    }

    fn update_toggles(
        &self,
        ui: &MineralUi<'_, Material>,
        r: &egui::Response,
        rect: Rect,
        centers: [Option<Pos2>; 2],
        pressed: bool,
    ) -> [Option<RowControl>; 2] {
        let slots = [&self.leading, &self.trailing];
        let mut controls = [0, 1].map(|side| {
            let (c, center) = slots[side].control_ref().zip(centers[side])?;
            Some(RowControl::new(
                ui,
                r,
                ("mineral::list_control", side),
                c,
                center,
                rect,
                self.disabled,
            ))
        });
        let on_control = controls.iter().flatten().any(|c| c.toggled);
        let space = r.has_focus() && ui.input(|i| i.key_pressed(Key::Space));
        if !self.disabled
            && pressed
            && !on_control
            && (space || !self.clickable)
            && let Some(c) = controls.iter_mut().flatten().next()
        {
            c.toggled = true;
        }
        let key = self.key.id(&self.salt);
        for (side, c) in controls.iter().enumerate() {
            let Some(c) = c else { continue };
            let k = if side == 0 {
                key.sub(c.control.kind())
            } else {
                key.sub(("trailing", c.control.kind()))
            };
            c.emit(ui.ctx(), k, &self.headline, 1, self.disabled);
        }
        controls
    }
}

fn side_of(offset: f32, dir: Direction) -> Edge {
    if (offset > 0.0) == (dir == Direction::Ltr) {
        Edge::Start
    } else {
        Edge::End
    }
}

impl ListItem {
    fn update_swipe(
        &self,
        ui: &MineralUi<'_, Material>,
        frame: &WidgetFrame<'_>,
        state: &mut ListItemState,
        swiped: &mut Option<Edge>,
    ) -> f32 {
        let rt = ui.rt();
        let ctx = ui.ctx();
        let r = &frame.response;
        let width = frame.zones.visual.width();
        let dir = rt.i18n().direction();
        let anim = r.id.with("mineral::swipe");
        let t = rt.tokens();
        let max = if self.swipe(side_of(1.0, dir)).is_some() {
            width
        } else {
            0.0
        };
        let min = if self.swipe(side_of(-1.0, dir)).is_some() {
            -width
        } else {
            0.0
        };
        for side in [Edge::Start, Edge::End] {
            let aid = r.id.with(("mineral::swipe_action", side));
            if self.swipe(side).is_some() && kit::input::accesskit_requested(ctx, aid, Action::Click) {
                *swiped = Some(side);
            }
        }
        if r.has_focus() && kit::input::key_presses(ctx).contains(&(Key::Delete, egui::Modifiers::NONE)) {
            *swiped = [Edge::End, Edge::Start]
                .into_iter()
                .find(|s| self.swipe(*s).is_some_and(|a| a.removes_row));
        }
        let pan = state.claim.update(ui, egui_mineral::layout::Axis::Horizontal, r);
        if let Some(pan) = pan {
            state.offset = (state.offset + pan.delta.x).clamp(min, max);
            state.rest = state.offset;
            if !pan.ended {
                return ctx.animate_value_with_time(anim, state.offset, 0.0);
            }
            let off = state.offset;
            let side = side_of(off, dir);
            let v = pan.velocity.x * off.signum();
            let fling = rt.core().policies().fling;
            state.rest = match self.swipe(side) {
                Some(a)
                    if a.removes_row
                        && fling.should_dismiss(
                            egui_mineral::motion::Release {
                                pos: off.abs(),
                                velocity: v,
                            },
                            width,
                        ) =>
                {
                    *swiped = Some(side);
                    off.signum() * width
                }
                Some(a) if !a.removes_row => {
                    let resting = fling.project(egui_mineral::motion::Release {
                        pos: off.abs(),
                        velocity: v,
                    });
                    off.signum() * if resting <= REVEAL_W / 2.0 { 0.0 } else { REVEAL_W }
                }
                _ => 0.0,
            };
        } else if (r.clicked() || frame.tapped.is_some()) && state.rest != 0.0 {
            let p = frame.tapped.or_else(|| r.interact_pointer_pos());
            let rect = frame.zones.visual;
            let side = side_of(state.rest, dir);
            let uncovered = if state.rest > 0.0 {
                Rect::from_x_y_ranges(rect.left()..=rect.left() + state.rest, rect.y_range())
            } else {
                Rect::from_x_y_ranges(rect.right() + state.rest..=rect.right(), rect.y_range())
            };
            if p.is_some_and(|p| uncovered.contains(p)) {
                *swiped = Some(side);
            }
            state.rest = 0.0;
        }
        if swiped.is_some() && state.rest.abs() < width {
            state.rest = 0.0;
        }
        state.offset = state.rest;
        let secs = rt.core().motion().duration(t.duration.medium[1]).as_secs_f32();
        ctx.animate_value_with_time(anim, state.rest, secs)
    }

    fn paint_swipe_background(&self, ui: &MineralUi<'_, Material>, rect: Rect, offset: f32, t: &Tokens) {
        let rt = ui.rt();
        if offset == 0.0 {
            return;
        }
        let dir = rt.i18n().direction();
        let side = side_of(offset, dir);
        let Some(a) = self.swipe(side) else { return };
        let c = &t.color;
        let (bg, fg) = if a.destructive {
            (c.error_container, c.on_error_container)
        } else {
            (c.tertiary_container, c.on_tertiary_container)
        };
        let p = ui.painter();
        p.rect_filled(rect, 0, bg);
        let x = if offset > 0.0 {
            rect.left() + REVEAL_W / 2.0
        } else {
            rect.right() - REVEAL_W / 2.0
        };
        common::icon(p, pos2(x, rect.center().y), a.icon, list::ITEM_LEADING_ICON_SIZE, fg);
    }

    fn swipe_semantics(&self, ui: &Ui, r: &egui::Response, rect: Rect) {
        for side in [Edge::Start, Edge::End] {
            let Some(a) = self.swipe(side) else { continue };
            let aid = r.id.with(("mineral::swipe_action", side));
            let tag = match side {
                Edge::Start => "start",
                Edge::End => "end",
            };
            semantic::emit(ui.ctx(), Some(aid), rect, || {
                let mut n = SemanticNode::new(
                    self.key.id(&self.salt).sub(("swipe", tag, "button")),
                    Role::Button,
                    &a.label,
                )
                .pressable(Availability::Enabled);
                n.depth = 1;
                n
            });
        }
    }

    fn paint_text(&self, p: &egui::Painter, rect: Rect, dir: Direction, start: f32, max_w: f32, col: &RowColors) {
        let top_aligned = self.resolved_lines() == ListLines::Three;
        let rows = if self.resolved_lines() == ListLines::Three && self.overline.is_none() {
            2
        } else {
            1
        };
        let mut stack: Vec<Arc<egui::Galley>> = Vec::with_capacity(3);
        if let Some(o) = &self.overline {
            stack.push(parts::type_wrapped(
                p,
                o,
                list::ITEM_OVERLINE_FONT,
                col.overline,
                max_w,
                1,
            ));
        }
        stack.push(parts::type_wrapped(
            p,
            &self.headline,
            list::ITEM_LABEL_TEXT_FONT,
            col.label,
            max_w,
            1,
        ));
        if let Some(s) = &self.supporting {
            stack.push(parts::type_wrapped(
                p,
                s,
                list::ITEM_SUPPORTING_TEXT_FONT,
                col.supporting,
                max_w,
                rows,
            ));
        }
        let total: f32 = stack.iter().map(|g| g.size().y).sum();
        let top = rect.top() + defaults::list::THREE_LINE_VERTICAL_PADDING;
        let mut y = if top_aligned {
            top
        } else {
            rect.center().y - total / 2.0
        };
        for g in stack {
            let h = g.size().y;
            let x = dir.start_x(rect, Rangef::new(start, start + g.size().x));
            p.galley(pos2(x, y), g, Color32::PLACEHOLDER);
            y += h;
        }
    }
}

impl MineralWidget for ListItem {
    type Kit = Material;
    type State = ListItemState;
    type Output = ListItemOutput;

    egui_mineral::kit::widget_meta!(salt, enabled);

    fn sense(&self) -> Sense {
        if self.swipeable() {
            Sense::click_and_drag()
        } else if self.interactive() || self.toggles() {
            Sense::click()
        } else {
            Sense::hover()
        }
    }

    fn layout(&self, ui: &MineralUi<'_, Material>, constraints: Constraints, _: &Self::State) -> Measure {
        constraints
            .constrain(vec2(constraints.max.x, self.height(ui.density())))
            .into()
    }

    fn update(
        &mut self,
        ui: &mut MineralUi<'_, Material>,
        frame: &WidgetFrame<'_>,
        state: &mut ListItemState,
    ) -> WidgetResponse<ListItemOutput> {
        let rt = ui.rt();
        let t = ui.tokens();
        let response = &frame.response;
        let mut rect = frame.zones.visual;
        let i = Interaction::of(response, Availability::disabled_if(self.disabled));
        let was_open = state.rest != 0.0;
        let mut swiped = None;
        let mut clicked = !self.disabled && (response.clicked() || frame.tapped.is_some());
        let clip = ui.clip_rect();
        if self.swipeable() {
            let offset = self.update_swipe(ui, frame, state, &mut swiped);
            clicked &= !was_open && swiped.is_none();
            self.paint_swipe_background(ui, rect, offset, &t);
            self.swipe_semantics(ui, response, rect);
            ui.set_clip_rect(rect.intersect(clip));
            rect = rect.translate(vec2(offset, 0.0));
        }
        let row_target = self.clickable || self.toggles();
        if row_target {
            kit::input::hover_cursor(ui.core(), response, Availability::disabled_if(self.disabled));
        }
        let dir = rt.i18n().direction();
        let row = RowState {
            selected: self.selected,
            disabled: self.disabled,
            dragged: self.dragged,
        };
        let col = RowColors::of(row, &t);
        let shape = row.shape(&t, dir);
        let lines = self.resolved_lines();

        // Compose `ListItemLayout`: three-line items align to the top padding, others center.
        let top_aligned = lines == ListLines::Three;
        let top = rect.top() + defaults::list::THREE_LINE_VERTICAL_PADDING;
        let anchor_y = |h: f32| if top_aligned { top + h / 2.0 } else { rect.center().y };
        let (start_pad, end_pad) = (defaults::list::START_PADDING, defaults::list::END_PADDING);
        let max_h = rect.height();
        let place = |d: Direction, pad: f32| {
            move |at: f32, w: f32, h: f32| {
                let x = d.start_x(rect, Rangef::new(pad + at, pad + at + w));
                Rect::from_min_size(pos2(x, anchor_y(h) - h / 2.0), vec2(w, h))
            }
        };
        let lead = self
            .leading
            .layout(rt, ui.density(), ui.painter(), max_h, place(dir, start_pad));
        let trail = self
            .trailing
            .layout(rt, ui.density(), ui.painter(), max_h, place(dir.reversed(), end_pad));
        let controls = self.update_toggles(ui, response, rect, [lead.control, trail.control], clicked);
        let [leading_toggled, trailing_toggled] = controls.each_ref().map(|c| c.as_ref().is_some_and(|c| c.toggled));
        if leading_toggled || trailing_toggled {
            clicked = false;
        }
        row_parts::paint_container(ui.painter(), rect, row, shape, &t);
        if row_target {
            // A hovered control is still over its row (Compose hover reaches both).
            let i = i.with_hovered(i.hovered || controls.iter().flatten().any(|c| c.target.hovered()));
            indication::paint(ui, response, rect, shape, col.label, i, &t);
        }
        for (slot, layout, color) in [
            (&self.leading, &lead, col.leading),
            (&self.trailing, &trail, col.trailing),
        ] {
            if let (Some(v), Some(r)) = (slot.visual_ref(), layout.visual) {
                list_slot::paint_visual(ui, v, r, self.disabled, color, &t);
            }
        }
        for c in controls.iter().flatten() {
            c.paint(ui, self.disabled, &t);
        }
        let gap = |extent: f32, pad: f32| if extent > 0.0 { extent + pad } else { 0.0 };
        let start = start_pad + gap(lead.extent, defaults::list::LEADING_CONTENT_END_PADDING);
        let end = end_pad + gap(trail.extent, defaults::list::TRAILING_CONTENT_START_PADDING);
        let max_w = rect.width() - start - end;

        self.paint_text(ui.painter(), rect, dir, start, max_w, &col);
        ui.set_clip_rect(clip);
        WidgetResponse::new(
            response.clone(),
            ListItemOutput {
                activated: clicked && self.clickable,
                swiped,
                leading_toggled,
                trailing_toggled,
            },
        )
    }

    fn semantics(&self, _: &I18n, _: &WidgetResponse<ListItemOutput>, _: &ListItemState) -> Option<SemanticNode> {
        let mut n = SemanticNode::new(self.key.id(&self.salt), Role::ListItem, &self.headline);
        n.value.clone_from(&self.supporting);
        n.state.disabled = self.disabled;
        n.state.selected = (self.clickable || self.selected).then_some(self.selected);
        n.state.checked = self
            .leading
            .control_ref()
            .or(self.trailing.control_ref())
            .map(ListControl::state);
        if self.interactive() || (self.toggles() && !self.disabled) {
            n.actions.extend([Action::Click, Action::Focus]);
        }
        Some(n)
    }
}
