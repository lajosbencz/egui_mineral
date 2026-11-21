//! Component gallery and e2e target for `egui_mineral_material` (native, and wasm via trunk); F12 opens the inspector.

mod pages;
mod spawn;

use std::{sync::Arc, time::Duration};

use egui::{CentralPanel, Key, KeyboardShortcut, Modifiers, Panel, scroll_area::ScrollAreaOutput};
use egui_mineral::{
    bus::{Bus, SpawnerExt},
    i18n::Direction,
    nav::Router,
    runtime::Runtime,
    shortcut,
};
use egui_mineral_eframe::{EframeApp, MineralApp};
pub(crate) use egui_mineral_material::{Material, Rt};
use egui_mineral_material::{adaptive::WindowClass, color::SchemeSpec, fonts::IconFonts, prelude::*, scroll};
use futures_util::StreamExt;
use mineral_inspector::{Inspector, Serde};
use pages::{Cmd, Evt, LOCALES, Page, Settings, Shared, States, THEMES, VARIANTS, snack};
use spawn::AppSpawner;

const CONTENT_MAX_W: f32 = 1040.0;
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, egui_mineral::semantic::KeySalt)]
enum BarAction {
    Search,
    Theme,
}

const PALETTE: KeyboardShortcut = KeyboardShortcut::new(Modifiers::COMMAND, Key::K);
const INSPECTOR: KeyboardShortcut = KeyboardShortcut::new(Modifiers::NONE, Key::F12);
// Subsets written by build.rs.
const ICON_FONTS: IconFonts = IconFonts::new(
    include_bytes!(concat!(env!("OUT_DIR"), "/MaterialSymbolsRounded_Filled-Regular.ttf")),
    include_bytes!(concat!(env!("OUT_DIR"), "/MaterialSymbolsRounded-Regular.ttf")),
);

type App = EframeApp<Material, Shell>;

struct Shell {
    router: Router<Page>,
    inspector: Inspector,
    shared: Shared,
    pages: States,
    applied: Option<Settings>,
    drawer_open: bool,
    rail_expanded: bool,
    palette_open: bool,
    palette_query: String,
    palette_pages: Vec<(Page, PaletteCommand)>,
    palette_commands: Vec<PaletteCommand>,
    #[cfg(not(target_arch = "wasm32"))]
    _executor: tokio::runtime::Runtime,
}

fn new_app(ctx: &egui::Context) -> Result<App, Box<dyn std::error::Error + Send + Sync>> {
    let rt = Runtime::new(ctx, Material::from_icon_fonts(ICON_FONTS.fonts()))?;
    #[cfg(not(target_arch = "wasm32"))]
    let executor = tokio::runtime::Builder::new_multi_thread().enable_time().build()?;
    #[cfg(not(target_arch = "wasm32"))]
    let spawner = Arc::new(AppSpawner::new(executor.handle().clone()));
    #[cfg(target_arch = "wasm32")]
    let spawner = Arc::new(AppSpawner::new());
    // The navigation page binds Cmd+Q and Cmd+/- as menu commands.
    ctx.options_mut(|o| {
        o.zoom_with_keyboard = false;
        o.quit_shortcuts.clear();
    });
    let (bus, mut handle) = Bus::<Cmd, Evt>::new(ctx);
    spawner.spawn_future(async move {
        while let Some(cmd) = handle.commands.next().await {
            match cmd {
                Cmd::Work { id, millis } => {
                    spawn::sleep(Duration::from_millis(millis)).await;
                    if handle.events.send(Evt::Done { id }).is_err() {
                        break;
                    }
                }
            }
        }
    });
    // A page's topics report the page.
    let palette_pages: Vec<(Page, PaletteCommand)> = Page::ALL
        .iter()
        .flat_map(|&p| {
            std::iter::once(PaletteCommand::new(p.title()).icon(p.icon()).group("Pages"))
                .chain(p.topics().iter().map(move |t| PaletteCommand::new(*t).group(p.title())))
                .map(move |c| (p, c))
        })
        .collect();
    // Same commands and shortcuts as the View menu on the navigation page.
    let palette_commands = [pages::navigation::ZOOM_IN, pages::navigation::ZOOM_OUT]
        .map(|i| {
            let (id, key, label) = &pages::navigation::COMMANDS[i];
            PaletteCommand::new(*label).group("View").command(id, *key)
        })
        .to_vec();
    let shell = Shell {
        router: Router::new(Page::Foundations),
        inspector: Inspector::default(),
        shared: Shared {
            settings: Settings::default(),
            toasts: egui_mineral_material::communication::snackbar_queue(),
            bus,
            done: Vec::new(),
            platform: Vec::new(),
            spawner,
        },
        pages: States::default(),
        applied: None,
        drawer_open: false,
        rail_expanded: true,
        palette_open: false,
        palette_query: String::new(),
        palette_pages,
        palette_commands,
        #[cfg(not(target_arch = "wasm32"))]
        _executor: executor,
    };
    Ok(EframeApp::new(rt, shell))
}

