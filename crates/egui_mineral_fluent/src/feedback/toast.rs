use crate::msg;
use crate::tokens::ShadowTint;
use egui::{
    Align, Align2, Color32, CornerRadius, Id, Layout, Margin, Order, Sense, Stroke, StrokeKind, UiBuilder, vec2,
};
use egui_mineral::{
    action::Clicked,
    i18n::Direction,
    kit::state::TempState,
    motion::{self, Visibility},
    semantic::{self, Live, Role, SemanticNode, StableId},
    toast::{ToastId, ToastQueue},
};

use crate::spec::ColorToken::{
    ColorNeutralBackground1, ColorNeutralBackgroundInverted, ColorNeutralForeground1, ColorNeutralForeground2,
    ColorNeutralForegroundInverted2, ColorStatusDangerForeground1, ColorStatusDangerForegroundInverted,
    ColorStatusSuccessForeground1, ColorStatusSuccessForegroundInverted, ColorStatusWarningForeground1,
    ColorStatusWarningForegroundInverted, ColorTransparentStroke,
};
use crate::{
    FluentExt, Icon, Rt,
    actions::TextLink,
    spec::ColorToken,
    spec::{
        defaults::toast as d,
        tokens::{geometry as g, typography as ty},
    },
    theme::Elevation,
    widget::paint,
};

/// Where a toast stacks on screen (`position`); start and end follow the text direction.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ToastPosition {
    /// `top-start`.
    TopStart,
    /// `top`.
    Top,
    /// `top-end`.
    TopEnd,
    /// `bottom-start`.
    BottomStart,
    /// `bottom`.
    Bottom,
    /// `bottom-end`.
    #[default]
    BottomEnd,
}

impl ToastPosition {
    /// Every variant.
    pub const ALL: &'static [Self] = &[
        Self::TopStart,
        Self::Top,
        Self::TopEnd,
        Self::BottomStart,
        Self::Bottom,
        Self::BottomEnd,
    ];

    const fn index(self) -> usize {
        match self {
            Self::TopStart => 0,
            Self::Top => 1,
            Self::TopEnd => 2,
            Self::BottomStart => 3,
            Self::Bottom => 4,
            Self::BottomEnd => 5,
        }
    }

    const fn bottom(self) -> bool {
        matches!(self, Self::BottomStart | Self::Bottom | Self::BottomEnd)
    }

    fn anchor(self, dir: Direction) -> Align2 {
        let (start, end) = match dir {
            Direction::Ltr => (Align::Min, Align::Max),
            Direction::Rtl => (Align::Max, Align::Min),
        };
        let (x, y) = match self {
            Self::TopStart => (start, Align::Min),
            Self::Top => (Align::Center, Align::Min),
            Self::TopEnd => (end, Align::Min),
            Self::BottomStart => (start, Align::Max),
            Self::Bottom => (Align::Center, Align::Max),
            Self::BottomEnd => (end, Align::Max),
        };
        Align2([x, y])
    }
}

/// Intent icon of a toast (`ToastTitle` `media`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ToastIntent {
    /// `info`.
    #[default]
    Info,
    /// `success`.
    Success,
    /// `warning`.
    Warning,
    /// `error`.
    Error,
}

impl ToastIntent {
    /// Every variant.
    pub const ALL: &'static [Self] = &[Self::Info, Self::Success, Self::Warning, Self::Error];
}

/// Link of a [`Toast`]: an action in its title row or footer.
#[derive(Clone, Debug)]
#[must_use]
pub struct ToastLink {
    label: String,
    action_key: Option<egui_mineral::action::ActionKey>,
}

impl ToastLink {
    /// `label` is the link text and accessible name.
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            action_key: None,
        }
    }

    crate::widget::setters! {
        action_key
    }
}

impl From<&str> for ToastLink {
    fn from(label: &str) -> Self {
        Self::new(label)
    }
}

impl From<String> for ToastLink {
    fn from(label: String) -> Self {
        Self::new(label)
    }
}

