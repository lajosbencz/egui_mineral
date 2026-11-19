//! Typed runtime: [`Runtime::new`] is the only way to obtain one, and widgets are shown only through the
//! [`MineralUi`] it hands out, so configuration cannot be read before it exists.
//!
//! A runtime serves any number of windows (viewports) of one `Context`; each window runs its own pass
//! ([`Runtime::pass`]) with its own input, semantic tree, shortcuts and agent. Several runtimes, also of
//! different stylekits, share a `Context` one per window; an embedded window's runtime nests its pass in the
//! host's. `eframe::App::ui` calls [`Runtime::pass`]. Outside any pass, shortcut calls go to the viewport's
//! outermost runtime.

use std::{
    collections::VecDeque,
    sync::{
        Arc, Weak,
        atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering},
    },
};

use egui::{
    Context, Id, Pos2, RawInput, Rect, Response, Style, Theme, ThemePreference, Ui, Vec2, ViewportId, ViewportIdMap,
};
use parking_lot::{Mutex, MutexGuard};

use crate::{
    agent::{AgentHandle, AgentHost},
    focus,
    i18n::{Catalog, I18n, I18nError},
    input::claim::Relay,
    input::{InputAdapter, InputContext, UnifiedInputEvent},
    motion::Motion,
    policy::Policies,
    semantic::{self, SemanticTree},
    shortcut,
    widget::MineralUi,
};

/// Visual layer over the headless core: design values, message catalogs, style and egui setup.
pub trait Stylekit: Send + Sync + 'static {
    /// For the platform of `ctx`; sanitized by [`Runtime::new`].
    fn policies(&self, ctx: &Context) -> Policies;
    /// Library catalogs, source language first.
    fn catalogs(&self) -> &'static [Catalog];
    /// Fonts and other additive `Context` setup; runs once in [`Runtime::new`].
    ///
    /// # Errors
    /// Kit-specific setup failure.
    fn setup(&self, ctx: &Context) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
    /// Style of the runtime's root `Ui`s (window roots, areas) at `theme` ([`Core::theme`]); called per root,
    /// so keep it cheap.
    fn style(&self, ctx: &Context, theme: Theme) -> Arc<Style>;
    /// Shapes of the close glyph of an embedded [`crate::window::WindowSpec`] title bar in `visual`, the
    /// icon-sized box centered in the larger hit target of `response` (hover, press and focus state).
    /// Defaults to [`kit::paint::window_close`](crate::kit::paint::window_close) in the colors of `ui.style()`.
    fn window_close(&self, ui: &Ui, visual: Rect, response: &Response) -> Vec<egui::Shape> {
        crate::kit::paint::window_close(ui, visual, response)
    }
}

