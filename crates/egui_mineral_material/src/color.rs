use std::{fmt, num::ParseFloatError, str::FromStr};

use egui::{Color32, Theme};
use material_colors::{color::Rgb, dynamic_color::Variant, theme::ThemeBuilder};
use serde::{Deserialize, Serialize};

/// MD3 baseline seed color (#6750A4).
pub const BASELINE_SEED: Color32 = Color32::from_rgb(0x67, 0x50, 0xA4);

/// Dynamic color algorithm (material-color-utilities `Variant`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum SchemeVariant {
    /// Grayscale.
    Monochrome,
    /// Near-grayscale, slight seed hue.
    Neutral,
    #[default]
    /// MD3 default.
    TonalSpot,
    /// Saturated, seed hue shifted.
    Vibrant,
    /// Hue-rotated, playful.
    Expressive,
    /// Keeps the seed color as `primaryContainer`.
    Fidelity,
    /// Like `Fidelity`, for content-derived seeds.
    Content,
    /// Hues spread across the wheel.
    Rainbow,
    /// Multiple distinct hues.
    FruitSalad,
}

impl SchemeVariant {
    /// Every variant.
    pub const ALL: &'static [Self] = &[
        Self::Monochrome,
        Self::Neutral,
        Self::TonalSpot,
        Self::Vibrant,
        Self::Expressive,
        Self::Fidelity,
        Self::Content,
        Self::Rainbow,
        Self::FruitSalad,
    ];

    /// Snake-case name, as `Display` and `FromStr` use it.
    pub const fn name(self) -> &'static str {
        match self {
            Self::Monochrome => "monochrome",
            Self::Neutral => "neutral",
            Self::TonalSpot => "tonal_spot",
            Self::Vibrant => "vibrant",
            Self::Expressive => "expressive",
            Self::Fidelity => "fidelity",
            Self::Content => "content",
            Self::Rainbow => "rainbow",
            Self::FruitSalad => "fruit_salad",
        }
    }

    fn material(self) -> Variant {
        match self {
            Self::Monochrome => Variant::Monochrome,
            Self::Neutral => Variant::Neutral,
            Self::TonalSpot => Variant::TonalSpot,
            Self::Vibrant => Variant::Vibrant,
            Self::Expressive => Variant::Expressive,
            Self::Fidelity => Variant::Fidelity,
            Self::Content => Variant::Content,
            Self::Rainbow => Variant::Rainbow,
            Self::FruitSalad => Variant::FruitSalad,
        }
    }
}

impl fmt::Display for SchemeVariant {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// Unknown [`SchemeVariant`] name.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
#[error("unknown scheme variant `{0}`")]
pub struct ParseSchemeVariantError(String);

impl FromStr for SchemeVariant {
    type Err = ParseSchemeVariantError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::ALL
            .iter()
            .copied()
            .find(|v| v.name() == s)
            .ok_or_else(|| ParseSchemeVariantError(s.to_owned()))
    }
}

/// material-color-utilities contrast level in -1..=1.
#[derive(Clone, Copy, Debug, Default, PartialEq, PartialOrd)]
pub struct Contrast(f64);

impl Contrast {
    /// -1: below standard.
    pub const REDUCED: Self = Self(-1.0);
    /// 0: MD3 default.
    pub const STANDARD: Self = Self(0.0);
    /// 0.5.
    pub const MEDIUM: Self = Self(0.5);
    /// 1: maximum.
    pub const HIGH: Self = Self(1.0);

    /// `level` in -1..=1; clamped, NaN becomes [`Self::STANDARD`].
    #[track_caller]
    pub fn new(level: f64) -> Self {
        if (-1.0..=1.0).contains(&level) {
            return Self(level);
        }
        egui_mineral::kit::sanitize::report(format_args!("contrast must be in -1..=1, got {level}"));
        if level.is_nan() {
            Self::STANDARD
        } else {
            Self(level.clamp(-1.0, 1.0))
        }
    }

    /// In -1..=1.
    pub const fn level(self) -> f64 {
        self.0
    }
}

/// Value that is not a valid [`Contrast`].
#[derive(Clone, Debug, PartialEq, thiserror::Error)]
#[non_exhaustive]
pub enum ContrastError {
    /// NaN, infinite or outside -1..=1.
    #[error("contrast must be finite and within -1..=1, got {0}")]
    OutOfRange(f64),
    /// Text that is not a number.
    #[error("contrast is not a number: {0}")]
    Parse(#[from] ParseFloatError),
}

impl TryFrom<f64> for Contrast {
    type Error = ContrastError;

    fn try_from(level: f64) -> Result<Self, ContrastError> {
        if (-1.0..=1.0).contains(&level) {
            Ok(Self(level))
        } else {
            Err(ContrastError::OutOfRange(level))
        }
    }
}

impl fmt::Display for Contrast {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}

impl FromStr for Contrast {
    type Err = ContrastError;

    fn from_str(s: &str) -> Result<Self, ContrastError> {
        s.parse::<f64>()?.try_into()
    }
}

/// Seed, contrast and variant that determine the [`ColorScheme`] of either brightness.
#[derive(Clone, Copy, Debug, PartialEq)]
#[must_use]
pub struct SchemeSpec {
    seed: Color32,
    pub(crate) contrast: Contrast,
    variant: SchemeVariant,
}

impl SchemeSpec {
    /// Defaults: [`Contrast::STANDARD`], [`SchemeVariant::TonalSpot`].
    pub const fn new(seed: Color32) -> Self {
        Self {
            seed,
            contrast: Contrast::STANDARD,
            variant: SchemeVariant::TonalSpot,
        }
    }
    /// Default [`Contrast::STANDARD`].
    pub const fn contrast(mut self, contrast: Contrast) -> Self {
        self.contrast = contrast;
        self
    }
    /// Default [`SchemeVariant::TonalSpot`].
    pub const fn variant(mut self, variant: SchemeVariant) -> Self {
        self.variant = variant;
        self
    }

