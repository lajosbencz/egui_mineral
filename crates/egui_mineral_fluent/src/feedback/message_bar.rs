use crate::msg;
use egui::{
    Align, Color32, CornerRadius, Layout, Margin, Sense, Shape, Stroke, StrokeKind, UiBuilder,
    epaint::RectShape,
    text::{LayoutJob, TextFormat},
    vec2,
};
use egui_mineral::{
    semantic::{self, AgentKey, KeyedSalt, Live, Role, SemanticNode},
    widget::MineralUi,
};

use crate::ColorToken::{
    ColorNeutralBackground3, ColorNeutralForeground1, ColorNeutralForeground3, ColorNeutralStroke1,
    ColorStatusDangerBackground1, ColorStatusDangerBorder1, ColorStatusDangerForeground1,
    ColorStatusSuccessBackground1, ColorStatusSuccessBorder1, ColorStatusSuccessForeground1,
    ColorStatusWarningBackground1, ColorStatusWarningBorder1, ColorStatusWarningForeground3,
};
use crate::{
    ColorToken, Fluent, FluentExt, Icon,
    actions::{ButtonAppearance, PushButton},
    spec::{
        defaults::message_bar as d,
        tokens::{geometry as g, typography as ty},
    },
    typography::TypeRamp,
    widget::paint,
};

/// `MessageBar` `intent`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum MessageBarIntent {
    /// `info`: neutral.
    #[default]
    Info,
    /// `success`.
    Success,
    /// `warning`.
    Warning,
    /// `error`.
    Error,
}

impl MessageBarIntent {
    /// Every variant.
    pub const ALL: &'static [Self] = &[Self::Info, Self::Success, Self::Warning, Self::Error];

    // react-message-bar useMessageBarStyles.styles.ts useRootIntentStyles and useIconIntentStyles
    fn palette(self, icons: &crate::IconSet) -> (ColorToken, ColorToken, ColorToken, Icon) {
        match self {
            Self::Info => (
                ColorNeutralBackground3,
                ColorNeutralStroke1,
                ColorNeutralForeground3,
                icons.info,
            ),
            Self::Success => (
                ColorStatusSuccessBackground1,
                ColorStatusSuccessBorder1,
                ColorStatusSuccessForeground1,
                icons.checkmark_circle,
            ),
            Self::Warning => (
                ColorStatusWarningBackground1,
                ColorStatusWarningBorder1,
                ColorStatusWarningForeground3,
                icons.warning,
            ),
            Self::Error => (
                ColorStatusDangerBackground1,
                ColorStatusDangerBorder1,
                ColorStatusDangerForeground1,
                icons.error_circle,
            ),
        }
    }
}

/// `MessageBar` `layout`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum MessageBarLayout {
    /// `singleline`: one row, the text truncated.
    #[default]
    Singleline,
    /// `multiline`: the text wraps and the actions move below it.
    Multiline,
}

impl MessageBarLayout {
    /// Every variant.
    pub const ALL: &'static [Self] = &[Self::Singleline, Self::Multiline];
}

/// `MessageBar` `shape`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum MessageBarShape {
    /// `rounded`: `borderRadiusMedium`.
    #[default]
    Rounded,
    /// `square`: no rounding.
    Square,
}

impl MessageBarShape {
    /// Every variant.
    pub const ALL: &'static [Self] = &[Self::Rounded, Self::Square];
}

/// Result of [`MessageBar::show`].
#[derive(Debug)]
#[non_exhaustive]
pub struct MessageBarResponse<R> {
    /// Response of the bar, sensing hover only.
    pub response: egui::Response,
    /// True on the frame the dismiss button is pressed; the app then stops showing the bar.
    pub dismiss_clicked: bool,
    /// What the actions closure returned; `None` for [`MessageBar::show`].
    pub inner: Option<R>,
}

/// Fluent `MessageBar`: an inline notice with intent icon, optional title, actions and a dismiss button.
#[derive(Clone, Debug)]
#[must_use = "call .show(..)"]
pub struct MessageBar {
    salt: KeyedSalt,
    key: AgentKey,
    body: String,
    title: Option<String>,
    intent: MessageBarIntent,
    layout: MessageBarLayout,
    shape: MessageBarShape,
    dismissible: bool,
}

impl MessageBar {
    /// Info, singleline; agent key `<id_salt>-message-bar`.
    pub fn new(id_salt: impl Into<KeyedSalt>, body: impl Into<String>) -> Self {
        Self {
            salt: id_salt.into(),
            key: AgentKey::Kind("message-bar"),
            body: body.into(),
            title: None,
            intent: MessageBarIntent::Info,
            layout: MessageBarLayout::Singleline,
            shape: MessageBarShape::Rounded,
            dismissible: false,
        }
    }

