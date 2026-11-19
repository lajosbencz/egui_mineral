use egui::{Align2, Frame, Id, Margin, Order, Rect, Response, Ui, Vec2, vec2};
use egui_mineral::{
    kit::{self, state::TempState},
    motion::{self, Visibility},
    overlay::{self, Dismiss},
    semantic::{self, AgentKey, KeyedSalt, Role, SemanticNode},
    widget::MineralUi,
};

use super::overlay::{Look, ModalFocus, framed, text};
use crate::ColorToken::{ColorBackgroundOverlay, ColorNeutralBackground1, ColorNeutralForeground1};
use crate::{
    Fluent, FluentExt, Rt,
    actions::PushButton,
    spec::{
        defaults::dialog as d,
        tokens::{geometry as g, motion as mo, typography as ty},
    },
    theme::Elevation,
    widget::vocab::ButtonAppearance,
};

/// `modalType` of a [`Dialog`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum DialogKind {
    /// `modal`: backdrop, focus trapped, Escape and backdrop presses dismiss.
    #[default]
    Modal,
    /// `non-modal`: no backdrop, page stays interactive; Escape dismisses.
    NonModal,
    /// `alert`: modal that only its own actions close, role `alertdialog`.
    Alert,
}

impl DialogKind {
    /// Every variant.
    pub const ALL: [Self; 3] = [Self::Modal, Self::NonModal, Self::Alert];
}

/// Where the actions sit (`DialogActions` `position`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum DialogActionsPosition {
    /// `end`: after the reading direction's end edge.
    #[default]
    End,
    /// `start`.
    Start,
}

/// Button of a [`Dialog`]'s actions.
#[derive(Clone, Debug)]
#[must_use]
pub struct DialogAction {
    pub(super) label: String,
    pub(super) appearance: ButtonAppearance,
    pub(super) disabled: bool,
    pub(super) salt: Option<KeyedSalt>,
}

impl DialogAction {
    /// `label` is the button text and accessible name; [`ButtonAppearance::Secondary`].
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            appearance: ButtonAppearance::Secondary,
            disabled: false,
            salt: None,
        }
    }

    crate::widget::setters! {
        item_salt
    }

    /// Default [`ButtonAppearance::Secondary`]; `Primary` marks the main action.
    pub fn appearance(mut self, appearance: ButtonAppearance) -> Self {
        self.appearance = appearance;
        self
    }

    crate::widget::setters! {
        /// Greys out the button.
        disabled
    }
}

impl From<&str> for DialogAction {
    fn from(label: &str) -> Self {
        Self::new(label)
    }
}

impl From<String> for DialogAction {
    fn from(label: String) -> Self {
        Self::new(label)
    }
}

/// Result of [`Dialog::show`].
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct DialogResponse<R> {
    /// Surface while shown.
    pub response: Option<Response>,
    /// Content closure result while shown.
    pub inner: Option<R>,
    /// Index of the action clicked this frame.
    pub clicked: Option<usize>,
    /// Why it closed without an action.
    pub dismissed: Option<Dismiss>,
}

impl<R> DialogResponse<R> {
    fn closed() -> Self {
        Self {
            response: None,
            inner: None,
            clicked: None,
            dismissed: None,
        }
    }
}

#[derive(Clone, Copy, Default)]
struct ActionsWidth(f32);

impl TempState for ActionsWidth {}

/// Fluent `Dialog`: surface with title, content and actions over a backdrop.
#[derive(Clone, Debug)]
#[must_use = "call .show(..)"]
pub struct Dialog {
    salt: KeyedSalt,
    key: AgentKey,
    id: Id,
    title: String,
    kind: DialogKind,
    actions: Vec<DialogAction>,
    position: DialogActionsPosition,
    fluid: bool,
    close_button: bool,
}

struct Shown<R> {
    inner: R,
    clicked: Option<usize>,
    close: bool,
    rect: Rect,
}

impl Dialog {
    /// `title` is the `DialogTitle` and accessible name. Agent key `<id_salt>-dialog`.
    pub fn new(id_salt: impl Into<KeyedSalt>, title: impl Into<String>) -> Self {
        let salt: KeyedSalt = id_salt.into();
        Self {
            id: Id::unique(("mineral::fluent-dialog", salt.id_salt())),
            salt,
            key: AgentKey::Kind("dialog"),
            title: title.into(),
            kind: DialogKind::Modal,
            actions: Vec::new(),
            position: DialogActionsPosition::End,
            fluid: false,
            close_button: false,
        }
    }

    crate::widget::setters! {
        /// Replaces the `<id_salt>-dialog` key.
        agent_key(key)
    }

    /// Default [`DialogKind::Modal`].
    pub fn kind(mut self, kind: DialogKind) -> Self {
        self.kind = kind;
        self
    }

