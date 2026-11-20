use crate::Material;
use crate::MaterialExt;
use crate::Rt;
use crate::msg;
use egui::{Id, Layout, Order, Rect, Sense, Stroke, StrokeKind};
use egui_mineral::overlay::Dismiss;
use egui_mineral::semantic::{KeyedSalt, SemanticNode, StableId};
use egui_mineral::widget::MineralUi;
use egui_mineral::{
    anchor::{Align as AnchorAlign, Anchor, Side},
    i18n::Direction,
    kit,
};

use crate::actions::PushButton;
use crate::extras::popover;
use crate::spec::{
    ShapeRole,
    defaults::tooltip,
    tokens::{outlined_text_field, rich_tooltip},
};
use crate::widget::parts;

// No MD3 counterpart

// Tooltip anchor spacing
const SPOTLIGHT_PAD: f32 = tooltip::ANCHOR_SPACING;
// Focused outlined text field outline
const SPOTLIGHT_WIDTH: f32 = outlined_text_field::FOCUS_OUTLINE_WIDTH;
// Small shape of chips and snackbars
const SPOTLIGHT_SHAPE: ShapeRole = ShapeRole::CornerSmall;

/// Highlighted region and text of one [`Tour`] step.
#[derive(Clone, Debug)]
#[must_use]
pub struct TourStep {
    target: Rect,
    title: String,
    body: Option<String>,
    side: Side,
}

impl TourStep {
    /// Callout titled `title` pointing at `target`, a screen rect; below it by default.
    pub fn new(target: Rect, title: impl Into<String>) -> Self {
        Self {
            target,
            title: title.into(),
            body: None,
            side: Side::Bottom,
        }
    }
    /// Body text below the title.
    pub fn supporting(mut self, supporting: impl Into<String>) -> Self {
        self.body = Some(supporting.into());
        self
    }
    /// Side of `target` the callout sits on; default [`Side::Bottom`].
    pub fn side(mut self, side: Side) -> Self {
        self.side = side;
        self
    }
}

/// Result of [`Tour::show`].
#[derive(Clone, Debug, Default)]
#[non_exhaustive]
pub struct TourResponse {
    /// Callout surface; `None` while no step is shown.
    pub response: Option<egui::Response>,
    /// Covers advancing, going back and finishing.
    pub changed: bool,
    /// Last step completed this frame; `step` is `None`.
    pub finished: bool,
    /// Close button ([`Dismiss::Close`]) or Escape ended the tour this frame; `step` is `None`.
    pub dismissed: Option<Dismiss>,
}

/// Guided overlay stepping through [`TourStep`]s; non-MD3 (Ant `Tour` pattern). The bound step is
/// the open state: `None` is closed.
#[derive(Debug)]
#[must_use = "call .show(..) or .show_modal(..)"]
pub struct Tour<'a> {
    salt: KeyedSalt,
    step: &'a mut Option<usize>,
    steps: Vec<TourStep>,
}

impl<'a> Tour<'a> {
    /// `step` is the bound current index; `None` hides the tour. Agent key prefix `<id_salt>`.
    pub fn new(id_salt: impl Into<KeyedSalt>, step: &'a mut Option<usize>) -> Self {
        let id_salt: KeyedSalt = id_salt.into();
        Self {
            salt: id_salt,
            step,
            steps: Vec::new(),
        }
    }

    crate::widget::setters! {
        /// Replaces the `<id_salt>` key prefix.
        agent_key(salt)
    }

    /// Appends one step.
    pub fn step(mut self, step: impl Into<TourStep>) -> Self {
        self.steps.push(step.into());
        self
    }

    /// Appends steps.
    pub fn steps(mut self, steps: impl IntoIterator<Item = impl Into<TourStep>>) -> Self {
        self.steps.extend(steps.into_iter().map(Into::into));
        self
    }

    /// Shows the current step over everything; the app stays interactive.
    pub fn show(self, rt: &Rt) -> TourResponse {
        self.run(rt, false)
    }

    /// Shows the current step over a scrim that dims and blocks everything but the target.
    pub fn show_modal(self, rt: &Rt) -> TourResponse {
        self.run(rt, true)
    }

