//! Component gallery and e2e target for `egui_mineral_material` (native, and wasm via trunk); F12 opens the inspector.

mod pages;
mod spawn;

use std::{sync::Arc, time::Duration};

use egui::{CentralPanel, Key, KeyboardShortcut, Modifiers, Panel, Ui};
use egui_mineral::{
    bus::{Bus, SpawnerExt},
    i18n::Direction,
    nav::Router,
    runtime::{InitError, Runtime},
    shortcut,
};
pub(crate) use egui_mineral_material::{Material, Rt};
use egui_mineral_material::{adaptive::WindowClass, color::SchemeSpec, prelude::*, scroll};
use futures_util::StreamExt;
use mineral_inspector::{Inspector, Serde};
use pages::{Cmd, Evt, LOCALES, Page, Settings, Shared, States, VARIANTS, snack};
use spawn::AppSpawner;

const CONTENT_MAX_W: f32 = 1040.0;
const PALETTE: KeyboardShortcut = KeyboardShortcut::new(Modifiers::COMMAND, Key::K);
const INSPECTOR: KeyboardShortcut = KeyboardShortcut::new(Modifiers::NONE, Key::F12);

pub(crate) type Mui<'a> = MineralUi<'a, Material>;

struct App {
    rt: Arc<Rt>,
    router: Router<Page>,
    inspector: Inspector,
    shared: Shared,
    pages: States,
    applied: Option<Settings>,
    drawer_open: bool,
    rail_expanded: bool,
    palette_open: bool,
    palette_query: String,
    commands: Vec<PaletteCommand>,
    targets: Vec<Page>,
    spawner: Arc<AppSpawner>,
}

impl App {
    fn new(ctx: &egui::Context) -> Result<Self, InitError> {
        let rt = Runtime::new(ctx, Material::new(egui::Theme::Light))?;
        // The navigation page binds Cmd+Q and Cmd+/- as menu commands.
        ctx.options_mut(|o| {
            o.zoom_with_keyboard = false;
            o.quit_shortcuts.clear();
        });
        let (bus, mut handle) = Bus::<Cmd, Evt>::new(ctx);
        let spawner = Arc::new(AppSpawner::default());
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
        let (mut commands, targets): (Vec<PaletteCommand>, Vec<Page>) = Page::ALL
            .iter()
            .flat_map(|&p| {
                std::iter::once((PaletteCommand::new(p.title()).icon(p.icon()).group("Pages"), p)).chain(
                    p.topics()
                        .iter()
                        .map(move |t| (PaletteCommand::new(*t).group(p.title()), p)),
                )
            })
            .unzip();
        // Same commands and shortcuts as the View menu on the navigation page; entries past `targets` never click.
        for i in [pages::navigation::ZOOM_IN, pages::navigation::ZOOM_OUT] {
            let (id, key, label) = &pages::navigation::COMMANDS[i];
            commands.push(PaletteCommand::new(*label).group("View").command(id, *key));
        }
        Ok(Self {
            rt,
            router: Router::new(Page::Foundations),
            inspector: Inspector::default(),
            shared: Shared {
                settings: Settings::default(),
                toasts: egui_mineral_material::communication::snackbar_queue(),
                bus,
                done: Vec::new(),
                platform: Vec::new(),
                spawner: spawner.clone(),
            },
            pages: States::default(),
            applied: None,
            drawer_open: false,
            rail_expanded: true,
            palette_open: false,
            palette_query: String::new(),
            commands,
            targets,
            spawner,
        })
    }

    fn apply_settings(&mut self) {
        let rt = Arc::clone(&self.rt);
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
        rt.kit().set_preference(
            ctx,
            if s.dark {
                egui::ThemePreference::Dark
            } else {
                egui::ThemePreference::Light
            },
        );
        rt.kit().set_density(Density::new(s.density));
        if self.applied.as_ref().is_none_or(|a| a.locale != s.locale) {
            rt.i18n().set_locale(LOCALES[s.locale]).expect("LOCALES are valid tags");
        }
        self.applied = Some(s.clone());
    }

    fn navigate(&mut self, page: Page) {
        if *self.router.current() != page {
            self.router.reset_to(page);
        }
    }

