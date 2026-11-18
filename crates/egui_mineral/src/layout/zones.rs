use egui::{Pos2, Rect, Vec2, epaint::MarginF32, vec2};

use crate::input::PointerKind;

/// Minimum hit target of a widget.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub enum MinTarget {
    /// Policy target for the current pointer modality ([`crate::policy::Policies::target`]); no growth while unset.
    #[default]
    Auto,
    /// Fixed edge in points.
    Points(f32),
    /// No growth; the hit zone equals the visual box.
    None,
}

impl MinTarget {
    /// Target edge in points for this pass.
    pub fn points(self, core: &crate::runtime::Core) -> f32 {
        match self {
            Self::Auto => core.policies().target.for_pointer(core.input().modality),
            Self::Points(v) => v,
            Self::None => 0.0,
        }
    }
}

/// Minimum target edge per input modality, in points.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TargetPolicy {
    /// Mouse and synthetic pointers.
    pub mouse: f32,
    /// Touch pointers.
    pub touch: f32,
}

impl TargetPolicy {
    /// Edge for `kind`; synthetic input uses the mouse value.
    pub fn for_pointer(&self, kind: PointerKind) -> f32 {
        match kind {
            PointerKind::Touch => self.touch,
            PointerKind::Mouse | PointerKind::Synthetic => self.mouse,
        }
    }
}

/// Size a widget asks for: its visual box plus layout space reserved around it.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct Measure {
    /// Painted box in points; also the hit box before target growth.
    pub visual: Vec2,
    /// Physical sides; the widget may paint there, it is never hit.
    pub margin: MarginF32,
}

impl Measure {
    /// `visual` with zero margin.
    pub fn new(visual: Vec2) -> Self {
        Self {
            visual,
            margin: MarginF32::ZERO,
        }
    }

    /// Margin around the visual rect; a negative or non-finite side counts as 0.
    #[must_use]
    pub fn margin(mut self, margin: MarginF32) -> Self {
        self.margin = MarginF32 {
            left: crate::kit::sanitize::at_least(margin.left, 0.0.., "Measure margin"),
            right: crate::kit::sanitize::at_least(margin.right, 0.0.., "Measure margin"),
            top: crate::kit::sanitize::at_least(margin.top, 0.0.., "Measure margin"),
            bottom: crate::kit::sanitize::at_least(margin.bottom, 0.0.., "Measure margin"),
        };
        self
    }

    /// `visual` plus margin on all sides.
    pub fn outer(&self) -> Vec2 {
        self.visual + self.margin.sum()
    }
}

impl From<Vec2> for Measure {
    fn from(visual: Vec2) -> Self {
        Self::new(visual)
    }
}

/// Rects of a placed widget.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct Zones {
    /// Allocated: the visual with its margin, unioned with `hit`.
    pub outer: Rect,
    /// Painted box inside `outer`, offset by the margin.
    pub visual: Rect,
    /// Visual grown to the minimum target edge, centered on it.
    pub hit: Rect,
}

impl Zones {
    /// Zones of `measure` with a `min` target edge, `outer` starting at the origin. A negative or
    /// non-finite `min` or measure size counts as 0.
    pub fn new(measure: Measure, min: f32) -> Self {
        let min = crate::kit::sanitize::at_least(min, 0.0.., "min target");
        let measure = Measure {
            visual: vec2(
                crate::kit::sanitize::at_least(measure.visual.x, 0.0.., "Measure visual width"),
                crate::kit::sanitize::at_least(measure.visual.y, 0.0.., "Measure visual height"),
            ),
            margin: measure.margin,
        };
        let outer = measure.outer();
        let visual = Rect::from_min_size(measure.margin.left_top().to_pos2(), measure.visual);
        let hit = Rect::from_center_size(visual.center(), measure.visual.max(Vec2::splat(min)));
        let union = Rect::from_min_size(Pos2::ZERO, outer).union(hit);
        Self {
            outer: union,
            visual,
            hit,
        }
        .translate(-union.min.to_vec2())
    }

