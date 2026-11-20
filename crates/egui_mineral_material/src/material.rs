use std::sync::{
    Arc,
    atomic::{AtomicI8, Ordering},
};

use egui::{Context, Style, Theme};
use egui_mineral::{
    i18n::Catalog,
    icon::IconFont,
    kit::state::PassCache,
    policy::Policies,
    runtime::{Runtime, Stylekit},
    widget::MineralUi,
};
use parking_lot::Mutex;

use crate::{
    adaptive::{self, Density},
    color::{Contrast, SchemeSpec},
    icon_set::IconSet,
    theme::{TokenStore, Tokens},
};

/// Material Design 3 [`Stylekit`]: tokens and styles per brightness, density. Pass it to
/// [`Runtime::new`](egui_mineral::runtime::Runtime::new); the brightness shown is the runtime's
/// ([`Core::theme`](egui_mineral::runtime::Core::theme)).
#[derive(Debug)]
pub struct Material {
    tokens: Mutex<TokenStore>,
    // Tokens frozen for one egui pass: setters apply from the next pass, no frame mixes two token sets.
    pass: PassCache<Theme, (Arc<Tokens>, Arc<Style>)>,
    density: AtomicI8,
    icons: Arc<IconSet>,
    fonts: Fonts,
}

#[derive(Debug)]
enum Fonts {
    #[cfg(feature = "bundled-icons")]
    Bundled,
    Custom(Vec<IconFont>),
}

impl Material {
    /// Baseline tokens of both brightnesses at [`Contrast::STANDARD`] and [`Density::DEFAULT`], with the bundled
    /// icon fonts (3 MB).
    #[cfg(feature = "bundled-icons")]
    pub fn new() -> Self {
        Self::build(Fonts::Bundled)
    }

    /// `Self::new` installing `fonts` instead of the bundled ones: Material Symbols fonts
    /// ([`IconFonts::fonts`](crate::fonts::IconFonts::fonts)) or the icon font of another kit, whose glyphs
    /// [`Self::with_icons`] names.
    pub fn from_fonts(fonts: impl IntoIterator<Item = IconFont>) -> Self {
        Self::build(Fonts::Custom(fonts.into_iter().collect()))
    }

    fn build(fonts: Fonts) -> Self {
        Self {
            tokens: Mutex::new(TokenStore::new(SchemeSpec::default())),
            pass: PassCache::default(),
            density: AtomicI8::new(Density::DEFAULT.step()),
            icons: Arc::new(IconSet::material()),
            fonts,
        }
    }

    /// Starts at `contrast`; [`Self::set_contrast`] changes it later.
    #[must_use]
    pub fn with_contrast(self, contrast: Contrast) -> Self {
        let mut store = self.tokens.lock();
        let spec = store.spec().contrast(contrast);
        store.apply(spec);
        drop(store);
        self
    }

    /// Starts at `density`; [`Self::set_density`] changes it later.
    #[must_use]
    pub fn with_density(self, density: Density) -> Self {
        self.set_density(density);
        self
    }

    /// Draws the kit's own parts from `icons` instead of [`IconSet::material`].
    #[must_use]
    pub fn with_icons(mut self, icons: IconSet) -> Self {
        self.icons = Arc::new(icons);
        self
    }

    /// Tokens of `theme`; widgets read `ui.tokens()` ([`MaterialExt`]).
    pub fn tokens(&self, theme: Theme) -> Arc<Tokens> {
        self.tokens.lock().tokens(theme)
    }

    // Tokens and style of one store state for the whole egui pass.
    fn pass_snapshot(&self, pass: u64, theme: Theme) -> (Arc<Tokens>, Arc<Style>) {
        self.pass.get_or_insert(pass, theme, || {
            let store = self.tokens.lock();
            (store.tokens(theme), store.style(theme))
        })
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

    /// Every component follows it: containers shrink 4dp per step down to their content, and so does every
    /// minimum hit target ([`Stylekit::min_target`], [`adaptive::min_interactive`]), never below the visual
    /// box. Prominence and message surfaces (FABs, dialogs, sheets, pickers, snackbars, badges, progress)
    /// keep their spec size; only their targets shrink.
    pub fn set_density(&self, density: Density) {
        self.density.store(density.step(), Ordering::Relaxed);
    }
}

#[expect(clippy::disallowed_methods, reason = "one pass after the theme changed")]
fn repaint(ctx: &Context) {
    ctx.request_repaint();
}

#[cfg(feature = "bundled-icons")]
impl Default for Material {
    fn default() -> Self {
        Self::new()
    }
}

impl Stylekit for Material {
    fn policies(&self, ctx: &Context) -> Policies {
        adaptive::policies(ctx.os())
    }

    fn catalogs(&self) -> &'static [Catalog] {
        static CATALOGS: [Catalog; 1] = [Catalog::new("en-US", include_str!("../locales/en-US.ftl"))];
        &CATALOGS
    }

    fn setup(&self, ctx: &Context) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        match &self.fonts {
            #[cfg(feature = "bundled-icons")]
            Fonts::Bundled => crate::fonts::install_bundled_icons(ctx),
            Fonts::Custom(fonts) => fonts.iter().for_each(|f| f.install(ctx)),
        }
        Ok(())
    }

    fn style(&self, ctx: &Context, theme: Theme) -> Arc<Style> {
        self.pass_snapshot(ctx.cumulative_pass_nr(), theme).1
    }

    fn min_target(&self, ui: &MineralUi<'_, Self>, target: f32) -> f32 {
        ui.density().target(target)
    }
}

/// Runtime of a Material app.
pub type Rt = Runtime<Material>;

mod sealed {
    pub trait Sealed {}
    impl Sealed for super::Rt {}
    impl Sealed for egui_mineral::widget::MineralUi<'_, super::Material> {}
}

/// Material state on a [`Rt`] or Material [`MineralUi`]; sealed.
pub trait MaterialExt: sealed::Sealed {
    /// Tokens of the current theme, frozen for the egui pass: [`Material`] setters apply from the next pass.
    fn tokens(&self) -> Arc<Tokens>;
    /// Glyphs of the kit's own parts.
    fn icons(&self) -> Arc<IconSet>;
}

impl MaterialExt for Rt {
    fn tokens(&self) -> Arc<Tokens> {
        self.kit()
            .pass_snapshot(self.ctx().cumulative_pass_nr(), self.theme())
            .0
    }

    fn icons(&self) -> Arc<IconSet> {
        Arc::clone(&self.kit().icons)
    }
}

impl MaterialExt for MineralUi<'_, Material> {
    fn tokens(&self) -> Arc<Tokens> {
        self.rt().tokens()
    }

    fn icons(&self) -> Arc<IconSet> {
        self.rt().icons()
    }
}

const DENSITY_TAG: &str = "mineral::density";

/// Density of a Material [`MineralUi`] subtree; sealed.
pub trait MaterialUiExt: sealed::Sealed {
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
