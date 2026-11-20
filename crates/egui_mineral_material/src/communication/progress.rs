use crate::Material;
use crate::MaterialExt;
use egui_mineral::layout::Fraction;
use egui_mineral::semantic::AgentKey;
use egui_mineral::semantic::KeyedSalt;
use egui_mineral::widget::MineralUi;
use std::f32::consts::TAU;

use egui::{Color32, CornerRadius, Painter, Pos2, Rangef, Rect, Shape, Stroke, Ui, Vec2, pos2, vec2};
use egui_mineral::{
    i18n::Direction,
    layout::Constraints,
    motion::CubicBezier,
    semantic::{NumericRange, Role, SemanticNode, StableId},
};

use crate::{
    spec::{
        defaults,
        tokens::{
            circular_progress_indicator as circular, linear_progress_indicator as linear, motion as m,
            progress_indicator as progress,
        },
    },
    widget::paint::paint_widget,
};

use defaults::progress_indicator as pi;

fn node(i18n: &egui_mineral::i18n::I18n, id: StableId, label: Option<&str>, value: Option<f32>) -> SemanticNode {
    let name = label.map_or_else(|| crate::tr(i18n, "mineral-progress"), str::to_owned);
    let mut n = SemanticNode::new(id, Role::ProgressIndicator, name);
    if let Some(v) = value {
        n.state.range = Some(NumericRange {
            value: f64::from(v),
            min: 0.0,
            max: 1.0,
            step: None,
        });
        let pct = (f64::from(v) * 100.0).round();
        n.value = Some(i18n.tr("mineral-percent", &[("value", pct.into())]));
    }
    n
}

/// Track shape of a progress indicator.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ProgressVariant {
    /// Compose `LinearProgressIndicator`, `CircularProgressIndicator`: a straight track.
    #[default]
    Flat,
    /// Compose `LinearWavyProgressIndicator`, `CircularWavyProgressIndicator`: the active track waves.
    Wavy,
}

#[derive(Clone, Debug)]
struct Spec {
    key: AgentKey,
    label: Option<String>,
    value: Option<f32>,
    wavy: bool,
}

impl Spec {
    fn new(value: Option<Fraction>) -> Self {
        Self {
            key: AgentKey::Kind("progress"),
            label: None,
            value: value.map(Fraction::get),
            wavy: false,
        }
    }

    fn node(&self, salt: &KeyedSalt, i18n: &egui_mineral::i18n::I18n) -> SemanticNode {
        node(i18n, self.key.id(salt), self.label.as_deref(), self.value)
    }
}

/// Constructors and setters over `salt` and `spec: Spec` fields.
macro_rules! indicator_setters {
    ($motion:literal, $wavy:literal) => {
        /// `value` is the fraction done; agent key `<id_salt>-progress`.
        pub fn determinate(id_salt: impl Into<KeyedSalt>, value: impl Into<Fraction>) -> Self {
            Self {
                salt: id_salt.into(),
                spec: Spec::new(Some(value.into())),
            }
        }

        #[doc = concat!($motion, ", no value; agent key `<id_salt>-progress`.")]
        pub fn indeterminate(id_salt: impl Into<KeyedSalt>) -> Self {
            Self {
                salt: id_salt.into(),
                spec: Spec::new(None),
            }
        }

        /// Accessible name; default the `mineral-progress` message.
        pub fn label(mut self, label: impl Into<String>) -> Self {
            self.spec.label = Some(label.into());
            self
        }

        #[doc = concat!("Default [`ProgressVariant::Flat`]; [`ProgressVariant::Wavy`] is Compose `", $wavy, "`.")]
        pub fn variant(mut self, variant: ProgressVariant) -> Self {
            self.spec.wavy = variant == ProgressVariant::Wavy;
            self
        }

        crate::widget::setters! {
            /// Replaces the `<id_salt>-progress` agent key.
            agent_key(key) |s| s.spec.key
        }

        fn node(&self, i18n: &egui_mineral::i18n::I18n) -> SemanticNode {
            self.spec.node(&self.salt, i18n)
        }
    };
}

