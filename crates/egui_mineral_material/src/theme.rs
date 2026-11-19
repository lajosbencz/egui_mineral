use std::sync::Arc;

use egui::{CornerRadius, FontId, Stroke, Style, TextStyle, Theme};

#[cfg(all(feature = "theme-file", not(target_arch = "wasm32")))]
mod file;
#[cfg(all(feature = "theme-file", not(target_arch = "wasm32")))]
pub use file::{ThemeError, ThemeWatcher, load_toml, watch};

pub use crate::tokens::{Durations, Shape, StateLayers, Tokens};
use crate::{color::SchemeSpec, spec::TypeScale};

#[derive(Debug)]
struct Themed {
    tokens: Arc<Tokens>,
    style: Arc<Style>,
}

impl Themed {
    fn new(tokens: Tokens) -> Self {
        let visuals = match tokens.theme() {
            Theme::Light => egui::Visuals::light(),
            Theme::Dark => egui::Visuals::dark(),
        };
        let mut style = Style {
            visuals,
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
    spec: SchemeSpec,
}

impl TokenStore {
    pub(crate) fn new(spec: SchemeSpec) -> Self {
        Self {
            light: Themed::new(Tokens::new(spec.scheme(Theme::Light))),
            dark: Themed::new(Tokens::new(spec.scheme(Theme::Dark))),
            spec,
        }
    }

    fn slot(&mut self, theme: Theme) -> &mut Themed {
        match theme {
            Theme::Light => &mut self.light,
            Theme::Dark => &mut self.dark,
        }
    }

    fn themed(&self, theme: Theme) -> &Themed {
        match theme {
            Theme::Light => &self.light,
            Theme::Dark => &self.dark,
        }
    }

    pub(crate) fn tokens(&self, theme: Theme) -> Arc<Tokens> {
        Arc::clone(&self.themed(theme).tokens)
    }

    pub(crate) fn style(&self, theme: Theme) -> Arc<Style> {
        Arc::clone(&self.themed(theme).style)
    }

    pub(crate) fn spec(&self) -> SchemeSpec {
        self.spec
    }

    pub(crate) fn set(&mut self, tokens: Tokens) {
        let theme = tokens.theme();
        *self.slot(theme) = Themed::new(tokens);
    }

    pub(crate) fn apply(&mut self, spec: SchemeSpec) {
        for theme in [Theme::Light, Theme::Dark] {
            *self.slot(theme) = Themed::new(Tokens::new(spec.scheme(theme)));
        }
        self.spec = spec;
    }
}

fn apply_style(style: &mut Style, t: &Tokens) {
    let c = &t.color;
    let layer = |base: egui::Color32, a: f32| base.blend(c.on_surface.gamma_multiply(a));
    let small = CornerRadius::same(t.shape.small);
    let v = &mut style.visuals;
    v.dark_mode = t.theme() == Theme::Dark;
    v.override_text_color = None;
    v.weak_text_color = Some(c.on_surface_variant);
    v.panel_fill = c.surface;
    v.window_fill = c.surface_container_high;
    v.window_stroke = Stroke::new(1.0, c.outline_variant);
    v.window_corner_radius = CornerRadius::same(t.shape.large);
    v.menu_corner_radius = CornerRadius::same(t.shape.extra_small);
    v.faint_bg_color = c.surface_container_low;
    v.extreme_bg_color = c.surface_container_highest;
    v.text_edit_bg_color = Some(c.surface_container_highest);
    v.code_bg_color = c.surface_container;
    v.hyperlink_color = c.primary;
    v.warn_fg_color = c.tertiary;
    v.error_fg_color = c.error;
    v.selection.bg_fill = c.primary.gamma_multiply(0.35);
    v.selection.stroke = Stroke::new(1.0, c.on_surface);
    v.text_cursor.stroke = Stroke::new(2.0, c.primary);
    v.window_shadow.color = c.shadow.gamma_multiply(0.3);
    v.popup_shadow.color = c.shadow.gamma_multiply(0.3);
    // Every scroll area is nested unless shown through `scroll::page`.
    style.spacing.scroll = crate::scroll::nested_style();

    let w = &mut v.widgets;
    w.noninteractive.bg_fill = c.surface;
    w.noninteractive.weak_bg_fill = c.surface;
    w.noninteractive.bg_stroke = Stroke::new(1.0, c.outline_variant);
    w.noninteractive.fg_stroke = Stroke::new(1.0, c.on_surface);
    let base = c.surface_container_highest;
    for (wv, a) in [
        (&mut w.inactive, 0.0),
        (&mut w.hovered, t.state.hover),
        (&mut w.active, t.state.pressed),
        (&mut w.open, t.state.focus),
    ] {
        wv.bg_fill = layer(base, a);
        wv.weak_bg_fill = layer(base, a);
        wv.bg_stroke = Stroke::new(if a > 0.0 { 1.0 } else { 0.0 }, c.outline);
        wv.fg_stroke = Stroke::new(1.5, if a > 0.0 { c.on_surface } else { c.on_surface_variant });
        wv.corner_radius = small;
    }

    for (style_key, role) in [
        (TextStyle::Small, TypeScale::BodySmall),
        (TextStyle::Body, TypeScale::BodyMedium),
        (TextStyle::Button, TypeScale::LabelLarge),
        (TextStyle::Heading, TypeScale::HeadlineSmall),
    ] {
        style.text_styles.insert(style_key, FontId::proportional(role.size()));
    }
}

#[cfg(all(test, feature = "bundled-icons"))]
mod tests {
    use egui::{Context, ThemePreference};

    use crate::color::Contrast;

    use super::*;
    use crate::Material;

    fn parse(edit: impl FnOnce(&mut toml::Table)) -> Result<Tokens, toml::de::Error> {
        toml::from_str(&encoded(edit))
    }

    fn encoded(edit: impl FnOnce(&mut toml::Table)) -> String {
        let mut table = toml::Table::try_from(Tokens::baseline(Theme::Light, Contrast::STANDARD)).expect("serialize");
        edit(&mut table);
        toml::to_string(&table).expect("encode")
    }

    #[test]
    fn toml_roundtrip_and_invalid_values_rejected() {
        parse(|_| {}).expect("valid tokens");
        let dark: Tokens =
            toml::from_str(&toml::to_string(&Tokens::baseline(Theme::Dark, Contrast::STANDARD)).expect("encode"))
                .expect("dark tokens");
        assert_eq!(dark.theme(), Theme::Dark, "the theme survives the TOML roundtrip");
        let err = parse(|t| {
            t["state"]
                .as_table_mut()
                .expect("state")
                .insert("hover".into(), 2.0.into());
        })
        .expect_err("opacity above 1");
        assert!(err.to_string().contains("state.hover"), "{err}");
        let err = parse(|t| {
            t["duration"].as_table_mut().expect("duration")["short"] = vec![-1.0, 0.1, 0.1, 0.1].into();
        })
        .expect_err("negative duration");
        assert!(err.to_string().contains("duration.short"), "{err}");
    }

    const SEED: egui::Color32 = egui::Color32::from_rgb(0x20, 0x90, 0x40);

    fn current(m: &Material, theme: Theme) -> Tokens {
        (*m.tokens(theme)).clone()
    }

    #[test]
    fn pass_styles_its_window_and_leaves_egui_alone() {
        let ctx = Context::default();
        let rt = crate::test_support::runtime(&ctx);
        rt.core().set_preference(ThemePreference::Dark);
        let global = ctx.global_style();
        let mut seen = None;
        crate::test_support::run(&ctx, egui::RawInput::default(), |ui| {
            rt.pass(ui, |ui| seen = Some((ui.visuals().dark_mode, ui.visuals().panel_fill)));
        });
        let dark = rt.kit().tokens(Theme::Dark);
        assert_eq!(seen, Some((true, dark.color.surface)));
        assert_eq!(
            ctx.global_style().visuals,
            global.visuals,
            "egui's app-wide style untouched"
        );
        assert_eq!(ctx.options(|o| o.theme_preference), ThemePreference::default());
    }

    #[test]
    fn apply_seeds_both_brightnesses() {
        let (ctx, m) = (Context::default(), Material::default());
        let spec = SchemeSpec::new(SEED);
        m.apply(&ctx, spec);
        for theme in [Theme::Light, Theme::Dark] {
            assert_eq!(current(&m, theme), Tokens::new(spec.scheme(theme)), "{theme:?}");
        }
    }

    #[test]
    fn set_replaces_one_brightness() {
        let (ctx, m) = (Context::default(), Material::default());
        let spec = SchemeSpec::new(SEED);
        m.apply(&ctx, spec);
        let custom = Tokens::baseline(Theme::Dark, Contrast::HIGH);
        m.set_tokens(&ctx, custom.clone());
        assert_eq!(current(&m, Theme::Dark), custom);
        assert_eq!(current(&m, Theme::Light), Tokens::new(spec.scheme(Theme::Light)));
    }

    #[test]
    fn set_contrast_keeps_the_seed() {
        let (ctx, m) = (Context::default(), Material::default());
        let spec = SchemeSpec::new(SEED).variant(crate::color::SchemeVariant::Vibrant);
        m.apply(&ctx, spec);
        m.set_contrast(&ctx, Contrast::HIGH);
        assert_eq!(m.contrast(), Contrast::HIGH);
        let spec = spec.contrast(Contrast::HIGH);
        for theme in [Theme::Light, Theme::Dark] {
            assert_eq!(current(&m, theme), Tokens::new(spec.scheme(theme)), "{theme:?}");
        }
    }

    #[test]
    fn new_material_holds_the_baseline_at_its_contrast() {
        let ctx = Context::default();
        let m = Material::new().with_contrast(Contrast::MEDIUM);
        assert_eq!(
            current(&m, Theme::Light),
            Tokens::baseline(Theme::Light, Contrast::MEDIUM)
        );
        assert_eq!(m.contrast(), Contrast::MEDIUM);
        m.set_contrast(&ctx, Contrast::HIGH);
        assert_eq!(current(&m, Theme::Dark), Tokens::baseline(Theme::Dark, Contrast::HIGH));
    }
}
