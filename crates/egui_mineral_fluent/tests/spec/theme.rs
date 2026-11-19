use egui::{Color32, Theme, ThemePreference};
use egui_mineral_fluent::{
    BrandRamp, BrandStep, ColorToken, FluentExt,
    spec::tokens::brand::BRAND_WEB,
    theme::{ContrastMode, Elevation, Radius, Scheme, ShadowTint, Tokens},
};

use crate::support::runtime;

#[test]
fn web_light_brand_background_is_brand_80() {
    let t = Tokens::new(Scheme::WebLight, BRAND_WEB);
    assert_eq!(
        t.color(ColorToken::ColorBrandBackground),
        BRAND_WEB.step(BrandStep::S80)
    );
}

// Tokens are frozen per egui pass.
fn next_pass(ctx: &egui::Context) {
    ctx.run_ui(egui::RawInput::default(), |_| {}).textures_delta.clear();
}

#[test]
fn custom_brand_reaches_light_and_dark_but_not_high_contrast() {
    let ctx = egui::Context::default();
    let rt = runtime(&ctx, Theme::Light);
    let brand = BrandRamp::new([Color32::from_rgb(200, 10, 10); 16]);
    rt.kit().set_brand(&ctx, brand);
    next_pass(&ctx);
    assert_eq!(
        rt.tokens().color(ColorToken::ColorBrandBackground),
        brand.step(BrandStep::S80)
    );
    rt.core().set_preference(ThemePreference::Dark);
    next_pass(&ctx);
    assert_eq!(rt.tokens().scheme(), Scheme::WebDark);
    assert_eq!(rt.tokens().brand(), brand);
    rt.kit().set_contrast_mode(&ctx, ContrastMode::High);
    next_pass(&ctx);
    let hc = rt.tokens();
    assert_eq!(hc.scheme(), Scheme::HighContrast);
    assert_eq!(
        hc.color(ColorToken::ColorBrandBackground),
        ColorToken::ColorBrandBackground.high_contrast()
    );
}

#[test]
fn every_token_resolves_in_every_scheme() {
    for &scheme in Scheme::ALL {
        let t = Tokens::new(scheme, BRAND_WEB);
        for &c in ColorToken::ALL {
            assert_ne!(t.color(c), Color32::PLACEHOLDER, "{scheme:?} {c:?}");
        }
    }
}

#[test]
fn shadows_and_radii_follow_the_tokens() {
    let t = Tokens::new(Scheme::WebLight, BRAND_WEB);
    let shadow = t.shadow(Elevation::E8, ShadowTint::Neutral);
    let key = shadow.key;
    assert_eq!(shadow.ambient.color, t.color(ColorToken::ColorNeutralShadowAmbient));
    assert_eq!((key.offset, key.blur), ([0, 4], 8));
    assert_eq!(Radius::Medium.points(), 4.0);
    let pill = Radius::Circular.on(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(80.0, 32.0)));
    assert_eq!(pill.nw, 16);
}

#[test]
fn a_setter_mid_pass_applies_from_the_next_pass() {
    let ctx = egui::Context::default();
    let rt = runtime(&ctx, Theme::Light);
    let mut seen = Vec::new();
    for _ in 0..2 {
        ctx.run_ui(egui::RawInput::default(), |_| {
            seen.push(rt.tokens().scheme());
            rt.kit().set_contrast_mode(&ctx, ContrastMode::High);
            seen.push(rt.tokens().scheme());
        })
        .textures_delta
        .clear();
    }
    assert_eq!(
        seen,
        [
            Scheme::WebLight,
            Scheme::WebLight,
            Scheme::HighContrast,
            Scheme::HighContrast
        ]
    );
}

#[test]
fn set_brand_mid_pass_keeps_style_for_the_pass() {
    use egui_mineral::runtime::Stylekit;
    let ctx = egui::Context::default();
    let rt = runtime(&ctx, Theme::Light);
    let mut styles = Vec::new();
    for _ in 0..2 {
        ctx.run_ui(egui::RawInput::default(), |_| {
            styles.push(rt.kit().style(&ctx, Theme::Light));
            rt.kit()
                .set_brand(&ctx, BrandRamp::new([Color32::from_rgb(200, 10, 10); 16]));
            styles.push(rt.kit().style(&ctx, Theme::Light));
        })
        .textures_delta
        .clear();
    }
    assert!(std::sync::Arc::ptr_eq(&styles[0], &styles[1]), "frozen within pass 1");
    assert!(!std::sync::Arc::ptr_eq(&styles[1], &styles[2]), "new brand from pass 2");
    assert!(std::sync::Arc::ptr_eq(&styles[2], &styles[3]), "frozen within pass 2");
}