impl Shell {
    fn apply_settings(&mut self, rt: &Rt) {
        let ctx = rt.ctx();
        let s = &self.shared.settings;
        if self.applied.as_ref() == Some(s) {
            return;
        }
        let [r, g, b] = s.seed;
        let spec = SchemeSpec::new(egui::Color32::from_rgb(r, g, b))
            .contrast(if s.high_contrast {
                Contrast::HIGH
            } else {
                Contrast::STANDARD
            })
            .variant(VARIANTS[s.variant].0);
        rt.kit().apply(ctx, spec);
        rt.core().set_preference(THEMES[s.theme].0);
        rt.core().set_global_style(if s.global_kit {
            egui_mineral::runtime::GlobalStyle::Kit
        } else {
            egui_mineral::runtime::GlobalStyle::Egui
        });
        rt.kit().set_density(rt.ctx(), Density::new(s.density));
        if self.applied.as_ref().is_none_or(|a| a.locale != s.locale) {
            rt.i18n()
                .set_locale(&LOCALES[s.locale])
                .expect("LOCALES have formatting data");
        }
        self.applied = Some(s.clone());
    }

    fn navigate(&mut self, page: Page) {
        if *self.router.current() != page {
            self.router.reset_to(page);
        }
    }

    fn top_bar(&mut self, ui: &mut MaterialUi<'_>, compact: bool) {
        let page = *self.router.current();
        let mut bar = TopAppBar::new("top-bar", page.title());
        if compact {
            bar = bar.navigation(icons::ICON_MENU, "Open navigation");
        }
        let search = format!("Search ({})", shortcut::label(ui.ctx(), &PALETTE));
        let (preference, theme_name) = THEMES[self.shared.settings.theme];
        let theme_icon = match preference {
            egui::ThemePreference::System => icons::ICON_BRIGHTNESS_AUTO,
            egui::ThemePreference::Light => icons::ICON_LIGHT_MODE,
            egui::ThemePreference::Dark => icons::ICON_DARK_MODE,
        };
        let bar = bar
            .action(BarAction::Search, IconAction::new(icons::ICON_SEARCH, search))
            .action(
                BarAction::Theme,
                IconAction::new(theme_icon, format!("Theme: {theme_name}")),
            );
        let r = bar.show(ui);
        if r.leading_clicked {
            self.drawer_open = true;
        }
        match r.clicked {
            Some(BarAction::Search) => self.palette_open = true,
            Some(BarAction::Theme) => self.shared.settings.theme = (self.shared.settings.theme + 1) % THEMES.len(),
            None => {}
        }
    }

