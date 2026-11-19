//! Shared harness of the material test binaries: an `egui_kittest` harness running a Material `Runtime`,
//! agent sessions over it, and a paint inspector.

use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use egui::{Event, Id, Key, Pos2, RawInput, Rect, Shape, Vec2, ViewportId, epaint::ClippedShape, vec2};
use egui_kittest::HarnessBuilder;
use egui_mineral::{
    agent::AgentHandle,
    date::{Date, Time, Weekday},
    i18n::{
        DateFields, DateLength, DatePart, DayPeriod, Direction, HourCycle, I18n, IcuFormat, ListKind, LocaleFormat,
        NameWidth,
    },
    layout::{MinTarget, Zones},
    overlay::Dismiss,
    runtime::Runtime,
    semantic::{SemanticNode, SemanticTree},
    tree::TreeData,
    widget::MineralUi,
};
use egui_mineral_material::{Material, MaterialExt, Rt, adaptive, spec::ColorRole, theme::Tokens};

pub(crate) const SCREEN: Vec2 = vec2(400.0, 600.0);
pub(crate) const WIDE: Vec2 = vec2(800.0, 800.0);
pub(crate) const MAX_STEPS: u64 = 600;
pub(crate) const AGENT_DEADLINE: Duration = Duration::from_secs(5);

pub(crate) struct TreeNode(u32, &'static str, Vec<TreeNode>);

impl TreeNode {
    pub(crate) fn leaf(key: u32, label: &'static str) -> Self {
        Self(key, label, Vec::new())
    }

    pub(crate) fn branch(key: u32, label: &'static str, children: Vec<TreeNode>) -> Self {
        Self(key, label, children)
    }
}

impl TreeData for TreeNode {
    type Key = u32;

    fn key(&self) -> &u32 {
        &self.0
    }

    fn label(&self) -> &str {
        self.1
    }

    fn children(&self) -> &[Self] {
        &self.2
    }
}

pub(crate) type Mui<'a> = MineralUi<'a, Material>;

pub(crate) type Setup = Box<dyn FnOnce(&Rt)>;

pub(crate) struct App<S> {
    rt: Option<Arc<Rt>>,
    setup: Option<Setup>,
    pub(crate) value: S,
}

pub(crate) fn run(ctx: &egui::Context, raw: RawInput, add: impl FnMut(&mut egui::Ui)) -> egui::FullOutput {
    let mut out = ctx.run_ui(raw, add);
    out.textures_delta.clear();
    out
}

pub(crate) fn runtime(ctx: &egui::Context, theme: egui::Theme) -> Arc<Rt> {
    let format = IcuFormat::new("en-US").expect("en-US ICU data");
    let i18n = I18n::with_format(ctx, "en-US".into(), format, &[]).expect("i18n");
    let rt = Runtime::with_i18n(ctx, Material::default(), i18n).expect("material runtime");
    rt.core().set_preference(theme.into());
    rt.set_collect_semantics(true);
    rt
}

fn start(ctx: &egui::Context, setup: Option<Setup>) -> Arc<Rt> {
    let rt = runtime(ctx, egui::Theme::Light);
    // kittest wraps the app in a panel frame.
    ctx.style_mut_of(egui::Theme::Light, |s| {
        s.visuals.panel_fill = egui::Color32::TRANSPARENT;
    });
    if let Some(setup) = setup {
        setup(&rt);
    }
    rt
}

pub(crate) type Harness<'a, S> = egui_kittest::Harness<'a, App<S>>;

pub(crate) fn min_interactive() -> f32 {
    let MinTarget::Points(p) = adaptive::MIN_INTERACTIVE else {
        unreachable!("MIN_INTERACTIVE is a fixed size")
    };
    p
}

pub(crate) fn builder<S>(size: Vec2) -> HarnessBuilder<App<S>> {
    egui_kittest::Harness::builder()
        .with_size(size)
        .with_theme(egui::Theme::Light)
        .with_step_dt(RawInput::default().predicted_dt)
        .with_max_steps(MAX_STEPS)
        // Headless: no text font is installed, text lays out as tofu boxes.
        .allow_missing_glyphs()
}

pub(crate) fn build<'a, S>(
    builder: HarnessBuilder<App<S>>,
    setup: Option<Setup>,
    value: S,
    mut app: impl FnMut(&mut Mui<'_>, &mut S) + 'a,
) -> Harness<'a, S> {
    builder.build_ui_state(
        move |ui, a: &mut App<S>| {
            // The harness owns the context, so the runtime starts in the first pass.
            let rt = Arc::clone(a.rt.get_or_insert_with(|| start(ui.ctx(), a.setup.take())));
            rt.pass(ui, |ui| app(ui, &mut a.value));
        },
        App { rt: None, setup, value },
    )
}

