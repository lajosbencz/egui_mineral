use crate::tokens::ShadowTint;
use egui::{
    Color32, Context, CornerRadius, Id, InnerResponse, Margin, Memory, Pos2, Rect, Response, Sense, Shape, Stroke,
    StrokeKind, Ui, vec2,
};
use egui_mineral::{anchor::Placement, kit::state::TempState, overlay};

use crate::{
    Rt,
    spec::ColorToken::ColorTransparentStroke,
    spec::tokens::{geometry as g, typography as ty},
    theme::{Elevation, Tokens},
};

// WAI-ARIA APG dialog pattern
#[derive(Clone, Copy, Default)]
pub(crate) struct ModalFocus {
    open: bool,
    enter: bool,
    since: u64,
    leave: bool,
    restore: Option<Id>,
}

impl TempState for ModalFocus {}

impl ModalFocus {
    pub(crate) fn scope<R>(ctx: &Context, id: Id, open: &mut bool, show: impl FnOnce(&mut Self, &mut bool) -> R) -> R {
        let mut focus = Self::load(ctx, id);
        focus.sync(ctx, *open);
        let r = show(&mut focus, open);
        focus.sync(ctx, *open);
        focus.leave(ctx);
        focus.store(ctx, id);
        r
    }

    fn sync(&mut self, ctx: &Context, open: bool) {
        if open && !self.open {
            *self = Self {
                open: true,
                enter: true,
                since: ctx.cumulative_pass_nr(),
                leave: false,
                restore: ctx.memory(Memory::focused),
            };
        } else if !open && self.open {
            self.open = false;
            self.enter = false;
            self.leave = true;
        }
    }

    pub(super) fn enter(&mut self, ctx: &Context, target: Id) {
        if self.enter && ctx.cumulative_pass_nr() > self.since {
            self.enter = false;
            ctx.memory_mut(|m| m.request_focus(target));
        }
    }

    fn leave(&mut self, ctx: &Context) {
        if self.leave
            && overlay::top_modal(ctx).is_none()
            && let Some(r) = self.restore.take()
        {
            self.leave = false;
            ctx.memory_mut(|m| m.request_focus(r));
        }
    }
}

pub(crate) struct Look {
    pub fill: Color32,
    pub radius: f32,
    pub elevation: Elevation,
}

pub(super) fn corner(radius: f32) -> CornerRadius {
    CornerRadius::from(radius)
}

pub(crate) fn framed<R>(
    ui: &mut Ui,
    t: &Tokens,
    look: &Look,
    margin: Margin,
    add: impl FnOnce(&mut Ui) -> R,
) -> InnerResponse<R> {
    let background = ui.painter().add(Shape::Noop);
    let inner = egui::Frame::NONE.inner_margin(margin).show(ui, |ui| {
        ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
        add(ui)
    });
    let rect = inner.response.rect;
    let radius = corner(look.radius);
    let mut shapes: Vec<Shape> = t
        .shadow(look.elevation, ShadowTint::Neutral)
        .layers()
        .into_iter()
        .map(|layer| Shape::from(layer.as_shape(rect, radius)))
        .collect();
    shapes.push(Shape::Rect(egui::epaint::RectShape::new(
        rect,
        radius,
        look.fill,
        Stroke::new(g::STROKE_WIDTH_THIN, t.color(ColorTransparentStroke)),
        StrokeKind::Inside,
    )));
    ui.painter().set(background, Shape::Vec(shapes));
    inner
}

pub(super) fn arrow(ui: &Ui, p: &Placement, rect: Rect, radius: f32, height: f32, fill: Color32) {
    let pad = (2.0 * radius + height).min(rect.width() / 2.0).min(rect.height() / 2.0);
    let x = p
        .arrow
        .x
        .clamp(rect.left() + pad, (rect.right() - pad).max(rect.left() + pad));
    let y = p
        .arrow
        .y
        .clamp(rect.top() + pad, (rect.bottom() - pad).max(rect.top() + pad));
    let points = if p.side.is_vertical() {
        let edge = p.arrow.y;
        let out = if edge <= rect.center().y { -height } else { height };
        vec![
            Pos2::new(x - height, edge),
            Pos2::new(x, edge + out),
            Pos2::new(x + height, edge),
        ]
    } else {
        let edge = p.arrow.x;
        let out = if edge <= rect.center().x { -height } else { height };
        vec![
            Pos2::new(edge, y - height),
            Pos2::new(edge + out, y),
            Pos2::new(edge, y + height),
        ]
    };
    ui.painter().add(Shape::convex_polygon(points, fill, Stroke::NONE));
}

pub(super) fn text(ui: &mut Ui, rt: &Rt, s: &str, (size, line, weight): (f32, f32, u16), color: Color32) -> Response {
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
    let (rect, response) = ui.allocate_exact_size(galley.size(), Sense::hover());
    ui.painter().galley(rect.min, galley, color);
    response
}

pub(super) const BODY1: (f32, f32, u16) = (
    ty::FONT_SIZE_BASE_300,
    ty::LINE_HEIGHT_BASE_300,
    ty::FONT_WEIGHT_REGULAR,
);
pub(super) const CAPTION1: (f32, f32, u16) = (
    ty::FONT_SIZE_BASE_200,
    ty::LINE_HEIGHT_BASE_200,
    ty::FONT_WEIGHT_REGULAR,
);

// Tab cycles inside the surface: a sentinel before and after the content hands focus to the other end.
#[derive(Clone, Copy, Default)]
struct Trap {
    inside: bool,
}

impl TempState for Trap {}

pub(super) struct FocusTrap {
    start: Id,
    end: Id,
    state: Id,
}

impl FocusTrap {
    pub(super) fn new(id: Id) -> Self {
        Self {
            start: id.with("trap-start"),
            end: id.with("trap-end"),
            state: id.with("trap-state"),
        }
    }

    pub(super) fn start(&self) -> Id {
        self.start
    }

    fn sentinel(ui: &Ui, id: Id) {
        ui.interact(
            Rect::from_min_size(ui.cursor().min, vec2(0.0, 0.0)),
            id,
            Sense::focusable_noninteractive(),
        );
    }

    pub(super) fn begin(&self, ui: &Ui) {
        Self::sentinel(ui, self.start);
    }

    pub(super) fn finish(&self, ui: &Ui) {
        Self::sentinel(ui, self.end);
        let ctx = ui.ctx();
        let layer = ui.layer_id();
        let was = Trap::load(ctx, self.state).inside;
        let focused = ctx.memory(Memory::focused);
        if focused == Some(self.end) {
            ctx.memory_mut(|m| {
                m.request_focus(self.start);
                m.move_focus(egui::FocusDirection::Next);
            });
        } else if focused == Some(self.start) && was {
            ctx.memory_mut(|m| {
                m.request_focus(self.end);
                m.move_focus(egui::FocusDirection::Previous);
            });
        }
        let inside = focused.is_some_and(|f| {
            f != self.start && f != self.end && ctx.read_response(f).is_some_and(|r| r.layer_id == layer)
        });
        Trap { inside }.store(ctx, self.state);
    }
}
