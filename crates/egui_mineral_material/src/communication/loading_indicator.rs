use crate::Material;
use crate::MaterialExt;
use egui_mineral::semantic::AgentKey;
use egui_mineral::semantic::KeyedSalt;
use egui_mineral::widget::MineralUi;
use std::{
    f32::consts::{PI, TAU},
    sync::OnceLock,
};

use egui::{Color32, Mesh, Pos2, Rect, Shape, Stroke, Ui, Vec2, vec2};
use egui_mineral::{
    layout::Constraints,
    motion,
    semantic::{Role, SemanticNode},
};

use crate::{
    spec::{defaults::loading_indicator as lx, tokens::loading_indicator as li},
    widget::paint::paint_widget,
};

const N: usize = 72;
const MORPH_SPRING: motion::Spring = motion::Spring {
    damping_ratio: lx::MORPH_DAMPING,
    stiffness: lx::MORPH_STIFFNESS,
    rest_threshold: crate::motion::REST,
};

fn shapes() -> &'static [[f32; N]; 7] {
    static SHAPES: OnceLock<[[f32; N]; 7]> = OnceLock::new();
    SHAPES.get_or_init(|| {
        let polar: [&dyn Fn(f32) -> f32; 7] = [
            &|a| lobed(a, 10, 0.16, 0.6),
            &|a| lobed(a, 9, 0.12, 1.0),
            &|a| rounded_polygon(a, &regular(5, -PI / 2.0), 0.18),
            &|a| rounded_polygon(a + PI / 4.0, &[vec2(-0.45, 0.0), vec2(0.45, 0.0)], 0.55),
            &|a| lobed(a, 8, 0.1, 0.6),
            &|a| lobed(a, 4, 0.2, 1.0),
            &|a| ellipse(a - PI / 4.0, 1.0, 0.64),
        ];
        polar.map(|f| {
            let mut r: [f32; N] = std::array::from_fn(|i| f(i as f32 / N as f32 * TAU));
            let max = r.iter().copied().fold(f32::MIN, f32::max);
            for v in &mut r {
                *v /= max;
            }
            r
        })
    })
}

fn lobed(a: f32, n: u32, k: f32, sharp: f32) -> f32 {
    let f = (1.0 - (a * n as f32).cos()) / 2.0;
    1.0 - k * f.powf(sharp)
}

fn ellipse(a: f32, rx: f32, ry: f32) -> f32 {
    rx * ry / ((ry * a.cos()).powi(2) + (rx * a.sin()).powi(2)).sqrt()
}

fn regular(n: usize, start: f32) -> Vec<Vec2> {
    (0..n)
        .map(|i| {
            let a = start + i as f32 / n as f32 * TAU;
            vec2(a.cos(), a.sin())
        })
        .collect()
}

fn distance(p: Vec2, poly: &[Vec2]) -> f32 {
    let edges = poly.iter().zip(poly.iter().cycle().skip(1));
    let inside = poly.len() > 2
        && edges
            .clone()
            .all(|(a, b)| (*b - *a).x * (p - *a).y - (*b - *a).y * (p - *a).x >= 0.0);
    if inside {
        return 0.0;
    }
    edges
        .map(|(a, b)| {
            let ab = *b - *a;
            let t = ((p - *a).dot(ab) / ab.length_sq().max(f32::EPSILON)).clamp(0.0, 1.0);
            (p - (*a + ab * t)).length()
        })
        .fold(f32::MAX, f32::min)
}

