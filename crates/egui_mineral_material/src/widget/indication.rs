// Compose ripple(), material-ripple RippleAnimation.kt

use egui::{
    Color32, CornerRadius, Pos2, Rect, Response, Shape, Stroke,
    epaint::{RoundedRect, tessellator::path},
};
use egui_mineral::kit::state::TempState;
use egui_mineral::motion::{CubicBezier, request_frame};

use crate::tokens::Tokens;
use crate::widget::common::{Interaction, StateLayer, with_alpha};

const FADE_IN: f64 = 0.075;
const GROW: f64 = 0.225;
const FADE_OUT: f64 = 0.15;
const START_RADIUS: f32 = 0.3;
const EXTRA_RADIUS: f32 = 10.0;
// Compose FastOutSlowInEasing
const FAST_OUT_SLOW_IN: CubicBezier = CubicBezier::new(0.4, 0.0, 0.2, 1.0);
const CIRCLE_SEGMENTS: usize = 48;

#[derive(Clone, Copy, Debug)]
struct Ripple {
    origin: Pos2,
    start: f64,
    release: Option<f64>,
}

impl TempState for Ripple {}

pub(crate) fn paint(
    ui: &egui_mineral::widget::MineralUi<'_, crate::Material>,
    response: &Response,
    rect: Rect,
    r: CornerRadius,
    content: Color32,
    i: Interaction,
    t: &Tokens,
) {
    let p = ui.painter();
    if let Some(layer) = egui_mineral::kit::paint::overlay(rect, r, content, i.with_pressed(false).layer_opacity(t)) {
        p.add(layer);
    }
    let Some((origin, k, alpha)) = ripple(ui, response, rect.center(), i) else {
        return;
    };
    let start = rect.size().max_elem() * START_RADIUS;
    let end = rect.size().length() / 2.0 + EXTRA_RADIUS;
    let radius = egui::lerp(start..=end, FAST_OUT_SLOW_IN.ease(k));
    let center = origin.lerp(rect.center(), k);
    let points = clip_circle(center, radius, RoundedRect::new(rect, r));
    if points.len() >= 3 {
        p.add(Shape::convex_polygon(
            points,
            with_alpha(content, t.state.pressed * alpha),
            Stroke::NONE,
        ));
    }
}

pub(crate) fn paint_unbounded(
    ui: &egui_mineral::widget::MineralUi<'_, crate::Material>,
    response: &Response,
    node: Rect,
    radius: f32,
    content: Color32,
    i: Interaction,
    t: &Tokens,
) {
    let p = ui.painter();
    let center = node.center();
    let layer = i.with_pressed(false).layer_opacity(t);
    if layer > 0.0 {
        p.circle_filled(center, radius, with_alpha(content, layer));
    }
    if let Some((_, k, alpha)) = ripple(ui, response, center, i) {
        let start = node.size().max_elem() * START_RADIUS;
        let r = egui::lerp(start..=radius, FAST_OUT_SLOW_IN.ease(k));
        p.circle_filled(center, r, with_alpha(content, t.state.pressed * alpha));
    }
}

fn ripple(
    ui: &egui_mineral::widget::MineralUi<'_, crate::Material>,
    response: &Response,
    center: Pos2,
    i: Interaction,
) -> Option<(Pos2, f32, f32)> {
    let rt = ui.rt();
    let ctx = ui.ctx();
    let key = response.id.with("mineral-material::ripple");
    let now = ctx.input(|input| input.time);
    let down = !i.disabled && response.is_pointer_button_down_on();
    let prev = Ripple::try_load(ctx, key);
    let ripple = match prev {
        Some(rp) if rp.release.is_none() && !down => Some(Ripple {
            release: Some(now),
            ..rp
        }),
        _ if down && prev.is_none_or(|rp| rp.release.is_some()) => Some(Ripple {
            origin: response.interact_pointer_pos().unwrap_or(center),
            start: now,
            release: None,
        }),
        // A press and release within one pass, or keyboard and accessibility activation (centered).
        _ if !i.disabled && response.clicked() => Some(Ripple {
            origin: response.interact_pointer_pos().unwrap_or(center),
            start: now,
            release: Some(now),
        }),
        _ => prev,
    };
    let Some(rp) = ripple.filter(|_| !rt.core().motion().reduced()) else {
        Ripple::remove(ctx, key);
        return None;
    };
    let age = now - rp.start;
    let alpha = match rp.release {
        // Compose jumps to full alpha when released while fading in.
        Some(release) => 1.0 - ((now - release.max(rp.start + GROW)) / FADE_OUT).clamp(0.0, 1.0),
        None => (age / FADE_IN).min(1.0),
    } as f32;
    if alpha <= 0.0 {
        Ripple::remove(ctx, key);
        return None;
    }
    rp.store(ctx, key);
    request_frame(ctx);
    Some((rp.origin, (age / GROW).min(1.0) as f32, alpha))
}

fn clip_circle(center: Pos2, radius: f32, shape: RoundedRect) -> Vec<Pos2> {
    let mut clip = Vec::new();
    path::rounded_rectangle(&mut clip, shape);
    let mut out: Vec<Pos2> = (0..CIRCLE_SEGMENTS)
        .map(|n| center + radius * egui::Vec2::angled(std::f32::consts::TAU * n as f32 / CIRCLE_SEGMENTS as f32))
        .collect();
    let area: f32 = clip
        .iter()
        .zip(clip.iter().cycle().skip(1))
        .map(|(a, b)| a.x * b.y - b.x * a.y)
        .sum();
    let side = |a: Pos2, b: Pos2, q: Pos2| (b - a).x * (q - a).y - (b - a).y * (q - a).x;
    for (&a, &b) in clip.iter().zip(clip.iter().cycle().skip(1)) {
        let inside = |q: Pos2| side(a, b, q) * area >= 0.0;
        let input = std::mem::take(&mut out);
        for (&s, &e) in input.iter().zip(input.iter().cycle().skip(1)) {
            let cross = || {
                let (ds, de) = (side(a, b, s), side(a, b, e));
                s + (e - s) * (ds / (ds - de))
            };
            match (inside(s), inside(e)) {
                (true, true) => out.push(e),
                (true, false) => out.push(cross()),
                (false, true) => out.extend([cross(), e]),
                (false, false) => {}
            }
        }
    }
    out
}
