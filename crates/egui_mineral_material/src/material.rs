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
    fonts::IconFonts,
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
    /// Baseline tokens of both brightnesses at `contrast`, with the bundled icon fonts (3 MB).
    #[cfg(feature = "bundled-icons")]
    pub fn new(contrast: Contrast) -> Self {
        Self::build(contrast, Arc::new(IconSet::material()), Fonts::Bundled)
    }

    /// `Self::new` with `icon_fonts` instead of the bundled ones.
    pub fn with_icon_fonts(contrast: Contrast, icon_fonts: IconFonts) -> Self {
        Self::with_icons(contrast, &IconSet::material(), &icon_fonts.fonts())
    }

    /// `Self::new` drawing the kit's own icons from `icons`, installing `fonts` instead of the bundled ones: Material
    /// Symbols fonts ([`IconFonts::fonts`]) or the icon font of another kit.
    pub fn with_icons(contrast: Contrast, icons: &IconSet, fonts: &[IconFont]) -> Self {
        Self::build(contrast, Arc::new(*icons), Fonts::Custom(fonts.to_vec()))
    }

    fn build(contrast: Contrast, icons: Arc<IconSet>, fonts: Fonts) -> Self {
        Self {
            tokens: Mutex::new(TokenStore::new(SchemeSpec::default().contrast(contrast))),
            pass: PassCache::default(),
            density: AtomicI8::new(Density::DEFAULT.step()),
            icons,
            fonts,
        }
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

    /// Shrinks visuals only; touch targets stay at [`adaptive::MIN_INTERACTIVE`].
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
    /// [`Contrast::STANDARD`].
    fn default() -> Self {
        Self::new(Contrast::STANDARD)
    }
}

impl Stylekit for Material {
    fn policies(&self, ctx: &Context) -> Policies {
        adaptive::policies(ctx.os())
    }

    fn catalogs(&self) -> &'static [Catalog] {
        &[("en-US", include_str!("../locales/en-US.ftl"))]
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

    fn paint_window_close(&self, ui: &egui::Ui, visual: egui::Rect, response: &egui::Response) {
        let visuals = ui.style().interact(response);
        let p = ui.painter();
        if response.hovered() || response.is_pointer_button_down_on() {
            p.circle_filled(visual.center(), visual.width() / 2.0, visuals.weak_bg_fill);
        }
        let arm = visual.width() / 4.0;
        let c = visual.center();
        for d in [egui::vec2(arm, arm), egui::vec2(arm, -arm)] {
            p.line_segment([c - d, c + d], visuals.fg_stroke);
        }
        if response.has_focus() {
            p.circle_stroke(c, visual.width() / 2.0 + 2.0, ui.visuals().selection.stroke);
        }
    }
}

/// Runtime of a Material app.
pub type Rt = Runtime<Material>;

/// Material state on a [`Rt`] or Material [`MineralUi`].
pub trait MaterialExt {
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