paint_widget!(LinearProgressIndicator);
paint_widget!(CircularProgressIndicator);

fn loop_time(ui: &egui_mineral::widget::MineralUi<'_, crate::Material>, cycle: std::time::Duration) -> f32 {
    let rt = ui.rt();
    let cycle = cycle.as_secs_f64();
    if rt.core().motion().reduced() {
        // A frame that shows the active indicator.
        return (cycle / 2.0) as f32;
    }
    egui_mineral::motion::request_frame(ui.ctx());
    ui.input(|i| i.time).rem_euclid(cycle) as f32
}

fn keyframe(t: f32, [delay, duration]: [std::time::Duration; 2], easing: CubicBezier) -> f32 {
    easing.ease(((t - delay.as_secs_f32()) / duration.as_secs_f32()).clamp(0.0, 1.0))
}

fn wave_amplitude(ui: &egui_mineral::widget::MineralUi<'_, crate::Material>, id: egui::Id, value: Option<f32>) -> f32 {
    let rt = ui.rt();
    let on = value.is_none_or(|v| v > pi::WAVE_START_PROGRESS && v < pi::WAVE_END_PROGRESS);
    let key = id.with("wave-amplitude");
    let ctx = ui.ctx();
    let secs = rt.core().motion().duration(m::DURATION_LONG2).as_secs_f32();
    let easing: fn(f32) -> f32 = if on {
        |t| m::EASING_STANDARD_CUBIC_BEZIER.ease(t)
    } else {
        |t| m::EASING_EMPHASIZED_ACCELERATE_CUBIC_BEZIER.ease(t)
    };
    ctx.animate_bool_with_time_and_easing(key, on, secs, easing)
}

fn wave_offset(ui: &egui_mineral::widget::MineralUi<'_, crate::Material>, amplitude: f32) -> f32 {
    let rt = ui.rt();
    if amplitude > 0.0 && !rt.core().motion().reduced() {
        egui_mineral::motion::request_frame(ui.ctx());
        ui.input(|i| i.time).rem_euclid(1.0) as f32
    } else {
        0.0
    }
}

fn stroke_path(p: &Painter, pts: Vec<Pos2>, width: f32, color: Color32) {
    if let (Some(&a), Some(&b)) = (pts.first(), pts.last()) {
        p.circle_filled(a, width / 2.0, color);
        p.circle_filled(b, width / 2.0, color);
        p.add(Shape::line(pts, Stroke::new(width, color)));
    }
}

#[derive(Clone, Copy, Debug)]
struct Wave {
    amplitude: f32,
    length: f32,
    offset: f32,
}

/// MD3 linear progress indicator (Compose `LinearProgressIndicator`); no value is indeterminate.
#[derive(Clone, Debug)]
#[must_use = "show with ui.mineral(..)"]
pub struct LinearProgressIndicator {
    salt: KeyedSalt,
    spec: Spec,
}

impl LinearProgressIndicator {
    indicator_setters!("Animated", "LinearWavyProgressIndicator");

    // Fills `rect`, e.g. a slot of a painted row, in a child `Ui`.
    #[cfg(feature = "extras")]
    pub(crate) fn show_in(self, ui: &mut MineralUi<'_, Material>, rect: Rect) {
        use egui_mineral::widget::SizeExt as _;
        let layout = egui::Layout::centered_and_justified(egui::Direction::TopDown);
        egui_mineral::layout::child(ui, self.salt.clone(), rect, layout, |ui| {
            ui.mineral(self.preferred_size(rect.size()))
        });
    }

    // Compose LinearIndicatorWidth
    fn extent(&self, _: &Ui, c: Constraints) -> Vec2 {
        c.constrain(vec2(
            pi::LINEAR_WIDTH,
            if self.spec.wavy {
                linear::WAVE_HEIGHT
            } else {
                linear::HEIGHT
            },
        ))
    }