/// Failure building a [`Runtime`].
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum InitError {
    /// Root locale failed to load or a catalog does not parse.
    #[error("i18n: {0}")]
    I18n(#[from] I18nError),
    /// [`Stylekit::setup`] failed.
    #[error("stylekit setup: {0}")]
    Stylekit(#[source] Box<dyn std::error::Error + Send + Sync>),
}

/// Scope of a runtime's styling beyond its own `Ui`s: egui's root tooltips, popups and window decorations
/// read egui's global state ([`Core::set_global_style`]).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum GlobalStyle {
    /// egui's global style and theme stay the app's (the default).
    #[default]
    Egui,
    /// Each outermost pass writes [`Runtime::style`] into egui's style of the shown theme when it differs, and
    /// the runtime's preference into egui's, which syncs the native window theme (`Options::sync_window_theme`,
    /// on by default) per viewport, also on an OS flip under `System`. Nested passes never write. Leaving
    /// `Kit` restores nothing. Runtimes sharing a `Context` should not all use `Kit`.
    Kit,
}

struct PassState {
    adapter: InputAdapter,
    input: InputContext,
    tree: Option<Arc<SemanticTree>>,
    history: VecDeque<UnifiedInputEvent>,
    semantics_requested: bool,
    agent: AgentHost,
    agent_handle: AgentHandle,
    active: bool,
    relay: Relay,
}

impl PassState {
    fn new(ctx: &Context, viewport: ViewportId) -> Self {
        let (agent, agent_handle) = AgentHost::new(ctx, viewport);
        Self {
            adapter: InputAdapter::default(),
            input: InputContext::default(),
            tree: None,
            history: VecDeque::new(),
            semantics_requested: false,
            agent,
            agent_handle,
            active: false,
            relay: Relay::default(),
        }
    }
}

struct Passes {
    ctx: Context,
    map: Mutex<ViewportIdMap<PassState>>,
}

impl Passes {
    fn lock(&self) -> MutexGuard<'_, ViewportIdMap<PassState>> {
        self.map.lock()
    }

    // Lock order: egui `Context` reads happen before this lock, never under it.
    fn with<R>(&self, viewport: ViewportId, f: impl FnOnce(&mut PassState) -> R) -> R {
        f(self
            .lock()
            .entry(viewport)
            .or_insert_with(|| PassState::new(&self.ctx, viewport)))
    }

    // `raw.viewports` lists every live viewport; an empty list is an integration that does not report them.
    fn prune(&self, raw: &RawInput) {
        if raw.viewports.is_empty() {
            return;
        }
        self.lock().retain(|id, st| {
            st.active || *id == ViewportId::ROOT || *id == raw.viewport_id || raw.viewports.contains_key(id)
        });
    }

    fn inject(&self, raw: &mut RawInput) {
        // `raw` carries the rect only when it changed; otherwise the previous pass's applies.
        let screen = raw
            .screen_rect
            .unwrap_or_else(|| self.ctx.input_for(raw.viewport_id, egui::InputState::viewport_rect));
        let mut map = self.lock();
        if let Some(st) = map.get_mut(&raw.viewport_id)
            && st.agent.inject(raw, st.tree.as_deref(), screen)
        {
            st.adapter.mark_synthetic();
        }
    }
}

#[derive(Default)]
struct AgentPlugin {
    runtimes: Vec<Weak<Passes>>,
}

impl egui::Plugin for AgentPlugin {
    fn debug_name(&self) -> &'static str {
        "egui_mineral::agent"
    }

    fn input_hook(&mut self, _ctx: &Context, raw: &mut RawInput) {
        self.runtimes.retain(|w| w.strong_count() > 0);
        for passes in self.runtimes.iter().filter_map(Weak::upgrade) {
            passes.prune(raw);
            passes.inject(raw);
        }
    }
}

static NEXT_ID: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Copy, Default)]
struct Owner {
    current: Option<u64>,
    outermost: u64,
}

// Leaves the pass on drop, also when `add` unwinds.
struct PassGuard<'a> {
    core: &'a Core,
    viewport: ViewportId,
    owner: Id,
    outer: Option<u64>,
}

impl Drop for PassGuard<'_> {
    fn drop(&mut self) {
        self.core.passes.with(self.viewport, |st| st.active = false);
        let outer = self.outer;
        self.core
            .ctx
            .data_mut(|d| d.get_temp_mut_or_default::<Owner>(self.owner).current = outer);
    }
}

fn owner_id(viewport: ViewportId) -> Id {
    Id::unique(("mineral::pass_owner", viewport))
}

pub(crate) fn pass_scoped(ctx: &Context, name: &'static str) -> Id {
    let viewport = ctx.viewport_id();
    ctx.data(|d| pass_scoped_in(d, viewport, name))
}

// For callers already inside a `ctx.data` read, with the viewport read beforehand.
pub(crate) fn pass_scoped_in(d: &egui::util::IdTypeMap, viewport: ViewportId, name: &'static str) -> Id {
    let owner = d
        .get_temp::<Owner>(owner_id(viewport))
        .map(|o| o.current.unwrap_or(o.outermost));
    Id::unique((name, viewport, owner))
}

