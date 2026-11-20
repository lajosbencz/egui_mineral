use crate::Material;
use crate::MaterialExt;
use crate::Rt;
use egui_mineral::kit::state::TempState;
use egui_mineral::semantic::KeyedSalt;
use egui_mineral::widget::MineralUi;
use std::time::Duration;

use egui::{Color32, Context, Frame, Id, NumExt, Rect, Response, Sense, Shape, Ui, pos2};
use egui_mineral::{
    anchor::{Align, Anchor, Placement, Side},
    input::HoverIntent,
    kit,
    overlay::{Dismiss, OverlayResponse, Popup},
    semantic::{self, Role, SemanticNode, StableId},
};

use crate::spec::{defaults, tokens::rich_tooltip};
use crate::widget::common;

// Non-MD3: the rich tooltip surface at a larger default width
// Compose TooltipDefaults.caretSize 16x8
const ARROW: f32 = defaults::tooltip::CARET_HEIGHT;
// Rich tooltip horizontal padding
const PADDING: f32 = defaults::tooltip::RICH_HORIZONTAL_PADDING;
// Beside the 320dp rich tooltip maximum
const POPOVER_MAX_WIDTH: f32 = 360.0;
// Non-MD3 desktop hover card convention
const HOVER_OPEN_DELAY: Duration = Duration::from_millis(700);
const HOVER_CLOSE_DELAY: Duration = Duration::from_millis(300);

fn popup(salt: &KeyedSalt, place: Anchor, arrow: bool) -> Popup {
    let gap = defaults::tooltip::ANCHOR_SPACING + if arrow { ARROW } else { 0.0 };
    Popup::new(salt.child("area"), place.gap(gap))
}

#[derive(Clone, Copy)]
pub(crate) struct Chrome {
    pub arrow: bool,
    pub max_width: f32,
}