    fn paint(&self, ui: &mut MineralUi<'_, Material>, rect: Rect) {
        let rt = ui.rt();
        let t = rt.tokens();
        let id = ui.make_persistent_id(self.salt.id_salt());
        if ui.is_rect_visible(rect) {
            let bar = LinearBar {
                rect,
                dir: rt.i18n().direction(),
                active: t.role(progress::ACTIVE_INDICATOR_COLOR),
                track: t.role(progress::TRACK_COLOR),
            };
            let p = ui.painter();
            let lines = |time: f32| {
                let e = m::EASING_EMPHASIZED_ACCELERATE_CUBIC_BEZIER;
                [
                    (
                        keyframe(time, pi::FIRST_LINE_TAIL, e),
                        keyframe(time, pi::FIRST_LINE_HEAD, e),
                    ),
                    (
                        keyframe(time, pi::SECOND_LINE_TAIL, e),
                        keyframe(time, pi::SECOND_LINE_HEAD, e),
                    ),
                ]
            };
            match (self.spec.value, self.spec.wavy) {
                (Some(v), false) => bar.determinate(p, v),
                (None, false) => bar.indeterminate(p, lines(loop_time(ui, pi::LINEAR_CYCLE))),
                (value, true) => {
                    let amplitude = match value {
                        Some(_) => wave_amplitude(ui, id, value),
                        None => 1.0,
                    };
                    let wave = Wave {
                        amplitude,
                        length: if value.is_some() {
                            linear::ACTIVE_WAVE_WAVELENGTH
                        } else {
                            linear::INDETERMINATE_ACTIVE_WAVE_WAVELENGTH
                        },
                        offset: wave_offset(ui, amplitude),
                    };
                    let pairs = match value {
                        Some(v) => vec![(0.0, v)],
                        None => lines(loop_time(ui, pi::LINEAR_CYCLE)).to_vec(),
                    };
                    bar.wavy(p, &pairs, wave, value);
                }
            }
        }
    }
}

const LINEAR_CAP: f32 = linear::TRACK_THICKNESS / 2.0;

struct LinearBar {
    rect: Rect,
    dir: Direction,
    active: Color32,
    track: Color32,
}

impl LinearBar {
    // Cap-inset position; collapses to `LINEAR_CAP` when the bar is narrower than two caps.
    fn inset(&self, along: f32) -> f32 {
        along.clamp(LINEAR_CAP, (self.rect.width() - LINEAR_CAP).max(LINEAR_CAP))
    }

    fn x(&self, along: f32) -> f32 {
        self.dir.start_x(self.rect, Rangef::point(along))
    }

    fn segment(&self, p: &Painter, a: f32, b: f32, width: f32, color: Color32) {
        let (x0, x1) = (self.x(a).min(self.x(b)), self.x(a).max(self.x(b)));
        let half = width / 2.0;
        let r = Rect::from_x_y_ranges(
            x0 - half..=x1 + half,
            self.rect.center().y - half..=self.rect.center().y + half,
        );
        p.rect_filled(r, CornerRadius::from(half), color);
    }

    fn line(&self, p: &Painter, a: f32, b: f32, color: Color32) {
        if (b - a).abs() > 0.0 {
            let w = self.rect.width();
            self.segment(p, self.inset(a * w), self.inset(b * w), linear::ACTIVE_THICKNESS, color);
        }
    }

    fn stop(&self, p: &Painter) {
        let h = self.rect.height();
        let size = linear::STOP_SIZE.min(h);
        let offset = ((h - size) / 2.0).min(pi::STOP_MAX_TRAILING_SPACE);
        let center = pos2(self.x(self.rect.width() - size / 2.0 - offset), self.rect.center().y);
        p.circle_filled(center, size / 2.0, self.active);
    }

