use crate::Material;
use crate::MaterialExt;
use crate::Rt;
use crate::TextAction;
use egui_mineral::action::Clicked;
use egui_mineral::kit::state::TempState;
use egui_mineral::widget::MineralUi;
use std::sync::Arc;

use egui::{
    Align2, Color32, Context, CornerRadius, Galley, Id, Order, Rangef, Rect, Response, Sense, Ui, Vec2, pos2, vec2,
};
use egui_mineral::widget::Availability;
use egui_mineral::{
    i18n::Direction,
    kit,
    motion::{self, Visibility},
    semantic::{self, Live, Role, SemanticNode, StableId},
    toast::{ToastId, ToastQueue},
};

use crate::spec::{
    defaults,
    tokens::{baseline_button, small_icon_button, snackbar},
};
use crate::widget::{
    common::{self, Interaction},
    indication, parts,
};

/// Payload of a queued snackbar: text, optional action and close affordance.
#[derive(Clone, Debug)]
#[must_use]
pub struct Snackbar {
    text: String,
    action: Option<TextAction>,
    close: bool,
}

impl Snackbar {
    /// `text` is the message line.
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            action: None,
            close: false,
        }
    }

    /// Action button reporting [`SnackbarHostResponse::action_clicked`].
    pub fn action(mut self, action: impl Into<TextAction>) -> Self {
        self.action = Some(action.into());
        self
    }

    /// Close icon button reporting [`SnackbarHostResponse::dismissed`].
    pub fn dismissible(mut self, dismissible: bool) -> Self {
        self.close = dismissible;
        self
    }
}

#[derive(Clone)]
struct Shown {
    id: ToastId,
    snackbar: Snackbar,
    live: Live,
}

/// Result of [`SnackbarHost::show`].
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct SnackbarHostResponse {
    /// Front snackbar; `None` while the queue is empty.
    pub response: Option<egui::Response>,
    /// Snackbar whose action was pressed this frame; it is dismissed.
    pub action_clicked: Option<ToastId>,
    /// Action of [`Self::action_clicked`], with its [`TextAction::key`]; the index is 0.
    pub clicked: Option<Clicked>,
    /// Snackbar closed from its close button this frame.
    pub dismissed: Option<ToastId>,
}

struct Painted {
    response: egui::Response,
    height: f32,
    hovered: bool,
    action: bool,
    close: bool,
}

// Non-Compose: Compose shows one snackbar at a time
const STACK_GAP: f32 = 8.0;
const SLIDE: f32 = 16.0;

/// Queue with the Compose host defaults: one snackbar at a time, `SnackbarDuration.Short`.
pub fn snackbar_queue() -> ToastQueue<Snackbar> {
    ToastQueue::new(defaults::snackbar::MAX_VISIBLE, defaults::snackbar::DURATION_SHORT)
}

/// MD3 snackbar host over a [`ToastQueue`] of [`Snackbar`]s (Compose `SnackbarHost`).
#[derive(Debug)]
#[must_use = "call .show(..)"]
pub struct SnackbarHost<'a> {
    queue: &'a mut ToastQueue<Snackbar>,
    key: StableId,
}

impl<'a> SnackbarHost<'a> {
    /// Shows the front of `queue`; agent key prefix `snackbar`.
    pub fn new(queue: &'a mut ToastQueue<Snackbar>) -> Self {
        Self {
            queue,
            key: StableId::new("snackbar"),
        }
    }

    /// Replaces the `snackbar` key prefix.
    pub fn agent_key(mut self, key: impl Into<StableId>) -> Self {
        self.key = key.into();
        self
    }

    /// Paints the front snackbar at the bottom center of the screen.
    pub fn show(self, rt: &Rt) -> SnackbarHostResponse {
        run(rt, self.queue, &self.key)
    }
}