fn frame<R>(
    ui: &mut MineralUi<'_, Material>,
    placement: &Placement,
    chrome: Chrome,
    focus_id: Option<Id>,
    add: impl FnOnce(&mut MineralUi<'_, Material>) -> R,
) -> (Rect, R) {
    let Chrome { arrow, max_width } = chrome;
    let rt = ui.rt();
    let t = rt.tokens();
    let r = rich_tooltip::CONTAINER_SHAPE.corner_radius(&t.shape, rt.i18n().direction());
    let fill = t.role(rich_tooltip::CONTAINER_COLOR);
    let framed = common::elevated_frame(
        ui,
        rich_tooltip::CONTAINER_ELEVATION,
        &t,
        Frame::new().fill(fill).corner_radius(r).inner_margin(PADDING),
        |ui| {
            ui.set_max_width(max_width - 2.0 * PADDING);
            if let Some(f) = focus_id {
                // Focus target ahead of the content in Tab order.
                ui.interact(
                    Rect::from_min_size(ui.cursor().min, egui::Vec2::ZERO),
                    f,
                    Sense::focusable_noninteractive(),
                );
            }
            add(&mut rt.ui(ui))
        },
    );
    let rect = framed.response.rect;
    if arrow {
        paint_arrow(ui, placement, rect, f32::from(r.nw.max(r.ne)), fill);
    }
    (rect, framed.inner)
}

fn emit(ctx: &Context, area: &Response, rect: Rect, node: SemanticNode) {
    semantic::emit(ctx, Some(area.id), rect, || node);
}

pub(crate) fn surface<R>(
    rt: &Rt,
    salt: &KeyedSalt,
    node: SemanticNode,
    anchor: Rect,
    place: Anchor,
    chrome: Chrome,
    add: impl FnOnce(&mut MineralUi<'_, Material>) -> R,
) -> (Response, R) {
    let ctx = rt.ctx();
    let area =
        popup(salt, place, chrome.arrow).show_at(rt, anchor, |ui, placement| frame(ui, placement, chrome, None, add));
    let (rect, inner) = area.inner;
    emit(ctx, &area.response, rect, node);
    (area.response, inner)
}

fn paint_arrow(ui: &Ui, p: &Placement, rect: Rect, corner: f32, fill: Color32) {
    let m = corner + ARROW;
    let (mx, my) = (m.at_most(rect.width() / 2.0), m.at_most(rect.height() / 2.0));
    let x = p.arrow.x.at_least(rect.left() + mx).at_most(rect.right() - mx);
    let y = p.arrow.y.at_least(rect.top() + my).at_most(rect.bottom() - my);
    let pts = if p.side.is_vertical() {
        let edge = p.arrow.y;
        let out = if edge <= rect.center().y { -ARROW } else { ARROW };
        vec![pos2(x - ARROW, edge), pos2(x, edge + out), pos2(x + ARROW, edge)]
    } else {
        let edge = p.arrow.x;
        let out = if edge <= rect.center().x { -ARROW } else { ARROW };
        vec![pos2(edge, y - ARROW), pos2(edge + out, y), pos2(edge, y + ARROW)]
    };
    ui.painter().add(Shape::convex_polygon(pts, fill, egui::Stroke::NONE));
}

#[derive(Clone, Debug)]
struct Anchored {
    salt: KeyedSalt,
    label: String,
    side: Side,
    align: Align,
    max_width: f32,
}

impl Anchored {
    fn new(salt: KeyedSalt, label: String, max_width: f32) -> Self {
        Self {
            salt,
            label,
            side: Side::Bottom,
            align: Align::Center,
            max_width,
        }
    }

    fn anchor(&self) -> Anchor {
        common::popup_anchor(self.side, self.align)
    }
}

macro_rules! anchored_setters {
    () => {
        crate::widget::setters! {
            /// Replaces the `<id_salt>` key prefix.
            agent_key(salt) |s| s.base.salt
        }

        /// Side of the trigger; default [`Side::Bottom`].
        pub fn side(mut self, side: Side) -> Self {
            self.base.side = side;
            self
        }

        /// Alignment along the trigger; default [`Align::Center`].
        pub fn align(mut self, align: Align) -> Self {
            self.base.align = align;
            self
        }

        /// Width cap in dp; non-positive values snap to the minimum.
        #[track_caller]
        pub fn max_width(mut self, max_width: f32) -> Self {
            self.base.max_width = egui_mineral::kit::sanitize::at_least(max_width, f32::MIN_POSITIVE.., "max_width");
            self
        }
    };
}

/// Click-anchored popup surface with APG dialog semantics; non-MD3 (Radix `Popover` pattern).
#[derive(Clone, Debug)]
#[must_use = "call .show(..)"]
pub struct Popover {
    base: Anchored,
    arrow: bool,
}

impl Popover {
    /// `label` names the popover for assistive technology; below the trigger, centered, with an arrow. Agent key prefix `<id_salt>`.
    pub fn new(id_salt: impl Into<KeyedSalt>, label: impl Into<String>) -> Self {
        Self {
            base: Anchored::new(id_salt.into(), label.into(), POPOVER_MAX_WIDTH),
            arrow: true,
        }
    }

    anchored_setters!();

    /// Caret pointing at the trigger; default `true`.
    pub fn arrow_visible(mut self, arrow_visible: bool) -> Self {
        self.arrow = arrow_visible;
        self
    }

    /// Shown while `open`, anchored to `trigger`: its clicks toggle `open`, an outside click or Escape clears it,
    /// and the app may set it, e.g. to open from code.
    pub fn show<R>(
        self,
        rt: &Rt,
        trigger: &Response,
        open: &mut bool,
        add: impl FnOnce(&mut MineralUi<'_, Material>) -> R,
    ) -> OverlayResponse<R> {
        let ctx = trigger.ctx.clone();
        let id = trigger.id.with(("mineral::popover", self.base.salt.id_salt()));
        let mut dismissed = None;
        if trigger.clicked() {
            *open = !*open;
            if !*open {
                dismissed = Some(Dismiss::Close);
            }
        }
        let focus_id = id.with("focus");
        let place = self.base.anchor();
        let chrome = Chrome {
            arrow: self.arrow,
            max_width: self.base.max_width,
        };
        let out = popup(&self.base.salt, place, self.arrow)
            .role(Role::Dialog)
            .focus(focus_id)
            .show(rt, trigger, open, |ui, placement| {
                frame(ui, placement, chrome, Some(focus_id), add)
            });
        let dismissed = dismissed.or(out.dismissed);
        if let (Some(area), Some((rect, inner))) = (out.response, out.inner) {
            emit(
                &ctx,
                &area,
                rect,
                SemanticNode::new(StableId::of(&self.base.salt, "popover"), Role::Dialog, &self.base.label),
            );
            OverlayResponse::shown(area, Some(inner), dismissed)
        } else {
            let mut closed = OverlayResponse::closed();
            closed.dismissed = dismissed;
            closed
        }
    }
}

impl Popover {
    /// Shown while `open`, anchored to `anchor`, like [`Menu::show_at`](crate::selection::Menu::show_at): nothing
    /// toggles it, so an anchor that opens it from code (a menu item) is not a trigger. An outside press other than
    /// on `anchor`, or Escape, clears `open`. Presses of the opening pass are ignored, and opening focuses the first
    /// focusable widget of the content, if any.
    pub fn show_at<R>(
        self,
        rt: &Rt,
        anchor: Rect,
        open: &mut bool,
        add: impl FnOnce(&mut MineralUi<'_, Material>) -> R,
    ) -> OverlayResponse<R> {
        let ctx = rt.ctx().clone();
        let focus_id =
            kit::scoped_id(rt.core(), Id::unique(("mineral::popover_at", self.base.salt.id_salt()))).with("focus");
        let chrome = Chrome {
            arrow: self.arrow,
            max_width: self.base.max_width,
        };
        let out = popup(&self.base.salt, self.base.anchor(), self.arrow)
            .role(Role::Dialog)
            .focus(focus_id)
            .focus_first(true)
            .show_open_at(rt, anchor, open, |ui, placement| {
                frame(ui, placement, chrome, Some(focus_id), add)
            });
        if let (Some(area), Some((rect, inner))) = (out.response, out.inner) {
            emit(
                &ctx,
                &area,
                rect,
                SemanticNode::new(StableId::of(&self.base.salt, "popover"), Role::Dialog, &self.base.label),
            );
            OverlayResponse::shown(area, Some(inner), out.dismissed)
        } else {
            let mut closed = OverlayResponse::closed();
            closed.dismissed = out.dismissed;
            closed
        }
    }
}

#[derive(Clone, Default)]
struct HoverState {
    intent: HoverIntent<bool>,
    open: bool,
    card: Option<Rect>,
}

impl TempState for HoverState {}

/// Hover-anchored popup surface with open and close delays; non-MD3 (Radix `HoverCard` pattern).
#[derive(Clone, Debug)]
#[must_use = "call .show(..)"]
pub struct HoverCard {
    base: Anchored,
    open_delay: Duration,
    close_delay: Duration,
}

impl HoverCard {
    /// `label` names the card for assistive technology; below the trigger, centered. Agent key prefix `<id_salt>`.
    pub fn new(id_salt: impl Into<KeyedSalt>, label: impl Into<String>) -> Self {
        Self {
            base: Anchored::new(id_salt.into(), label.into(), defaults::tooltip::RICH_MAX_WIDTH),
            open_delay: HOVER_OPEN_DELAY,
            close_delay: HOVER_CLOSE_DELAY,
        }
    }

    anchored_setters!();

    /// Hover time before it opens; default 700ms.
    pub fn open_delay(mut self, open_delay: Duration) -> Self {
        self.open_delay = open_delay;
        self
    }

    /// Time it stays after the pointer leaves trigger and card; default 300ms.
    pub fn close_delay(mut self, close_delay: Duration) -> Self {
        self.close_delay = close_delay;
        self
    }

    /// Shows while `trigger` or the card is hovered, after the delays.
    pub fn show<R>(
        self,
        rt: &Rt,
        trigger: &Response,
        add: impl FnOnce(&mut MineralUi<'_, Material>) -> R,
    ) -> OverlayResponse<R> {
        let ctx = trigger.ctx.clone();
        let id = trigger.id.with(("mineral::hover_card", self.base.salt.id_salt()));
        let mut st = HoverState::load(&ctx, id);
        let over_card = st
            .card
            .zip(ctx.pointer_hover_pos())
            .is_some_and(|(r, p)| st.open && r.contains(p));
        let engaged = trigger.hovered() || trigger.has_focus() || over_card;
        let delays = egui_mineral::input::HoverDelays {
            open: self.open_delay,
            close: self.close_delay,
        };
        st.intent.follow(&ctx, &mut st.open, engaged.into(), delays);
        if st.open && kit::layer::consume_escape(&ctx) {
            st.open = false;
            st.intent.cancel();
        }
        let mut out = OverlayResponse::closed();
        if st.open {
            let place = self.base.anchor();
            let (surface, inner) = surface(
                rt,
                &self.base.salt,
                SemanticNode::new(
                    StableId::of(&self.base.salt, "hover-card"),
                    Role::Group,
                    &self.base.label,
                ),
                trigger.rect,
                place,
                Chrome {
                    arrow: false,
                    max_width: self.base.max_width,
                },
                add,
            );
            st.card = Some(surface.rect);
            out = OverlayResponse::shown(surface, Some(inner), None);
        } else {
            st.card = None;
        }
        st.store(&ctx, id);
        out
    }
}