    fn gap(&self) -> f32 {
        (linear::TRACK_ACTIVE_SPACE + linear::TRACK_THICKNESS) / self.rect.width().max(1.0)
    }

    fn determinate(&self, p: &Painter, v: f32) {
        let start = v + v.min(self.gap());
        if start <= 1.0 {
            self.line(p, start, 1.0, self.track);
        }
        self.line(p, 0.0, v, self.active);
        self.stop(p);
    }

    // Compose LinearProgressIndicator
    fn indeterminate(&self, p: &Painter, lines: [(f32, f32); 2]) {
        let gap = self.gap();
        let [(tail1, head1), (tail2, head2)] = lines;
        if head1 < 1.0 - gap {
            let start = if head1 > 0.0 { head1 + gap } else { 0.0 };
            self.line(p, start, 1.0, self.track);
        }
        if head1 - tail1 > 0.0 {
            self.line(p, head1, tail1, self.active);
        }
        if tail1 > gap {
            let start = if head2 > 0.0 { head2 + gap } else { 0.0 };
            let end = if tail1 < 1.0 { tail1 - gap } else { 1.0 };
            self.line(p, start, end, self.track);
        }
        if head2 - tail2 > 0.0 {
            self.line(p, head2, tail2, self.active);
        }
        if tail2 > gap {
            let end = if tail2 < 1.0 { tail2 - gap } else { 1.0 };
            self.line(p, 0.0, end, self.track);
        }
    }

    fn wavy(&self, p: &Painter, pairs: &[(f32, f32)], wave: Wave, value: Option<f32>) {
        let (w, cap) = (self.rect.width(), LINEAR_CAP);
        let half_height = self.rect.height() / 2.0;
        let peak = self.rect.height() - linear::ACTIVE_THICKNESS;
        let shift = if wave.amplitude > 0.0 {
            wave.offset * wave.length
        } else {
            0.0
        };
        // Quadratic half waves alternating in sign, the `fullProgressPath`.
        let wave_y = |s: f32| {
            let half = wave.length / 2.0;
            let k = (s / half).floor();
            let u = s / half - k;
            let sign = if k.rem_euclid(2.0) == 0.0 { 1.0 } else { -1.0 };
            sign * 2.0 * u * (1.0 - u) * peak
        };
        let flip = self.dir.sign();
        let mut gap = 0.0;
        let mut visible = false;
        let mut next_end = self.inset(w);
        for (i, &(tail, head)) in pairs.iter().enumerate() {
            let (bar_tail, bar_head) = (tail * w, head * w);
            if i == 0 {
                gap = if bar_head < cap {
                    0.0
                } else {
                    (bar_head - cap).min(linear::TRACK_ACTIVE_SPACE)
                };
                visible = bar_head >= cap;
            }
            let a_head = self.inset(bar_head);
            let a_tail = self.inset(bar_tail);
            if (head - tail).abs() > 0.0 {
                #[expect(clippy::cast_sign_loss, reason = ">= 1")]
                let steps = ((a_head - a_tail).abs() / 2.0).ceil().max(1.0) as usize;
                let pts = (0..=steps)
                    .map(|n| {
                        let x = egui::lerp(a_tail..=a_head, n as f32 / steps as f32);
                        let y = half_height + wave.amplitude * wave_y(x + shift);
                        pos2(self.x(x), self.rect.top() + half_height + flip * (y - half_height))
                    })
                    .collect();
                stroke_path(p, pts, linear::ACTIVE_THICKNESS, self.active);
            }
            let spacing = gap + if visible { 2.0 * cap } else { 0.0 };
            if next_end > a_head + spacing {
                let to = cap.max(a_head + spacing);
                self.segment(p, to, next_end, linear::TRACK_THICKNESS, self.track);
            }
            if bar_head > bar_tail {
                next_end = cap.max(a_tail - spacing);
            }
        }
        if next_end > cap {
            self.segment(p, cap, next_end, linear::TRACK_THICKNESS, self.track);
        }
        if let Some(v) = value {
            // Wavy `drawStopIndicator`: it shrinks once the progress reaches it.
            let mut size = linear::TRACK_THICKNESS.min(linear::STOP_SIZE);
            let mut x = w - size;
            let progress_x = w * v + cap;
            if x <= progress_x {
                size = (size - (progress_x - x)).max(0.0);
                x = progress_x;
            }
            if size > 0.0 {
                p.circle_filled(
                    pos2(self.x(x + size / 2.0), self.rect.center().y),
                    size / 2.0,
                    self.active,
                );
            }
        }
    }
}

