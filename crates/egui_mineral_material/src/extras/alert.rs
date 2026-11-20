use crate::IconSet;
use crate::Material;
use crate::MaterialExt;
use crate::MaterialUiExt as _;
use crate::Rt;
use crate::TextAction;
use crate::msg;
use egui::{Color32, Frame, Sense, Stroke, Ui, Vec2, vec2};
use egui_mineral::action::Clicked;
use egui_mineral::semantic::KeyedSalt;
use egui_mineral::widget::MineralUi;
use egui_mineral::{
    i18n, kit,
    overlay::Dismiss,
    semantic::{self, Live, Role, SemanticNode, StableId},
};

use crate::actions::{IconButton, PushButton};
use crate::widget::parts;
use crate::{
    Icon,
    spec::{
        ShapeRole, defaults,
        tokens::{divider, filled_card, list, outlined_card, rich_tooltip},
    },
    tokens::Tokens,
};

// No MD3 counterpart

// List item start padding; three-line item top padding
const PADDING: Vec2 = vec2(
    defaults::list::START_PADDING,
    defaults::list::THREE_LINE_VERTICAL_PADDING,
);

/// Status level of an [`Alert`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
pub enum Severity {
    #[default]
    /// Neutral information.
    Info,
    /// Completed operation.
    Success,
    /// Needs attention.
    Warning,
    /// Failed operation.
    Error,
}

impl Severity {
    fn icon(self, icons: &IconSet) -> Icon {
        match self {
            Self::Info => icons.info,
            Self::Success => icons.check_circle,
            Self::Warning => icons.warning,
            Self::Error => icons.error,
        }
    }

    // MD3 has no success/warning roles: tertiary and secondary stand in
    fn colors(self, t: &Tokens) -> (Color32, Color32, Color32) {
        let c = &t.color;
        match self {
            Self::Info => (c.primary_container, c.on_primary_container, c.primary),
            Self::Success => (c.tertiary_container, c.on_tertiary_container, c.tertiary),
            Self::Warning => (c.secondary_container, c.on_secondary_container, c.secondary),
            Self::Error => (c.error_container, c.on_error_container, c.error),
        }
    }

    fn message_id(self) -> crate::Msg {
        match self {
            Self::Info => msg::ALERT_INFO,
            Self::Success => msg::ALERT_SUCCESS,
            Self::Warning => msg::ALERT_WARNING,
            Self::Error => msg::ALERT_ERROR,
        }
    }
}

/// Fill style of an [`Alert`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
pub enum AlertVariant {
    #[default]
    /// Severity container color.
    Standard,
    /// Surface with a severity-colored outline.
    Outlined,
    /// Full-width surface strip with a bottom divider, square corners.
    Banner,
}

/// Result of [`Alert::show`].
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct AlertResponse {
    /// Alert surface.
    pub response: egui::Response,
    /// Clicked action, with its [`TextAction::key`].
    pub clicked: Option<Clicked>,
    /// Why it closed this frame; only with [`Alert::dismissible`].
    pub dismissed: Option<Dismiss>,
}

/// Inline status message with severity, actions and optional dismissal; non-MD3 (MUI `Alert` pattern).
#[derive(Clone, Debug)]
#[must_use = "call .show(..)"]
pub struct Alert {
    salt: KeyedSalt,
    severity: Severity,
    variant: AlertVariant,
    title: Option<String>,
    message: String,
    actions: Vec<TextAction>,
    dismissible: bool,
}

impl Alert {
    /// `message` is the body text and accessible name. Agent key prefix `<id_salt>`.
    pub fn new(id_salt: impl Into<KeyedSalt>, message: impl Into<String>) -> Self {
        let id_salt: KeyedSalt = id_salt.into();
        Self {
            salt: id_salt,
            severity: Severity::default(),
            variant: AlertVariant::default(),
            title: None,
            message: message.into(),
            actions: Vec::new(),
            dismissible: false,
        }
    }

    crate::widget::setters! {
        /// Replaces the `<id_salt>` key prefix.
        agent_key(salt)
    }

    /// Default [`Severity::Info`].
    pub fn severity(mut self, severity: Severity) -> Self {
        self.severity = severity;
        self
    }

    /// Default [`AlertVariant::Standard`].
    pub fn variant(mut self, variant: AlertVariant) -> Self {
        self.variant = variant;
        self
    }

    /// Bold line above the message.
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Appends a text button; reported in [`AlertResponse::clicked`].
    pub fn action(mut self, action: impl Into<TextAction>) -> Self {
        self.actions.push(action.into());
        self
    }

    /// Appends text buttons; see [`Self::action`].
    pub fn actions(mut self, actions: impl IntoIterator<Item = impl Into<TextAction>>) -> Self {
        self.actions.extend(actions.into_iter().map(Into::into));
        self
    }

    /// Close icon button that reports [`AlertResponse::dismissed`].
    pub fn dismissible(mut self, dismissible: bool) -> Self {
        self.dismissible = dismissible;
        self
    }

