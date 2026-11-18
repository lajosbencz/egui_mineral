use egui::Vec2;

/// Minimum and maximum size a widget may take.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Constraints {
    /// Lower bound per axis in points; finite.
    pub min: Vec2,
    /// Upper bound per axis in points; may be infinite.
    pub max: Vec2,
}

impl Constraints {
    /// No bounds: `0..=INFINITY` on both axes.
    pub const UNBOUNDED: Self = Self {
        min: Vec2::ZERO,
        max: Vec2::INFINITY,
    };

    /// `min == max == size`.
    pub fn tight(size: Vec2) -> Self {
        Self { min: size, max: size }
    }

    /// `0..=max` on each axis.
    pub fn loose(max: Vec2) -> Self {
        Self { min: Vec2::ZERO, max }
    }

    /// Clamps `size` per axis into `min..=max`; a larger `min` wins (CSS, taffy), NaN bounds are ignored.
    pub fn constrain(&self, size: Vec2) -> Vec2 {
        let axis = |v: f32, min: f32, max: f32| {
            let lo = if min.is_nan() { 0.0 } else { min };
            let hi = if max.is_nan() { f32::INFINITY } else { max };
            (if v.is_nan() { lo } else { v }).min(hi).max(lo)
        };
        Vec2::new(
            axis(size.x, self.min.x, self.max.x),
            axis(size.y, self.min.y, self.max.y),
        )
    }

    /// `min == max` on both axes.
    pub fn is_tight(&self) -> bool {
        self.min == self.max
    }

    /// `other` coerced into `self` per axis, so `self` wins where they conflict (Compose `Constraints.constrain`).
    #[must_use]
    pub fn intersect(&self, other: Self) -> Self {
        Self {
            min: self.constrain(other.min),
            max: self.constrain(other.max),
        }
    }
}

#[cfg(test)]
mod tests {
    use egui::vec2;

    use super::Constraints;

    #[test]
    fn invalid_bounds_never_panic() {
        let crossed = Constraints {
            min: vec2(50.0, f32::NAN),
            max: vec2(10.0, f32::NAN),
        };
        assert_eq!(crossed.constrain(vec2(30.0, 7.0)), vec2(50.0, 7.0));
        let c = crossed.intersect(Constraints::tight(vec2(f32::NAN, 1.0)));
        assert!(!c.min.x.is_nan() && !c.max.x.is_nan());
    }

    #[test]
    fn intersect_coerces_into_the_parent() {
        let parent = Constraints::loose(vec2(300.0, f32::INFINITY));
        let fixed = Constraints {
            min: vec2(120.0, 0.0),
            max: vec2(120.0, f32::INFINITY),
        };
        assert_eq!(
            parent.intersect(fixed),
            Constraints {
                min: vec2(120.0, 0.0),
                max: vec2(120.0, f32::INFINITY)
            }
        );
        let wide = Constraints::tight(vec2(500.0, 40.0));
        assert_eq!(
            parent.intersect(wide),
            Constraints::tight(vec2(300.0, 40.0)),
            "the parent wins where they conflict"
        );
        assert_eq!(parent.intersect(Constraints::UNBOUNDED), parent);
        let tight = Constraints::tight(vec2(80.0, 80.0));
        assert_eq!(tight.intersect(Constraints::loose(vec2(40.0, 40.0))), tight);
    }
}