    /// Moves all three rects.
    #[must_use]
    pub fn translate(self, by: Vec2) -> Self {
        Self {
            outer: self.outer.translate(by),
            visual: self.visual.translate(by),
            hit: self.hit.translate(by),
        }
    }
}

/// No margin, no target growth: all three zones equal.
impl From<Rect> for Zones {
    fn from(rect: Rect) -> Self {
        Self {
            outer: rect,
            visual: rect,
            hit: rect,
        }
    }
}

/// Insets of display cutouts and system bars.
pub fn safe_area(ctx: &egui::Context) -> MarginF32 {
    ctx.input(|i| i.safe_area_insets().0)
}

/// Width class: the number of thresholds at or below `width`.
pub fn breakpoint(core: &crate::runtime::Core, width: f32) -> usize {
    core.policies().breakpoints.partition_point(|t| *t <= width)
}

#[cfg(test)]
mod tests {
    use egui::{pos2, vec2};

    use super::*;

    #[test]
    fn hit_zone_inflates_and_stays_centered() {
        let z = Zones::new(vec2(20.0, 60.0).into(), 48.0);
        assert_eq!(z.hit.size(), vec2(48.0, 60.0));
        assert_eq!(z.hit.center(), z.visual.center());
        assert_eq!(z.outer, z.hit);
        assert!(z.hit.contains_rect(z.visual));
    }

    #[test]
    fn margin_is_reserved_outside_the_hit_zone() {
        let m = Measure::new(vec2(200.0, 56.0)).margin(MarginF32 {
            top: 8.0,
            bottom: 20.0,
            ..MarginF32::ZERO
        });
        let z = Zones::new(m, 48.0).translate(vec2(10.0, 10.0));
        assert_eq!(z.outer, Rect::from_min_size(pos2(10.0, 10.0), vec2(200.0, 84.0)));
        assert_eq!(z.visual, Rect::from_min_size(pos2(10.0, 18.0), vec2(200.0, 56.0)));
        assert_eq!(z.hit, z.visual);
    }

    #[test]
    fn hit_zone_past_an_asymmetric_margin_grows_the_outer_box() {
        let m = Measure::new(vec2(24.0, 24.0)).margin(MarginF32 {
            top: 30.0,
            ..MarginF32::ZERO
        });
        let z = Zones::new(m, 48.0);
        assert_eq!(z.outer.min, Pos2::ZERO);
        assert_eq!(z.outer.size(), vec2(48.0, 66.0));
        assert_eq!(z.hit.center(), z.visual.center());
        assert!(z.outer.contains_rect(z.hit));
    }

    #[test]
    fn invalid_sizes_count_as_zero() {
        let m = Measure::new(vec2(-5.0, f32::NAN)).margin(MarginF32 {
            left: -1.0,
            right: f32::INFINITY,
            ..MarginF32::ZERO
        });
        let z = Zones::new(m, f32::NAN);
        assert_eq!(
            (z.visual.size(), z.hit.size(), z.outer.size()),
            (Vec2::ZERO, Vec2::ZERO, Vec2::ZERO)
        );
    }

    #[test]
    fn breakpoint_classes() {
        let ctx = egui::Context::default();
        let with = |breakpoints: &[f32]| crate::policy::Policies {
            breakpoints: breakpoints.into(),
            ..crate::test_support::policies()
        };
        let rt = crate::test_support::runtime_with(&ctx, with(&[]));
        assert_eq!(breakpoint(rt.core(), 2000.0), 0);
        let rt = crate::test_support::runtime_with(&ctx, with(&[600.0, 840.0]));
        assert_eq!(breakpoint(rt.core(), 599.0), 0);
        assert_eq!(breakpoint(rt.core(), 600.0), 1);
        assert_eq!(breakpoint(rt.core(), 900.0), 2);
    }
}
