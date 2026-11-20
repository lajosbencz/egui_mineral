use std::{
    sync::atomic::{AtomicBool, Ordering},
    time::Duration,
};

use egui::{Context, Id};

use crate::runtime::Core;

use crate::kit::sanitize::{at_least, finite, within};
use crate::kit::state::TempState;

const MIN_SPRING_PARAM: f32 = 1e-3;
pub(crate) const MAX_DECELERATION: f32 = 0.999;

/// CSS `cubic-bezier(x1, y1, x2, y2)` timing function.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CubicBezier {
    /// First control point x, 0.0..=1.0.
    pub x1: f32,
    /// First control point y; may leave 0.0..=1.0 for overshoot.
    pub y1: f32,
    /// Second control point x, 0.0..=1.0.
    pub x2: f32,
    /// Second control point y; may leave 0.0..=1.0 for overshoot.
    pub y2: f32,
}

impl CubicBezier {
    /// Control points as in CSS; x values must stay in 0.0..=1.0 for a monotonic curve.
    pub const fn new(x1: f32, y1: f32, x2: f32, y2: f32) -> Self {
        Self { x1, y1, x2, y2 }
    }

    fn sample(a1: f32, a2: f32, t: f32) -> f32 {
        let u = 1.0 - t;
        3.0 * u * u * t * a1 + 3.0 * u * t * t * a2 + t * t * t
    }

    fn slope(a1: f32, a2: f32, t: f32) -> f32 {
        let u = 1.0 - t;
        3.0 * u * u * a1 + 6.0 * u * t * (a2 - a1) + 3.0 * t * t * (1.0 - a2)
    }

    /// Eased progress for linear progress `x` (clamped to 0.0..=1.0); Newton iteration on x.
    pub fn ease(&self, x: f32) -> f32 {
        let x = x.clamp(0.0, 1.0);
        let mut t = x;
        for _ in 0..8 {
            let err = Self::sample(self.x1, self.x2, t) - x;
            if err.abs() < 1e-5 {
                return Self::sample(self.y1, self.y2, t);
            }
            let d = Self::slope(self.x1, self.x2, t);
            if d.abs() < 1e-6 {
                break;
            }
            t = (t - err / d).clamp(0.0, 1.0);
        }
        let (mut lo, mut hi) = (0.0, 1.0);
        for _ in 0..30 {
            t = f32::midpoint(lo, hi);
            if Self::sample(self.x1, self.x2, t) < x {
                lo = t;
            } else {
                hi = t;
            }
        }
        Self::sample(self.y1, self.y2, t)
    }
}

/// Unit-mass spring (Compose `spring` parameters). Fields are checked on use, so literals stay `const`:
/// `damping_ratio` below 0 or non-finite counts as 0, `stiffness` and `rest_threshold` as at least 0.001.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Spring {
    /// Damping relative to critical; below 1 overshoots.
    pub damping_ratio: f32,
    /// Spring constant of the unit mass.
    pub stiffness: f32,
    /// Distance to the target, in value units, below which [`spring`] settles.
    pub rest_threshold: f32,
}

impl Spring {
    fn valid(self) -> Self {
        Self {
            damping_ratio: at_least(self.damping_ratio, 0.0.., "Spring damping_ratio"),
            stiffness: at_least(self.stiffness, MIN_SPRING_PARAM.., "Spring stiffness"),
            rest_threshold: at_least(self.rest_threshold, MIN_SPRING_PARAM.., "Spring rest_threshold"),
        }
    }

    /// Step response from 0 to 1 at rest, `t` seconds in.
    pub fn at(&self, t: f32) -> f32 {
        let start = SpringPoint {
            offset: 1.0,
            velocity: 0.0,
        };
        1.0 - self.advance(start, t).offset
    }

    /// Point `t` seconds after `from`.
    pub fn advance(&self, from: SpringPoint, t: f32) -> SpringPoint {
        let SpringPoint {
            offset: x0,
            velocity: v0,
        } = from;
        let t = t.max(0.0);
        let Self {
            damping_ratio: zeta,
            stiffness,
            ..
        } = self.valid();
        let omega = stiffness.sqrt();
        if zeta < 1.0 {
            let wd = omega * (1.0 - zeta * zeta).sqrt();
            let decay = (-zeta * omega * t).exp();
            let (sin, cos) = (wd * t).sin_cos();
            let b = (v0 + zeta * omega * x0) / wd;
            let x = decay * (x0 * cos + b * sin);
            let v = decay * ((b * wd - zeta * omega * x0) * cos - (x0 * wd + zeta * omega * b) * sin);
            SpringPoint::new(x, v)
        } else if (zeta - 1.0).abs() < 1e-4 {
            let decay = (-omega * t).exp();
            let b = v0 + omega * x0;
            SpringPoint::new((x0 + b * t) * decay, (b - omega * (x0 + b * t)) * decay)
        } else {
            let root = (zeta * zeta - 1.0).sqrt();
            let (r1, r2) = (-omega * (zeta - root), -omega * (zeta + root));
            let c2 = (v0 - r1 * x0) / (r2 - r1);
            let c1 = x0 - c2;
            let (e1, e2) = ((r1 * t).exp(), (r2 * t).exp());
            SpringPoint::new(c1 * e1 + c2 * e2, c1 * r1 * e1 + c2 * r2 * e2)
        }
    }
}

