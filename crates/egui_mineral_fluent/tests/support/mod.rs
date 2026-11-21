//! Shared harness of the fluent test binaries: an `egui_kittest` harness running a Fluent `Runtime` and
//! lookups into its semantic tree and painted shapes.

use std::sync::Arc;

use egui::{Color32, Key, Pos2, RawInput, Rect, Shape, Vec2, ViewportId, epaint::ClippedShape, vec2};
use egui_kittest::HarnessBuilder;
use egui_mineral::{
    i18n::{I18n, IcuFormat},
    runtime::Runtime,
    semantic::{SemanticNode, SemanticTree},
};
use egui_mineral_fluent::{Fluent, Rt};

pub(crate) const SCREEN: Vec2 = vec2(600.0, 600.0);

pub(crate) use egui_mineral_fluent::FluentUi;

pub(crate) fn runtime(ctx: &egui::Context, theme: egui::Theme) -> Arc<Rt> {
    runtime_with(ctx, theme, Fluent::default())
}

pub(crate) fn runtime_with(ctx: &egui::Context, theme: egui::Theme, kit: Fluent) -> Arc<Rt> {
    let format = IcuFormat::new(&egui_mineral::i18n::locale!("en-US")).expect("en-US ICU data");
    let i18n = I18n::with_format(ctx, egui_mineral::i18n::locale!("en-US"), format, &[]).expect("i18n");
    let rt = Runtime::with_i18n(ctx, kit, i18n).expect("fluent runtime");
    rt.core().set_preference(theme.into());
    rt.set_collect_semantics(true);
    rt.core().motion().set_reduced(true);
    rt
}

pub(crate) struct App<S> {
    rt: Option<Arc<Rt>>,
    pub(crate) value: S,
}

pub(crate) type Harness<'a, S> = egui_kittest::Harness<'a, App<S>>;

fn builder<S>(size: Vec2) -> HarnessBuilder<App<S>> {
    egui_kittest::Harness::builder()
        .with_size(size)
        .with_theme(egui::Theme::Light)
        .with_step_dt(RawInput::default().predicted_dt)
        // Headless: no text font is installed, text lays out as tofu boxes.
        .allow_missing_glyphs()
}

pub(crate) fn harness<'a, S>(value: S, mut app: impl FnMut(&mut FluentUi<'_>, &mut S) + 'a) -> Harness<'a, S> {
    builder(SCREEN).build_ui_state(
        move |ui, a: &mut App<S>| {
            // The harness owns the context, so the runtime starts in the first pass.
            let rt = Arc::clone(a.rt.get_or_insert_with(|| runtime(ui.ctx(), egui::Theme::Light)));
            rt.pass(ui, |ui| app(ui, &mut a.value));
        },
        App { rt: None, value },
    )
}

pub(crate) trait FluentHarness {
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

    fn widget(&self, key: &str) -> egui::Id {
        self.node(key).widget_id.unwrap_or_else(|| panic!("no widget {key}"))
    }

    fn focused(&self, key: &str) -> bool;

    fn click_at(&mut self, pos: Pos2);

    fn click(&mut self, key: &str) {
        let at = self.rect(key).center();
        self.click_at(at);
    }

    fn press(&mut self, key: Key);
}

impl<S> FluentHarness for Harness<'_, S> {
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

    fn press(&mut self, key: Key) {
        self.key_press(key);
        self.run();
    }
}

pub(crate) struct Pass {
    pub(crate) shapes: Vec<ClippedShape>,
    pub(crate) rt: Arc<Rt>,
}

impl Pass {
    pub(crate) fn flat(&self) -> Vec<Shape> {
        fn walk(s: &Shape, out: &mut Vec<Shape>) {
            match s {
                Shape::Vec(v) => v.iter().for_each(|s| walk(s, out)),
                s => out.push(s.clone()),
            }
        }
        let mut out = Vec::new();
        self.shapes.iter().for_each(|c| walk(&c.shape, &mut out));
        out
    }

    pub(crate) fn fills(&self, color: Color32) -> Vec<Rect> {
        self.flat()
            .into_iter()
            .filter_map(|s| match s {
                Shape::Rect(r) if r.fill == color => Some(r.rect),
                _ => None,
            })
            .collect()
    }

    pub(crate) fn one(&self, color: Color32) -> Rect {
        let f = self.fills(color);
        assert_eq!(f.len(), 1, "fills of {color:?}: {f:?}");
        f[0]
    }

    pub(crate) fn text(&self, text: &str) -> Rect {
        self.flat()
            .into_iter()
            .find_map(|s| match s {
                Shape::Text(t) if t.galley.text() == text => Some(t.galley.rect.translate(t.pos.to_vec2())),
                _ => None,
            })
            .unwrap_or_else(|| panic!("no text {text:?}"))
    }

    pub(crate) fn text_clip(&self, prefix: &str) -> Rect {
        self.shapes
            .iter()
            .find_map(|c| match &c.shape {
                Shape::Text(t) if t.galley.text().starts_with(prefix) => Some(c.clip_rect),
                _ => None,
            })
            .unwrap_or_else(|| panic!("no text starting {prefix:?}"))
    }

    pub(crate) fn text_family(&self, text: &str) -> Option<egui::FontFamily> {
        self.flat().into_iter().find_map(|s| match s {
            Shape::Text(t) if t.galley.text() == text => {
                t.galley.job.sections.first().map(|s| s.format.font_id.family.clone())
            }
            _ => None,
        })
    }

    pub(crate) fn text_color(&self, text: &str) -> Color32 {
        self.flat()
            .into_iter()
            .find_map(|s| match s {
                Shape::Text(t) if t.galley.text() == text => Some(t.fallback_color),
                _ => None,
            })
            .unwrap_or_else(|| panic!("no text {text:?}"))
    }

    pub(crate) fn segments(&self) -> Vec<([Pos2; 2], egui::Stroke)> {
        self.flat()
            .into_iter()
            .filter_map(|s| match s {
                Shape::LineSegment { points, stroke } => Some((points, stroke)),
                _ => None,
            })
            .collect()
    }

    pub(crate) fn hlines(&self) -> Vec<(f32, f32, f32)> {
        self.flat()
            .into_iter()
            .filter_map(|s| match s {
                Shape::LineSegment { points: [a, b], .. } if (a.y - b.y).abs() < f32::EPSILON => {
                    Some((a.x.min(b.x), a.x.max(b.x), a.y))
                }
                _ => None,
            })
            .collect()
    }
}

pub(crate) fn render(add: impl FnMut(&mut FluentUi<'_>)) -> Pass {
    render_in(egui::Theme::Light, add)
}

pub(crate) fn render_in(theme: egui::Theme, add: impl FnMut(&mut FluentUi<'_>)) -> Pass {
    render_with(theme, Fluent::default(), add)
}

pub(crate) fn render_with(theme: egui::Theme, kit: Fluent, mut add: impl FnMut(&mut FluentUi<'_>)) -> Pass {
    let ctx = egui::Context::default();
    let rt = runtime_with(&ctx, theme, kit);
    let raw = || RawInput {
        screen_rect: Some(Rect::from_min_size(Pos2::ZERO, SCREEN)),
        ..RawInput::default()
    };
    let mut shapes = Vec::new();
    for _ in 0..2 {
        let mut out = ctx.run_ui(raw(), |ui| {
            egui::Frame::NONE.show(ui, |ui| rt.pass(ui, |ui| add(ui)));
        });
        out.textures_delta.clear();
        shapes = out.shapes;
    }
    Pass { shapes, rt }
}