/// MD3 circular progress indicator (Compose `CircularProgressIndicator`); no value is indeterminate.
#[derive(Clone, Debug)]
#[must_use = "show with ui.mineral(..)"]
pub struct CircularProgressIndicator {
    salt: KeyedSalt,
    spec: Spec,
}

impl CircularProgressIndicator {
    indicator_setters!("Spinning", "CircularWavyProgressIndicator");

    // Fills `rect`, e.g. an icon slot of a painted row, in a child `Ui`.
    #[cfg(feature = "extras")]
    pub(crate) fn show_in(self, ui: &mut MineralUi<'_, Material>, rect: Rect) {
        use egui_mineral::widget::SizeExt as _;
        let layout = egui::Layout::centered_and_justified(egui::Direction::TopDown);
        egui_mineral::layout::child(ui, self.salt.clone(), rect, layout, |ui| {
            ui.mineral(self.preferred_size(rect.size()))
        });
    }

    fn extent(&self, _: &Ui, c: Constraints) -> Vec2 {
        let extent = c.constrain(Vec2::splat(if self.spec.wavy {
            circular::WAVE_SIZE
        } else {
            circular::SIZE
        }));
        if self.spec.wavy && extent.min_elem() < circular::WAVE_SIZE {
            egui_mineral::kit::sanitize::report(format_args!(
                "wavy circular progress needs size >= {}, got {extent:?}; painted flat",
                circular::WAVE_SIZE
            ));
        }
        extent
    }

    fn paint(&self, ui: &mut MineralUi<'_, Material>, rect: Rect) {
        let rt = ui.rt();
        let size = rect.size().min_elem();
        let waving = self.spec.wavy && size >= circular::WAVE_SIZE;
        let t = rt.tokens();
        let id = ui.make_persistent_id(self.salt.id_salt());
        if ui.is_rect_visible(rect) {
            let k = ring_stroke(size) / circular::ACTIVE_THICKNESS;
            let ring = Ring {
                center: rect.center(),
                size,
                thickness: circular::ACTIVE_THICKNESS * k,
                radius: ((size - circular::TRACK_THICKNESS * k) / 2.0
                    - if waving { circular::ACTIVE_WAVE_AMPLITUDE } else { 0.0 })
                .max(0.0),
                active: t.role(progress::ACTIVE_INDICATOR_COLOR),
                track: t.role(progress::TRACK_COLOR),
            };
            let wave = waving.then(|| {
                let amplitude = wave_amplitude(ui, id, self.spec.value);
                Wave {
                    amplitude,
                    length: circular::ACTIVE_WAVE_WAVELENGTH,
                    offset: wave_offset(ui, amplitude),
                }
            });
            let p = ui.painter();
            match self.spec.value {
                Some(v) => ring.determinate(p, v, wave),
                None => ring.indeterminate(p, loop_time(ui, pi::CIRCULAR_CYCLE), wave),
            }
        }
    }
}

pub(crate) fn paint_indeterminate_ring(
    ui: &egui_mineral::widget::MineralUi<'_, crate::Material>,
    p: &Painter,
    center: Pos2,
    size: f32,
    thickness: f32,
    color: Color32,
) {
    let ring = Ring {
        center,
        size,
        thickness,
        radius: (size - thickness) / 2.0,
        active: color,
        track: Color32::TRANSPARENT,
    };
    ring.indeterminate(p, loop_time(ui, pi::CIRCULAR_CYCLE), None);
}

