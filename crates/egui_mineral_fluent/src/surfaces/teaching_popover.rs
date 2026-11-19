use egui::{CornerRadius, Layout, Rect, Response, Sense, UiBuilder, vec2};
use egui_mineral::{
    anchor::{Align, Side},
    kit::state::TempState,
    overlay::Dismiss,
    semantic::{self, AgentKey, KeyedSalt, Role, SemanticNode, StableId},
    widget::{Availability, MineralUi},
};

use super::overlay::{BODY1, text};
use super::popover::{PopoverAppearance, Surface, show_anchored};
use crate::ColorToken::{
    ColorBrandBackground, ColorNeutralForeground1, ColorNeutralForeground3, ColorNeutralForegroundOnBrand,
};
use crate::{
    Fluent, FluentExt, Rt, Size,
    actions::PushButton,
    spec::{
        defaults::teaching_popover as d,
        tokens::{geometry as g, typography as ty},
    },
    widget::vocab::ButtonAppearance,
};

/// One step of a [`TeachingPopover`]: optional header caption and title above the body.
#[derive(Clone, Debug)]
#[must_use]
pub struct TeachingStep {
    header: Option<String>,
    title: Option<String>,
    body: String,
}

impl TeachingStep {
    /// `body` is the `TeachingPopoverBody` text.
    pub fn new(body: impl Into<String>) -> Self {
        Self {
            header: None,
            title: None,
            body: body.into(),
        }
    }

    /// `TeachingPopoverHeader` caption.
    pub fn header(mut self, header: impl Into<String>) -> Self {
        self.header = Some(header.into());
        self
    }

    /// `TeachingPopoverTitle`.
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }
}

/// Result of [`TeachingPopover::show`].
#[derive(Clone, Debug, Default)]
#[non_exhaustive]
pub struct TeachingPopoverResponse {
    /// Surface while shown.
    pub response: Option<Response>,
    /// Index of the shown step.
    pub step: usize,
    /// Primary footer button, or the final carousel button, clicked this frame.
    pub primary_clicked: bool,
    /// Secondary footer button clicked this frame.
    pub secondary_clicked: bool,
    /// Why it closed this frame without the primary button.
    pub dismissed: Option<Dismiss>,
}

#[derive(Clone, Copy, Default)]
struct StepState(usize);

impl TempState for StepState {}

#[derive(Clone, Copy, Default)]
struct Pressed {
    primary: bool,
    secondary: bool,
    close: bool,
}

/// Fluent `TeachingPopover`: a brand or neutral coach mark with header, title, body and footer; more than one
/// [`TeachingStep`] makes it a carousel.
#[derive(Clone, Debug)]
#[must_use = "call .show(..)"]
pub struct TeachingPopover {
    salt: KeyedSalt,
    key: AgentKey,
    label: String,
    steps: Vec<TeachingStep>,
    brand: bool,
    arrow: bool,
    side: Side,
    align: Align,
    primary: Option<String>,
    secondary: Option<String>,
    previous: Option<String>,
    next: Option<String>,
}

impl TeachingPopover {
    /// `label` names the surface; above the trigger, centered, with an arrow. Agent key `<id_salt>-teaching-popover`.
    pub fn new(id_salt: impl Into<KeyedSalt>, label: impl Into<String>) -> Self {
        Self {
            salt: id_salt.into(),
            key: AgentKey::Kind("teaching-popover"),
            label: label.into(),
            steps: Vec::new(),
            brand: false,
            arrow: true,
            side: Side::Top,
            align: Align::Center,
            primary: None,
            secondary: None,
            previous: None,
            next: None,
        }
    }

    crate::widget::setters! {
        /// Replaces the `<id_salt>-teaching-popover` key.
        agent_key(key)
    }

    /// Appends a step; more than one makes a carousel.
    pub fn step(mut self, step: TeachingStep) -> Self {
        self.steps.push(step);
        self
    }

    /// `appearance="brand"`; default `false`.
    pub fn brand(mut self, brand: bool) -> Self {
        self.brand = brand;
        self
    }

    /// Caret pointing at the trigger; default `true`.
    pub fn arrow(mut self, arrow: bool) -> Self {
        self.arrow = arrow;
        self
    }

    /// Side of the trigger; default [`Side::Top`].
    pub fn side(mut self, side: Side) -> Self {
        self.side = side;
        self
    }

    /// Alignment along the trigger; default [`Align::Center`].
    pub fn align(mut self, align: Align) -> Self {
        self.align = align;
        self
    }

    /// `TeachingPopoverFooter` primary button text; the carousel's final button text.
    pub fn primary(mut self, primary: impl Into<String>) -> Self {
        self.primary = Some(primary.into());
        self
    }