#[derive(Clone, Debug)]
struct ActionLink {
    link: ToastLink,
    footer: bool,
}

/// Action of a toast pressed this frame ([`ToasterResponse::action_clicked`]).
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct ToastAction {
    /// Toast the action belongs to.
    pub toast: ToastId,
    /// Index among the toast's title and footer actions, in the order added, and the link's key.
    pub clicked: Clicked,
}

/// Payload of a queued toast: intent icon, title, body, subtitle, actions and dismiss link.
#[derive(Clone, Debug)]
#[must_use]
pub struct Toast {
    title: String,
    body: Option<String>,
    subtitle: Option<String>,
    intent: Option<ToastIntent>,
    inverted: bool,
    position: ToastPosition,
    actions: Vec<ActionLink>,
    dismissible: bool,
}

impl Toast {
    /// Bottom end, no intent icon.
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            body: None,
            subtitle: None,
            intent: None,
            inverted: false,
            position: ToastPosition::BottomEnd,
            actions: Vec::new(),
            dismissible: false,
        }
    }

    /// `ToastBody` text.
    pub fn body(mut self, body: impl Into<String>) -> Self {
        self.body = Some(body.into());
        self
    }

    /// `ToastBody` `subtitle` text.
    pub fn secondary_text(mut self, text: impl Into<String>) -> Self {
        self.subtitle = Some(text.into());
        self
    }

    /// Icon before the title.
    pub fn intent(mut self, intent: ToastIntent) -> Self {
        self.intent = Some(intent);
        self
    }

    /// `backgroundAppearance="inverted"`.
    pub fn inverted(mut self, inverted: bool) -> Self {
        self.inverted = inverted;
        self
    }

    /// Default [`ToastPosition::BottomEnd`].
    pub fn position(mut self, position: ToastPosition) -> Self {
        self.position = position;
        self
    }

    /// Link at the end of the title row; reported by [`ToasterResponse::action_clicked`].
    pub fn action(mut self, link: impl Into<ToastLink>) -> Self {
        self.actions.push(ActionLink {
            link: link.into(),
            footer: false,
        });
        self
    }

    /// Link in the `ToastFooter`; reported by [`ToasterResponse::action_clicked`].
    pub fn footer_action(mut self, link: impl Into<ToastLink>) -> Self {
        self.actions.push(ActionLink {
            link: link.into(),
            footer: true,
        });
        self
    }

    /// Adds the `mineral-dismiss` link to the title row unless [`Self::action`] fills it.
    pub fn dismissible(mut self, dismissible: bool) -> Self {
        self.dismissible = dismissible;
        self
    }
}

/// Queue showing 20 toasts at once with the `dispatchToast` default timeout.
pub fn toast_queue() -> ToastQueue<Toast> {
    ToastQueue::new(d::LIMIT, d::TIMEOUT)
}

#[derive(Clone)]
struct Shown {
    id: ToastId,
    toast: Toast,
    live: Live,
}

/// Result of [`Toaster::show`].
#[derive(Clone, Debug, Default)]
#[non_exhaustive]
pub struct ToasterResponse {
    /// Action pressed this frame.
    pub action_clicked: Option<ToastAction>,
    /// Toast closed by its dismiss link this frame; it is dismissed.
    pub dismissed: Option<ToastId>,
}

struct Painted {
    height: f32,
    hovered: bool,
    clicked: Option<usize>,
    dismissed: bool,
}

/// Fluent `Toaster` over a [`ToastQueue`] of [`Toast`]s.
#[derive(Debug)]
#[must_use = "call .show(..)"]
pub struct Toaster<'a> {
    queue: &'a mut ToastQueue<Toast>,
    key: StableId,
    pause_on_hover: bool,
}

impl<'a> Toaster<'a> {
    /// Shows `queue`; agent key prefix `toast`.
    pub fn new(queue: &'a mut ToastQueue<Toast>) -> Self {
        Self {
            queue,
            key: StableId::new("toast"),
            pause_on_hover: false,
        }
    }