fn run(rt: &Rt, queue: &mut ToastQueue<Snackbar>, agent_key: &StableId) -> SnackbarHostResponse {
    let ctx = rt.ctx();
    queue.update(ctx);
    let t = rt.tokens();
    let host = egui_mineral::kit::scoped_id(rt.core(), Id::unique("mineral::snackbar_host"));
    let mut shown = Vec::<Shown>::load(ctx, host);
    let current: Vec<Shown> = queue
        .visible()
        .map(|q| Shown {
            id: q.id,
            snackbar: q.payload.clone(),
            live: q.live,
        })
        .collect();
    for c in &current {
        if !shown.iter().any(|s| s.id == c.id) {
            shown.push(c.clone());
        }
    }
    let (mut action_clicked, mut clicked, mut dismissed) = (None, None, None);
    let mut front = None;
    let mut stack = 0.0;
    shown.retain(|s| {
        let live = current.iter().any(|c| c.id == s.id);
        let key = host.with(s.id);
        let Some(k) = motion::presence(
            rt.core(),
            key,
            if live { Visibility::Visible } else { Visibility::Hidden },
            t.duration.medium[0],
        ) else {
            return false;
        };
        let eased = if live {
            crate::spec::tokens::motion::EASING_STANDARD_DECELERATE_CUBIC_BEZIER.ease(k)
        } else {
            crate::spec::tokens::motion::EASING_STANDARD_ACCELERATE_CUBIC_BEZIER.ease(k)
        };
        let out = paint(
            rt,
            &t,
            s,
            Keys {
                id: key,
                agent: agent_key,
            },
            eased,
            stack,
            live,
        );
        stack += out.height + STACK_GAP;
        if front.is_none() {
            front = Some(out.response.clone());
        }
        if live {
            if out.hovered {
                queue.pause(s.id);
            } else {
                queue.resume(s.id);
            }
            if out.action {
                queue.dismiss(s.id);
                if let Some(action) = &s.snackbar.action {
                    action.fire(ctx);
                    clicked = Some(Clicked::new(0, action.action_key.clone()));
                }
                action_clicked = Some(s.id);
            } else if out.close {
                queue.dismiss(s.id);
                dismissed = Some(s.id);
            }
        }
        true
    });
    shown.store(ctx, host);
    SnackbarHostResponse {
        response: front,
        action_clicked,
        clicked,
        dismissed,
    }
}

fn paint(
    rt: &Rt,
    t: &crate::tokens::Tokens,
    s: &Shown,
    keys: Keys<'_>,
    eased: f32,
    stack: f32,
    interactive: bool,
) -> Painted {
    use defaults::snackbar as d;
    let ctx = rt.ctx();
    let offset = vec2(0.0, -(d::OUTER_PADDING + stack) + (1.0 - eased) * SLIDE);
    // Below modals, menus and popups (`Foreground`), above content.
    let inner = egui_mineral::kit::overlay::area(rt.core(), keys.id.with("area"))
        .order(Order::Middle)
        .anchor(Align2::CENTER_BOTTOM, offset)
        .show(ctx, |ui| {
            ui.multiply_opacity(eased);
            paint_content(ui, rt, t, s, keys, interactive)
        });
    inner.inner
}

#[derive(Clone, Copy)]
struct Keys<'a> {
    id: Id,
    agent: &'a StableId,
}

struct Row<'a> {
    rect: Rect,
    dir: Direction,
    end: f32,
    close_w: f32,
    action_w: f32,
    target: f32,
    sense: Sense,
    keys: Keys<'a>,
}

impl Row<'_> {
    fn x(&self, from_start: f32, w: f32) -> f32 {
        self.dir.start_x(self.rect, Rangef::new(from_start, from_start + w))
    }
}