pub(crate) fn harness<'a, S>(size: Vec2, value: S, app: impl FnMut(&mut Mui<'_>, &mut S) + 'a) -> Harness<'a, S> {
    build(builder(size), None, value, app)
}

fn reduced_motion() -> Setup {
    Box::new(|rt: &Rt| {
        rt.core().motion().set_reduced(true);
    })
}

pub(crate) fn still<'a, S>(size: Vec2, value: S, app: impl FnMut(&mut Mui<'_>, &mut S) + 'a) -> Harness<'a, S> {
    build(builder(size), Some(reduced_motion()), value, app)
}

pub(crate) trait MineralHarness {
    fn rt(&self) -> Arc<Rt>;

    fn tree(&self) -> Arc<SemanticTree> {
        self.rt().tree(ViewportId::ROOT).expect("semantics collected")
    }

    fn node(&self, key: &str) -> SemanticNode {
        let tree = self.tree();
        tree.find_key(key).cloned().unwrap_or_else(|| {
            let keys: Vec<_> = tree.nodes.iter().map(|n| n.id.key()).collect();
            panic!("no node {key} in {keys:?}")
        })
    }

    fn rect(&self, key: &str) -> Rect {
        self.node(key).rect
    }

    fn widget(&self, key: &str) -> Id {
        self.node(key).widget_id.unwrap_or_else(|| panic!("no widget {key}"))
    }

    fn focused(&self, key: &str) -> bool;

    /// Screen corner, 1pt in, outside every semantic node (for outside clicks and hovers).
    fn outside(&self) -> Pos2 {
        let screen = self.rt().ctx().content_rect().shrink(1.0);
        let tree = self.tree();
        [
            screen.left_top(),
            screen.right_top(),
            screen.left_bottom(),
            screen.right_bottom(),
        ]
        .into_iter()
        .find(|p| !tree.nodes.iter().any(|n| n.rect.contains(*p)))
        .expect("a screen corner outside every node")
    }

    /// Mouse click at `pos`, then settle.
    fn click_at(&mut self, pos: Pos2);

    /// Press and release at `pos` in one pass (a fast click, synthetic input), then settle.
    fn click_at_once(&mut self, pos: Pos2);

    fn click(&mut self, key: &str) {
        let at = self.rect(key).center();
        self.click_at(at);
    }

    /// Key press and release, then settle.
    fn press(&mut self, key: Key);

    /// Text input into the focused widget, then settle.
    fn type_text(&mut self, text: &str);

    /// Runs `session` on its own thread while stepping passes; returns its result.
    fn drive_agent<T: Send + 'static>(&mut self, session: impl FnOnce(AgentHandle) -> T + Send + 'static) -> T {
        let agent = self.rt().agent(ViewportId::ROOT);
        #[expect(clippy::disallowed_methods, reason = "agent client thread, joined below")]
        let client = std::thread::spawn(move || session(agent));
        let deadline = Instant::now() + AGENT_DEADLINE;
        while !client.is_finished() && Instant::now() < deadline {
            self.step_once();
            std::thread::yield_now();
        }
        assert!(
            client.is_finished(),
            "agent session still running after {AGENT_DEADLINE:?}"
        );
        client.join().expect("agent thread")
    }

    fn step_once(&mut self);

    /// Steps passes until `done` holds; panics with `what` after `MAX_STEPS`.
    fn step_until(&mut self, what: &str, done: impl Fn(&Self) -> bool)
    where
        Self: Sized,
    {
        for _ in 0..MAX_STEPS {
            if done(self) {
                return;
            }
            self.step_once();
        }
        panic!("{what}: not reached in {MAX_STEPS} steps");
    }
}