/// Kit-independent configuration and per-viewport pass state; what core models read instead of the egui
/// `Context`.
pub struct Core {
    id: u64,
    ctx: Context,
    i18n: I18n,
    policies: Policies,
    motion: Motion,
    passes: Arc<Passes>,
    collect_semantics: AtomicBool,
    input_history: AtomicUsize,
    preference: Mutex<ThemePreference>,
    global_style: Mutex<GlobalStyle>,
}

crate::kit::opaque_debug!(Core);

impl Core {
    pub(crate) fn id(&self) -> u64 {
        self.id
    }

    /// Context this runtime was built for.
    pub fn ctx(&self) -> &Context {
        &self.ctx
    }

    /// Locale, formatter and messages.
    pub fn i18n(&self) -> &I18n {
        &self.i18n
    }

    /// Sanitized at construction.
    pub fn policies(&self) -> &Policies {
        &self.policies
    }

    /// Reduced-motion preference.
    pub fn motion(&self) -> &Motion {
        &self.motion
    }

    /// Light, dark or `System` (the default); this runtime's own, independent of egui's app-wide preference.
    pub fn preference(&self) -> ThemePreference {
        *self.preference.lock()
    }

    /// Switches the brightness of this runtime's windows.
    #[expect(clippy::disallowed_methods, reason = "one pass after the theme changed")]
    pub fn set_preference(&self, preference: ThemePreference) {
        *self.preference.lock() = preference;
        self.ctx.request_repaint();
    }

    /// What egui-global state this runtime styles; [`GlobalStyle::Egui`] by default.
    pub fn global_style(&self) -> GlobalStyle {
        *self.global_style.lock()
    }

    /// Takes effect in the next pass.
    #[expect(clippy::disallowed_methods, reason = "one pass after the setting changed")]
    pub fn set_global_style(&self, global_style: GlobalStyle) {
        *self.global_style.lock() = global_style;
        self.ctx.request_repaint();
    }

    /// Brightness shown: the preference, `System` resolved through the platform (egui's fallback when unknown).
    pub fn theme(&self) -> Theme {
        match self.preference() {
            ThemePreference::Light => Theme::Light,
            ThemePreference::Dark => Theme::Dark,
            ThemePreference::System => {
                let ctx = &self.ctx;
                ctx.system_theme().unwrap_or_else(|| ctx.options(|o| o.fallback_theme))
            }
        }
    }

    pub(crate) fn coast(&self, velocity: Vec2, owner: Id) {
        let viewport = self.ctx.viewport_id();
        self.passes.with(viewport, |st| {
            st.relay.coast = velocity;
            st.relay.owner = Some(owner);
        });
    }

    pub(crate) fn relay(&self, origin: Pos2, owner: Id) {
        let viewport = self.ctx.viewport_id();
        self.passes.with(viewport, |st| {
            st.relay.origin = Some(origin);
            st.relay.owner = Some(owner);
        });
    }

    pub(crate) fn take_relay(&self, owner: Id) -> Vec2 {
        let viewport = self.ctx.viewport_id();
        self.passes.with(viewport, |st| st.relay.take(owner))
    }

    /// Input of the current viewport's pass; empty before its first [`Runtime::pass`].
    pub fn input(&self) -> InputContext {
        let viewport = self.ctx.viewport_id();
        self.passes.lock().get(&viewport).map_or_default(|st| st.input.clone())
    }
}

/// [`Core`] plus the stylekit `K`; shared (`Arc`) by its windows and every [`MineralUi`] it hands out.
pub struct Runtime<K> {
    core: Core,
    kit: K,
    style: fn(&K, &Context, Theme) -> Arc<Style>,
}

crate::kit::opaque_debug!(Runtime<K>);

impl<K: Stylekit> Runtime<K> {
    /// Runs [`Stylekit::setup`] and selects the system locale, else the CLDR root; semantics off, no input
    /// history.
    ///
    /// # Errors
    /// [`InitError::Stylekit`] when setup fails, [`InitError::I18n`] when the root locale fails to load or a
    /// catalog does not parse.
    pub fn new(ctx: &Context, kit: K) -> Result<Arc<Self>, InitError> {
        let i18n = I18n::new(ctx, kit.catalogs())?;
        Self::with_i18n(ctx, kit, i18n)
    }