    fn top_bar(&mut self, ui: &mut Mui<'_>, compact: bool) {
        let page = *self.router.current();
        let mut bar = TopAppBar::new("top-bar", page.title());
        if compact {
            bar = bar.navigation(icons::ICON_MENU, "Open navigation");
        }
        let search = format!("Search ({})", shortcut::label(ui.ctx(), &PALETTE));
        let bar = bar
            .action(IconAction::new(icons::ICON_SEARCH, search))
            .action(IconAction::new(
                if self.shared.settings.dark {
                    icons::ICON_LIGHT_MODE
                } else {
                    icons::ICON_DARK_MODE
                },
                "Toggle theme",
            ));
        let r = bar.show(ui);
        if r.leading_clicked {
            self.drawer_open = true;
        }
        match r.clicked {
            Some(0) => self.palette_open = true,
            Some(1) => self.shared.settings.dark = !self.shared.settings.dark,
            _ => {}
        }
    }

    fn rail(&mut self, ui: &mut Mui<'_>, class: WindowClass) {
        let mut selected = self.router.current().index();
        let rail = Page::ALL
            .iter()
            .fold(NavigationRail::new("rail", &mut selected).menu_button(true), |r, p| {
                r.item(NavDestination::new(p.icon(), p.title()))
            });
        let r = if class >= WindowClass::Expanded {
            rail.expanded(self.rail_expanded).show(ui)
        } else {
            rail.show_modal(ui, &mut self.rail_expanded)
        };
        if r.leading_clicked && class >= WindowClass::Expanded {
            self.rail_expanded = !self.rail_expanded;
        }
        if let Some(i) = r.selection.selected.filter(|_| r.selection.changed) {
            self.navigate(Page::ALL[i]);
        }
    }

    fn drawer(&mut self, rt: &Rt) {
        let mut selected = self.router.current().index();
        let drawer = Page::ALL.iter().fold(
            NavigationDrawer::new("drawer", &mut selected).section("egui_mineral"),
            |d, p| d.item(NavDestination::new(p.icon(), p.title())),
        );
        if let Some(i) = drawer
            .show_modal(rt, &mut self.drawer_open)
            .selection
            .and_then(|s| s.selected)
        {
            self.navigate(Page::ALL[i]);
        }
    }
}

impl App {
    fn shell(&mut self, ui: &mut Ui) {
        let rt = Arc::clone(&self.rt);
        let ctx = rt.ctx();
        self.inspector.toggle_on(ctx, INSPECTOR);
        self.spawner.reap();
        self.apply_settings();
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

        rt.pass(ui, |ui| self.body(ui));
        self.inspector
            .show_with_state(&rt, &mut Serde(&mut self.shared.settings));
    }