    fn rail(&mut self, ui: &mut MaterialUi<'_>, class: WindowClass) {
        let mut selected = *self.router.current();
        let rail = Page::ALL
            .iter()
            .fold(NavigationRail::new("rail", &mut selected).menu_button(true), |r, p| {
                r.item(*p, NavDestination::new(p.icon(), p.title()))
            });
        let r = if class >= WindowClass::Expanded {
            rail.expanded(self.rail_expanded).show(ui)
        } else {
            rail.show_modal(ui, &mut self.rail_expanded)
        };
        if r.leading_clicked && class >= WindowClass::Expanded {
            self.rail_expanded = !self.rail_expanded;
        }
        if let Some(page) = r.selection.selected.filter(|_| r.selection.changed) {
            self.navigate(page);
        }
    }

    fn drawer(&mut self, rt: &Rt) {
        let mut selected = *self.router.current();
        let drawer = Page::ALL.iter().fold(
            NavigationDrawer::new("drawer", &mut selected).section("egui_mineral"),
            |d, p| d.item(*p, NavDestination::new(p.icon(), p.title())),
        );
        if let Some(page) = drawer
            .show_modal(rt, &mut self.drawer_open)
            .selection
            .and_then(|s| s.selected)
        {
            self.navigate(page);
        }
    }
}

impl Shell {
    fn page(&mut self, ui: &mut MaterialUi<'_>) -> ScrollAreaOutput<()> {
        let rt = ui.rt();
        let ctx = rt.ctx();
        self.inspector.toggle_on(ctx, INSPECTOR);
        self.apply_settings(rt);
        let events: Vec<Evt> = self.shared.bus.drain().collect();
        for e in events {
            match e {
                Evt::Done { id } => {
                    self.shared.done.push(id);
                    self.shared.toast(format!("Job {id} finished"));
                }
                Evt::Platform(r) => self.shared.platform.push(r),
            }
        }

        self.body(ui)
    }

    fn body(&mut self, ui: &mut MaterialUi<'_>) -> ScrollAreaOutput<()> {
        let rt = ui.rt();
        pages::navigation::run_commands(rt, &mut self.pages.navigation, &mut self.shared);
        let class = WindowClass::of(rt.core());
        let compact = class == WindowClass::Compact;
        if !compact {
            let panel = match rt.i18n().direction() {
                Direction::Ltr => Panel::left("rail"),
                Direction::Rtl => Panel::right("rail"),
            };
            panel
                .frame(egui::Frame::NONE)
                .resizable(false)
                .show_separator_line(false)
                .show(ui, |ui| self.rail(&mut rt.ui(ui), class));
        }
        Panel::top("top-bar")
            .frame(egui::Frame::NONE)
            .show_separator_line(false)
            .show(ui, |ui| self.top_bar(&mut rt.ui(ui), compact));
        let page = *self.router.current();
        // The page bar reaches the window edge; the panel margin pads the content inside the scroll area.
        let panel = egui::Frame::central_panel(ui.style());
        let margin = panel.inner_margin;
        let page_out = CentralPanel::default().frame(panel.inner_margin(0)).show(ui, |ui| {
            scroll::page(&mut rt.ui(ui), ("page", page.index()), |ui| {
                egui::Frame::NONE.inner_margin(margin).show(ui, |ui| {
                    let w = ui.available_width().min(CONTENT_MAX_W);
                    let pad = (ui.available_width() - w) / 2.0;
                    ui.horizontal(|ui| {
                        ui.add_space(pad);
                        ui.vertical(|ui| {
                            ui.set_width(w);
                            pages::show(page, &mut rt.ui(ui), &mut self.pages, &mut self.shared);
                        });
                    });
                });
            })
        });
        if compact {
            self.drawer(rt);
        }

        let picked = CommandPalette::new("palette", &mut self.palette_query)
            .actions(self.palette_pages.iter().cloned())
            .commands(self.palette_commands.iter().cloned())
            .placeholder("Jump to a page or component")
            .shortcut(PALETTE)
            .show_modal(rt, &mut self.palette_open)
            .clicked;
        if let Some(page) = picked {
            self.navigate(page);
        }
        SnackbarHost::new(&mut self.shared.toasts).show(rt);
        let ctx = rt.ctx();
        if shortcut::take_action(ctx, &snack::UNDO) {
            self.shared.toast("Conversation restored");
        }
        if shortcut::take_action(ctx, &snack::OPEN) {
            self.shared.toast("Draft opened");
        }
        if shortcut::take_action(ctx, &snack::RETRY) {
            self.shared.toast("Retrying connection");
        }
        page_out.inner
    }
}