    /// Scheme of one brightness; runs material-color-utilities, not cheap: cache the result.
    pub fn scheme(self, theme: Theme) -> ColorScheme {
        let [light, dark] = self.schemes();
        match theme {
            Theme::Light => light,
            Theme::Dark => dark,
        }
    }

    pub(crate) fn schemes(self) -> [ColorScheme; 2] {
        let theme = ThemeBuilder::with_source(Rgb::new(self.seed.r(), self.seed.g(), self.seed.b()))
            .variant(self.variant.material())
            .contrast_level(self.contrast.0)
            .build();
        [
            ColorScheme::from_scheme(&theme.schemes.light, Theme::Light),
            ColorScheme::from_scheme(&theme.schemes.dark, Theme::Dark),
        ]
    }
}

impl Default for SchemeSpec {
    fn default() -> Self {
        Self::new(BASELINE_SEED)
    }
}

macro_rules! roles {
    ($($role:ident),* $(,)?) => {
        /// MD3 color roles resolved for one theme mode.
        #[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
        #[non_exhaustive]
        #[expect(clippy::partial_pub_fields, reason = "the theme is fixed at generation, roles are tweakable")]
        pub struct ColorScheme {
            $(#[doc = concat!("MD3 `", stringify!($role), "` role.")] pub $role: Color32,)*
            theme: Theme,
        }

        impl ColorScheme {
            fn from_scheme(s: &material_colors::scheme::Scheme, theme: Theme) -> Self {
                let c = |c: Rgb| Color32::from_rgb(c.red, c.green, c.blue);
                Self { $($role: c(s.$role),)* theme }
            }
        }
    };
}

roles!(
    primary,
    on_primary,
    primary_container,
    on_primary_container,
    inverse_primary,
    primary_fixed,
    primary_fixed_dim,
    on_primary_fixed,
    on_primary_fixed_variant,
    secondary,
    on_secondary,
    secondary_container,
    on_secondary_container,
    secondary_fixed,
    secondary_fixed_dim,
    on_secondary_fixed,
    on_secondary_fixed_variant,
    tertiary,
    on_tertiary,
    tertiary_container,
    on_tertiary_container,
    tertiary_fixed,
    tertiary_fixed_dim,
    on_tertiary_fixed,
    on_tertiary_fixed_variant,
    error,
    on_error,
    error_container,
    on_error_container,
    surface_dim,
    surface,
    surface_tint,
    surface_bright,
    surface_container_lowest,
    surface_container_low,
    surface_container,
    surface_container_high,
    surface_container_highest,
    on_surface,
    on_surface_variant,
    outline,
    outline_variant,
    inverse_surface,
    inverse_on_surface,
    surface_variant,
    background,
    on_background,
    shadow,
    scrim,
);

impl ColorScheme {
    /// Mode the scheme was generated for; fixed.
    pub const fn theme(&self) -> Theme {
        self.theme
    }

    /// Scheme of [`BASELINE_SEED`] with default spec settings.
    pub fn baseline(theme: Theme) -> Self {
        SchemeSpec::default().scheme(theme)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn baseline_matches_md3_reference() {
        let light = ColorScheme::baseline(Theme::Light);
        // MD3 baseline primary: tone 40 of the baseline seed.
        assert_eq!(light.primary, Color32::from_rgb(0x65, 0x55, 0x8F));
        let hc = SchemeSpec::default().contrast(Contrast::HIGH).scheme(Theme::Light);
        assert_ne!(hc.primary, light.primary);
    }

    #[test]
    fn one_spec_yields_both_brightnesses() {
        let spec = SchemeSpec::new(Color32::from_rgb(0x20, 0x90, 0x40));
        let [light, dark] = spec.schemes();
        assert_eq!((light.theme(), dark.theme()), (Theme::Light, Theme::Dark));
        assert_eq!(spec.scheme(Theme::Light), light);
        assert_eq!(spec.scheme(Theme::Dark), dark);
        assert_ne!(light.primary, dark.primary);
    }

    #[test]
    fn contrast_text_roundtrip_and_validation() {
        assert_eq!(Contrast::MEDIUM.to_string().parse(), Ok(Contrast::MEDIUM));
        assert_eq!("-1".parse(), Ok(Contrast::REDUCED));
        assert_eq!(Contrast::try_from(0.25).map(Contrast::level), Ok(0.25));
        assert_eq!(Contrast::try_from(1.5), Err(ContrastError::OutOfRange(1.5)));
        assert!(matches!(
            Contrast::try_from(f64::NAN),
            Err(ContrastError::OutOfRange(_))
        ));
        assert!(matches!("inf".parse::<Contrast>(), Err(ContrastError::OutOfRange(_))));
        assert!(matches!("high".parse::<Contrast>(), Err(ContrastError::Parse(_))));
    }

    #[test]
    fn variant_names_roundtrip() {
        for &v in SchemeVariant::ALL {
            assert_eq!(v.to_string().parse(), Ok(v));
        }
        assert_eq!("tonal_spot".parse(), Ok(SchemeVariant::TonalSpot));
        assert!("TonalSpot".parse::<SchemeVariant>().is_err());
    }
}