    /// Bold lead-in before the body.
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Default [`MessageBarIntent::Info`].
    pub fn intent(mut self, intent: MessageBarIntent) -> Self {
        self.intent = intent;
        self
    }

    /// Default [`MessageBarLayout::Singleline`].
    pub fn layout(mut self, layout: MessageBarLayout) -> Self {
        self.layout = layout;
        self
    }

    /// Default [`MessageBarShape::Rounded`].
    pub fn shape(mut self, shape: MessageBarShape) -> Self {
        self.shape = shape;
        self
    }

    /// Shows the dismiss button, reported by [`MessageBarResponse::dismiss_clicked`].
    pub fn dismissible(mut self, dismissible: bool) -> Self {
        self.dismissible = dismissible;
        self
    }

    crate::widget::setters! {
        /// Replaces the `<id_salt>-message-bar` agent key.
        agent_key(key)
    }

    /// Shows the bar without actions.
    pub fn show(self, ui: &mut MineralUi<'_, Fluent>) -> MessageBarResponse<()> {
        self.render(ui, None::<fn(&mut MineralUi<'_, Fluent>)>)
    }

    /// Shows the bar with `actions`, laid out in a row with `spacingHorizontalM` between buttons.
    pub fn show_with<R>(
        self,
        ui: &mut MineralUi<'_, Fluent>,
        actions: impl FnOnce(&mut MineralUi<'_, Fluent>) -> R,
    ) -> MessageBarResponse<R> {
        self.render(ui, Some(actions))
    }

    fn text(&self, ui: &MineralUi<'_, Fluent>, color: Color32, width: f32) -> std::sync::Arc<egui::Galley> {
        let regular = TypeRamp::Body1.format(ui.ctx(), color);
        let mut job = LayoutJob::default();
        if let Some(title) = &self.title {
            let strong = TextFormat {
                font_id: crate::typography::font(ui.ctx(), ty::FONT_SIZE_BASE_300, ty::FONT_WEIGHT_SEMIBOLD),
                ..regular.clone()
            };
            job.append(&format!("{title} "), 0.0, strong);
        }
        job.append(&self.body, 0.0, regular);
        job.wrap.max_width = width.max(0.0);
        if self.layout == MessageBarLayout::Singleline {
            job.wrap.max_rows = 1;
        }
        ui.painter().layout_job(job)
    }

    fn lead(&self, ui: &mut egui::Ui, rt: &crate::Rt, icon: Icon, icon_color: Color32, text_color: Color32) {
        let (rect, _) = ui.allocate_exact_size(vec2(d::ICON_SIZE, d::ICON_SIZE), Sense::hover());
        paint::icon(ui.painter(), rect.center(), icon, true, d::ICON_SIZE, icon_color);
        ui.add_space(g::SPACING_HORIZONTAL_S);
        let room = ui.available_width() - g::SPACING_HORIZONTAL_M;
        let galley = self.text(&rt.ui(ui), text_color, room);
        let (rect, _) = ui.allocate_exact_size(galley.size(), Sense::hover());
        ui.painter().galley(rect.min, galley, text_color);
    }

    fn action_row<R>(
        ui: &mut egui::Ui,
        rt: &crate::Rt,
        layout: Layout,
        add: impl FnOnce(&mut MineralUi<'_, Fluent>) -> R,
    ) -> R {
        ui.scope_builder(UiBuilder::new().layout(layout), |ui| {
            ui.spacing_mut().item_spacing.x = g::SPACING_HORIZONTAL_M;
            add(&mut rt.ui(ui))
        })
        .inner
    }

    fn dismiss(&self, ui: &mut egui::Ui, rt: &crate::Rt) -> bool {
        ui.add_space(g::SPACING_HORIZONTAL_M);
        let label = crate::tr(rt.i18n(), msg::DISMISS);
        let button = PushButton::new(self.salt.child("dismiss"), label)
            .icon(rt.icons().dismiss)
            .label_visible(false)
            .appearance(ButtonAppearance::Transparent);
        rt.ui(ui).mineral(button).response().clicked()
    }

    fn render<R>(
        self,
        ui: &mut MineralUi<'_, Fluent>,
        actions: Option<impl FnOnce(&mut MineralUi<'_, Fluent>) -> R>,
    ) -> MessageBarResponse<R> {
        let rt = ui.rt();
        let t = ui.tokens();
        let dir = ui.i18n().direction();
        let (fill, border, icon_color, icon) = self.intent.palette(&ui.icons());
        let (fill, border, icon_color) = (t.color(fill), t.color(border), t.color(icon_color));
        let text_color = t.color(ColorNeutralForeground1);
        let radius = CornerRadius::from(match self.shape {
            MessageBarShape::Rounded => g::BORDER_RADIUS_MEDIUM,
            MessageBarShape::Square => g::BORDER_RADIUS_NONE,
        });
        let stroke = g::STROKE_WIDTH_THIN;
        let multiline = self.layout == MessageBarLayout::Multiline;
        let top = if multiline { g::SPACING_VERTICAL_M_NUDGE } else { 0.0 };
        let bottom = if multiline { g::SPACING_VERTICAL_S } else { 0.0 };
        let margin = Margin {
            left: (g::SPACING_HORIZONTAL_M + stroke) as i8,
            right: stroke as i8,
            top: (top + stroke) as i8,
            bottom: (bottom + stroke) as i8,
        };
        let background = ui.painter().add(Shape::Noop);
        let width = ui.available_width();
        let align = if multiline { Align::Min } else { Align::Center };
        let (end, start) = match dir {
            egui_mineral::i18n::Direction::Ltr => (Layout::right_to_left(align), Layout::left_to_right(align)),
            egui_mineral::i18n::Direction::Rtl => (Layout::left_to_right(align), Layout::right_to_left(align)),
        };
        let mut dismissed = false;
        let mut inner = None;
        let mut actions = actions;
        let framed = egui::Frame::NONE.inner_margin(margin).show(ui, |ui| {
            let inner_w = (width - f32::from(margin.left) - f32::from(margin.right)).max(0.0);
            let min_h = if multiline { 0.0 } else { d::MIN_HEIGHT - 2.0 * stroke };
            ui.allocate_ui_with_layout(vec2(inner_w, min_h), end, |ui| {
                ui.spacing_mut().item_spacing.x = 0.0;
                if self.dismissible {
                    dismissed = self.dismiss(ui, rt);
                }
                if !multiline && let Some(add) = actions.take() {
                    ui.add_space(g::SPACING_HORIZONTAL_M);
                    inner = Some(Self::action_row(ui, rt, start, add));
                }
                ui.with_layout(start, |ui| self.lead(ui, rt, icon, icon_color, text_color));
            });
            if multiline && let Some(add) = actions.take() {
                ui.add_space(g::SPACING_VERTICAL_M_NUDGE);
                ui.allocate_ui_with_layout(vec2(inner_w, 0.0), end, |ui| {
                    ui.add_space(g::SPACING_HORIZONTAL_M);
                    inner = Some(Self::action_row(ui, rt, start, add));
                });
            }
        });
        let rect = framed.response.rect;
        ui.painter().set(
            background,
            Shape::Rect(RectShape::new(
                rect,
                radius,
                fill,
                Stroke::new(stroke, border),
                StrokeKind::Inside,
            )),
        );
        let id = ui.make_persistent_id((self.salt.id_salt(), "message-bar"));
        let response = ui.interact(rect, id, Sense::hover());
        semantic::emit(ui.ctx(), Some(response.id), rect, || {
            let assertive = matches!(self.intent, MessageBarIntent::Warning | MessageBarIntent::Error);
            let (name, value) = match &self.title {
                Some(title) => (title.clone(), Some(self.body.clone())),
                None => (self.body.clone(), None),
            };
            let mut node = SemanticNode::new(
                self.key.id(&self.salt),
                if assertive { Role::Alert } else { Role::Status },
                name,
            );
            node.value = value;
            node.state.live = Some(if assertive { Live::Assertive } else { Live::Polite });
            node
        });
        MessageBarResponse {
            response,
            dismiss_clicked: dismissed,
            inner,
        }
    }
}

/// Result of [`MessageBarGroup::show`].
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct MessageBarGroupResponse<R> {
    /// Spans the bars.
    pub response: egui::Response,
    /// Value of the content closure.
    pub inner: R,
}

/// Fluent `MessageBarGroup`: stacks message bars in a column.
#[derive(Clone, Debug)]
#[must_use = "call .show(..)"]
pub struct MessageBarGroup {
    salt: KeyedSalt,
}

impl MessageBarGroup {
    /// Bars follow each other without a gap.
    pub fn new(id_salt: impl Into<KeyedSalt>) -> Self {
        Self { salt: id_salt.into() }
    }

    /// Lays out `add` in a column.
    pub fn show<R>(
        self,
        ui: &mut MineralUi<'_, Fluent>,
        add: impl FnOnce(&mut MineralUi<'_, Fluent>) -> R,
    ) -> MessageBarGroupResponse<R> {
        let rt = ui.rt();
        let dir = ui.i18n().direction();
        let out = ui
            .push_id(self.salt.id_salt(), |ui| {
                ui.scope_builder(UiBuilder::new().layout(dir.column()), |ui| {
                    ui.spacing_mut().item_spacing.y = 0.0;
                    add(&mut rt.ui(ui))
                })
            })
            .inner;
        MessageBarGroupResponse {
            response: out.response,
            inner: out.inner,
        }
    }
}