    /// `pauseOnHover`: a hovered toast keeps its timer paused; default off.
    pub fn pause_on_hover(mut self, pause_on_hover: bool) -> Self {
        self.pause_on_hover = pause_on_hover;
        self
    }

    /// Replaces the `toast` key prefix.
    pub fn agent_key(mut self, key: impl Into<StableId>) -> Self {
        self.key = key.into();
        self
    }

    /// Paints the visible toasts stacked at their positions.
    pub fn show(self, rt: &Rt) -> ToasterResponse {
        let ctx = rt.ctx();
        let queue = self.queue;
        queue.update(ctx);
        let host = egui_mineral::kit::scoped_id(rt.core(), Id::unique("mineral::toaster"));
        let mut shown = Vec::<Shown>::load(ctx, host);
        let current: Vec<Shown> = queue
            .visible()
            .map(|q| Shown {
                id: q.id,
                toast: q.payload.clone(),
                live: q.live,
            })
            .collect();
        for c in &current {
            if !shown.iter().any(|s| s.id == c.id) {
                shown.push(c.clone());
            }
        }
        let mut out = ToasterResponse::default();
        let mut stacks = [0.0_f32; 6];
        let mut index = 0_usize;
        shown.retain(|s| {
            let live = current.iter().any(|c| c.id == s.id);
            let key = host.with(s.id);
            let Some(k) = motion::presence(
                rt.core(),
                key,
                if live { Visibility::Visible } else { Visibility::Hidden },
                d::MOTION,
            ) else {
                return false;
            };
            let eased = if live {
                crate::spec::tokens::motion::CURVE_DECELERATE_MID.ease(k)
            } else {
                crate::spec::tokens::motion::CURVE_ACCELERATE_MID.ease(k)
            };
            let stack_slot = &mut stacks[s.toast.position.index()];
            let node = self.key.sub(index);
            let at = Placement {
                key,
                index,
                node: &node,
                eased,
                stack: *stack_slot,
                interactive: live,
            };
            let painted = paint_toast(rt, s, &at);
            *stack_slot += painted.height + d::GAP;
            index += 1;
            if live {
                if self.pause_on_hover {
                    if painted.hovered {
                        queue.pause(s.id);
                    } else {
                        queue.resume(s.id);
                    }
                }
                if let Some(i) = painted.clicked {
                    let key = s.toast.actions.get(i).and_then(|a| a.link.action_key.clone());
                    out.action_clicked = Some(ToastAction {
                        toast: s.id,
                        clicked: Clicked::new(i, key),
                    });
                }
                if painted.dismissed {
                    queue.dismiss(s.id);
                    out.dismissed = Some(s.id);
                }
            }
            true
        });
        shown.store(ctx, host);
        out
    }
}

// react-toast useToastTitleStyles.styles.ts useIntentIconStyles and useIntentIconStylesInverted
fn media(icons: &crate::IconSet, intent: ToastIntent, inverted: bool) -> (Icon, ColorToken) {
    match (intent, inverted) {
        (ToastIntent::Info, false) => (icons.info, ColorNeutralForeground2),
        (ToastIntent::Info, true) => (icons.info, ColorNeutralForegroundInverted2),
        (ToastIntent::Success, false) => (icons.checkmark_circle, ColorStatusSuccessForeground1),
        (ToastIntent::Success, true) => (icons.checkmark_circle, ColorStatusSuccessForegroundInverted),
        (ToastIntent::Warning, false) => (icons.warning, ColorStatusWarningForeground1),
        (ToastIntent::Warning, true) => (icons.warning, ColorStatusWarningForegroundInverted),
        (ToastIntent::Error, false) => (icons.error_circle, ColorStatusDangerForeground1),
        (ToastIntent::Error, true) => (icons.error_circle, ColorStatusDangerForegroundInverted),
    }
}