// material-components-android circular sizes: extra small, small, medium (diameter, stroke)
const RING_STROKES: [(f32, f32); 3] = [(20.0, 2.5), (28.0, 3.0), (circular::SIZE, circular::ACTIVE_THICKNESS)];

// Stroke of a `size` ring; the track gap scales with it.
fn ring_stroke(size: f32) -> f32 {
    let [first, .., last] = RING_STROKES;
    if size <= first.0 {
        return first.1;
    }
    RING_STROKES
        .windows(2)
        .find(|w| size <= w[1].0)
        .map_or(last.1, |w| egui::remap(size, w[0].0..=w[1].0, w[0].1..=w[1].1))
}

struct Ring {
    center: Pos2,
    size: f32,
    thickness: f32,
    radius: f32,
    active: Color32,
    track: Color32,
}

impl Ring {
    fn gap(&self) -> f32 {
        let k = self.thickness / circular::ACTIVE_THICKNESS;
        (circular::TRACK_ACTIVE_SPACE + circular::TRACK_THICKNESS) * k / (std::f32::consts::PI * self.size) * 360.0
    }

    // Compose drawArc
    fn arc(&self, p: &Painter, start: f32, sweep: f32, wave: Option<Wave>, color: Color32) {
        if sweep <= 0.0 {
            return;
        }
        let (a0, a1) = (start.to_radians(), (start + sweep).to_radians());
        #[expect(clippy::cast_sign_loss, reason = "abs() * radius >= 0")]
        let steps = (((a1 - a0).abs() * self.radius / 1.5).ceil() as usize).max(2);
        let waves = wave.filter(|w| w.amplitude > 0.0).map(|w| {
            let count = (TAU * self.radius / w.length).round().max(1.0);
            (w, count)
        });
        let pts = (0..=steps)
            .map(|i| {
                let a = egui::lerp(a0..=a1, i as f32 / steps as f32);
                let r = self.radius
                    + waves.map_or(0.0, |(w, n)| {
                        w.amplitude * circular::ACTIVE_WAVE_AMPLITUDE * (a * n - w.offset * TAU).sin()
                    });
                self.center + vec2(a.cos(), a.sin()) * r
            })
            .collect();
        stroke_path(p, pts, self.thickness, color);
    }

    fn determinate(&self, p: &Painter, v: f32, wave: Option<Wave>) {
        let start = 270.0;
        let sweep = v * 360.0;
        let gap = sweep.min(self.gap());
        self.arc(p, start + sweep + gap, 360.0 - sweep - 2.0 * gap, None, self.track);
        self.arc(p, start, sweep, wave, self.active);
    }

    // Compose CircularProgressIndicator
    fn indeterminate(&self, p: &Painter, time: f32, wave: Option<Wave>) {
        let cycle = pi::CIRCULAR_CYCLE.as_secs_f32();
        let global = pi::GLOBAL_ROTATION_DEGREES * time / cycle;
        let step = pi::ROTATION_STEP_DELAY.as_secs_f32();
        let quarter = (time / step).floor();
        let turn = ((time - quarter * step) / pi::ROTATION_STEP_DURATION.as_secs_f32()).clamp(0.0, 1.0);
        let extra = 90.0 * (quarter + turn);
        let half = cycle / 2.0;
        let (min, max) = (pi::INDETERMINATE_MIN_PROGRESS, pi::INDETERMINATE_MAX_PROGRESS);
        let progress = if time < half {
            egui::lerp(min..=max, time / half)
        } else {
            egui::lerp(max..=min, m::EASING_STANDARD_CUBIC_BEZIER.ease((time - half) / half))
        };
        self.arc(p, global + extra, progress * 360.0, wave, self.active);
    }
}
