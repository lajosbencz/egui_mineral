use egui::{Color32, Theme};

use crate::{
    spec::tokens::{geometry, shadow},
    spec::{BrandRamp, ColorToken, Shadow},
};

/// One of the three Fluent UI React v9 themes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Scheme {
    /// `webLightTheme`.
    WebLight,
    /// `webDarkTheme`.
    WebDark,
    /// `createHighContrastTheme` alias colors; dark, no brand.
    HighContrast,
}

impl Scheme {
    /// Every variant.
    pub const ALL: &'static [Self] = &[Self::WebLight, Self::WebDark, Self::HighContrast];

    /// Light for [`Self::WebLight`], dark otherwise.
    pub const fn theme(self) -> Theme {
        match self {
            Self::WebLight => Theme::Light,
            Self::WebDark | Self::HighContrast => Theme::Dark,
        }
    }
}

impl From<Theme> for Scheme {
    fn from(theme: Theme) -> Self {
        match theme {
            Theme::Light => Self::WebLight,
            Theme::Dark => Self::WebDark,
        }
    }
}

/// Contrast axis of [`Fluent`](crate::Fluent), independent of the brightness
/// ([`Fluent::set_contrast_mode`](crate::Fluent::set_contrast_mode)).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ContrastMode {
    /// [`Scheme::WebLight`] or [`Scheme::WebDark`], by brightness.
    #[default]
    Standard,
    /// [`Scheme::HighContrast`] at either brightness.
    High,
}

/// Elevation level of the `shadow*` tokens.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Elevation {
    /// `shadow2`.
    E2,
    /// `shadow4`.
    E4,
    /// `shadow8`.
    E8,
    /// `shadow16`.
    E16,
    /// `shadow28`.
    E28,
    /// `shadow64`.
    E64,
}

impl Elevation {
    /// Every variant, lowest first.
    pub const ALL: &'static [Self] = &[Self::E2, Self::E4, Self::E8, Self::E16, Self::E28, Self::E64];

    /// Layer geometry.
    pub const fn shadow(self) -> Shadow {
        match self {
            Self::E2 => shadow::SHADOW_2,
            Self::E4 => shadow::SHADOW_4,
            Self::E8 => shadow::SHADOW_8,
            Self::E16 => shadow::SHADOW_16,
            Self::E28 => shadow::SHADOW_28,
            Self::E64 => shadow::SHADOW_64,
        }
    }
}

/// Corner radius scale (`borderRadius*` tokens).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Radius {
    /// `borderRadiusNone`.
    None,
    /// `borderRadiusSmall`.
    Small,
    /// `borderRadiusMedium`.
    Medium,
    /// `borderRadiusLarge`.
    Large,
    /// `borderRadiusXLarge`.
    XLarge,
    /// `borderRadius2XLarge`.
    X2Large,
    /// `borderRadius3XLarge`.
    X3Large,
    /// `borderRadius4XLarge`.
    X4Large,
    /// `borderRadius5XLarge`.
    X5Large,
    /// `borderRadius6XLarge`.
    X6Large,
    /// `borderRadiusCircular`: a pill once clamped to the shape.
    Circular,
}

impl Radius {
    /// Every variant, smallest first.
    pub const ALL: &'static [Self] = &[
        Self::None,
        Self::Small,
        Self::Medium,
        Self::Large,
        Self::XLarge,
        Self::X2Large,
        Self::X3Large,
        Self::X4Large,
        Self::X5Large,
        Self::X6Large,
        Self::Circular,
    ];

    /// In points.
    pub const fn points(self) -> f32 {
        match self {
            Self::None => geometry::BORDER_RADIUS_NONE,
            Self::Small => geometry::BORDER_RADIUS_SMALL,
            Self::Medium => geometry::BORDER_RADIUS_MEDIUM,
            Self::Large => geometry::BORDER_RADIUS_LARGE,
            Self::XLarge => geometry::BORDER_RADIUS_X_LARGE,
            Self::X2Large => geometry::BORDER_RADIUS_2X_LARGE,
            Self::X3Large => geometry::BORDER_RADIUS_3X_LARGE,
            Self::X4Large => geometry::BORDER_RADIUS_4X_LARGE,
            Self::X5Large => geometry::BORDER_RADIUS_5X_LARGE,
            Self::X6Large => geometry::BORDER_RADIUS_6X_LARGE,
            Self::Circular => geometry::BORDER_RADIUS_CIRCULAR,
        }
    }

    /// Radius on `rect`, clamped to half its shorter side.
    pub fn on(self, rect: egui::Rect) -> egui::CornerRadius {
        egui::CornerRadius::from(self.points().min(rect.width().min(rect.height()) / 2.0).max(0.0))
    }
}