fn text(ui: &mut egui::Ui, rt: &Rt, s: &str, (size, line, weight): (f32, f32, u16), color: Color32) {
    let format = crate::typography::format(
        rt.ctx(),
        crate::typography::TypeSpec {
            size,
            line_height: line,
            weight,
        },
        color,
    );
    let mut job = egui::text::LayoutJob::single_section(s.to_owned(), format);
    job.wrap.max_width = ui.available_width();
    let galley = ui.painter().layout_job(job);
    let (rect, _) = ui.allocate_exact_size(galley.size(), Sense::hover());
    ui.painter().galley(rect.min, galley, color);
}

struct Placement<'a> {
    key: Id,
    index: usize,
    node: &'a StableId,
    eased: f32,
    stack: f32,
    interactive: bool,
}

struct Palette {
    fill: Color32,
    fg: Color32,
    fg2: Color32,
}

#[derive(Default)]
struct Pressed {
    action: Option<usize>,
    dismissed: bool,
}

fn link(ui: &mut egui::Ui, rt: &Rt, salt: (&'static str, usize, &'static str, usize), label: &str) -> bool {
    rt.ui(ui).mineral(TextLink::new(salt, label)).response().clicked()
}

fn content(ui: &mut egui::Ui, rt: &Rt, toast: &Toast, palette: &Palette, slot: usize, pressed: &mut Pressed) {
    let t = rt.tokens();
    let dir = rt.i18n().direction();
    if let Some(intent) = toast.intent {
        let (icon, color) = media(&rt.icons(), intent, toast.inverted);
        let (rect, _) = ui.allocate_exact_size(
            vec2(d::MEDIA_SIZE, d::MEDIA_SIZE + d::MEDIA_PADDING_TOP),
            Sense::hover(),
        );
        let center = egui::pos2(rect.center().x, rect.top() + d::MEDIA_PADDING_TOP + d::MEDIA_SIZE / 2.0);
        paint::icon(ui.painter(), center, icon, true, d::MEDIA_SIZE, t.color(color));
        ui.add_space(d::MEDIA_GAP);
    }
    ui.scope_builder(UiBuilder::new().layout(dir.column()), |ui| {
        let title = (ty::FONT_SIZE_BASE_300, d::TITLE_LINE_HEIGHT, ty::FONT_WEIGHT_SEMIBOLD);
        text(ui, rt, &toast.title, title, palette.fg);
        let regular = (
            ty::FONT_SIZE_BASE_300,
            ty::LINE_HEIGHT_BASE_300,
            ty::FONT_WEIGHT_REGULAR,
        );
        if let Some(body) = &toast.body {
            ui.add_space(d::BODY_PADDING_TOP);
            text(ui, rt, body, regular, palette.fg);
        }
        if let Some(subtitle) = &toast.subtitle {
            ui.add_space(d::SUBTITLE_PADDING_TOP);
            let small = (ty::FONT_SIZE_BASE_200, ty::FONT_SIZE_BASE_200, ty::FONT_WEIGHT_REGULAR);
            text(ui, rt, subtitle, small, palette.fg2);
        }
        let footer: Vec<(usize, &ActionLink)> = toast.actions.iter().enumerate().filter(|(_, a)| a.footer).collect();
        if !footer.is_empty() {
            ui.add_space(d::FOOTER_PADDING_TOP);
            ui.scope_builder(UiBuilder::new().layout(dir.row()), |ui| {
                ui.spacing_mut().item_spacing.x = d::FOOTER_GAP;
                for (i, a) in footer {
                    if link(ui, rt, ("toast", slot, "footer", i), &a.link.label) {
                        pressed.action = Some(i);
                    }
                }
            });
        }
    });
}