fn paint_content(
    ui: &mut Ui,
    rt: &Rt,
    t: &crate::tokens::Tokens,
    s: &Shown,
    keys: Keys<'_>,
    interactive: bool,
) -> Painted {
    use defaults::snackbar as d;
    let ctx = rt.ctx();
    let sb = &s.snackbar;
    let p = ui.painter().clone();
    let action_color = t.role(snackbar::ACTION_LABEL_TEXT_COLOR);
    let icon_color = t.role(snackbar::ICON_COLOR);
    let text_color = t.role(snackbar::SUPPORTING_TEXT_COLOR);
    // Compose legacy `OneRowSnackbar`: the text row fills up to `CONTAINER_MAX_WIDTH`.
    let end_pad = if sb.close {
        0.0
    } else {
        d::HORIZONTAL_SPACING_BUTTON_SIDE
    };
    let available = ctx.content_rect().width() - 2.0 * d::OUTER_PADDING;
    let row_w = (available - d::HORIZONTAL_SPACING - end_pad).clamp(0.0, d::CONTAINER_MAX_WIDTH);
    let action_g = sb.action.as_ref().map(|a| {
        parts::type_galley(
            rt.core(),
            &p,
            &a.label,
            snackbar::ACTION_LABEL_TEXT_FONT,
            action_color,
            None,
        )
    });
    // A `TextButton`: `TextButtonContentPadding` around the label, `ButtonDefaults.MinWidth`.
    let action_w = action_g.as_ref().map_or(0.0, |g| {
        (g.size().x + 2.0 * defaults::button::TEXT_SPACE).max(defaults::button::MIN_WIDTH)
    });
    // A popup root: the app density.
    let target = crate::adaptive::min_interactive(rt.kit().density());
    let close_w = if sb.close { target } else { 0.0 };
    let extra = if sb.close { 0.0 } else { d::TEXT_END_EXTRA_SPACING };
    let text_g = parts::type_wrapped(
        &p,
        &sb.text,
        snackbar::SUPPORTING_TEXT_FONT,
        text_color,
        (row_w - action_w - close_w - extra).max(0.0),
        0,
    );
    let text_h = text_g.size().y + 2.0 * d::TEXT_VERTICAL_PADDING;
    let (h, text_y) = if text_g.rows.len() <= 1 {
        let h = snackbar::SINGLE_LINE_CONTAINER_HEIGHT.max(target);
        (h, (h - text_h) / 2.0)
    } else {
        let y = d::HEIGHT_TO_FIRST_LINE - d::TEXT_VERTICAL_PADDING - parts::first_baseline(&text_g);
        (snackbar::TWO_LINES_CONTAINER_HEIGHT.max(y + text_h), y)
    };
    let w = d::HORIZONTAL_SPACING + row_w + end_pad;
    let (rect, status) = ui.allocate_exact_size(vec2(w, h), Sense::hover());
    let r = snackbar::CONTAINER_SHAPE.corner_radius(&t.shape, rt.i18n().direction());
    common::elevation(&p, rect, r, snackbar::CONTAINER_ELEVATION, t);
    p.rect_filled(rect, r, t.role(snackbar::CONTAINER_COLOR));
    let row = Row {
        rect,
        dir: rt.i18n().direction(),
        end: d::HORIZONTAL_SPACING + row_w,
        close_w,
        action_w,
        target,
        sense: if interactive { Sense::click() } else { Sense::hover() },
        keys,
    };
    p.galley(
        pos2(
            row.x(d::HORIZONTAL_SPACING, text_g.size().x),
            rect.top() + text_y + d::TEXT_VERTICAL_PADDING,
        ),
        text_g,
        text_color,
    );

    let (mut close, mut action, mut focused) = (false, false, false);
    if sb.close {
        let r = close_button(ui, rt, &row, icon_color);
        close = r.clicked();
        focused |= r.has_focus();
    }
    if let (Some(g), Some(a)) = (action_g, &sb.action) {
        let r = action_button(&mut rt.ui(ui), ctx, t, &row, g, &a.label, action_color);
        action = r.clicked();
        focused |= r.has_focus();
    }
    semantic::emit(ctx, Some(status.id), rect, || {
        let mut node = SemanticNode::new(keys.agent.sub("status"), Role::Status, &sb.text);
        node.state.live = Some(s.live);
        node
    });
    let hovered = interactive && (ctx.rect_contains_pointer(ui.layer_id(), rect) || focused);
    Painted {
        response: status,
        height: h,
        hovered,
        action,
        close,
    }
}

fn close_button(ui: &mut Ui, rt: &Rt, row: &Row, color: Color32) -> Response {
    let ctx = rt.ctx();
    let target = Rect::from_min_size(
        pos2(
            row.x(row.end - row.close_w, row.close_w),
            row.rect.center().y - row.target / 2.0,
        ),
        Vec2::splat(row.target),
    );
    let size = small_icon_button::CONTAINER_HEIGHT;
    let part = parts::IconPart {
        icon: rt.icons().close,
        icon_size: snackbar::ICON_SIZE,
        layer: Vec2::splat(size),
        radius: CornerRadius::from(size / 2.0),
        color,
        availability: Availability::Enabled,
    };
    let resp = parts::icon_part(&mut rt.ui(ui), target, row.keys.id.with("close"), row.sense, &part);
    semantic::emit(ctx, Some(resp.id), target, || {
        let label = crate::tr(rt.i18n(), "mineral-dismiss");
        SemanticNode::button(row.keys.agent.sub("close"), &label, Availability::Enabled)
    });
    resp
}

fn action_button(
    ui: &mut MineralUi<'_, Material>,
    ctx: &Context,
    t: &crate::tokens::Tokens,
    row: &Row,
    galley: Arc<Galley>,
    label: &str,
    color: Color32,
) -> Response {
    let rt = ui.rt();
    let target = Rect::from_min_size(
        pos2(
            row.x(row.end - row.close_w - row.action_w, row.action_w),
            row.rect.center().y - row.target / 2.0,
        ),
        vec2(row.action_w, row.target),
    );
    let resp = kit::input::interact(
        &rt.ui(ui),
        target,
        row.keys.id.with("action"),
        row.sense,
        Availability::Enabled,
    );
    let height = baseline_button::CONTAINER_HEIGHT;
    let container = Rect::from_center_size(target.center(), vec2(row.action_w, height));
    let i = Interaction::of(&resp, Availability::Enabled);
    indication::paint(ui, &resp, container, CornerRadius::from(height / 2.0), color, i, t);
    ui.painter()
        .galley(container.center() - galley.size() / 2.0, galley, color);
    semantic::emit(ctx, Some(resp.id), target, || {
        SemanticNode::button(row.keys.agent.sub("action"), label, Availability::Enabled)
    });
    resp
}