    /// `TeachingPopoverFooter` secondary button text; single step only.
    pub fn secondary(mut self, secondary: impl Into<String>) -> Self {
        self.secondary = Some(secondary.into());
        self
    }

    /// Carousel previous button text; default the `mineral-teaching-previous` message.
    pub fn previous_label(mut self, previous: impl Into<String>) -> Self {
        self.previous = Some(previous.into());
        self
    }

    /// Carousel next button text; default the `mineral-teaching-next` message.
    pub fn next_label(mut self, next: impl Into<String>) -> Self {
        self.next = Some(next.into());
        self
    }

    /// Shown while `open`, anchored to `trigger`; trigger clicks toggle `open`, Escape and outside presses clear it.
    /// Focus stays inside until it closes.
    pub fn show(self, rt: &Rt, trigger: &Response, open: &mut bool) -> TeachingPopoverResponse {
        let ctx = trigger.ctx.clone();
        let state_id = trigger.id.with(("mineral::teaching-popover", self.salt.id_salt()));
        let mut step = StepState::load(&ctx, state_id).0;
        let mut pressed = Pressed::default();
        let appearance = if self.brand {
            PopoverAppearance::Brand
        } else {
            PopoverAppearance::Default
        };
        let spec = Surface {
            salt: &self.salt,
            node: self.key.id(&self.salt),
            label: &self.label,
            side: self.side,
            align: self.align,
            arrow: self.arrow.then_some(d::ARROW_HEIGHT),
            appearance,
            padding: d::PADDING,
            radius: g::BORDER_RADIUS_X_LARGE,
            width: Some(d::BODY_WIDTH),
            trap_focus: true,
        };
        let count = self.steps.len().max(1);
        step = step.min(count - 1);
        let mut next = step;
        let out = show_anchored(rt, &spec, trigger, open, |ui| {
            self.content(ui, step, &mut next, &mut pressed);
        });
        step = next;
        let mut dismissed = out.dismissed;
        if pressed.close {
            *open = false;
            dismissed = Some(Dismiss::Close);
        }
        let last = step + 1 >= count;
        let primary_clicked = pressed.primary && (count == 1 || last);
        if primary_clicked {
            *open = false;
        }
        if *open {
            StepState(step).store(&ctx, state_id);
        } else {
            StepState::remove(&ctx, state_id);
        }
        TeachingPopoverResponse {
            response: out.response,
            step,
            primary_clicked,
            secondary_clicked: pressed.secondary,
            dismissed,
        }
    }

    fn content(&self, ui: &mut MineralUi<'_, Fluent>, shown: usize, step: &mut usize, pressed: &mut Pressed) {
        let rt = ui.rt();
        let t = rt.tokens();
        let (fg, muted) = if self.brand {
            (ColorNeutralForegroundOnBrand, ColorNeutralForegroundOnBrand)
        } else {
            (ColorNeutralForeground1, ColorNeutralForeground3)
        };
        let current = self.steps.get(shown);
        let dir = rt.i18n().direction();
        let dismiss = crate::tr(rt.i18n(), "mineral-dismiss");
        let header = current.and_then(|s| s.header.as_deref());
        ui.scope_builder(UiBuilder::new().layout(Layout::top_down(egui::Align::Min)), |ui| {
            let row = ui.available_width();
            ui.allocate_ui_with_layout(vec2(row, 0.0), dir.row(), |ui| {
                ui.spacing_mut().item_spacing.x = 0.0;
                let width = ui.available_width() - 28.0;
                ui.scope_builder(UiBuilder::new().layout(dir.column()), |ui| {
                    ui.set_max_width(width.max(0.0));
                    if let Some(h) = header {
                        let format = (
                            ty::FONT_SIZE_BASE_200,
                            ty::LINE_HEIGHT_BASE_200,
                            ty::FONT_WEIGHT_SEMIBOLD,
                        );
                        text(ui, rt, h, format, t.color(muted));
                        ui.add_space(d::HEADER_PADDING_BOTTOM);
                    }
                    if let Some(title) = current.and_then(|s| s.title.as_deref()) {
                        let format = (
                            ty::FONT_SIZE_BASE_400,
                            ty::LINE_HEIGHT_BASE_400,
                            ty::FONT_WEIGHT_SEMIBOLD,
                        );
                        text(ui, rt, title, format, t.color(fg));
                        ui.add_space(d::TITLE_PADDING_BOTTOM);
                    }
                });
                ui.with_layout(dir.reversed().row(), |ui| {
                    let mut dismiss_button = PushButton::new(self.salt.child("dismiss"), &dismiss)
                        .appearance(ButtonAppearance::Subtle)
                        .size(Size::Small)
                        .icon(rt.icons().dismiss)
                        .label_visible(false);
                    if self.brand {
                        dismiss_button = dismiss_button.appearance(ButtonAppearance::Transparent);
                    }
                    pressed.close = rt.ui(ui).mineral(dismiss_button).response().clicked();
                });
            });
            if let Some(s) = current {
                text(ui, rt, &s.body, BODY1, t.color(fg));
            }
            ui.add_space(d::FOOTER_PADDING_TOP);
            self.footer(rt, ui, shown, step, pressed);
        });
    }