    /// Appends an action button.
    pub fn action(mut self, action: impl Into<DialogAction>) -> Self {
        self.actions.push(action.into());
        self
    }

    /// Default [`DialogActionsPosition::End`].
    pub fn actions_position(mut self, position: DialogActionsPosition) -> Self {
        self.position = position;
        self
    }

    /// `fluid` actions: the buttons share the full width of the surface; default `false`.
    pub fn fluid_actions(mut self, fluid: bool) -> Self {
        self.fluid = fluid;
        self
    }

    /// Close button in the title row; default `false`.
    pub fn close_button(mut self, close_button: bool) -> Self {
        self.close_button = close_button;
        self
    }

    /// Shown while `open`; an action click or a dismissal clears it. Opening focuses the close button, else the
    /// first action; closing returns focus to the widget focused before opening.
    pub fn show<R>(
        mut self,
        rt: &Rt,
        open: &mut bool,
        add: impl FnOnce(&mut MineralUi<'_, Fluent>) -> R,
    ) -> DialogResponse<R> {
        let ctx = rt.ctx();
        self.id = kit::scoped_id(rt.core(), self.id);
        ModalFocus::scope(ctx, self.id.with("focus"), open, |focus, open| {
            let visibility = if *open { Visibility::Visible } else { Visibility::Hidden };
            let Some(k) = motion::presence(rt.core(), self.id.with("presence"), visibility, d::MOTION) else {
                return DialogResponse::closed();
            };
            let eased = if *open {
                mo::CURVE_DECELERATE_MID.ease(k)
            } else {
                mo::CURVE_ACCELERATE_MIN.ease(k)
            };
            let r = self.run(rt, focus, eased, add);
            if *open && (r.clicked.is_some() || r.dismissed.is_some()) {
                *open = false;
            }
            r
        })
    }

    fn run<R>(
        &self,
        rt: &Rt,
        focus: &mut ModalFocus,
        eased: f32,
        add: impl FnOnce(&mut MineralUi<'_, Fluent>) -> R,
    ) -> DialogResponse<R> {
        let ctx = rt.ctx();
        let t = rt.tokens();
        let width = d::MAX_WIDTH.min(ctx.content_rect().width());
        match self.kind {
            DialogKind::NonModal => {
                let area = kit::overlay::area(rt.core(), self.id)
                    .order(Order::Foreground)
                    .anchor(Align2::CENTER_CENTER, Vec2::ZERO);
                let r = area.show(ctx, |ui| self.surface(rt, ui, focus, eased, width, add));
                let rect = r.inner.rect;
                let dismissed = overlay::dismissal(rt.core(), r.response.layer_id, &[rect], None)
                    .filter(|d| *d == Dismiss::Escape)
                    .or(r.inner.close.then_some(Dismiss::Close));
                self.finish(rt, r.response, r.inner, dismissed)
            }
            DialogKind::Modal | DialogKind::Alert => {
                let modal = egui::Modal {
                    area: kit::overlay::modal_area(rt.core(), self.id),
                    backdrop_color: t.color(ColorBackgroundOverlay).gamma_multiply(eased),
                    frame: Some(Frame::NONE),
                };
                let r = modal.show(ctx, |ui| self.surface(rt, ui, focus, eased, width, add));
                let mut dismissed = overlay::modal(&r);
                if self.kind == DialogKind::Alert {
                    dismissed = dismissed.filter(|d| *d == Dismiss::Close);
                }
                let dismissed = dismissed.or(r.inner.close.then_some(Dismiss::Close));
                self.finish(rt, r.response, r.inner, dismissed)
            }
        }
    }

    fn finish<R>(&self, rt: &Rt, response: Response, shown: Shown<R>, dismissed: Option<Dismiss>) -> DialogResponse<R> {
        let role = if self.kind == DialogKind::Alert {
            Role::AlertDialog
        } else {
            Role::Dialog
        };
        semantic::emit(rt.ctx(), None, shown.rect, || {
            SemanticNode::new(self.key.id(&self.salt), role, &self.title)
        });
        DialogResponse {
            response: Some(response),
            inner: Some(shown.inner),
            clicked: shown.clicked,
            dismissed,
        }
    }

    fn surface<R>(
        &self,
        rt: &Rt,
        ui: &mut Ui,
        focus: &mut ModalFocus,
        eased: f32,
        width: f32,
        add: impl FnOnce(&mut MineralUi<'_, Fluent>) -> R,
    ) -> Shown<R> {
        let t = rt.tokens();
        ui.multiply_opacity(eased);
        let look = Look {
            fill: t.color(ColorNeutralBackground1),
            radius: g::BORDER_RADIUS_X_LARGE,
            elevation: Elevation::E64,
        };
        let pad = d::PADDING + g::STROKE_WIDTH_THIN;
        let mut close = false;
        let mut clicked = None;
        let body = framed(ui, &t, &look, Margin::same(pad as i8), |ui| {
            ui.set_width((width - 2.0 * pad).max(0.0));
            ui.visuals_mut().override_text_color = Some(t.color(ColorNeutralForeground1));
            close = self.title_row(rt, ui, focus);
            ui.add_space(d::GAP);
            let inner = ui
                .scope(|ui| {
                    ui.set_min_height(d::CONTENT_MIN_HEIGHT);
                    add(&mut rt.ui(ui))
                })
                .inner;
            if !self.actions.is_empty() {
                ui.add_space(d::GAP);
                clicked = self.actions_row(rt, ui, focus, width - 2.0 * pad);
            }
            inner
        });
        Shown {
            inner: body.inner,
            clicked,
            close,
            rect: body.response.rect,
        }
    }

    fn title_row(&self, rt: &Rt, ui: &mut Ui, focus: &mut ModalFocus) -> bool {
        let t = rt.tokens();
        let dir = rt.i18n().direction();
        let mut close = false;
        let width = ui.available_width();
        ui.allocate_ui_with_layout(vec2(width, 0.0), dir.row(), |ui| {
            ui.spacing_mut().item_spacing.x = d::GAP;
            let reserve = if self.close_button { 32.0 + d::GAP } else { 0.0 };
            ui.scope_builder(egui::UiBuilder::new().layout(dir.column()), |ui| {
                ui.set_max_width((width - reserve).max(0.0));
                let format = (
                    ty::FONT_SIZE_BASE_500,
                    ty::LINE_HEIGHT_BASE_500,
                    ty::FONT_WEIGHT_SEMIBOLD,
                );
                text(ui, rt, &self.title, format, t.color(ColorNeutralForeground1));
            });
            if self.close_button {
                ui.with_layout(dir.reversed().row(), |ui| {
                    let label = crate::tr(rt.i18n(), "mineral-dialog-close");
                    let salt = self.salt.child("close");
                    focus.enter(rt.ctx(), ui.make_persistent_id(salt.id_salt()));
                    let b = PushButton::new(salt, label)
                        .appearance(ButtonAppearance::Subtle)
                        .icon(rt.icons().dismiss)
                        .label_visible(false);
                    close = rt.ui(ui).mineral(b).response().clicked();
                });
            }
        });
        close
    }

    fn action_salt(&self, i: usize, action: &DialogAction) -> KeyedSalt {
        action.salt.clone().unwrap_or_else(|| self.salt.child(("action", i)))
    }

    fn actions_row(&self, rt: &Rt, ui: &mut Ui, focus: &mut ModalFocus, width: f32) -> Option<usize> {
        let dir = rt.i18n().direction();
        let narrow = rt.ctx().content_rect().width() <= d::BREAKPOINT;
        let count = self.actions.len();
        let stacked = narrow && count > 1;
        let fluid = self.fluid || narrow;
        let share = if stacked {
            width
        } else if fluid {
            ((width - d::GAP * (count - 1) as f32) / count as f32).max(0.0)
        } else {
            0.0
        };
        let width_id = self.id.with("actions-width");
        let known = ActionsWidth::load(rt.ctx(), width_id).0;
        let layout = if stacked { dir.column() } else { dir.row() };
        let mut clicked = None;
        let used = ui
            .allocate_ui_with_layout(vec2(width, 0.0), layout, |ui| {
                ui.spacing_mut().item_spacing = Vec2::splat(d::GAP);
                if !(stacked || fluid) {
                    // Buttons take their natural width; the lead space only positions them.
                    ui.set_max_width(f32::INFINITY);
                }
                if !self.close_button
                    && let Some((first, action)) = self.actions.iter().enumerate().find(|(_, a)| !a.disabled)
                {
                    focus.enter(
                        rt.ctx(),
                        ui.make_persistent_id(self.action_salt(first, action).id_salt()),
                    );
                }
                let free = (width - known).max(0.0);
                let lead = match (self.position, stacked || fluid) {
                    (DialogActionsPosition::End, false) => free,
                    _ => 0.0,
                };
                if lead > 0.0 {
                    ui.add_space(lead);
                }
                for (i, a) in self.actions.iter().enumerate() {
                    let mut b = PushButton::new(self.action_salt(i, a), &a.label)
                        .appearance(a.appearance)
                        .disabled(a.disabled);
                    if share > 0.0 {
                        b.min_width = Some(share);
                    }
                    if rt.ui(ui).mineral(b).response().clicked() {
                        clicked = Some(i);
                    }
                }
                ui.min_rect().width() - lead
            })
            .inner;
        if (used - known).abs() > 0.5 {
            rt.ctx().request_repaint_after(std::time::Duration::ZERO);
        }
        ActionsWidth(used.max(0.0)).store(rt.ctx(), width_id);
        clicked
    }
}
