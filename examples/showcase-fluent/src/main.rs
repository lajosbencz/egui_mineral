//! Component gallery and e2e target for `egui_mineral_fluent` (native, and wasm via trunk); F12 opens the inspector.

mod pages;

use std::sync::Arc;

use egui::{CentralPanel, Color32, Key, KeyboardShortcut, Modifiers, Panel, Ui, scroll_area::ScrollAreaOutput};
use egui_mineral_fluent::{
    BrandRamp,
    egui_mineral::runtime::{InitError, Runtime},
    prelude::*,
    scroll,
    spec::tokens::brand::BRAND_WEB,
};
use mineral_inspector::Inspector;
use pages::{LOCALES, Page};

pub(crate) use egui_mineral_fluent::Rt;

const CONTENT_MAX_W: f32 = 1040.0;
const INSPECTOR: KeyboardShortcut = KeyboardShortcut::new(Modifiers::NONE, Key::F12);

pub(crate) type Fui<'a> = MineralUi<'a, Fluent>;

// Ramps from the Fluent theme designer's 16 steps: the web brand, a teal and a berry one.
fn brands() -> [(&'static str, BrandRamp); 3] {
    let ramp = |base: [u8; 3]| {
        BrandRamp(std::array::from_fn(|i| {
            #[expect(clippy::cast_precision_loss, reason = "i < 16")]
            let t = i as f32 / 15.0;
            let mix = |c: u8| {
                let c = f32::from(c);
                let v = if t < 0.5 {
                    c * t * 2.0
                } else {
                    c + (255.0 - c) * (t - 0.5) * 2.0
                };
                #[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss, reason = "clamped")]
                let v = v.round().clamp(0.0, 255.0) as u8;
                v
            };
            Color32::from_rgb(mix(base[0]), mix(base[1]), mix(base[2]))
        }))
    };
    [
        ("Web", BRAND_WEB),
        ("Teal", ramp([3, 131, 135])),
        ("Berry", ramp([194, 57, 179])),
    ]
}

const THEMES: [(egui::ThemePreference, &str, FluentIcon); 3] = [
    (egui::ThemePreference::System, "System", icons::ICON_DARK_THEME),
    (egui::ThemePreference::Light, "Light", icons::ICON_WEATHER_SUNNY),
    (egui::ThemePreference::Dark, "Dark", icons::ICON_WEATHER_MOON),
];

#[derive(Clone, Debug, Default, PartialEq)]
struct Settings {
    theme: usize,
    high_contrast: bool,
    brand: usize,
    locale: usize,
}

struct App {
    rt: Arc<Rt>,
    page: Page,
    inspector: Inspector,
    settings: Settings,
    applied: Option<Settings>,
    state: pages::State,
}

impl App {
    fn new(ctx: &egui::Context) -> Result<Self, InitError> {
        let rt = Runtime::new(ctx, Fluent::default())?;
        Ok(Self {
            rt,
            page: Page::Foundations,
            inspector: Inspector::default(),
            settings: Settings::default(),
            applied: None,
            state: pages::State::default(),
        })
    }

    fn navigate(&mut self, page: Page) {
        self.page = page;
    }

    fn apply_settings(&mut self) {
        let s = &self.settings;
        if self.applied.as_ref() == Some(s) {
            return;
        }
        let rt = Arc::clone(&self.rt);
        let ctx = rt.ctx();
        let kit = rt.kit();
        rt.core().set_preference(THEMES[s.theme].0);
        rt.core()
            .set_global_style(egui_mineral_fluent::egui_mineral::runtime::GlobalStyle::Kit);
        kit.set_high_contrast(ctx, s.high_contrast);
        let brands = brands();
        if let Some((_, brand)) = brands.get(s.brand) {
            kit.set_brand(ctx, *brand);
        }
        if self.applied.as_ref().is_none_or(|a| a.locale != s.locale) {
            rt.i18n().set_locale(LOCALES[s.locale]).expect("LOCALES are valid tags");
        }
        self.applied = Some(s.clone());
    }

    fn top_bar(&mut self, ui: &mut Fui<'_>) {
        let rt = ui.rt();
        pages::row(ui, |ui| {
            for page in Page::all() {
                let appearance = if page == self.page {
                    ButtonAppearance::Primary
                } else {
                    ButtonAppearance::Subtle
                };
                let title = page.title(rt.i18n());
                if ui
                    .mineral(PushButton::new(("nav", page.key()), title).appearance(appearance))
                    .clicked()
                {
                    self.navigate(page);
                }
            }
            let s = &mut self.settings;
            let (_, theme, icon) = THEMES[s.theme];
            if ui
                .mineral(PushButton::new("theme", format!("Theme: {theme}")).icon(icon))
                .clicked()
            {
                s.theme = (s.theme + 1) % THEMES.len();
            }
            ui.mineral(ToggleButton::new(
                "high-contrast",
                "High contrast",
                &mut s.high_contrast,
            ));
            let brands = brands();
            let brand = brands.get(s.brand).map_or("", |b| b.0);
            if ui
                .mineral(PushButton::new("brand", format!("Brand: {brand}")).icon(icons::ICON_COLOR))
                .clicked()
            {
                s.brand = (s.brand + 1) % brands.len();
            }
            if ui
                .mineral(PushButton::new("locale", LOCALES[s.locale]).icon(icons::ICON_LOCAL_LANGUAGE))
                .clicked()
            {
                s.locale = (s.locale + 1) % LOCALES.len();
            }
        });
    }

    fn shell(&mut self, ui: &mut Ui) -> ScrollAreaOutput<()> {
        let rt = Arc::clone(&self.rt);
        self.inspector.toggle_on(rt.ctx(), INSPECTOR);
        self.apply_settings();
        let page = rt.pass(ui, |ui| self.body(ui));
        self.inspector.show(&rt);
        page
    }

    fn body(&mut self, ui: &mut Fui<'_>) -> ScrollAreaOutput<()> {
        let rt = ui.rt();
        Panel::top("top-bar").show(ui, |ui| self.top_bar(&mut rt.ui(ui)));
        let page = self.page;
        // The page bar reaches the window edge; the panel margin pads the content inside the scroll area.
        let panel = egui::Frame::central_panel(ui.style());
        let margin = panel.inner_margin;
        let page_out = CentralPanel::default().frame(panel.inner_margin(0)).show(ui, |ui| {
            scroll::page(&mut rt.ui(ui), ("page", page.key()), |ui| {
                egui::Frame::NONE.inner_margin(margin).show(ui, |ui| {
                    let w = ui.available_width().min(CONTENT_MAX_W);
                    let pad = (ui.available_width() - w) / 2.0;
                    let spacing = ui.spacing().item_spacing;
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 0.0;
                        ui.add_space(pad);
                        ui.vertical(|ui| {
                            ui.spacing_mut().item_spacing = spacing;
                            ui.set_width(w);
                            pages::show(page, &mut rt.ui(ui), &mut self.state);
                        });
                    });
                });
            })
        });
        page_out.inner
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
    std::panic::set_hook(Box::new(|info| {
        let backtrace = std::backtrace::Backtrace::capture();
        tracing::error!(%backtrace, "{info}");
    }));
    eframe::run_native(
        "egui_mineral Fluent showcase",
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
                Box::new(|cc| Ok(Box::new(App::new(&cc.egui_ctx)?))),
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
    use std::collections::{BTreeSet, HashSet};

    use egui::RawInput;
    use egui_mineral_fluent::catalog::Category;

    use super::*;

    // Page overflow settles once its scroll bar appears.
    const SETTLE: usize = 8;

    macro_rules! every_page_holds_up {
        ($($test:ident: $page:expr),* $(,)?) => {
            mod every_page_holds_up {
                use super::*;

                #[test]
                fn covers_every_page() {
                    let tested = [$($page),*];
                    assert!(Page::all().all(|p| tested.contains(&p)), "a page has no test");
                }

                $(
                    #[test]
                    fn $test() {
                        holds_up($page);
                    }
                )*
            }
        };
    }

    every_page_holds_up! {
        foundations: Page::Foundations,
        actions: Page::Category(Category::Actions),
        inputs: Page::Category(Category::Inputs),
        feedback: Page::Category(Category::Feedback),
        surfaces: Page::Category(Category::Surfaces),
        navigation: Page::Category(Category::Navigation),
        data: Page::Category(Category::Data),
    }

    fn holds_up(page: Page) {
        use std::panic::{self, AssertUnwindSafe};

        // Narrowest phone in portrait; the other widths start at the breakpoints.
        const COMPACT_MIN: f32 = 320.0;
        const DEGENERATE: [egui::Vec2; 4] = [
            egui::Vec2::ZERO,
            egui::vec2(1.0, 1.0),
            egui::vec2(40.0, 40.0),
            egui::vec2(4000.0, 3000.0),
        ];
        let rtl = LOCALES.iter().position(|l| *l == "ar-EG").expect("an RTL locale");
        let fresh = |locale, high_contrast, zoom| {
            let ctx = egui::Context::default();
            ctx.set_zoom_factor(zoom);
            let mut app = App::new(&ctx).expect("showcase app");
            app.rt.set_collect_semantics(true);
            app.settings.locale = locale;
            app.settings.high_contrast = high_contrast;
            (ctx, app)
        };
        let classes: Vec<egui::Vec2> = std::iter::once(COMPACT_MIN)
            .chain(fresh(0, false, 1.0).1.rt.core().policies().breakpoints.iter().copied())
            .map(|w| egui::vec2(w, 900.0))
            .collect();
        let mut failed = Vec::new();
        let configs = [(0, false, 1.0), (rtl, true, 0.5), (0, true, 3.0), (rtl, false, 3.0)];
        for (i, (locale, high_contrast, zoom)) in configs.into_iter().enumerate() {
            let (mut ctx, mut app) = fresh(locale, high_contrast, zoom);
            for size in DEGENERATE.into_iter().chain(classes.iter().copied()) {
                let width = i == 0 && classes.contains(&size);
                let case = format!("{size:?} locale={locale} hc={high_contrast} zoom={zoom}");
                let run = panic::catch_unwind(AssertUnwindSafe(|| check(&ctx, &mut app, page, size, width)));
                if let Ok(problems) = run {
                    failed.extend(problems.into_iter().map(|p| format!("{case}: {p}")));
                } else {
                    failed.push(format!("{case}: panicked"));
                    (ctx, app) = fresh(locale, high_contrast, zoom);
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
        app.navigate(page);
        let mut problems = BTreeSet::new();
        let mut over = 0.0;
        for _ in 0..if width { SETTLE } else { 2 } {
            let raw = RawInput {
                screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, size)),
                ..RawInput::default()
            };
            let mut out = ctx.run_ui(raw, |ui| {
                let page = app.shell(ui);
                over = page.content_size.x - page.inner_rect.width();
            });
            out.textures_delta.clear();
            // egui paints id clashes as debug text (`Options::warn_on_id_clash`, on in debug builds).
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
        let tree = app.rt.tree(egui::ViewportId::ROOT).expect("semantic tree collected");
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
