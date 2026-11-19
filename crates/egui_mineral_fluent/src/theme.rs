use std::sync::Arc;

use egui::{CornerRadius, Stroke, Style, TextStyle};

use crate::ColorToken::{
    ColorBrandBackground, ColorBrandForegroundLink, ColorNeutralBackground1, ColorNeutralBackground1Hover,
    ColorNeutralBackground1Pressed, ColorNeutralBackground1Selected, ColorNeutralBackground2, ColorNeutralBackground3,
    ColorNeutralForeground1, ColorNeutralForeground1Hover, ColorNeutralForeground1Pressed,
    ColorNeutralForeground1Selected, ColorNeutralForeground3, ColorNeutralForegroundOnBrand, ColorNeutralStroke1,
    ColorNeutralStroke1Hover, ColorNeutralStroke1Pressed, ColorNeutralStroke1Selected, ColorNeutralStroke2,
    ColorStatusDangerForeground1, ColorStatusWarningForeground1, ColorTransparentStroke,
};
pub use crate::tokens::{Elevation, Radius, Scheme, Tokens};
use crate::{BrandRamp, ColorToken, spec::tokens::geometry, typography::TypeRamp};

#[derive(Debug)]
struct Themed {
    tokens: Arc<Tokens>,
    style: Arc<Style>,
}

impl Themed {
    fn new(tokens: Tokens) -> Self {
        let mut style = Style {
            visuals: match tokens.scheme().theme() {
                egui::Theme::Light => egui::Visuals::light(),
                egui::Theme::Dark => egui::Visuals::dark(),
            },
            ..Style::default()
        };
        apply_style(&mut style, &tokens);
        Self {
            tokens: Arc::new(tokens),
            style: Arc::new(style),
        }
    }
}

#[derive(Debug)]
pub(crate) struct TokenStore {
    light: Themed,
    dark: Themed,
    high_contrast: Themed,
    brand: BrandRamp,
}

impl TokenStore {
    pub(crate) fn new(brand: BrandRamp) -> Self {
        Self {
            light: Themed::new(Tokens::new(Scheme::WebLight, brand)),
            dark: Themed::new(Tokens::new(Scheme::WebDark, brand)),
            high_contrast: Themed::new(Tokens::new(Scheme::HighContrast, brand)),
            brand,
        }
    }

    fn themed(&self, scheme: Scheme) -> &Themed {
        match scheme {
            Scheme::WebLight => &self.light,
            Scheme::WebDark => &self.dark,
            Scheme::HighContrast => &self.high_contrast,
        }
    }

    pub(crate) fn tokens(&self, scheme: Scheme) -> Arc<Tokens> {
        Arc::clone(&self.themed(scheme).tokens)
    }

    pub(crate) fn style(&self, scheme: Scheme) -> Arc<Style> {
        Arc::clone(&self.themed(scheme).style)
    }

    pub(crate) fn brand(&self) -> BrandRamp {
        self.brand
    }

    pub(crate) fn set_brand(&mut self, brand: BrandRamp) {
        self.light = Themed::new(Tokens::new(Scheme::WebLight, brand));
        self.dark = Themed::new(Tokens::new(Scheme::WebDark, brand));
        self.brand = brand;
    }
}

fn apply_style(style: &mut Style, t: &Tokens) {
    let c = |token: ColorToken| t.color(token);
    let thin = geometry::STROKE_WIDTH_THIN;
    let medium = CornerRadius::from(Radius::Medium.points());
    let v = &mut style.visuals;
    v.dark_mode = t.scheme().theme() == egui::Theme::Dark;
    v.override_text_color = None;
    v.weak_text_color = Some(c(ColorNeutralForeground3));
    v.panel_fill = c(ColorNeutralBackground2);
    v.window_fill = c(ColorNeutralBackground1);
    v.window_stroke = Stroke::new(thin, c(ColorTransparentStroke));
    v.window_corner_radius = CornerRadius::from(Radius::XLarge.points());
    v.menu_corner_radius = medium;
    v.faint_bg_color = c(ColorNeutralBackground3);
    v.extreme_bg_color = c(ColorNeutralBackground1);
    v.text_edit_bg_color = Some(c(ColorNeutralBackground1));
    v.code_bg_color = c(ColorNeutralBackground3);
    v.hyperlink_color = c(ColorBrandForegroundLink);
    v.warn_fg_color = c(ColorStatusWarningForeground1);
    v.error_fg_color = c(ColorStatusDangerForeground1);
    v.selection.bg_fill = c(ColorBrandBackground);
    v.selection.stroke = Stroke::new(thin, c(ColorNeutralForegroundOnBrand));
    v.text_cursor.stroke = Stroke::new(thin, c(ColorNeutralForeground1));
    let [_, key] = t.shadow(Elevation::E16, false);
    v.window_shadow = key;
    let [_, key] = t.shadow(Elevation::E8, false);
    v.popup_shadow = key;

    let w = &mut v.widgets;
    w.noninteractive.bg_fill = c(ColorNeutralBackground1);
    w.noninteractive.weak_bg_fill = c(ColorNeutralBackground1);
    w.noninteractive.bg_stroke = Stroke::new(thin, c(ColorNeutralStroke2));
    w.noninteractive.fg_stroke = Stroke::new(thin, c(ColorNeutralForeground1));
    // react-button useButtonStyles.styles.ts secondary appearance
    for (wv, bg, stroke, fg) in [
        (
            &mut w.inactive,
            ColorNeutralBackground1,
            ColorNeutralStroke1,
            ColorNeutralForeground1,
        ),
        (
            &mut w.hovered,
            ColorNeutralBackground1Hover,
            ColorNeutralStroke1Hover,
            ColorNeutralForeground1Hover,
        ),
        (
            &mut w.active,
            ColorNeutralBackground1Pressed,
            ColorNeutralStroke1Pressed,
            ColorNeutralForeground1Pressed,
        ),
        (
            &mut w.open,
            ColorNeutralBackground1Selected,
            ColorNeutralStroke1Selected,
            ColorNeutralForeground1Selected,
        ),
    ] {
        wv.bg_fill = c(bg);
        wv.weak_bg_fill = c(bg);
        wv.bg_stroke = Stroke::new(thin, c(stroke));
        wv.fg_stroke = Stroke::new(thin, c(fg));
        wv.corner_radius = medium;
    }

    for (style_key, ramp) in [
        (TextStyle::Small, TypeRamp::Caption1),
        (TextStyle::Body, TypeRamp::Body1),
        (TextStyle::Button, TypeRamp::Body1Strong),
        (TextStyle::Heading, TypeRamp::Subtitle1),
    ] {
        style
            .text_styles
            .insert(style_key, egui::FontId::proportional(ramp.size()));
    }
}
