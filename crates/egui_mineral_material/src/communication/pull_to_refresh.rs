use crate::Material;
use crate::MaterialExt;
use crate::Rt;
use egui::{Color32, Painter, Pos2, Rect, Sense, Shape, Stroke, Vec2, emath::Rot2, pos2, vec2};
use egui_mineral::kit::state::TempState;
use egui_mineral::semantic::KeyedSalt;
use egui_mineral::widget::Availability;
use egui_mineral::widget::MineralUi;
use egui_mineral::{
    input::{Pull, PullConfig, PullDrag, PullInput},
    kit,
    layout::Axis,
    motion::Spring,
    semantic::{self, Role, SemanticNode, StableId},
};

use crate::communication::progress;
use crate::widget::{
    common::{self, Interaction},
    indication,
};
use crate::{
    motion,
    spec::{defaults::pull_to_refresh as m, tokens::motion::DURATION_SHORT4},
};

const ANIMATION: Spring = Spring {
    damping_ratio: m::ANIMATION_DAMPING,
    stiffness: m::ANIMATION_STIFFNESS,
    rest_threshold: crate::motion::REST,
};
// MD3 short4 duration
const WHEEL_IDLE: std::time::Duration = DURATION_SHORT4;
const ARC_SEGMENTS: usize = 24;

/// Result of [`PullToRefresh::show`].
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct PullToRefreshResponse<R> {
    /// Hover response over the scroll region.
    pub response: egui::Response,
    /// Output of the content closure.
    pub inner: R,
    /// Pull released past the threshold this frame; start refreshing.
    pub refreshed: bool,
}

#[derive(Clone, Copy, Default)]
struct State {
    pull: Pull,
}

impl TempState for State {}

/// Compose `PullToRefreshBox` with the default `PullToRefreshDefaults.Indicator`. The indicator is
/// also a focusable refresh button for keyboard and agents, shown at the threshold while focused.
#[derive(Clone, Debug)]
#[must_use = "call .show(..)"]
pub struct PullToRefresh {
    salt: KeyedSalt,
    refreshing: bool,
    threshold: f32,
}

impl PullToRefresh {
    /// Agent key prefix `<id_salt>`; threshold `PullToRefreshDefaults.PositionalThreshold`.
    pub fn new(id_salt: impl Into<KeyedSalt>) -> Self {
        let id_salt: KeyedSalt = id_salt.into();
        Self {
            salt: id_salt,
            refreshing: false,
            threshold: m::POSITIONAL_THRESHOLD,
        }
    }

    crate::widget::setters! {
        /// Replaces the `<id_salt>` prefix of the `-refresh-button` key.
        agent_key(salt)
    }

    /// App-owned busy state: shows the spinner and blocks pulls while `true`.
    pub fn refreshing(mut self, refreshing: bool) -> Self {
        self.refreshing = refreshing;
        self
    }

    /// Pull distance that refreshes on release (`PullToRefreshBox` `threshold`).
    ///
    /// Not finite or below 1 becomes 1.
    #[track_caller]
    pub fn threshold(mut self, threshold: f32) -> Self {
        self.threshold = egui_mineral::kit::sanitize::at_least(threshold, 1.0.., "threshold");
        self
    }

    // Compose calculateVerticalOffset
    fn offset(&self, distance: f32) -> f32 {
        let t = self.threshold;
        if distance <= t {
            distance
        } else {
            let over = (distance / t - 1.0).clamp(0.0, 2.0);
            t + t * (over - over * over / 4.0)
        }
    }