impl MineralApp<Material> for Shell {
    fn ui(&mut self, ui: &mut MaterialUi<'_>, frame: &mut eframe::Frame) {
        self.inspector
            .set_cpu_time(frame.info().cpu_usage.map(Duration::from_secs_f32));
        self.page(ui);
    }

    fn after_pass(&mut self, rt: &Rt) {
        self.inspector
            .show_with_state(rt, &mut Serde(&mut self.shared.settings));
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn main() -> eframe::Result {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();
    std::panic::set_hook(Box::new(|info| {
        let backtrace = std::backtrace::Backtrace::capture();
        tracing::error!(%backtrace, "{info}");
    }));
    eframe::run_native(
        "egui_mineral showcase",
        eframe::NativeOptions::default(),
        Box::new(|cc| Ok(Box::new(new_app(&cc.egui_ctx)?))),
    )
}

#[cfg(target_arch = "wasm32")]
fn main() {
    use tracing_subscriber::{filter::LevelFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt};

    tracing_subscriber::registry()
        .with(LevelFilter::INFO)
        .with(
            fmt::layer()
                .with_ansi(false)
                .without_time()
                .with_writer(tracing_web::MakeWebConsoleWriter::new()),
        )
        .init();
    wasm_bindgen_futures::spawn_local(async {
        if let Err(e) = web::start().await {
            tracing::error!(error = %e, "showcase failed to start");
            web::report(&format!("Failed to start: {e}"));
        }
    });
}

#[cfg(target_arch = "wasm32")]
mod web {
    use eframe::{WebOptions, WebRunner};
    use web_sys::{
        HtmlCanvasElement,
        wasm_bindgen::{JsCast, JsValue},
    };

    use super::new_app;

    const CANVAS_ID: &str = "showcase-canvas";
    const STATUS_ID: &str = "showcase-status";

    fn status() -> Option<web_sys::Element> {
        web_sys::window()?.document()?.get_element_by_id(STATUS_ID)
    }

    pub(super) async fn start() -> Result<(), String> {
        let canvas = web_sys::window()
            .and_then(|w| w.document())
            .and_then(|d| d.get_element_by_id(CANVAS_ID))
            .ok_or_else(|| format!("no #{CANVAS_ID} element"))?
            .dyn_into::<HtmlCanvasElement>()
            .map_err(|_| format!("#{CANVAS_ID} is not a canvas"))?;
        let runner = WebRunner::new();
        // Chains eframe's hook, which logs and stops the runner.
        let eframe_hook = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            eframe_hook(info);
            report(&format!("Crashed: {info}"));
        }));
        runner
            .start(
                canvas,
                WebOptions::default(),
                Box::new(|cc| Ok(Box::new(new_app(&cc.egui_ctx)?))),
            )
            .await
            .map_err(|e: JsValue| format!("{e:?}"))?;
        if let Some(status) = status() {
            status.set_text_content(None);
        }
        Ok(())
    }

    pub(super) fn report(msg: &str) {
        if let Some(status) = status() {
            status.set_text_content(Some(msg));
            if let Err(e) = status.set_attribute("data-failed", "") {
                tracing::warn!(error = ?e, "marking #{STATUS_ID} failed");
            }
        } else {
            tracing::warn!("no #{STATUS_ID} element to report into");
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{
        collections::{BTreeSet, HashSet},
        time::Instant,
    };

    use egui::RawInput;
    use egui_mineral::{
        agent::{AgentAction, AgentError},
        input::synth,
    };

    use super::*;

    const FRAME_BUDGET: usize = 20;
    const AGENT_DEADLINE: Duration = Duration::from_secs(5);
    // Page overflow settles once its scroll bar appears.
    const SETTLE: usize = 8;

    fn screen() -> RawInput {
        RawInput {
            screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1280.0, 900.0))),
            ..RawInput::default()
        }
    }

    #[test]
    fn rail_click_navigates() {
        let ctx = egui::Context::default();
        let mut app = new_app(&ctx).expect("showcase app");
        app.rt().core().motion().set_reduced(true);
        let agent = app.rt().agent(egui::ViewportId::ROOT);
        let target = Page::ALL[2];
        assert_ne!(*app.app().router.current(), target);
        let session = std::thread::spawn(move || {
            futures_executor::block_on(async {
                agent.snapshot().await?;
                agent.act("rail-communication-nav-item", AgentAction::Click).await
            })
        });
        let deadline = Instant::now() + AGENT_DEADLINE;
        while !session.is_finished() && Instant::now() < deadline {
            step(&mut app, &ctx, Vec::new());
            std::thread::sleep(Duration::from_millis(2));
        }
        assert!(
            session.is_finished(),
            "agent session still running after {AGENT_DEADLINE:?}"
        );
        session
            .join()
            .expect("agent thread")
            .unwrap_or_else(|e: AgentError| panic!("{e}"));
        settle(&mut app, &ctx);
        assert_eq!(*app.app().router.current(), target);
    }

    fn show_page(app: &mut App, ui: &mut egui::Ui) -> ScrollAreaOutput<()> {
        let rt = Arc::clone(app.rt());
        let page = rt.pass(ui, |ui| app.app_mut().page(ui));
        app.app_mut().after_pass(&rt);
        page
    }

    fn step(app: &mut App, ctx: &egui::Context, events: Vec<egui::Event>) -> Vec<egui::ViewportCommand> {
        let mut raw = screen();
        raw.events = events;
        let mut out = ctx.run_ui(raw, |ui| {
            eframe::App::ui(app, ui, &mut eframe::Frame::_new_kittest());
        });
        out.textures_delta.clear();
        out.viewport_output
            .remove(&egui::ViewportId::ROOT)
            .map_or_default(|v| v.commands)
    }

    fn settle(app: &mut App, ctx: &egui::Context) {
        for _ in 0..FRAME_BUDGET {
            step(app, ctx, Vec::new());
            if !ctx.requested_repaint_last_pass() {
                return;
            }
        }
        panic!("no settled frame within {FRAME_BUDGET} frames");
    }

    fn press(key: Key, modifiers: Modifiers) -> Vec<egui::Event> {
        synth::key_tap(key, modifiers).into_iter().collect()
    }

    fn step_until(app: &mut App, ctx: &egui::Context, what: &str, done: impl Fn(&App) -> bool) {
        for _ in 0..FRAME_BUDGET {
            if done(app) {
                return;
            }
            step(app, ctx, Vec::new());
        }
        assert!(done(app), "{what} did not happen within {FRAME_BUDGET} frames");
    }

    fn menu_item_open(app: &App, key: &str) -> bool {
        app.rt()
            .tree(egui::ViewportId::ROOT)
            .expect("semantic tree collected")
            .find_key(key)
            .is_some()
    }

    fn navigation_app() -> (egui::Context, App) {
        let ctx = egui::Context::default();
        // Material enables mnemonics off Apple platforms.
        ctx.set_os(egui::os::OperatingSystem::Nix);
        let mut app = new_app(&ctx).expect("showcase app");
        app.rt().core().motion().set_reduced(true);
        app.rt().set_collect_semantics(true);
        app.app_mut().navigate(Page::Navigation);
        settle(&mut app, &ctx);
        (ctx, app)
    }

    #[test]
    fn alt_mnemonic_opens_menu_then_bare_letter_activates_item() {
        let (ctx, mut app) = navigation_app();
        assert!(!menu_item_open(&app, "nav-mb-file-file-new-menu-item"));
        step(&mut app, &ctx, press(Key::F, Modifiers::ALT));
        step_until(&mut app, &ctx, "File menu opening", |a| {
            menu_item_open(a, "nav-mb-file-file-new-menu-item")
        });
        settle(&mut app, &ctx);
        assert!(
            shortcut::mnemonics_visible(app.rt().core()),
            "keyboard-opened menu reveals mnemonics"
        );
        let new = app
            .rt()
            .tree(egui::ViewportId::ROOT)
            .and_then(|t| t.find_key("nav-mb-file-file-new-menu-item").cloned())
            .expect("menu item in tree");
        assert_eq!(new.access_key.map(shortcut::Mnemonic::char), Some('N'));
        assert_eq!(new.shortcut, Some(KeyboardShortcut::new(Modifiers::COMMAND, Key::N)));

        step(&mut app, &ctx, press(Key::N, Modifiers::NONE));
        step_until(&mut app, &ctx, "New activating", |a| {
            a.app().pages.navigation.last_action == "Command: New"
        });
        settle(&mut app, &ctx);
        assert!(
            !menu_item_open(&app, "nav-mb-file-file-new-menu-item"),
            "menu closes after activation"
        );
        assert!(!shortcut::mnemonics_visible(app.rt().core()));
    }

    #[test]
    fn menu_and_palette_share_a_command_that_fires_once() {
        let (ctx, mut app) = navigation_app();
        let command = Modifiers::COMMAND | Modifiers::CTRL;
        step(&mut app, &ctx, press(Key::Plus, command));
        step_until(&mut app, &ctx, "Zoom in firing", |a| {
            a.app().pages.navigation.last_action == "Command: Zoom in"
        });
        assert!(!menu_item_open(&app, "nav-mb-view-0-menu-item"));
        assert_eq!(app.app().pages.navigation.zoom, 110);
    }

    #[test]
    fn quit_shortcut_closes_through_the_app_command() {
        let (ctx, mut app) = navigation_app();
        assert!(!ctx.options(|o| o.zoom_with_keyboard) && ctx.options(|o| o.quit_shortcuts.is_empty()));
        let mut closed = false;
        let mut frames = vec![step(
            &mut app,
            &ctx,
            press(Key::Q, Modifiers::COMMAND | Modifiers::CTRL),
        )];
        for _ in 0..FRAME_BUDGET {
            frames.push(step(&mut app, &ctx, Vec::new()));
        }
        for cmd in frames.iter().flatten() {
            closed |= matches!(cmd, egui::ViewportCommand::Close);
        }
        assert!(closed, "Quit command did not close the viewport");
        assert_eq!(app.app().pages.navigation.last_action, "Command: Quit");
    }

    macro_rules! every_page_holds_up {
        ($($test:ident: $page:ident),* $(,)?) => {
            mod every_page_holds_up {
                // Exhaustive: a new page fails to compile until it has a test.
                const _: fn(super::Page) = |page| match page {
                    $(super::Page::$page => ()),*
                };
                $(
                    #[test]
                    fn $test() {
                        super::holds_up(super::Page::$page);
                    }
                )*
            }
        };
    }

    every_page_holds_up! {
        foundations: Foundations,
        actions: Actions,
        communication: Communication,
        containment: Containment,
        navigation: Navigation,
        selection: Selection,
        text_inputs: TextInputs,
        data: Data,
        system: System,
        interop: Interop,
    }

    fn holds_up(page: Page) {
        use std::panic::{self, AssertUnwindSafe};

        // Narrowest phone in portrait (dp); the other window classes start at their breakpoints.
        const COMPACT_MIN: f32 = 320.0;
        const DEGENERATE: [egui::Vec2; 4] = [
            egui::Vec2::ZERO,
            egui::vec2(1.0, 1.0),
            egui::vec2(40.0, 40.0),
            egui::vec2(4000.0, 3000.0),
        ];
        let rtl = LOCALES
            .iter()
            .position(|l| *l == egui_mineral::i18n::locale!("ar-EG"))
            .expect("an RTL locale");
        let fresh = |locale, density, zoom| {
            let ctx = egui::Context::default();
            ctx.set_zoom_factor(zoom);
            let mut app = new_app(&ctx).expect("showcase app");
            app.rt().set_collect_semantics(true);
            app.app_mut().shared.settings.locale = locale;
            app.app_mut().shared.settings.density = density;
            (ctx, app)
        };
        let classes: Vec<egui::Vec2> = std::iter::once(COMPACT_MIN)
            .chain(fresh(0, 0, 1.0).1.rt().core().policies().breakpoints.iter().copied())
            .map(|w| egui::vec2(w, 900.0))
            .collect();
        let mut failed = Vec::new();
        let configs = [(0, 0, 1.0), (rtl, -6, 0.5), (0, -3, 3.0), (rtl, 0, 3.0)];
        for (i, (locale, density, zoom)) in configs.into_iter().enumerate() {
            let (mut ctx, mut app) = fresh(locale, density, zoom);
            // Every size in the plain config; the others take the degenerate sizes once and the extreme classes.
            let degenerate = if i < 2 { &DEGENERATE[..] } else { &[] };
            let extremes = [classes.first(), classes.last()];
            let sized: Vec<egui::Vec2> = if i == 0 {
                classes.clone()
            } else {
                extremes.into_iter().flatten().copied().collect()
            };
            for size in degenerate.iter().copied().chain(sized) {
                let width = i == 0 && classes.contains(&size);
                let case = format!("{size:?} locale={locale} density={density} zoom={zoom}");
                let run = panic::catch_unwind(AssertUnwindSafe(|| check(&ctx, &mut app, page, size, width)));
                if let Ok(problems) = run {
                    failed.extend(problems.into_iter().map(|p| format!("{case}: {p}")));
                } else {
                    failed.push(format!("{case}: panicked"));
                    (ctx, app) = fresh(locale, density, zoom);
                }
            }
        }
        assert!(
            failed.is_empty(),
            "{} failures (panic messages above):\n{}",
            failed.len(),
            failed.join("\n")
        );
    }

    fn check(ctx: &egui::Context, app: &mut App, page: Page, size: egui::Vec2, width: bool) -> BTreeSet<String> {
        app.app_mut().navigate(page);
        let mut problems = BTreeSet::new();
        let mut over = 0.0;
        for _ in 0..if width { SETTLE } else { 2 } {
            let raw = RawInput {
                screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, size)),
                ..RawInput::default()
            };
            let mut out = ctx.run_ui(raw, |ui| {
                let page = show_page(app, ui);
                over = page.content_size.x - page.inner_rect.width();
            });
            out.textures_delta.clear();
            // egui paints id clashes as debug text (`Options::warn_on_id_clash`, on in debug builds):
            // "First use of {what} ID", "Second use of ...", "Double use of ...".
            problems.extend(out.shapes.iter().filter_map(|c| match &c.shape {
                egui::Shape::Text(t) if t.galley.text().contains(" use of ") => {
                    Some(format!("id clash: {}", t.galley.text()))
                }
                _ => None,
            }));
        }
        if width && over > 0.5 {
            problems.insert(format!("{over:.1} px too wide"));
        }
        let tree = app.rt().tree(egui::ViewportId::ROOT).expect("semantic tree collected");
        let mut seen = HashSet::new();
        problems.extend(
            tree.nodes
                .iter()
                .filter(|n| !seen.insert(n.id.key()))
                .map(|n| format!("duplicate agent key {:?}", n.id.key())),
        );
        problems
    }
}