impl TempState for SpringState {}

#[derive(Clone, Copy, Debug, PartialEq)]
struct SpringState {
    target: f32,
    from: SpringPoint,
    start: f64,
}

impl SpringState {
    fn rest(value: f32, now: f64) -> Self {
        Self {
            target: value,
            from: SpringPoint::new(0.0, 0.0),
            start: now,
        }
    }

    fn at(self, spring: Spring, now: f64) -> (f32, f32) {
        let p = spring.advance(self.from, (now - self.start) as f32);
        (self.target + p.offset, p.velocity)
    }

    fn retarget(self, spring: Spring, target: f32, now: f64) -> Self {
        let (value, v0) = self.at(spring, now);
        Self {
            target,
            from: SpringPoint::new(value - target, v0),
            start: now,
        }
    }
}

fn spring_key(id: Id) -> Id {
    id.with("mineral::spring")
}

/// Value springing toward `target`; starts at rest on first sight and retargets from the current
/// value and velocity. Snaps under reduced motion. A non-finite `target` keeps the previous one (0 on
/// first sight).
#[expect(clippy::float_cmp, reason = "targets are recomputed identically each pass")]
pub fn spring(core: &Core, id: Id, target: f32, spring: Spring) -> f32 {
    let ctx = core.ctx();
    let key = spring_key(id);
    let prev = SpringState::try_load(ctx, key);
    let target = finite(target, "spring target").unwrap_or(prev.map_or(0.0, |s| s.target));
    let now = finite(ctx.input(|i| i.time), "spring time").unwrap_or(prev.map_or(0.0, |s| s.start));
    let spring = spring.valid();
    let state = match prev {
        Some(s) if s.target == target => s,
        Some(s) if !core.motion().reduced() => s.retarget(spring, target, now),
        _ => SpringState::rest(target, now),
    };
    if prev != Some(state) {
        state.store(ctx, key);
    }
    let (value, _) = state.at(spring, now);
    // A frame later too, so an oscillating spring crossing its target does not stop early.
    let resting = |v: f32| (v - target).abs() < spring.rest_threshold;
    if resting(value) && resting(state.at(spring, now + 1.0 / 60.0).0) {
        target
    } else {
        request_frame(ctx);
        value
    }
}

/// Puts the [`spring`] at `id` at rest on `value`; a non-finite `value` counts as 0.
pub fn snap(ctx: &Context, id: Id, value: f32) {
    let state = SpringState::rest(
        finite(value, "snap value").unwrap_or(0.0),
        finite(ctx.input(|i| i.time), "snap time").unwrap_or(0.0),
    );
    state.store(ctx, spring_key(id));
}

/// Reduced-motion preference of a [`Core`], honoured by every library animation. egui's own
/// (`animate_bool`, area fade-in, `scroll_to_me`) read the app-wide `Style::animation_time` instead.
#[derive(Debug, Default)]
pub struct Motion {
    reduced: AtomicBool,
}

impl Motion {
    /// Off by default.
    pub fn reduced(&self) -> bool {
        self.reduced.load(Ordering::Relaxed)
    }

    /// Switches library animations to their end state.
    pub fn set_reduced(&self, reduced: bool) {
        self.reduced.store(reduced, Ordering::Relaxed);
    }

    /// `d`, or zero while reduced.
    pub fn duration(&self, d: Duration) -> Duration {
        if self.reduced() { Duration::ZERO } else { d }
    }
}

/// Call only while the animation is visible and unfinished.
pub fn request_frame(ctx: &Context) {
    #[expect(
        clippy::disallowed_methods,
        reason = "the single sanctioned per-frame repaint, at display refresh rate"
    )]
    ctx.request_repaint();
}