    /// Wraps `add` in a vertical scroll area that refreshes on overscroll at the top.
    pub fn show<R>(
        self,
        ui: &mut MineralUi<'_, Material>,
        add: impl FnOnce(&mut MineralUi<'_, Material>) -> R,
    ) -> PullToRefreshResponse<R> {
        let rt = ui.rt();
        let ctx = rt.ctx();
        let id = ui.make_persistent_id(self.salt.id_salt());
        let mut st = State::load(ctx, id);
        let scroll = egui::ScrollArea::vertical()
            .id_salt(id.with("scroll"))
            .auto_shrink([false, false])
            .show(ui, |ui| add(&mut rt.ui(ui)));
        let region = scroll.inner_rect;
        let response = ui.interact(region, id.with("region"), Sense::hover());
        let at_top = scroll.state.clamped_offset().y <= 0.5;
        let mut refresh = self.update_pull(&rt.ui(ui), &mut st, &response, at_top);

        // `PullToRefreshState.distanceFraction`: snapped while pulled, animated otherwise.
        let indicator_salt = id.with("indicator");
        let focused = ctx.memory(|mem| mem.has_focus(indicator_salt));
        let fraction_id = id.with("fraction");
        let fraction = if self.refreshing || focused {
            egui_mineral::motion::spring(rt.core(), fraction_id, 1.0, ANIMATION)
        } else if st.pull.live() {
            let f = self.offset(st.pull.distance()) / self.threshold;
            egui_mineral::motion::snap(ctx, fraction_id, f);
            f
        } else {
            egui_mineral::motion::spring(rt.core(), fraction_id, 0.0, ANIMATION)
        };
        let size = m::CONTAINER_SIZE;
        let top = region.top() + fraction * m::INDICATOR_MAX_DISTANCE - size;
        let rect = Rect::from_min_size(pos2(region.center().x - size / 2.0, top), Vec2::splat(size));
        // Above the region while hidden: no pointer hits, but focus and AccessKit still reach it.
        let resp = kit::input::interact(&rt.ui(ui), rect, indicator_salt, Sense::click(), Availability::Enabled);
        if !self.refreshing && resp.clicked() {
            refresh = true;
        }
        let crossfade = egui_mineral::motion::spring(
            rt.core(),
            id.with("crossfade"),
            f32::from(self.refreshing),
            motion::DEFAULT_EFFECTS,
        );
        let alpha = egui_mineral::motion::spring(
            rt.core(),
            id.with("arrow-alpha"),
            if fraction >= 1.0 { m::MAX_ALPHA } else { m::MIN_ALPHA },
            motion::DEFAULT_EFFECTS,
        );
        if (fraction > 0.0 || self.refreshing) && rect.bottom() > region.top() {
            let indicator = Indicator {
                rect,
                fraction,
                crossfade,
                alpha,
            };
            self.paint_indicator(ui, &resp, region.top(), &indicator);
        }
        self.emit_semantics(rt, resp.id, rect);
        st.store(ctx, id);
        PullToRefreshResponse {
            response,
            inner: scroll.inner,
            refreshed: refresh,
        }
    }

    fn update_pull(
        &self,
        ui: &MineralUi<'_, Material>,
        st: &mut State,
        response: &egui::Response,
        at_top: bool,
    ) -> bool {
        if self.refreshing {
            *st = State::default();
            return false;
        }
        let ctx = ui.ctx();
        let cfg = PullConfig::new(self.threshold, WHEEL_IDLE).resistance(m::DRAG_MULTIPLIER);
        let hovered = ctx.rect_contains_pointer(ui.layer_id(), response.rect);
        let wheel = if hovered {
            ctx.input(|i| i.smooth_scroll_delta.y)
        } else {
            0.0
        };
        let now = ctx.input(|i| i.time);
        let step = st.pull.update(
            &cfg,
            PullInput {
                at_edge: at_top,
                drag: ui.pass_input().pan_on(response).map(|p| PullDrag {
                    delta: Axis::Vertical.main(p.delta),
                    ended: p.ended,
                }),
                wheel,
                now,
            },
        );
        if step.wheel_consumed {
            ctx.input_mut(|i| i.smooth_scroll_delta.y = 0.0);
        }
        if let Some(wait) = step.repaint_after {
            ctx.request_repaint_after(wait);
        }
        step.triggered
    }

    fn paint_indicator(
        &self,
        ui: &mut MineralUi<'_, Material>,
        resp: &egui::Response,
        region_top: f32,
        ind: &Indicator,
    ) {
        let rt = ui.rt();
        let t = rt.tokens();
        let rect = ind.rect;
        // `IndicatorBox` clips at the top of the box only.
        let clip = ui.clip_rect();
        ui.set_clip_rect(clip.intersect(Rect::from_min_max(
            pos2(f32::NEG_INFINITY, region_top),
            pos2(f32::INFINITY, f32::INFINITY),
        )));
        let round = m::INDICATOR_SHAPE.corner_radius(&t.shape, rt.i18n().direction());
        let color = t.role(m::INDICATOR_COLOR);
        let p = ui.painter();
        common::elevation(p, rect, round, m::ELEVATION, &t);
        p.rect_filled(rect, round, t.role(m::INDICATOR_CONTAINER_COLOR));
        indication::paint(
            ui,
            resp,
            rect,
            round,
            color,
            Interaction::of(resp, Availability::disabled_if(self.refreshing)),
            &t,
        );
        let k = ind.crossfade.clamp(0.0, 1.0);
        if k < 1.0 {
            paint_arrow(
                ui.painter(),
                rect.center(),
                ind.fraction,
                common::with_alpha(color, ind.alpha * (1.0 - k)),
            );
        }
        if k > 0.0 {
            progress::paint_indeterminate_ring(
                ui,
                ui.painter(),
                rect.center(),
                m::SPINNER_SIZE,
                m::STROKE_WIDTH,
                common::with_alpha(color, k),
            );
        }
        ui.set_clip_rect(clip);
    }

    fn emit_semantics(&self, rt: &Rt, id: egui::Id, rect: Rect) {
        let ctx = rt.ctx();
        semantic::emit(ctx, Some(id), rect, || {
            let name = crate::tr(rt.i18n(), "mineral-refresh");

            SemanticNode::new(StableId::of(&self.salt, "refresh-button"), Role::Button, name)
                .clickable(Availability::disabled_if(self.refreshing))
        });
    }
}

struct Indicator {
    rect: Rect,
    fraction: f32,
    crossfade: f32,
    alpha: f32,
}

// Compose CircularArrowProgressIndicator, distanceFraction
fn paint_arrow(p: &Painter, center: Pos2, progress: f32, color: Color32) {
    // `ArrowValues`, from `SwipeRefreshLayout`.
    let adjusted = (progress.min(1.0) - 0.4).max(0.0) * 5.0 / 3.0;
    let over = (progress.abs() - 1.0).clamp(0.0, 2.0);
    let tension = over - over * over / 4.0;
    let rotation = (-0.25 + 0.4 * adjusted + tension) * 0.5;
    let start = rotation * 360.0;
    let end = (rotation + adjusted * m::MAX_PROGRESS_ARC) * 360.0;
    let scale = adjusted.min(1.0);
    // Compose rotates the canvas by `rotation` (a fraction of a turn) as degrees.
    let turn = Rot2::from_angle(rotation.to_radians());
    let at = |v: egui::Vec2| center + turn * v;
    let radius = m::ARC_RADIUS + m::STROKE_WIDTH / 2.0;
    let stroke = Stroke::new(m::STROKE_WIDTH, color);
    if end > start {
        let pts = (0..=ARC_SEGMENTS)
            .map(|i| {
                let a = egui::lerp(start..=end, i as f32 / ARC_SEGMENTS as f32).to_radians();
                at(radius * vec2(a.cos(), a.sin()))
            })
            .collect();
        p.add(Shape::line(pts, stroke));
    }
    // `drawArrow`: a stroked open triangle at the arc end, turned back by the stroke width in px,
    // which Compose passes as degrees.
    let (w, h) = (m::ARROW_WIDTH * scale, m::ARROW_HEIGHT * scale);
    let ppp = p.ctx().pixels_per_point();
    let tip = Rot2::from_angle((end - m::STROKE_WIDTH * ppp).to_radians());
    let offset = vec2(radius - w / 2.0, -m::STROKE_WIDTH);
    let pts = [vec2(0.0, 0.0), vec2(w / 2.0, h), vec2(w, 0.0)]
        .map(|v| at(tip * (offset + v)))
        .to_vec();
    p.add(Shape::line(pts, stroke));
}
