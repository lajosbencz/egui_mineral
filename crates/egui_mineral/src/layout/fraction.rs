/// Share of a whole in `0.0..=1.0`: progress done, a split ratio, a dismiss threshold.
///
/// Saturating: every constructor clamps into the range and maps NaN to 0, so a `Fraction` is always valid
/// (docs/rules.md rule 10). Sizes stay plain `f32` points.
#[derive(Clone, Copy, Debug, Default, PartialEq, PartialOrd)]
pub struct Fraction(f32);

impl Fraction {
    /// `0.0`.
    pub const ZERO: Self = Self(0.0);
    /// `0.5`.
    pub const HALF: Self = Self(0.5);
    /// `1.0`.
    pub const ONE: Self = Self(1.0);

    /// `value` clamped to `0.0..=1.0`; NaN counts as 0.
    pub const fn new(value: f32) -> Self {
        if value.is_nan() {
            Self::ZERO
        } else {
            Self(value.clamp(0.0, 1.0))
        }
    }

    /// `part` of `whole`; a zero, negative or non-finite `whole` counts as nothing done.
    pub fn of(part: f32, whole: f32) -> Self {
        if whole.is_finite() && whole > 0.0 {
            Self::new(part / whole)
        } else {
            Self::ZERO
        }
    }

    /// Value in `0.0..=1.0`.
    pub const fn get(self) -> f32 {
        self.0
    }

    /// `1.0 - self`.
    #[must_use]
    pub const fn complement(self) -> Self {
        Self(1.0 - self.0)
    }
}

impl From<f32> for Fraction {
    fn from(value: f32) -> Self {
        Self::new(value)
    }
}

impl From<Fraction> for f32 {
    fn from(fraction: Fraction) -> Self {
        fraction.0
    }
}

impl From<Fraction> for f64 {
    fn from(fraction: Fraction) -> Self {
        Self::from(fraction.0)
    }
}

#[cfg(test)]
mod tests {
    use super::Fraction;

    #[test]
    fn constructors_saturate() {
        assert_eq!(Fraction::new(0.25).get(), 0.25);
        assert_eq!(Fraction::new(-1.0), Fraction::ZERO);
        assert_eq!(Fraction::new(f32::INFINITY), Fraction::ONE);
        assert_eq!(Fraction::new(f32::NEG_INFINITY), Fraction::ZERO);
        assert_eq!(Fraction::from(f32::NAN), Fraction::ZERO);
        assert_eq!(Fraction::of(1.0, 4.0).get(), 0.25);
        assert_eq!(Fraction::of(1.0, 0.0), Fraction::ZERO);
        assert_eq!(Fraction::of(1.0, f32::NAN), Fraction::ZERO);
        assert_eq!(Fraction::new(0.25).complement().get(), 0.75);
    }
}