/// Every alias color of one [`Scheme`], resolved against a brand ramp.
#[derive(Clone, Debug, PartialEq)]
pub struct Tokens {
    scheme: Scheme,
    brand: BrandRamp,
    colors: Box<[Color32]>,
}

impl Tokens {
    /// `scheme`'s alias colors; [`Scheme::HighContrast`] ignores `brand`.
    pub fn new(scheme: Scheme, brand: BrandRamp) -> Self {
        let colors = ColorToken::ALL
            .iter()
            .map(|&c| match scheme {
                Scheme::WebLight => c.web_light(&brand),
                Scheme::WebDark => c.web_dark(&brand),
                Scheme::HighContrast => c.high_contrast(),
            })
            .collect();
        Self { scheme, brand, colors }
    }

    /// Scheme of these tokens.
    pub const fn scheme(&self) -> Scheme {
        self.scheme
    }

    /// Brand ramp the brand tokens resolved against.
    pub const fn brand(&self) -> BrandRamp {
        self.brand
    }

    // Container and content of a non-default tone.
    pub(crate) fn tone(&self, tone: egui_mineral::tone::Tone) -> Option<(Color32, Color32)> {
        use egui_mineral::tone::Tone;
        match tone {
            Tone::Custom { container, content } => Some((container, content)),
            Tone::Error => Some((
                self.color(ColorToken::ColorPaletteRedBackground1),
                self.color(ColorToken::ColorPaletteRedForeground1),
            )),
            _ => None,
        }
    }

    /// Color of `token`.
    pub fn color(&self, token: ColorToken) -> Color32 {
        // `colors` maps `ColorToken::ALL`.
        crate::widget::dense_tables!(ColorToken:);
        self.colors.get(token as usize).copied().unwrap_or(Color32::PLACEHOLDER)
    }

    /// egui shadows of `level`, colored by `tint`.
    pub fn shadow(&self, level: Elevation, tint: ShadowTint) -> ElevationShadow {
        let (ambient, key) = match tint {
            ShadowTint::Brand => (ColorToken::ColorBrandShadowAmbient, ColorToken::ColorBrandShadowKey),
            ShadowTint::Neutral => (ColorToken::ColorNeutralShadowAmbient, ColorToken::ColorNeutralShadowKey),
        };
        let s = level.shadow();
        ElevationShadow {
            ambient: layer(s.ambient, self.color(ambient)),
            key: layer(s.key, self.color(key)),
        }
    }
}

/// egui shadows of one [`Elevation`], colored ([`Tokens::shadow`]).
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct ElevationShadow {
    /// Wide, faint layer, painted first.
    pub ambient: egui::Shadow,
    /// Offset layer.
    pub key: egui::Shadow,
}

impl ElevationShadow {
    /// Layers in paint order.
    pub const fn layers(self) -> [egui::Shadow; 2] {
        [self.ambient, self.key]
    }
}

/// Color family of a shadow.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ShadowTint {
    /// Neutral shadow colors.
    #[default]
    Neutral,
    /// Brand shadow colors.
    Brand,
}

fn layer(l: crate::spec::ShadowLayer, color: Color32) -> egui::Shadow {
    let px = |v: f32| -> i8 {
        #[expect(clippy::cast_possible_truncation, reason = "clamped to i8")]
        let px = v.round().clamp(f32::from(i8::MIN), f32::from(i8::MAX)) as i8;
        px
    };
    #[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss, reason = "clamped to u8")]
    let blur = l.blur.round().clamp(0.0, f32::from(u8::MAX)) as u8;
    egui::Shadow {
        offset: [px(l.x), px(l.y)],
        blur,
        spread: 0,
        color,
    }
}