/// Target of a [`presence`] animation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Visibility {
    /// Enters toward progress 1.
    Visible,
    /// Exits toward progress 0, then unmounts.
    Hidden,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct PresenceState {
    visible: bool,
    from: f32,
    start: f64,
}

impl TempState for PresenceState {}

/// Enter/exit transition: `Some(progress)` while the content should stay mounted (including
/// during its exit), `None` once fully hidden. Enters from hidden on first sight. Linear 0..=1;
/// apply an easing on top.
pub fn presence(core: &Core, id: Id, visibility: Visibility, d: Duration) -> Option<f32> {
    let ctx = core.ctx();
    let key = id.with("mineral::presence");
    let visible = visibility == Visibility::Visible;
    let now = ctx.input(|i| i.time);
    let secs = core.motion().duration(d).as_secs_f32();
    let progress = |s: PresenceState| {
        let step = if secs > 0.0 { (now - s.start) as f32 / secs } else { 1.0 };
        if s.visible { s.from + step } else { s.from - step }.clamp(0.0, 1.0)
    };
    let prev = PresenceState::try_load(ctx, key);
    let state = match prev {
        Some(s) if s.visible == visible => s,
        Some(s) => PresenceState {
            visible,
            from: progress(s),
            start: now,
        },
        None if visible => PresenceState {
            visible,
            from: 0.0,
            start: now,
        },
        None => return None,
    };
    let k = progress(state);
    if !visible && k <= 0.0 {
        PresenceState::remove(ctx, key);
        return None;
    }
    if prev != Some(state) {
        state.store(ctx, key);
    }
    if !visible || k < 1.0 {
        request_frame(ctx);
    }
    Some(k)
}

/// Spring motion relative to its target.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct SpringPoint {
    /// Signed distance from the target.
    pub offset: f32,
    /// Units/s.
    pub velocity: f32,
}

impl SpringPoint {
    const fn new(offset: f32, velocity: f32) -> Self {
        Self { offset, velocity }
    }
}

/// State of a drag at release.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Release {
    /// Position along the drag axis; the signed offset from rest for [`Fling::should_dismiss`].
    pub pos: f32,
    /// Velocity in points/s, signed along the drag axis.
    pub velocity: f32,
}

/// Momentum and dismissal thresholds for fling gestures.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Fling {
    /// Velocity retained per millisecond.
    pub deceleration: f32,
    /// Fraction of the extent past which a release dismisses.
    pub dismiss_fraction: crate::layout::Fraction,
    /// Outward release speed (points/s) that dismisses at any offset.
    pub dismiss_velocity: f32,
}

impl Fling {
    /// Resting position of a flung drag released as `release`, before snapping;
    /// `deceleration` counts as within `0..=0.999`.
    pub fn project(&self, release: Release) -> f32 {
        let d = within(self.deceleration, 0.0..=MAX_DECELERATION, "Fling deceleration");
        release.pos + release.velocity / 1000.0 * d / (1.0 - d)
    }

    /// Nearest of `points` to the projected resting position; `None` when `points` is empty.
    pub fn snap_target(&self, points: &[f32], release: Release) -> Option<f32> {
        let p = self.project(release);
        points
            .iter()
            .copied()
            .min_by(|a, b| (a - p).abs().total_cmp(&(b - p).abs()))
    }