    fn footer(&self, rt: &Rt, ui: &mut egui::Ui, shown: usize, step: &mut usize, pressed: &mut Pressed) {
        let dir = rt.i18n().direction();
        let count = self.steps.len().max(1);
        let width = ui.available_width();
        ui.allocate_ui_with_layout(vec2(width, 0.0), dir.row(), |ui| {
            ui.spacing_mut().item_spacing.x = d::FOOTER_GAP;
            if count > 1 {
                self.dots(rt, ui, count, shown, step);
            }
            ui.with_layout(dir.reversed().row(), |ui| {
                let last = shown + 1 >= count;
                let button = |ui: &mut egui::Ui, salt: &'static str, label: &str, appearance| {
                    let b = PushButton::new(self.salt.child(salt), label).appearance(appearance);
                    rt.ui(ui).mineral(b).response().clicked()
                };
                if count > 1 {
                    let label = if last {
                        crate::tr_or(rt.i18n(), self.primary.as_deref(), "mineral-teaching-done")
                    } else {
                        crate::tr_or(rt.i18n(), self.next.as_deref(), "mineral-teaching-next")
                    };
                    if button(ui, "primary", &label, ButtonAppearance::Primary) {
                        if last {
                            pressed.primary = true;
                        } else {
                            *step = shown + 1;
                        }
                    }
                    if shown > 0 {
                        let label = crate::tr_or(rt.i18n(), self.previous.as_deref(), "mineral-teaching-previous");
                        if button(ui, "previous", &label, ButtonAppearance::Secondary) {
                            *step = shown - 1;
                        }
                    }
                } else {
                    if let Some(p) = &self.primary
                        && button(ui, "primary", p, ButtonAppearance::Primary)
                    {
                        pressed.primary = true;
                    }
                    if let Some(sec) = &self.secondary
                        && button(ui, "secondary", sec, ButtonAppearance::Secondary)
                    {
                        pressed.secondary = true;
                    }
                }
            });
        });
    }

    fn dots(&self, rt: &Rt, ui: &mut egui::Ui, count: usize, shown: usize, step: &mut usize) {
        let t = rt.tokens();
        let base = if self.brand {
            t.color(ColorNeutralForegroundOnBrand)
        } else {
            t.color(ColorBrandBackground)
        };
        ui.spacing_mut().item_spacing.x = 0.0;
        for i in 0..count {
            let selected = i == shown;
            let w = if selected {
                d::NAV_DOT_SELECTED_WIDTH
            } else {
                d::NAV_DOT
            };
            let (rect, _) = ui.allocate_exact_size(vec2(w + d::NAV_DOT, MIN_HIT), Sense::hover());
            let dot = Rect::from_center_size(rect.center(), vec2(w, d::NAV_DOT));
            let id = ui.make_persistent_id(self.salt.child(("step", i)).id_salt());
            let response = ui.interact(rect, id, Sense::click());
            let radius = if selected {
                d::NAV_DOT_SELECTED_RADIUS
            } else {
                d::NAV_DOT / 2.0
            };
            let color = if selected {
                base
            } else {
                base.gamma_multiply(d::NAV_DOT_UNSELECTED_ALPHA)
            };
            ui.painter().rect_filled(dot, CornerRadius::from(radius), color);
            if response.has_focus() {
                crate::widget::focus::outline(ui.painter(), dot, CornerRadius::from(radius), &t);
            }
            let label = rt.i18n().tr(
                "mineral-teaching-step",
                &[(
                    "step",
                    rt.i18n()
                        .format()
                        .integer(i64::try_from(i + 1).unwrap_or(i64::MAX))
                        .into(),
                )],
            );
            semantic::emit(ui.ctx(), Some(id), rect, || {
                SemanticNode::new(StableId::of(&self.salt, (i, "step")), Role::Button, label)
                    .clickable(Availability::Enabled)
            });
            if response.clicked() {
                *step = i;
            }
        }
    }
}

const MIN_HIT: f32 = 24.0;