    /// Paints the alert at the available width.
    pub fn show(self, ui: &mut MineralUi<'_, Material>) -> AlertResponse {
        let rt = ui.rt();
        let t = ui.tokens();
        let ctx = ui.ctx().clone();
        let id = ui.make_persistent_id(self.salt.id_salt());
        let dir = rt.i18n().direction();
        let (container, content, accent) = self.severity.colors(&t);
        let surface = t.role(outlined_card::CONTAINER_COLOR);
        let on_surface = t.role(rich_tooltip::SUPPORTING_TEXT_COLOR);
        let (fill, stroke, shape, text) = match self.variant {
            AlertVariant::Standard => (container, Stroke::NONE, filled_card::CONTAINER_SHAPE, content),
            AlertVariant::Outlined => (
                surface,
                Stroke::new(outlined_card::OUTLINE_WIDTH, accent),
                outlined_card::CONTAINER_SHAPE,
                on_surface,
            ),
            AlertVariant::Banner => (surface, Stroke::NONE, ShapeRole::CornerNone, on_surface),
        };
        let icon_color = if self.variant == AlertVariant::Standard {
            content
        } else {
            accent
        };
        let frame = Frame::new()
            .fill(fill)
            .stroke(stroke)
            .corner_radius(shape.corner_radius(&t.shape, dir))
            .inner_margin(PADDING);
        self.emit_semantics(rt, id);
        let inner = frame.show(ui, |ui| self.row(ui, rt, dir, icon_color, text));
        let (action, dismissed) = inner.inner;
        let rect = inner.response.rect;
        if self.variant == AlertVariant::Banner {
            let w = divider::THICKNESS;
            ui.painter().hline(
                rect.x_range(),
                rect.bottom() - w / 2.0,
                Stroke::new(w, t.role(divider::COLOR)),
            );
        }
        kit::semantic::remember_rect(&ctx, id, rect);
        if let Some(a) = action.and_then(|i| self.actions.get(i)) {
            a.fire(&ctx);
        }
        AlertResponse {
            response: inner.response,
            clicked: crate::widget::clicked(&self.actions, action, |a| a.action_key.as_ref()),
            dismissed,
        }
    }

    fn emit_semantics(&self, rt: &Rt, id: egui::Id) {
        let ctx = rt.ctx();
        semantic::emit_parent(ctx, id, || {
            let sev = crate::tr(rt.i18n(), self.severity.message_id());
            let name = self.title.clone().unwrap_or(sev);
            let mut n = SemanticNode::new(StableId::of(&self.salt, "alert"), Role::Alert, name).value(&self.message);
            n.state.live = Some(match self.severity {
                Severity::Error | Severity::Warning => Live::Assertive,
                Severity::Info | Severity::Success => Live::Polite,
            });
            n
        });
    }

    fn row(
        &self,
        ui: &mut Ui,
        rt: &Rt,
        dir: i18n::Direction,
        icon_color: Color32,
        text: Color32,
    ) -> (Option<usize>, Option<Dismiss>) {
        let mut action = None;
        let mut dismissed = None;
        ui.set_width(ui.available_width());
        ui.allocate_ui_with_layout(
            vec2(ui.available_width(), 0.0),
            dir.row().with_cross_align(egui::Align::Min),
            |ui| {
                let gap = defaults::list::LEADING_CONTENT_END_PADDING;
                ui.spacing_mut().item_spacing.x = gap;
                let size = list::ITEM_LEADING_ICON_SIZE;
                let (ir, _) = ui.allocate_exact_size(vec2(size, size), Sense::hover());
                crate::widget::common::icon(
                    ui.painter(),
                    ir.center(),
                    self.severity.icon(&rt.icons()),
                    size,
                    icon_color,
                );
                let target = crate::adaptive::min_interactive(rt.ui(ui).density());
                let close_w = if self.dismissible { target + gap } else { 0.0 };
                let text_w = (ui.available_width() - close_w).max(0.0);
                ui.allocate_ui_with_layout(vec2(text_w, 0.0), dir.column(), |ui| {
                    ui.set_min_width(text_w);
                    ui.spacing_mut().item_spacing.y = 0.0;
                    if let Some(title) = &self.title {
                        parts::type_label(ui, title, rich_tooltip::SUBHEAD_FONT, text, 0);
                    }
                    parts::type_label(ui, &self.message, rich_tooltip::SUPPORTING_TEXT_FONT, text, 0);
                    if !self.actions.is_empty() {
                        parts::row(&mut rt.ui(ui), dir, target, |ui| {
                            let buttons = self
                                .actions
                                .iter()
                                .enumerate()
                                .map(|(i, a)| (i, PushButton::new(self.salt.child(i), &a.label)));
                            action = crate::actions::text_actions(ui, buttons);
                        });
                    }
                });
                if self.dismissible {
                    dismissed = rt
                        .ui(ui)
                        .mineral(IconButton::new(
                            self.salt.child("dismiss"),
                            rt.icons().close,
                            crate::tr(rt.i18n(), msg::DISMISS),
                        ))
                        .clicked()
                        .then_some(Dismiss::Close);
                }
            },
        );
        (action, dismissed)
    }
}