    /// Swipe-to-dismiss release over `extent`. `release.pos` (the offset) and `release.velocity` are signed along the
    /// dismiss direction.
    pub fn should_dismiss(&self, release: Release, extent: f32) -> bool {
        release.pos > extent * self.dismiss_fraction.get() || release.velocity > self.dismiss_velocity
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spring_of(damping_ratio: f32) -> Spring {
        Spring {
            damping_ratio,
            stiffness: 200.0,
            rest_threshold: 0.01,
        }
    }

    fn release(pos: f32, velocity: f32) -> Release {
        Release { pos, velocity }
    }

    #[test]
    fn spring_settles_from_zero() {
        for s in [spring_of(0.6), spring_of(1.0), spring_of(2.0)] {
            assert!(s.at(0.0).abs() < 1e-5);
            assert!((s.at(5.0) - 1.0).abs() < 1e-3);
        }
        assert!(spring_of(0.6).at(0.25) > 1.0);
    }

    #[test]
    fn non_finite_inputs_keep_the_spring_finite_and_resting() {
        let ctx = Context::default();
        let rt = crate::test_support::runtime(&ctx);
        let id = Id::unique("s");
        let s = Spring {
            damping_ratio: f32::NAN,
            stiffness: -1.0,
            rest_threshold: 0.0,
        };
        let mut out = Vec::new();
        for (time, target) in [
            (0.0, 5.0),
            (1.0, f32::NAN),
            (2.0, f32::INFINITY),
            (f64::NAN, 5.0),
            (30.0, 5.0),
        ] {
            crate::test_support::run(&ctx, crate::test_support::at(time, vec![]), |_| {
                out.push(spring(rt.core(), id, target, s));
            });
        }
        assert!(out.iter().all(|v| v.is_finite()), "{out:?}");
        assert_eq!(out.last(), Some(&5.0), "settles on the last finite target");
    }

    #[test]
    fn spring_retarget_keeps_velocity() {
        for s in [spring_of(0.6), spring_of(1.0), spring_of(2.0)] {
            let moving = SpringState::rest(0.0, 0.0).retarget(s, 100.0, 0.0);
            let before = moving.at(s, 0.05);
            assert!(before.1 > 0.0, "moving toward the target");
            let after = moving.retarget(s, 200.0, 0.05).at(s, 0.05);
            assert!((before.0 - after.0).abs() < 1e-3 && (before.1 - after.1).abs() < 1e-2);
        }
    }

    #[test]
    fn displacement_velocity_matches_slope() {
        for s in [spring_of(0.6), spring_of(1.0), spring_of(2.0)] {
            let (t, h) = (0.07, 1e-3);
            let from = SpringPoint {
                offset: 3.0,
                velocity: -40.0,
            };
            let slope = (s.advance(from, t + h).offset - s.advance(from, t - h).offset) / (2.0 * h);
            assert!((s.advance(from, t).velocity - slope).abs() < 0.05, "{s:?}");
        }
    }

    #[test]
    fn presence_animates_on_first_show() {
        let ctx = Context::default();
        let rt = crate::test_support::runtime(&ctx);
        let id = Id::unique("p");
        let pass = |time: f64, visibility: Visibility| {
            let mut out = None;
            crate::test_support::run(&ctx, crate::test_support::at(time, vec![]), |_| {
                out = presence(rt.core(), id, visibility, Duration::from_millis(200));
            });
            out
        };
        assert_eq!(pass(0.0, Visibility::Hidden), None);
        assert_eq!(pass(1.0, Visibility::Visible), Some(0.0));
        let k = pass(1.1, Visibility::Visible).unwrap();
        assert!((k - 0.5).abs() < 1e-3);
        assert_eq!(pass(2.0, Visibility::Visible), Some(1.0));
        assert_eq!(pass(2.1, Visibility::Hidden), Some(1.0));
        assert!(pass(2.2, Visibility::Hidden).is_some_and(|k| (k - 0.5).abs() < 1e-3));
        assert_eq!(pass(3.0, Visibility::Hidden), None);
    }

    #[test]
    fn bezier_endpoints_and_linear() {
        let e = CubicBezier::new(0.2, 0.0, 0.0, 1.0);
        assert!(e.ease(0.0).abs() < 1e-4);
        assert!((e.ease(1.0) - 1.0).abs() < 1e-4);
        assert!(e.ease(0.5) > 0.5, "emphasized is front-loaded");
        let lin = CubicBezier::new(0.0, 0.0, 1.0, 1.0);
        assert!((lin.ease(0.3) - 0.3).abs() < 1e-3);
    }

    #[test]
    fn fling_snaps_forward() {
        let detents = [0.0, 300.0, 600.0];
        assert_eq!(
            crate::test_support::FLING.snap_target(&detents, release(200.0, 0.0)),
            Some(300.0)
        );
        assert_eq!(
            crate::test_support::FLING.snap_target(&detents, release(200.0, -600.0)),
            Some(0.0)
        );
    }

    #[test]
    fn fling_dismisses_by_distance_or_velocity() {
        let f = crate::test_support::FLING;
        let extent = 200.0;
        let past = extent * f.dismiss_fraction.get() + 1.0;
        let short = extent * f.dismiss_fraction.get() - 1.0;
        let slow = f.dismiss_velocity - 1.0;
        assert!(f.should_dismiss(release(past, 0.0), extent));
        assert!(f.should_dismiss(release(0.0, f.dismiss_velocity + 1.0), extent));
        assert!(!f.should_dismiss(release(short, slow), extent));
    }

    #[test]
    fn reduced_motion_zeroes_durations() {
        let m = Motion::default();
        let d = Duration::from_millis(200);
        assert_eq!(m.duration(d), d);
        m.set_reduced(true);
        assert!(m.reduced());
        assert_eq!(m.duration(d), Duration::ZERO);
        m.set_reduced(false);
        assert_eq!(m.duration(d), d);
    }
}