impl<S> MineralHarness for Harness<'_, S> {
    fn rt(&self) -> Arc<Rt> {
        Arc::clone(self.state().rt.as_ref().expect("first pass ran"))
    }

    fn focused(&self, key: &str) -> bool {
        self.ctx.memory(egui::Memory::focused) == Some(self.widget(key))
    }

    fn click_at(&mut self, pos: Pos2) {
        self.hover_at(pos);
        self.drag_at(pos);
        self.drop_at(pos);
        self.run();
    }

    fn click_at_once(&mut self, pos: Pos2) {
        self.hover_at(pos);
        self.run();
        let button = |pressed| Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        };
        self.input_mut().events.extend([button(true), button(false)]);
        self.run();
    }

    fn press(&mut self, key: Key) {
        self.key_press(key);
        self.run();
    }

    fn type_text(&mut self, text: &str) {
        self.event(Event::Text(text.to_owned()));
        self.run();
    }

    fn step_once(&mut self) {
        self.step();
    }
}

pub(crate) type Overlay = (bool, Option<Dismiss>);

pub(crate) fn dismissals(
    mut h: Harness<'_, Overlay>,
    outside: Pos2,
    reopen: impl Fn(&mut Harness<'_, Overlay>),
) -> [Option<Dismiss>; 2] {
    h.press(Key::Escape);
    let escape = h.state_mut().value.1.take();
    assert!(!h.state().value.0, "Escape closes");
    reopen(&mut h);
    assert!(h.state().value.0, "reopened");
    h.click_at(outside);
    assert!(!h.state().value.0, "an outside press closes");
    [escape, h.state().value.1]
}

pub(crate) fn flag_dismissals(h: Harness<'_, Overlay>, outside: Pos2) -> [Option<Dismiss>; 2] {
    dismissals(h, outside, |h| {
        h.state_mut().value.0 = true;
        h.run();
    })
}

pub(crate) struct Pass {
    pub(crate) shapes: Vec<ClippedShape>,
    pub(crate) outer: Rect,
    pub(crate) zones: Option<Zones>,
    pub(crate) tree: SemanticTree,
    pub(crate) tokens: Arc<Tokens>,
}

impl Pass {
    fn shapes(&self) -> impl Iterator<Item = &Shape> {
        self.shapes.iter().map(|s| &s.shape)
    }

    pub(crate) fn color(&self, role: ColorRole) -> egui::Color32 {
        role.resolve(&self.tokens.color)
    }

    pub(crate) fn filled(&self, role: ColorRole) -> Vec<Rect> {
        let fill = self.color(role);
        self.shapes()
            .filter_map(|s| match s {
                Shape::Rect(r) if r.fill == fill => Some(r.rect),
                _ => None,
            })
            .collect()
    }

    pub(crate) fn one(&self, role: ColorRole) -> Rect {
        let rects = self.filled(role);
        assert_eq!(rects.len(), 1, "one {role:?} container in {rects:?}");
        rects[0]
    }

    pub(crate) fn stroked(&self, role: ColorRole) -> Vec<Rect> {
        let color = self.color(role);
        self.shapes()
            .filter_map(|s| match s {
                Shape::Rect(r) if r.stroke.color == color && r.stroke.width > 0.0 => Some(r.rect),
                _ => None,
            })
            .collect()
    }

    pub(crate) fn circles(&self, role: ColorRole) -> Vec<(Pos2, f32)> {
        let fill = self.color(role);
        self.shapes()
            .filter_map(|s| match s {
                Shape::Rect(r) if r.fill == fill => Some((r.rect.center(), r.rect.width())),
                Shape::Circle(c) if c.fill == fill => Some((c.center, 2.0 * c.radius)),
                _ => None,
            })
            .collect()
    }

    pub(crate) fn text(&self, text: &str) -> Rect {
        self.shapes()
            .find_map(|s| match s {
                Shape::Text(t) if t.galley.text() == text => Some(t.galley.rect.translate(t.pos.to_vec2())),
                _ => None,
            })
            .unwrap_or_else(|| panic!("{text} painted"))
    }

    pub(crate) fn node(&self, key: &str) -> &SemanticNode {
        self.tree.find_key(key).unwrap_or_else(|| panic!("no node {key}"))
    }

    pub(crate) fn rect(&self, key: &str) -> Rect {
        self.node(key).rect
    }
}

struct Directed {
    inner: IcuFormat,
    direction: Direction,
}

impl LocaleFormat for Directed {
    fn tag(&self) -> &str {
        self.inner.tag()
    }
    fn direction(&self) -> Direction {
        self.direction
    }
    fn first_weekday(&self) -> Weekday {
        self.inner.first_weekday()
    }
    fn hour_cycle(&self) -> HourCycle {
        self.inner.hour_cycle()
    }
    fn month_name(&self, month: i8, width: NameWidth) -> String {
        self.inner.month_name(month, width)
    }
    fn weekday_name(&self, day: Weekday, width: NameWidth) -> String {
        self.inner.weekday_name(day, width)
    }
    fn day_period(&self, period: DayPeriod) -> String {
        self.inner.day_period(period)
    }
    fn date(&self, d: Date, fields: DateFields, length: DateLength) -> String {
        self.inner.date(d, fields, length)
    }
    fn month_year(&self, year: i16, month: i8) -> String {
        self.inner.month_year(year, month)
    }
    fn time(&self, t: Time, cycle: HourCycle) -> String {
        self.inner.time(t, cycle)
    }
    fn number(&self, value: f64, fraction_digits: Option<i16>) -> String {
        self.inner.number(value, fraction_digits)
    }
    fn digits(&self) -> [char; 10] {
        self.inner.digits()
    }
    fn grouping_separator(&self) -> Option<char> {
        self.inner.grouping_separator()
    }
    fn decimal_separator(&self) -> char {
        self.inner.decimal_separator()
    }
    fn date_order(&self) -> [DatePart; 3] {
        self.inner.date_order()
    }
    fn compare(&self, a: &str, b: &str) -> std::cmp::Ordering {
        self.inner.compare(a, b)
    }
    fn list(&self, items: &[&str], kind: ListKind) -> String {
        self.inner.list(items, kind)
    }
}

pub(crate) fn render_directed(direction: Direction, size: Vec2, mut add: impl FnMut(&mut Mui<'_>)) -> Pass {
    let setup = Box::new(move |rt: &Rt| {
        rt.core().motion().set_reduced(true);
        let inner = IcuFormat::new("en-US").expect("en-US ICU data");
        rt.i18n().set_format(Directed { inner, direction });
    });
    inspect(size, setup, |ui| {
        add(ui);
        None
    })
}

pub(crate) fn render(add: impl FnMut(&mut Mui<'_>)) -> Pass {
    render_on(SCREEN, add)
}

pub(crate) fn render_on(size: Vec2, mut add: impl FnMut(&mut Mui<'_>)) -> Pass {
    inspect(size, reduced_motion(), |ui| {
        add(ui);
        None
    })
}

pub(crate) fn render_zones(mut add: impl FnMut(&mut Mui<'_>) -> Zones) -> Pass {
    inspect(SCREEN, reduced_motion(), |ui| Some(add(ui)))
}

fn inspect(size: Vec2, setup: Setup, mut add: impl FnMut(&mut Mui<'_>) -> Option<Zones>) -> Pass {
    // Geometry passes include unnamed edge cases such as empty labels.
    let builder = builder(size).with_accessibility_check(false);
    let h = build(builder, Some(setup), (Rect::NOTHING, None), |ui, (outer, zones)| {
        let rt = ui.rt();
        let r = ui.scope(|ui| add(&mut rt.ui(ui)));
        (*outer, *zones) = (r.response.rect, r.inner);
    });
    let (outer, zones) = h.state().value;
    Pass {
        shapes: h.output().shapes.clone(),
        outer,
        zones,
        tree: (*h.tree()).clone(),
        tokens: h.rt().tokens(),
    }
}