    fn body(&mut self, ui: &mut Mui<'_>) {
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
        CentralPanel::default().show(ui, |ui| {
            scroll::page(&mut rt.ui(ui), ("page", page.index()), |ui| {
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
        });
        if compact {
            self.drawer(rt);
        }

        let picked = CommandPalette::new("palette", self.commands.iter().cloned(), &mut self.palette_query)
            .placeholder("Jump to a page or component")
            .shortcut(PALETTE)
            .show_modal(rt, &mut self.palette_open)
            .clicked;
        if let Some(i) = picked {
            self.navigate(self.targets[i]);
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
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut Ui, frame: &mut eframe::Frame) {
        self.inspector
            .set_cpu_time(frame.info().cpu_usage.map(std::time::Duration::from_secs_f32));
        self.shell(ui);
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn main() -> eframe::Result {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();
    eframe::run_native(
        "egui_mineral showcase",
        eframe::NativeOptions::default(),
        Box::new(|cc| Ok(Box::new(App::new(&cc.egui_ctx)?))),
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
            web::report(&e);
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

    use super::App;

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
        WebRunner::new()
            .start(
                canvas,
                WebOptions::default(),
                Box::new(|cc| Ok(Box::new(App::new(&cc.egui_ctx)?))),
            )
            .await
            .map_err(|e: JsValue| format!("{e:?}"))?;
        if let Some(status) = status() {
            status.remove();
        }
        Ok(())
    }

    pub(super) fn report(msg: &str) {
        if let Some(status) = status() {
            status.set_text_content(Some(&format!("Failed to start: {msg}")));
        } else {
            tracing::warn!("no #{STATUS_ID} element to report into");
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Instant;

    use egui::RawInput;
    use egui_mineral::{
        agent::{AgentAction, AgentError},
        input::synth,
    };

    use super::*;

    const FRAME_BUDGET: usize = 20;
    const AGENT_DEADLINE: Duration = Duration::from_secs(5);

    fn screen() -> RawInput {
        RawInput {
            screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1280.0, 900.0))),
            ..RawInput::default()
        }
    }

    #[test]
    fn rail_click_navigates() {
        let ctx = egui::Context::default();
        let mut app = App::new(&ctx).expect("showcase app");
        app.rt.core().motion().set_reduced(true);
        let agent = app.rt.agent(egui::ViewportId::ROOT);
        let target = Page::ALL[2];
        assert_ne!(*app.router.current(), target);
        let session = std::thread::spawn(move || {
            futures_executor::block_on(async {
                agent.snapshot().await?;
                agent.act("rail-2-nav-item", AgentAction::Click).await
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
        assert_eq!(*app.router.current(), target);
    }

    fn step(app: &mut App, ctx: &egui::Context, events: Vec<egui::Event>) -> Vec<egui::ViewportCommand> {
        let mut raw = screen();
        raw.events = events;
        let mut out = ctx.run_ui(raw, |ui| app.shell(ui));
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
        app.rt
            .tree(egui::ViewportId::ROOT)
            .expect("semantic tree collected")
            .find_key(key)
            .is_some()
    }

    fn navigation_app() -> (egui::Context, App) {
        let ctx = egui::Context::default();
        // Material enables mnemonics off Apple platforms.
        ctx.set_os(egui::os::OperatingSystem::Nix);
        let mut app = App::new(&ctx).expect("showcase app");
        app.rt.core().motion().set_reduced(true);
        app.rt.set_collect_semantics(true);
        app.navigate(Page::Navigation);
        settle(&mut app, &ctx);
        (ctx, app)
    }

    #[test]
    fn alt_mnemonic_opens_menu_then_bare_letter_activates_item() {
        let (ctx, mut app) = navigation_app();
        assert!(!menu_item_open(&app, "nav-mb-file-0-menu-item"));
        step(&mut app, &ctx, press(Key::F, Modifiers::ALT));
        step_until(&mut app, &ctx, "File menu opening", |a| {
            menu_item_open(a, "nav-mb-file-0-menu-item")
        });
        settle(&mut app, &ctx);
        assert!(
            shortcut::mnemonics_visible(app.rt.core()),
            "keyboard-opened menu reveals mnemonics"
        );
        let new = app
            .rt
            .tree(egui::ViewportId::ROOT)
            .and_then(|t| t.find_key("nav-mb-file-0-menu-item").cloned())
            .expect("menu item in tree");
        assert_eq!(new.access_key.map(shortcut::Mnemonic::char), Some('N'));
        let ctrl_n = shortcut::label(&ctx, &KeyboardShortcut::new(Modifiers::COMMAND, Key::N));
        assert_eq!(new.shortcut.as_deref(), Some(ctrl_n.as_str()));

        step(&mut app, &ctx, press(Key::N, Modifiers::NONE));
        step_until(&mut app, &ctx, "New activating", |a| {
            a.pages.navigation.last_action == "Command: New"
        });
        settle(&mut app, &ctx);
        assert!(
            !menu_item_open(&app, "nav-mb-file-0-menu-item"),
            "menu closes after activation"
        );
        assert!(!shortcut::mnemonics_visible(app.rt.core()));
    }

    #[test]
    fn menu_and_palette_share_a_command_that_fires_once() {
        let (ctx, mut app) = navigation_app();
        let command = Modifiers::COMMAND | Modifiers::CTRL;
        step(&mut app, &ctx, press(Key::Plus, command));
        step_until(&mut app, &ctx, "Zoom in firing", |a| {
            a.pages.navigation.last_action == "Command: Zoom in"
        });
        assert!(!menu_item_open(&app, "nav-mb-view-0-menu-item"));
        assert_eq!(app.pages.navigation.zoom, 110);
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
        assert_eq!(app.pages.navigation.last_action, "Command: Quit");
    }

    #[test]
    fn every_page_renders() {
        let ctx = egui::Context::default();
        let mut app = App::new(&ctx).expect("showcase app");
        app.rt.set_collect_semantics(true);
        for page in Page::ALL {
            for _ in 0..FRAME_BUDGET {
                let mut out = ctx.run_ui(screen(), |ui| {
                    app.apply_settings();
                    let rt = Arc::clone(&app.rt);
                    rt.pass(ui, |ui| {
                        CentralPanel::default().show(ui, |ui| {
                            scroll::page(&mut rt.ui(ui), ("page", page.index()), |ui| {
                                pages::show(page, ui, &mut app.pages, &mut app.shared);
                            });
                        });
                    });
                });
                out.textures_delta.clear();
                // egui paints id clashes as debug text (`Options::warn_on_id_clash`, on in debug builds).
                let clashes: Vec<String> = out
                    .shapes
                    .iter()
                    .filter_map(|c| match &c.shape {
                        egui::Shape::Text(t) if t.galley.text().contains("use of widget ID") => {
                            Some(t.galley.text().to_owned())
                        }
                        _ => None,
                    })
                    .collect();
                assert!(clashes.is_empty(), "{page:?}: id clashes {clashes:?}");
                if !ctx.requested_repaint_last_pass() {
                    break;
                }
            }
            let tree = app.rt.tree(egui::ViewportId::ROOT).expect("semantic tree collected");
            let mut seen = std::collections::HashSet::new();
            let dups: Vec<_> = tree
                .nodes
                .iter()
                .filter(|n| !seen.insert(n.id.key()))
                .map(|n| n.id.key())
                .collect();
            assert!(dups.is_empty(), "{page:?}: duplicate agent keys {dups:?}");
        }
    }

    #[test]
    fn every_page_survives_degenerate_layouts() {
        use std::{
            collections::BTreeSet,
            panic::{self, AssertUnwindSafe},
            sync::{Arc, Mutex},
        };

        const VIEWPORTS: [egui::Vec2; 5] = [
            egui::Vec2::ZERO,
            egui::vec2(1.0, 1.0),
            egui::vec2(40.0, 40.0),
            egui::vec2(320.0, 568.0),
            egui::vec2(4000.0, 3000.0),
        ];
        let rtl = LOCALES.iter().position(|l| *l == "ar-EG").expect("an RTL locale");
        let sites = Arc::new(Mutex::new(BTreeSet::new()));
        let hook = panic::take_hook();
        let record = Arc::clone(&sites);
        panic::set_hook(Box::new(move |info| {
            let at = info.location().map_or_default(|l| format!("{}:{}", l.file(), l.line()));
            let msg = info.payload_as_str().unwrap_or_default().to_owned();
            record.lock().expect("panic log").insert(format!("{at}: {msg}"));
        }));
        let fresh = |locale, density, zoom| {
            let ctx = egui::Context::default();
            ctx.set_zoom_factor(zoom);
            let mut app = App::new(&ctx).expect("showcase app");
            app.shared.settings.locale = locale;
            app.shared.settings.density = density;
            (ctx, app)
        };
        let mut failed = Vec::new();
        for (locale, density, zoom) in [(0, 0, 1.0), (rtl, -3, 0.5), (0, -3, 3.0), (rtl, 0, 3.0)] {
            let (mut ctx, mut app) = fresh(locale, density, zoom);
            for size in VIEWPORTS {
                for page in Page::ALL {
                    let run = panic::catch_unwind(AssertUnwindSafe(|| {
                        app.navigate(page);
                        for _ in 0..2 {
                            let raw = RawInput {
                                screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, size)),
                                ..RawInput::default()
                            };
                            let mut out = ctx.run_ui(raw, |ui| app.shell(ui));
                            out.textures_delta.clear();
                        }
                    }));
                    if run.is_err() {
                        failed.push(format!(
                            "{page:?} {size:?} locale={locale} density={density} zoom={zoom}"
                        ));
                        (ctx, app) = fresh(locale, density, zoom);
                    }
                }
            }
        }
        panic::set_hook(hook);
        let sites = sites.lock().expect("panic log");
        assert!(
            failed.is_empty(),
            "{} cases panicked at:\n{}",
            failed.len(),
            sites.iter().cloned().collect::<Vec<_>>().join("\n")
        );
    }
}