fn rounded_polygon(a: f32, poly: &[Vec2], round: f32) -> f32 {
    let dir = vec2(a.cos(), a.sin());
    let (mut lo, mut hi) = (0.0_f32, 4.0_f32);
    for _ in 0..24 {
        let mid = f32::midpoint(lo, hi);
        if distance(dir * mid, poly) < round {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    lo
}

fn fill(p: &egui::Painter, center: Pos2, pts: Vec<Pos2>, color: Color32) {
    let mut mesh = Mesh::default();
    mesh.colored_vertex(center, color);
    for &q in &pts {
        mesh.colored_vertex(q, color);
    }
    let n = pts.len() as u32;
    for i in 0..n {
        mesh.add_triangle(0, 1 + i, 1 + (i + 1) % n);
    }
    p.add(Shape::mesh(mesh));
    p.add(Shape::closed_line(pts, Stroke::new(1.0, color)));
}

/// M3 Expressive loading indicator (Compose `LoadingIndicator`, indeterminate): a morphing shape,
/// optionally on its container (`ContainedLoadingIndicator`). The `MaterialShapes` outlines are
/// polar approximations.
#[derive(Clone, Debug)]
#[must_use = "show with ui.mineral(..)"]
pub struct LoadingIndicator {
    salt: KeyedSalt,
    key: AgentKey,
    label: Option<String>,
    contained: bool,
}

impl LoadingIndicator {
    /// Agent key `<id_salt>-loading`.
    pub fn new(id_salt: impl Into<KeyedSalt>) -> Self {
        let id_salt: KeyedSalt = id_salt.into();
        Self {
            key: AgentKey::Kind("loading"),
            salt: id_salt,
            label: None,
            contained: false,
        }
    }
    /// Accessible name; default the `mineral-loading` message.
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }
    /// Draws the indicator on a circular container (`LoadingIndicatorTokens.Contained*`).
    pub fn contained(mut self, contained: bool) -> Self {
        self.contained = contained;
        self
    }
    crate::widget::setters! {
        /// Replaces the `<id_salt>-loading` agent key.
        agent_key(key)
    }

    #[expect(clippy::unused_self, reason = "`paint_widget!` calls it as a method")]
    fn extent(&self, _: &Ui, c: Constraints) -> Vec2 {
        c.constrain(Vec2::splat(li::CONTAINER_WIDTH))
    }

    fn paint(&self, ui: &mut MineralUi<'_, Material>, rect: Rect) {
        let rt = ui.rt();
        let ctx = rt.ctx();
        let size = rect.size().min_elem();
        if ui.is_rect_visible(rect) {
            let t = rt.tokens();
            let p = ui.painter();
            let active = if self.contained {
                p.circle_filled(rect.center(), size / 2.0, t.role(li::CONTAINED_CONTAINER_COLOR));
                t.role(li::CONTAINED_ACTIVE_COLOR)
            } else {
                t.role(li::ACTIVE_INDICATOR_COLOR)
            };
            let (from, to, k, angle) = if rt.core().motion().reduced() {
                (0, 0, 0.0, 0.0)
            } else {
                egui_mineral::motion::request_frame(ctx);
                let time = ui.input(|i| i.time);
                let interval = lx::MORPH_INTERVAL.as_secs_f64();
                let step = (time / interval).floor();
                let k = MORPH_SPRING.at((time - step * interval) as f32);
                #[expect(clippy::cast_sign_loss, reason = "floor of non-negative egui time")]
                let i = step as usize;
                let global = (time / lx::GLOBAL_ROTATION.as_secs_f64()).fract() as f32 * TAU;
                let n = shapes().len();
                (
                    i % n,
                    (i + 1) % n,
                    k,
                    (step as f32 + k) * lx::MORPH_ROTATION.to_radians() + global,
                )
            };
            let shapes = shapes();
            let radius = size * li::ACTIVE_SIZE / li::CONTAINER_WIDTH / 2.0;
            // Compose clips the overshooting morph to the circular container.
            let clip = size / 2.0 / radius;
            let pts = (0..N)
                .map(|j| {
                    let r = (shapes[from][j] + (shapes[to][j] - shapes[from][j]) * k).clamp(0.0, clip);
                    let a = j as f32 / N as f32 * TAU + angle;
                    rect.center() + vec2(a.cos(), a.sin()) * r * radius
                })
                .collect();
            fill(p, rect.center(), pts, active);
        }
    }

    fn node(&self, i18n: &egui_mineral::i18n::I18n) -> SemanticNode {
        let name = crate::tr_or(i18n, self.label.as_deref(), "mineral-loading");
        SemanticNode::new(self.key.id(&self.salt), Role::ProgressIndicator, name)
    }
}

paint_widget!(LoadingIndicator);