    /// As [`Self::new`] with a prepared `i18n` (e.g. a fixed locale or [`AsciiFormat`](crate::i18n::AsciiFormat));
    /// the kit's catalogs are added to it.
    ///
    /// # Errors
    /// As [`Self::new`].
    pub fn with_i18n(ctx: &Context, kit: K, i18n: I18n) -> Result<Arc<Self>, InitError> {
        #[cfg(all(feature = "unstable-hotpatch", not(target_arch = "wasm32")))]
        crate::hotpatch::connect(ctx);
        kit.setup(ctx).map_err(InitError::Stylekit)?;
        for catalog in kit.catalogs() {
            i18n.add_resource(catalog)?;
        }
        let passes = Arc::new(Passes {
            ctx: ctx.clone(),
            map: Mutex::default(),
        });
        ctx.plugin_or_default::<AgentPlugin>()
            .lock()
            .runtimes
            .push(Arc::downgrade(&passes));
        let core = Core {
            id: NEXT_ID.fetch_add(1, Ordering::Relaxed),
            ctx: ctx.clone(),
            i18n,
            policies: kit.policies(ctx).sanitized(),
            motion: Motion::default(),
            passes,
            collect_semantics: AtomicBool::new(false),
            input_history: AtomicUsize::new(0),
            preference: Mutex::new(ThemePreference::System),
            global_style: Mutex::new(GlobalStyle::Egui),
        };
        Ok(Arc::new(Self {
            core,
            kit,
            style: K::style,
        }))
    }
}

impl<K> Runtime<K> {
    /// Wraps a plain `Ui`, e.g. inside egui closures: `ui.horizontal(|ui| { let ui = &mut rt.ui(ui); .. })`.
    /// Root `Ui`s (area, window) get the kit's style.
    pub fn ui<'a>(&'a self, ui: &'a mut Ui) -> MineralUi<'a, K> {
        if ui.stack().is_root_ui() {
            ui.set_style(self.style());
        }
        MineralUi::new(ui, self)
    }

