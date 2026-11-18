use std::sync::{
    Arc,
    atomic::{AtomicI8, Ordering},
};

use egui::{Context, Style, Theme, ThemePreference};
use egui_mineral::{
    i18n::Catalog,
    policy::Policies,
    runtime::{Runtime, Stylekit},
    widget::MineralUi,
};
use parking_lot::Mutex;

use crate::{
    adaptive::{self, Density},
    color::{Contrast, SchemeSpec},
    fonts::{self, IconFonts},
    theme::{ThemeMode, TokenStore, Tokens},
};

/// Material Design 3 [`Stylekit`]: theme preference, tokens and styles per brightness, density. Pass it to
/// [`Runtime::new`](egui_mineral::runtime::Runtime::new).
#[derive(Debug)]
pub struct Material {
    preference: Mutex<ThemePreference>,
    tokens: Mutex<TokenStore>,
    density: AtomicI8,
    icon_fonts: Option<IconFonts>,
}

impl Material {
    /// Baseline tokens of both brightnesses at the contrast of `mode`, shown at its preference.
    pub fn new(mode: impl Into<ThemeMode>) -> Self {
        let mode = mode.into();
        Self {
            preference: Mutex::new(mode.preference()),
            tokens: Mutex::new(TokenStore::new(SchemeSpec::default().contrast(mode.contrast()))),
            density: AtomicI8::new(Density::DEFAULT.step()),
            icon_fonts: None,
        }
    }

    /// Icon fonts instead of the bundled ones; required without the `bundled-icons` feature, which
    /// otherwise links both bundled fonts (3 MB).
    #[must_use]
    pub fn icon_fonts(mut self, icon_fonts: IconFonts) -> Self {
        self.icon_fonts = Some(icon_fonts);
        self
    }

    /// Light, dark or `System`; this runtime's own, independent of egui's app-wide preference.
    pub fn preference(&self) -> ThemePreference {
        *self.preference.lock()
    }

    /// Switches the brightness of this runtime's windows.
    pub fn set_preference(&self, ctx: &Context, preference: ThemePreference) {
        *self.preference.lock() = preference;
        repaint(ctx);
    }

    /// Brightness shown: the preference, `System` resolved through the platform (egui's fallback when unknown).
    pub fn theme(&self, ctx: &Context) -> Theme {
        match self.preference() {
            ThemePreference::Light => Theme::Light,
            ThemePreference::Dark => Theme::Dark,
            ThemePreference::System => ctx.system_theme().unwrap_or_else(|| ctx.options(|o| o.fallback_theme)),
        }
    }

    /// Tokens of `theme`; widgets read `ui.tokens()` ([`MaterialExt`]).
    pub fn tokens(&self, theme: Theme) -> Arc<Tokens> {
        self.tokens.lock().tokens(theme)
    }

    /// Makes `tokens` current for `tokens.theme()`; the other brightness and the preference stay.
    pub fn set_tokens(&self, ctx: &Context, tokens: Tokens) {
        self.tokens.lock().set(tokens);
        repaint(ctx);
    }

    /// Default tokens of both brightnesses from `spec`; replaces [`Self::set_tokens`] ones.
    pub fn apply(&self, ctx: &Context, spec: SchemeSpec) {
        self.tokens.lock().apply(spec);
        repaint(ctx);
    }

    /// Contrast of the current spec.
    pub fn contrast(&self) -> Contrast {
        self.tokens.lock().spec().contrast
    }

    /// [`Self::apply`] of the current spec at `contrast`.
    pub fn set_contrast(&self, ctx: &Context, contrast: Contrast) {
        let mut store = self.tokens.lock();
        let spec = store.spec().contrast(contrast);
        store.apply(spec);
        drop(store);
        repaint(ctx);
    }

    /// App-wide [`Density`]; [`MaterialUiExt::with_density`] overrides it in a subtree.
    pub fn density(&self) -> Density {
        Density::new(self.density.load(Ordering::Relaxed))
    }

    /// Shrinks visuals only; touch targets stay at [`adaptive::MIN_INTERACTIVE`].
    pub fn set_density(&self, density: Density) {
        self.density.store(density.step(), Ordering::Relaxed);
    }
}

#[expect(clippy::disallowed_methods, reason = "one pass after the theme changed")]
fn repaint(ctx: &Context) {
    ctx.request_repaint();
}

impl Stylekit for Material {
    fn policies(&self, ctx: &Context) -> Policies {
        adaptive::policies(ctx.os())
    }

    fn catalogs(&self) -> &'static [Catalog] {
        &[("en-US", include_str!("../locales/en-US.ftl"))]
    }

    fn setup(&self, ctx: &Context) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if let Some(icon_fonts) = self.icon_fonts {
            fonts::install_icons(ctx, icon_fonts);
        } else {
            #[cfg(feature = "bundled-icons")]
            egui_material_icons::initialize(ctx);
            #[cfg(not(feature = "bundled-icons"))]
            return Err("no icon fonts: enable `bundled-icons` or set `Material::icon_fonts`".into());
        }
        Ok(())
    }

    fn style(&self, ctx: &Context) -> Arc<Style> {
        let theme = self.theme(ctx);
        self.tokens.lock().style(theme)
    }
}

/// Runtime of a Material app.
pub type Rt = Runtime<Material>;

/// Material state on a [`Rt`] or Material [`MineralUi`].
pub trait MaterialExt {
    /// Tokens of the current theme.
    fn tokens(&self) -> Arc<Tokens>;
}

impl MaterialExt for Rt {
    fn tokens(&self) -> Arc<Tokens> {
        self.kit().tokens(self.kit().theme(self.ctx()))
    }
}

impl MaterialExt for MineralUi<'_, Material> {
    fn tokens(&self) -> Arc<Tokens> {
        self.rt().tokens()
    }
}

const DENSITY_TAG: &str = "mineral::density";

/// Density of a Material [`MineralUi`] subtree.
pub trait MaterialUiExt {
    /// Density here: the nearest [`Self::with_density`] scope, else [`Material::density`].
    fn density(&self) -> Density;
    /// `size` shrunk by [`Self::density`], clamped at `min`.
    fn dense(&self, size: f32, min: Density) -> f32 {
        self.density().dense(size, min)
    }
    /// Runs `add` in a child whose widgets, and their descendants, use `density`.
    /// Popups and dialogs opened from it are separate roots and keep the app density.
    fn with_density<R>(&mut self, density: Density, add: impl FnOnce(&mut MineralUi<'_, Material>) -> R) -> R;
}

impl MaterialUiExt for MineralUi<'_, Material> {
    fn density(&self) -> Density {
        self.stack()
            .iter()
            .find_map(|s| s.info.tags.get_downcast::<Density>(DENSITY_TAG).copied())
            .unwrap_or_else(|| self.kit().density())
    }

    fn with_density<R>(&mut self, density: Density, add: impl FnOnce(&mut MineralUi<'_, Material>) -> R) -> R {
        let rt = self.rt();
        let mut info = egui::UiStackInfo::default();
        info.tags.insert(DENSITY_TAG, Some(Arc::new(density)));
        self.scope_builder(egui::UiBuilder::new().ui_stack_info(info), |ui| add(&mut rt.ui(ui)))
            .inner
    }
}
