use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
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
    BrandRamp, IconSet,
    spec::tokens::brand::BRAND_WEB,
    theme::{Scheme, TokenStore, Tokens},
};

/// Fluent 2 [`Stylekit`]: high contrast, brand ramp and the tokens and styles of each [`Scheme`]. Pass it to
/// [`Runtime::new`](egui_mineral::runtime::Runtime::new); the brightness shown is the runtime's
/// ([`Core::theme`](egui_mineral::runtime::Core::theme)).
#[derive(Debug)]
pub struct Fluent {
    high_contrast: AtomicBool,
    tokens: Mutex<TokenStore>,
    // Scheme and tokens frozen for one egui pass: setters apply from the next pass, no frame mixes two schemes.
    pass: PassCache<Theme, PassState>,
    icons: Arc<IconSet>,
    fonts: Vec<IconFont>,
}

impl Fluent {
    /// Web light and dark tokens of the Fluent web brand (`BRAND_WEB`), with the bundled icon font (1.6 MB).
    #[cfg(feature = "bundled-icons")]
    pub fn new() -> Self {
        Self::with_icon_font(crate::fonts::BUNDLED_ICONS)
    }

    /// `Self::new` with `ttf` instead of the bundled icon font: a Fluent UI System Icons resizable font (or a
    /// subset of it).
    pub fn with_icon_font(ttf: &'static [u8]) -> Self {
        Self::with_icons(&IconSet::fluent(), &[crate::fonts::icon_font(ttf)])
    }

    /// `Self::new` drawing the kit's own icons from `icons`, installing `fonts` instead of the Fluent icon font: Fluent
    /// icons ([`fonts::icon_font`](crate::fonts::icon_font)) or the icon font of another kit.
    pub fn with_icons(icons: &IconSet, fonts: &[IconFont]) -> Self {
        Self {
            high_contrast: AtomicBool::new(false),
            tokens: Mutex::new(TokenStore::new(BRAND_WEB)),
            pass: PassCache::default(),
            icons: Arc::new(*icons),
            fonts: fonts.to_vec(),
        }
    }

    /// High contrast replaces the light and dark schemes.
    pub fn high_contrast(&self) -> bool {
        self.high_contrast.load(Ordering::Relaxed)
    }

    /// Switches to or from the high contrast scheme.
    pub fn set_high_contrast(&self, ctx: &Context, high_contrast: bool) {
        self.high_contrast.store(high_contrast, Ordering::Relaxed);
        repaint(ctx);
    }

    /// Brand ramp of the web light and dark schemes.
    pub fn brand(&self) -> BrandRamp {
        self.tokens.lock().brand()
    }

    /// Rebuilds the web light and dark tokens from `brand`; high contrast has no brand.
    pub fn set_brand(&self, ctx: &Context, brand: BrandRamp) {
        self.tokens.lock().set_brand(brand);
        repaint(ctx);
    }

    /// Scheme shown at `theme` ([`Core::theme`](egui_mineral::runtime::Core::theme)): [`Scheme::HighContrast`]
    /// when set, else that brightness.
    pub fn scheme(&self, theme: Theme) -> Scheme {
        if self.high_contrast() {
            Scheme::HighContrast
        } else {
            theme.into()
        }
    }

    fn pass_state(&self, pass: u64, theme: Theme) -> PassState {
        self.pass.get_or_insert(pass, theme, || {
            let scheme = self.scheme(theme);
            let store = self.tokens.lock();
            PassState {
                tokens: store.tokens(scheme),
                style: store.style(scheme),
            }
        })
    }

    /// Tokens of `scheme`; widgets read `ui.tokens()` ([`FluentExt`]).
    pub fn tokens(&self, scheme: Scheme) -> Arc<Tokens> {
        self.tokens.lock().tokens(scheme)
    }
}

#[derive(Debug, Clone)]
struct PassState {
    tokens: Arc<Tokens>,
    style: Arc<Style>,
}

#[expect(clippy::disallowed_methods, reason = "one pass after the theme changed")]
fn repaint(ctx: &Context) {
    ctx.request_repaint();
}

#[cfg(feature = "bundled-icons")]
impl Default for Fluent {
    fn default() -> Self {
        Self::new()
    }
}

impl Stylekit for Fluent {
    fn policies(&self, ctx: &Context) -> Policies {
        crate::policy::policies(ctx.os())
    }

    fn catalogs(&self) -> &'static [Catalog] {
        &[("en-US", include_str!("../locales/en-US.ftl"))]
    }

    fn setup(&self, ctx: &Context) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.fonts.iter().for_each(|f| f.install(ctx));
        Ok(())
    }

    fn style(&self, ctx: &Context, theme: Theme) -> Arc<Style> {
        self.pass_state(ctx.cumulative_pass_nr(), theme).style
    }

    fn paint_window_close(&self, ui: &egui::Ui, visual: egui::Rect, response: &egui::Response) {
        ui.painter()
            .extend(egui_mineral::kit::paint::window_close(ui, visual, response));
    }
}

/// Runtime of a Fluent app.
pub type Rt = Runtime<Fluent>;

/// Fluent state on a [`Rt`] or Fluent [`MineralUi`].
pub trait FluentExt {
    /// Tokens of the current scheme, frozen for the egui pass: [`Fluent`] setters apply from the next pass.
    fn tokens(&self) -> Arc<Tokens>;
    /// Glyphs of the kit's own parts.
    fn icons(&self) -> Arc<IconSet>;
}

impl FluentExt for Rt {
    fn tokens(&self) -> Arc<Tokens> {
        self.kit()
            .pass_state(self.ctx().cumulative_pass_nr(), self.theme())
            .tokens
    }

    fn icons(&self) -> Arc<IconSet> {
        Arc::clone(&self.kit().icons)
    }
}

impl FluentExt for MineralUi<'_, Fluent> {
    fn tokens(&self) -> Arc<Tokens> {
        self.rt().tokens()
    }

    fn icons(&self) -> Arc<IconSet> {
        self.rt().icons()
    }
}