    /// One pass of the current viewport: agent queue, semantic collection, shortcuts and IAL, then `add`
    /// on the viewport's root `Ui`, then the semantic tree and agent replies. Inside another pass of the
    /// same viewport (embedded window) only wraps `ui`.
    pub fn pass<R>(&self, ui: &mut Ui, add: impl FnOnce(&mut MineralUi<'_, K>) -> R) -> R {
        let viewport = self.core.ctx.viewport_id();
        let nested = self
            .core
            .passes
            .with(viewport, |st| std::mem::replace(&mut st.active, true));
        if nested {
            return add(&mut self.ui(ui));
        }
        let ctx = &self.core.ctx;
        let owner = owner_id(viewport);
        let outer = ctx.data_mut(|d| {
            let o = d.get_temp_mut_or_default::<Owner>(owner);
            let outer = o.current.replace(self.core.id);
            o.outermost = outer.map_or(self.core.id, |_| o.outermost);
            outer
        });
        let _restore = PassGuard {
            core: &self.core,
            viewport,
            owner,
            outer,
        };
        self.begin_pass(viewport);
        let (mut relay, pan) = self.core.passes.with(viewport, |st| (st.relay, st.input.pan));
        relay.step(ui, &self.core.policies.fling, pan);
        self.core.passes.with(viewport, |st| st.relay = relay);
        let style = self.style();
        if outer.is_none() {
            self.sync_global(&style);
        }
        ui.set_style(style);
        let r = self.add_hot(ui, add);
        self.end_pass(viewport);
        r
    }

    fn sync_global(&self, style: &Arc<Style>) {
        let core = &self.core;
        if core.global_style() != GlobalStyle::Kit {
            return;
        }
        let ctx = &core.ctx;
        let theme = core.theme();
        let current = ctx.style_of(theme);
        if !Arc::ptr_eq(&current, style) && *current != **style {
            ctx.set_style_of(theme, Arc::clone(style));
        }
        let preference = core.preference();
        if ctx.options(|o| o.theme_preference) != preference {
            ctx.set_theme(preference);
        }
    }

    #[cfg(not(all(feature = "unstable-hotpatch", not(target_arch = "wasm32"))))]
    fn add_hot<R>(&self, ui: &mut Ui, add: impl FnOnce(&mut MineralUi<'_, K>) -> R) -> R {
        add(&mut MineralUi::new(ui, self))
    }

    // subsecond::call wants FnMut; 0.7.10 never raises HotFnPanic, so the body runs once.
    #[cfg(all(feature = "unstable-hotpatch", not(target_arch = "wasm32")))]
    #[expect(clippy::expect_used, reason = "dev-only feature; the hot closure runs exactly once")]
    fn add_hot<R>(&self, ui: &mut Ui, add: impl FnOnce(&mut MineralUi<'_, K>) -> R) -> R {
        let mut add = Some(add);
        subsecond::call(|| add.take().map(|add| add(&mut MineralUi::new(ui, self)))).expect("hot closure ran once")
    }

    /// Kit style for root `Ui`s and frames built outside one; egui's global style stays the app's unless
    /// [`Core::set_global_style`] says [`GlobalStyle::Kit`].
    pub fn style(&self) -> Arc<Style> {
        (self.style)(&self.kit, &self.core.ctx, self.core.theme())
    }

    /// Kit-independent part.
    pub fn core(&self) -> &Core {
        &self.core
    }

    /// Stylekit state.
    pub fn kit(&self) -> &K {
        &self.kit
    }

    /// Context this runtime was built for.
    pub fn ctx(&self) -> &Context {
        &self.core.ctx
    }

    /// Shortcut for `core().i18n()`.
    pub fn i18n(&self) -> &I18n {
        &self.core.i18n
    }

    /// Shortcut for `core().theme()`.
    pub fn theme(&self) -> Theme {
        self.core.theme()
    }

    /// Collects a semantic tree every pass of every viewport (inspector); agents get one on demand regardless.
    pub fn set_collect_semantics(&self, collect_semantics: bool) {
        self.core.collect_semantics.store(collect_semantics, Ordering::Relaxed);
    }

    /// Toggled by [`Self::set_collect_semantics`].
    pub fn collect_semantics(&self) -> bool {
        self.core.collect_semantics.load(Ordering::Relaxed)
    }

    /// Asks for a tree in the next pass of `viewport` only; repeat each pass.
    pub fn request_semantics(&self, viewport: ViewportId) {
        self.core.passes.with(viewport, |st| st.semantics_requested = true);
    }

    /// Keeps the last `n` input events of each viewport for [`Self::recent_input`].
    pub fn set_input_history(&self, input_history: usize) {
        self.core.input_history.store(input_history, Ordering::Relaxed);
    }

    /// Retained event count per viewport; 0 keeps none.
    pub fn input_history(&self) -> usize {
        self.core.input_history.load(Ordering::Relaxed)
    }

    /// Cloneable handle for async agent hosts acting on `viewport`; safe to send to other threads.
    pub fn agent(&self, viewport: ViewportId) -> AgentHandle {
        self.core.passes.with(viewport, |st| st.agent_handle.clone())
    }

    /// Tree of the latest pass of `viewport` that collected one.
    pub fn tree(&self, viewport: ViewportId) -> Option<Arc<SemanticTree>> {
        self.core.passes.lock().get(&viewport).and_then(|st| st.tree.clone())
    }

    /// Oldest first, at most [`Self::input_history`] events of `viewport`.
    pub fn recent_input(&self, viewport: ViewportId) -> Vec<UnifiedInputEvent> {
        self.core
            .passes
            .lock()
            .get(&viewport)
            .map_or_default(|st| st.history.iter().copied().collect())
    }

    fn begin_pass(&self, viewport: ViewportId) {
        let core = &self.core;
        let ctx = &core.ctx;
        let collect = self.collect_semantics();
        let wanted = core.passes.with(viewport, |st| {
            st.agent.begin_pass();
            std::mem::take(&mut st.semantics_requested) || st.agent.wants_tree() || collect
        });
        semantic::begin_pass(ctx, wanted);
        shortcut::begin_pass(core);
        focus::track_modality(ctx);
        let keep = self.input_history();
        // The adapter reads `ctx.input`, which must not happen under the passes lock.
        let mut adapter = core.passes.with(viewport, |st| std::mem::take(&mut st.adapter));
        let (input, repaint_after) = adapter.process(ctx, &core.policies.gestures);
        core.passes.with(viewport, |st| {
            st.adapter = adapter;
            st.input = input;
            if keep > 0 {
                st.history.extend(st.input.events.iter().copied());
                let excess = st.history.len().saturating_sub(keep);
                st.history.drain(..excess);
            }
        });
        if let Some(after) = repaint_after {
            ctx.request_repaint_after(after);
        }
    }

    fn end_pass(&self, viewport: ViewportId) {
        let tree = semantic::end_pass(&self.core.ctx).map(Arc::new);
        // A discarded pass changed what it showed: its tree and the previous one are stale, and the rerun
        // (whose input hook runs too) collects again for the queued requests.
        let discard = self.core.ctx.will_discard();
        let pending = self.core.passes.with(viewport, |st| {
            if discard {
                st.tree = None;
            } else {
                if let Some(tree) = &tree {
                    st.agent.respond(tree);
                }
                st.tree = tree;
            }
            st.agent.wants_tree()
        });
        // Acts queued during a pass that had no tree are injected by the next one.
        if pending {
            self.core.ctx.request_repaint_of(viewport);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::panic::{AssertUnwindSafe, catch_unwind};

    use super::*;
    use crate::test_support::{ctx, input, run, runtime};

    #[test]
    fn theme_follows_the_preference() {
        let ctx = ctx();
        let rt = runtime(&ctx);
        assert_eq!(rt.core().preference(), ThemePreference::System);
        let fallback = ctx.options(|o| o.fallback_theme);
        assert_eq!(rt.theme(), ctx.system_theme().unwrap_or(fallback));
        rt.core().set_preference(ThemePreference::Dark);
        assert_eq!(rt.theme(), Theme::Dark);
    }

    struct StyledKit;

    impl Stylekit for StyledKit {
        fn policies(&self, _: &Context) -> Policies {
            crate::test_support::policies()
        }

        fn catalogs(&self) -> &'static [Catalog] {
            &[]
        }

        fn setup(&self, _: &Context) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
            Ok(())
        }

        fn style(&self, ctx: &Context, theme: Theme) -> Arc<Style> {
            let mut style = (*ctx.style_of(theme)).clone();
            style.spacing.item_spacing.x = 37.0;
            Arc::new(style)
        }
    }

    fn styled(ctx: &Context) -> Arc<Runtime<StyledKit>> {
        Runtime::new(ctx, StyledKit).expect("runtime")
    }

    fn themes_sent(out: &egui::FullOutput) -> Vec<egui::SystemTheme> {
        out.viewport_output
            .values()
            .flat_map(|v| &v.commands)
            .filter_map(|c| match c {
                egui::ViewportCommand::SetTheme(t) => Some(*t),
                _ => None,
            })
            .collect()
    }

    fn pass_once(ctx: &Context, rt: &Runtime<StyledKit>) -> egui::FullOutput {
        run(ctx, input(vec![]), |ui| rt.pass(ui, |_| {}))
    }

    #[test]
    fn kit_global_style_writes_style_and_theme_once() {
        let ctx = ctx();
        let rt = styled(&ctx);
        assert_eq!(rt.core().global_style(), GlobalStyle::Egui);
        pass_once(&ctx, &rt);
        assert_ne!(
            ctx.global_style().spacing.item_spacing.x,
            37.0,
            "default leaves egui alone"
        );
        rt.core().set_global_style(GlobalStyle::Kit);
        rt.core().set_preference(ThemePreference::Dark);
        let out = pass_once(&ctx, &rt);
        assert_eq!(ctx.style_of(Theme::Dark).spacing.item_spacing.x, 37.0);
        assert_eq!(ctx.options(|o| o.theme_preference), ThemePreference::Dark);
        assert_eq!(themes_sent(&out), [egui::SystemTheme::Dark]);
        let written = ctx.style_of(Theme::Dark);
        let out = pass_once(&ctx, &rt);
        assert!(Arc::ptr_eq(&written, &ctx.style_of(Theme::Dark)), "style rewritten");
        assert!(themes_sent(&out).is_empty(), "theme resent");
        rt.core().set_preference(ThemePreference::Light);
        let out = pass_once(&ctx, &rt);
        assert_eq!(themes_sent(&out), [egui::SystemTheme::Light]);
        assert_eq!(ctx.style_of(Theme::Light).spacing.item_spacing.x, 37.0);
    }

    #[test]
    fn kit_global_style_follows_an_os_flip_under_system() {
        let ctx = ctx();
        let rt = styled(&ctx);
        rt.core().set_global_style(GlobalStyle::Kit);
        let with_os = |theme| {
            let mut raw = input(vec![]);
            raw.system_theme = Some(theme);
            run(&ctx, raw, |ui| rt.pass(ui, |_| {}))
        };
        with_os(Theme::Light);
        assert_eq!(ctx.style_of(Theme::Light).spacing.item_spacing.x, 37.0);
        assert_ne!(ctx.style_of(Theme::Dark).spacing.item_spacing.x, 37.0);
        let out = with_os(Theme::Dark);
        assert_eq!(rt.theme(), Theme::Dark);
        assert_eq!(
            ctx.style_of(Theme::Dark).spacing.item_spacing.x,
            37.0,
            "flip restyles the new theme"
        );
        assert!(
            themes_sent(&out).is_empty(),
            "System stays SystemDefault, the window follows the OS"
        );
    }

    #[test]
    fn nested_pass_leaves_globals_alone() {
        let ctx = ctx();
        let host = runtime(&ctx);
        let inner = styled(&ctx);
        inner.core().set_global_style(GlobalStyle::Kit);
        run(&ctx, input(vec![]), |ui| host.pass(ui, |ui| inner.pass(ui, |_| {})));
        assert_ne!(ctx.global_style().spacing.item_spacing.x, 37.0);
    }

    #[test]
    fn unwinding_add_leaves_the_pass() {
        let ctx = ctx();
        let rt = runtime(&ctx);
        run(&ctx, input(vec![]), |ui| {
            let viewport = ui.ctx().viewport_id();
            let unwound = catch_unwind(AssertUnwindSafe(|| rt.pass(ui, |_| panic!("app"))));
            assert!(unwound.is_err());
            assert!(!rt.core.passes.with(viewport, |st| st.active), "pass left active");
            let current = ui
                .ctx()
                .data_mut(|d| d.get_temp_mut_or_default::<Owner>(owner_id(viewport)).current);
            assert_eq!(current, None, "owner left set");
        });
    }

    #[test]
    fn closed_viewport_state_is_pruned() {
        let ctx = ctx();
        let rt = runtime(&ctx);
        let child = ViewportId::from_hash_of("child");
        let raw = |current: ViewportId, live: &[ViewportId]| RawInput {
            viewport_id: current,
            viewports: live.iter().map(|id| (*id, egui::ViewportInfo::default())).collect(),
            ..RawInput::default()
        };
        let live = [ViewportId::ROOT, child];
        run(&ctx, raw(child, &live), |ui| rt.pass(ui, |_| {}));
        assert!(rt.core.passes.lock().contains_key(&child));
        run(&ctx, raw(ViewportId::ROOT, &live[..1]), |ui| rt.pass(ui, |_| {}));
        assert!(!rt.core.passes.lock().contains_key(&child));
    }
}