    fn callout_body(
        &self,
        ui: &mut MineralUi<'_, Material>,
        step: &TourStep,
        index: usize,
        end: Layout,
        t: &crate::tokens::Tokens,
    ) -> (bool, Option<usize>) {
        let rt = ui.rt();
        let count = self.steps.len();
        let last = index + 1 == count;
        let mut skipped = false;
        let mut next = None;
        let width = ui.available_width();
        ui.set_width(width);
        ui.spacing_mut().item_spacing = egui::Vec2::ZERO;
        ui.horizontal(|ui| {
            ui.with_layout(end, |ui| {
                skipped |= parts::close_button(&mut rt.ui(ui), self.salt.child("close"));
                ui.with_layout(rt.i18n().direction().row(), |ui| {
                    let color = t.role(rich_tooltip::SUBHEAD_COLOR);
                    parts::type_label(ui, &step.title, rich_tooltip::SUBHEAD_FONT, color, 0);
                });
            });
        });
        if let Some(body) = &step.body {
            parts::type_label(
                ui,
                body,
                rich_tooltip::SUPPORTING_TEXT_FONT,
                t.role(rich_tooltip::SUPPORTING_TEXT_COLOR),
                0,
            );
        }
        ui.add_space(tooltip::TEXT_BOTTOM_PADDING);
        ui.horizontal(|ui| {
            ui.set_min_height(tooltip::ACTION_LABEL_MIN_HEIGHT);
            ui.with_layout(end, |ui| {
                let msg = if last { msg::TOUR_DONE } else { msg::TOUR_NEXT };
                let mut actions = vec![(
                    index + 1,
                    PushButton::new(self.salt.child("next"), crate::tr(rt.i18n(), msg)),
                )];
                if index > 0 {
                    actions.push((
                        index - 1,
                        PushButton::new(self.salt.child("back"), crate::tr(rt.i18n(), msg::BACK)),
                    ));
                }
                next = crate::actions::text_actions(&mut rt.ui(ui), actions);
                let progress = rt.i18n().tr(msg::TourProgress {
                    index: (index + 1).into(),
                    count: count.into(),
                });
                ui.with_layout(rt.i18n().direction().row(), |ui| {
                    parts::type_label(
                        ui,
                        &progress,
                        rich_tooltip::SUPPORTING_TEXT_FONT,
                        t.role(rich_tooltip::SUPPORTING_TEXT_COLOR),
                        1,
                    );
                });
            });
        });
        (skipped, next)
    }

    fn run(self, rt: &Rt, mask: bool) -> TourResponse {
        let ctx = rt.ctx();
        let hidden = TourResponse::default();
        let Some(index) = *self.step else {
            return hidden;
        };
        let Some(step) = self.steps.get(index) else {
            *self.step = None;
            return hidden;
        };
        let mut finished = false;
        let mut dismissed = None;
        let t = rt.tokens();
        let dir = rt.i18n().direction();
        let id = egui_mineral::kit::scoped_id(rt.core(), Id::unique(("mineral::tour", self.salt.id_salt())));
        let hole = step.target.expand(SPOTLIGHT_PAD);
        paint_spotlight(rt, id, hole, mask, &t, dir);

        let count = self.steps.len();
        let mut next = None;
        let place = Anchor::new(step.side, AnchorAlign::Center).padding(SPOTLIGHT_PAD);
        let end = dir.reversed().row();
        let (callout, ()) = popover::surface(
            rt,
            &self.salt.child("callout"),
            SemanticNode::new(
                StableId::of(&self.salt, "tour"),
                egui::accesskit::Role::Dialog,
                &step.title,
            ),
            hole,
            place,
            popover::Chrome {
                arrow: true,
                max_width: tooltip::RICH_MAX_WIDTH,
            },
            |ui| {
                let closed;
                (closed, next) = self.callout_body(ui, step, index, end, &t);
                dismissed = closed.then_some(Dismiss::Close);
            },
        );
        ctx.move_to_top(callout.layer_id);
        if kit::overlay::consume_escape(ctx) {
            dismissed = Some(Dismiss::Escape);
        }
        let before = *self.step;
        match next {
            Some(n) if n >= count => {
                finished = true;
                *self.step = None;
            }
            Some(n) => *self.step = Some(n),
            None if dismissed.is_some() => *self.step = None,
            None => {}
        }
        let changed = *self.step != before;
        let mut callout = callout;
        if changed {
            callout.mark_changed();
        }
        TourResponse {
            response: Some(callout),
            changed,
            finished,
            dismissed,
        }
    }
}

fn paint_spotlight(rt: &Rt, id: Id, hole: Rect, mask: bool, t: &crate::tokens::Tokens, dir: Direction) {
    let ctx = rt.ctx();
    if mask {
        let screen = ctx.content_rect();
        egui_mineral::kit::overlay::floating(rt.core(), id.with("mask"), screen.min).show(ctx, |ui| {
            let fill = t.scrim(1.0);
            let parts = [
                Rect::from_min_max(screen.min, egui::pos2(screen.right(), hole.top())),
                Rect::from_min_max(egui::pos2(screen.left(), hole.bottom()), screen.max),
                Rect::from_min_max(
                    egui::pos2(screen.left(), hole.top()),
                    egui::pos2(hole.left(), hole.bottom()),
                ),
                Rect::from_min_max(
                    egui::pos2(hole.right(), hole.top()),
                    egui::pos2(screen.right(), hole.bottom()),
                ),
            ];
            for (i, r) in parts.into_iter().enumerate() {
                if r.is_positive() {
                    ui.painter().rect_filled(r, 0, fill);
                    ui.interact(r, id.with(("block", i)), Sense::click_and_drag());
                }
            }
        });
    }
    egui_mineral::kit::overlay::area(rt.core(), id.with("ring"))
        .order(Order::Foreground)
        .fixed_pos(hole.min)
        .interactable(false)
        .show(ctx, |ui| {
            ui.painter().rect_stroke(
                hole,
                SPOTLIGHT_SHAPE.corner_radius(&t.shape, dir),
                Stroke::new(SPOTLIGHT_WIDTH, t.role(outlined_text_field::FOCUS_OUTLINE_COLOR)),
                StrokeKind::Outside,
            );
        });
}