fn title_row(ui: &mut egui::Ui, rt: &Rt, toast: &Toast, palette: &Palette, slot: usize, pressed: &mut Pressed) {
    let dir = rt.i18n().direction();
    let (end, start) = match dir {
        Direction::Ltr => (Layout::right_to_left(Align::Min), Layout::left_to_right(Align::Min)),
        Direction::Rtl => (Layout::left_to_right(Align::Min), Layout::right_to_left(Align::Min)),
    };
    let title_action = toast.actions.iter().enumerate().find(|(_, a)| !a.footer);
    ui.allocate_ui_with_layout(vec2(ui.available_width(), 0.0), end, |ui| {
        if let Some((i, a)) = title_action {
            if link(ui, rt, ("toast", slot, "action", i), &a.link.label) {
                pressed.action = Some(i);
            }
            ui.add_space(d::ACTION_PADDING_START);
        } else if toast.dismissible {
            let label = crate::tr(rt.i18n(), msg::DISMISS);
            pressed.dismissed = link(ui, rt, ("toast", slot, "dismiss", 0), &label);
            ui.add_space(d::ACTION_PADDING_START);
        }
        ui.with_layout(start, |ui| content(ui, rt, toast, palette, slot, pressed));
    });
}

fn paint_toast(rt: &Rt, s: &Shown, at: &Placement<'_>) -> Painted {
    let ctx = rt.ctx();
    let t = rt.tokens();
    let dir = rt.i18n().direction();
    let toast = &s.toast;
    let pos = toast.position;
    let edge = d::EDGE_OFFSET + at.stack - (1.0 - at.eased) * d::SLIDE;
    let offset = vec2(0.0, if pos.bottom() { -edge } else { edge });
    let area = egui_mineral::kit::layer::area(rt.core(), at.key.with("area"))
        .order(Order::Middle)
        .anchor(pos.anchor(dir), offset)
        .interactable(at.interactive);
    area.show(ctx, |ui| {
        ui.multiply_opacity(at.eased);
        let (fill, fg, fg2) = if toast.inverted {
            (
                ColorNeutralBackgroundInverted,
                ColorNeutralForegroundInverted2,
                ColorNeutralForegroundInverted2,
            )
        } else {
            (
                ColorNeutralBackground1,
                ColorNeutralForeground1,
                ColorNeutralForeground2,
            )
        };
        let palette = Palette {
            fill: t.color(fill),
            fg: t.color(fg),
            fg2: t.color(fg2),
        };
        let radius = CornerRadius::from(g::BORDER_RADIUS_MEDIUM);
        let width = d::WIDTH.min(ctx.content_rect().width() - 2.0 * d::EDGE_OFFSET).max(0.0);
        let pad = d::PADDING + g::STROKE_WIDTH_THIN;
        let background = ui.painter().add(egui::Shape::Noop);
        let mut pressed = Pressed::default();
        let framed = egui::Frame::NONE.inner_margin(Margin::same(pad as i8)).show(ui, |ui| {
            ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
            ui.set_width((width - 2.0 * pad).max(0.0));
            title_row(ui, rt, toast, &palette, at.index, &mut pressed);
        });
        let rect = framed.response.rect;
        let mut shapes: Vec<egui::Shape> = t
            .shadow(Elevation::E8, ShadowTint::Neutral)
            .layers()
            .into_iter()
            .map(|layer| egui::Shape::from(layer.as_shape(rect, radius)))
            .collect();
        shapes.push(egui::Shape::Rect(egui::epaint::RectShape::new(
            rect,
            radius,
            palette.fill,
            Stroke::new(g::STROKE_WIDTH_THIN, t.color(ColorTransparentStroke)),
            StrokeKind::Inside,
        )));
        ui.painter().set(background, egui::Shape::Vec(shapes));
        let status = ui.interact(rect, at.key.with("status"), Sense::hover());
        semantic::emit(ctx, Some(status.id), rect, || {
            let role = if s.live == Live::Assertive {
                Role::Alert
            } else {
                Role::Status
            };
            let mut n = SemanticNode::new(at.node.clone(), role, &toast.title);
            n.value.clone_from(&toast.body);
            n.state.live = Some(s.live);
            n
        });
        Painted {
            height: rect.height(),
            hovered: at.interactive && ctx.rect_contains_pointer(ui.layer_id(), rect),
            clicked: pressed.action,
            dismissed: pressed.dismissed,
        }
    })
    .inner
}
