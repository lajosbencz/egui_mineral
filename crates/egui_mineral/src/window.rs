use egui::{
    Frame, Id, LayerId, Margin, Rect, Sense, TextStyle, Ui, UiBuilder, Vec2, ViewportBuilder, ViewportId, pos2, vec2,
};

use crate::{
    input::Gesture,
    layout::{Measure, MinTarget, Zones},
    runtime::{Runtime, Stylekit},
    semantic::{KeyedSalt, Role, SemanticNode, StableId},
    widget::{Availability, MineralUi},
};

/// Native viewport window description; shows contents in a separate OS window.
#[derive(Clone, Debug)]
#[must_use = "call .show(..)"]
pub struct WindowSpec {
    id: ViewportId,
    salt: KeyedSalt,
    builder: ViewportBuilder,
}

impl WindowSpec {
    /// `id_salt` hashes into the viewport id; `title` is the OS window title.
    pub fn new(id_salt: impl Into<crate::semantic::KeyedSalt>, title: impl Into<String>) -> Self {
        let salt = id_salt.into();
        Self {
            id: ViewportId::from_hash_of(salt.id_salt()),
            salt,
            builder: ViewportBuilder::default().with_title(title),
        }
    }

    /// Inner size in points.
    pub fn size(mut self, size: egui::Vec2) -> Self {
        self.builder = self.builder.with_inner_size(size);
        self
    }

    /// `add_contents` runs in a [`Runtime::pass`] of the new viewport on its root `Ui`, unframed (panels are
    /// the caller's); embedded (no native window support) it runs inside the current pass.
    pub fn show<K: Stylekit>(
        self,
        rt: &Runtime<K>,
        open: &mut bool,
        mut add_contents: impl FnMut(&mut MineralUi<'_, K>),
    ) {
        let ctx = rt.ctx();
        if !*open {
            return;
        }
        if ctx.embed_viewports() {
            self.show_embedded(rt, open, add_contents);
            return;
        }
        let mut close = false;
        ctx.show_viewport_immediate(self.id, self.builder, |ui, _| {
            if ui.ctx().input(|i| i.viewport().close_requested()) {
                close = true;
            }
            rt.pass(ui, |ui| add_contents(ui));
        });
        if close {
            *open = false;
        }
    }
}

impl WindowSpec {
    // egui draws a close button only when given `open`, sized by the style's `icon_width`, so the window
    // keeps egui's title bar (title, drag, resize) and reserves the right end of it for a close control
    // owned here: its hit target follows the pointer modality, its glyph is the kit's. The close node and
    // the contents share one runtime pass, rooted off-screen, so the node joins the runtime's tree.
    fn show_embedded<K: Stylekit>(
        self,
        rt: &Runtime<K>,
        open: &mut bool,
        add_contents: impl FnMut(&mut MineralUi<'_, K>),
    ) {
        let mut root = Ui::new(
            rt.ctx().clone(),
            Id::unique(self.id).with("root"),
            UiBuilder::new().layer_id(LayerId::background()).max_rect(Rect::ZERO),
        );
        rt.pass(&mut root, |_| self.window_with_close(rt, open, add_contents));
    }

    fn window_with_close<K: Stylekit>(
        self,
        rt: &Runtime<K>,
        open: &mut bool,
        mut add_contents: impl FnMut(&mut MineralUi<'_, K>),
    ) {
        let ctx = rt.ctx();
        let style = ctx.global_style();
        let target = MinTarget::Auto.points(rt.core());
        let heading = ctx.fonts_mut(|f| f.row_height(&TextStyle::Heading.resolve(&style)));
        let frame = Frame::window(&style);
        let pad = frame.inner_margin;
        let title_frame = frame.inner_margin(Margin {
            right: to_margin(f32::from(pad.right) + heading),
            ..pad
        });
        let shown = egui::Window::from_viewport(self.id, self.builder)
            .frame(frame)
            .title_frame(title_frame)
            .collapsible(false)
            .show(ctx, |ui| rt.pass(ui, |ui| add_contents(ui)));
        let Some(shown) = shown else { return };
        let outer = shown.response.rect;
        let inset = frame.stroke.width;
        let center = pos2(
            outer.right() - inset - f32::from(pad.right) - heading / 2.0,
            outer.top() + inset + f32::from(pad.top) + heading / 2.0,
        );
        let icon = style.spacing.icon_width;
        let zones = Zones::new(Measure::new(Vec2::splat(icon)), target);
        let zones = zones.translate(center - zones.visual.center());
        // The title bar does not grow with the target, so layout is the same for every modality; the target
        // slides inward to stay on the window's layer, always around the glyph.
        let room = outer.shrink(inset);
        let hit = zones.hit.translate(vec2(
            (room.right() - zones.hit.right()).min(0.0),
            (room.top() - zones.hit.top()).max(0.0),
        ));
        let mut ui = Ui::new(
            ctx.clone(),
            shown.response.id.with("close"),
            UiBuilder::new().layer_id(shown.response.layer_id).max_rect(hit),
        );
        ui.set_clip_rect(hit);
        ui.set_style(rt.style());
        if close_control(rt, &mut ui, &self.salt, zones.visual, hit) {
            *open = false;
        }
    }
}

fn to_margin(points: f32) -> i8 {
    points.clamp(0.0, f32::from(i8::MAX)) as i8
}

/// Close control of an embedded window title bar; true when activated. Its hit target is `hit`, sized by
/// the current pass's modality, and a tap is tested against it directly, not against the previous pass's rect
/// like egui's click: the first touch of a session reaches this pass with the touch modality (and the
/// touch-sized target) its own events established, while the previous pass still held the mouse size.
fn close_control<K: Stylekit>(rt: &Runtime<K>, ui: &mut Ui, salt: &KeyedSalt, visual: Rect, hit: Rect) -> bool {
    let id = ui.scope_id();
    let mut response = ui.interact(hit, id, Sense::click());
    crate::kit::focus::follow_press(&response);
    rt.kit().paint_window_close(ui, visual, &response);
    let layer = response.layer_id;
    let tapped = rt.core().input().gestures().any(|g| {
        matches!(g, Gesture::Tap { pos }
            if hit.contains(*pos) && rt.ctx().layer_id_at(*pos).is_none_or(|l| l == layer))
    });
    if tapped {
        // doc(hidden) on egui 0.37; see `InputContext::tap_on`.
        response.flags |= egui::response::Flags::FAKE_PRIMARY_CLICKED;
    }
    let label = rt.i18n().tr("mineral-window-close", &[]);
    crate::semantic::emit(ui.ctx(), Some(id), hit, || {
        SemanticNode::new(StableId::of(salt, "close"), Role::Button, label).clickable(Availability::Enabled)
    });
    response.clicked()
}

/// Modals render in push order; egui blocks input below the topmost.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ModalStack {
    stack: Vec<Id>,
}

impl ModalStack {
    /// Opens `id` on top; no-op when already open.
    pub fn push(&mut self, id: Id) {
        if !self.stack.contains(&id) {
            self.stack.push(id);
        }
    }

    /// Closes the last pushed modal, which renders on top.
    pub fn pop(&mut self) -> Option<Id> {
        self.stack.pop()
    }

    /// Closes `id` wherever it is in the stack.
    pub fn close(&mut self, id: Id) {
        self.stack.retain(|m| *m != id);
    }

    /// Entry that egui routes input to ([`crate::overlay::top_modal`]); `None` while a modal outside the stack is on
    /// top or before the first [`Self::show`].
    pub fn top(&self, ctx: &egui::Context) -> Option<Id> {
        crate::overlay::top_modal(ctx)
            .map(|l| l.id)
            .filter(|id| self.is_open(*id))
    }

    /// `id` anywhere in the stack.
    pub fn is_open(&self, id: Id) -> bool {
        self.stack.contains(&id)
    }

    /// `content` returns `true` to close its modal; backdrop click and Escape also close.
    pub fn show<K>(&mut self, rt: &Runtime<K>, mut content: impl FnMut(&mut MineralUi<'_, K>, Id) -> bool) {
        let mut closed = Vec::new();
        for &id in &self.stack {
            // The default frame reads the app-wide style.
            let r = egui::Modal::new(id)
                .area(crate::kit::overlay::modal_area(rt.core(), id))
                .frame(Frame::popup(&rt.style()))
                .show(rt.ctx(), |ui| content(&mut rt.ui(ui), id));
            if r.inner || r.should_close() {
                closed.push(id);
            }
        }
        self.stack.retain(|id| !closed.contains(id));
    }
}

#[cfg(test)]
mod tests {
    use egui::{LayerId, Order};

    use std::sync::{
        Arc,
        atomic::{AtomicU32, Ordering},
    };

    use egui::{Context, Event, Pos2, Rect, TouchPhase, pos2};

    use super::*;
    use crate::{
        input::synth,
        shortcut::{Scope, scope_of},
        test_support::{ctx, input, run},
    };

    struct GlyphKit(Arc<AtomicU32>);

    impl Stylekit for GlyphKit {
        fn policies(&self, _: &Context) -> crate::policy::Policies {
            crate::test_support::policies()
        }
        fn catalogs(&self) -> &'static [crate::i18n::Catalog] {
            &[]
        }
        fn setup(&self, _: &Context) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
            Ok(())
        }
        fn style(&self, ctx: &Context, theme: egui::Theme) -> Arc<egui::Style> {
            ctx.style_of(theme)
        }
        fn paint_window_close(&self, _: &Ui, visual: Rect, _: &egui::Response) {
            self.0.store(visual.width().to_bits(), Ordering::Relaxed);
        }
    }

    struct Fixture {
        ctx: Context,
        rt: Arc<Runtime<GlyphKit>>,
        glyph: Arc<AtomicU32>,
        open: bool,
        time: f64,
    }

    impl Fixture {
        fn new() -> Self {
            let ctx = ctx();
            let glyph = Arc::new(AtomicU32::new(0));
            let rt = Runtime::new(&ctx, GlyphKit(glyph.clone())).expect("runtime");
            rt.set_collect_semantics(true);
            let mut f = Self {
                ctx,
                rt,
                glyph,
                open: true,
                time: 0.0,
            };
            f.frame(vec![]);
            f.frame(vec![]);
            f
        }

        fn frame(&mut self, events: Vec<Event>) {
            let (rt, open) = (&self.rt, &mut self.open);
            self.time += 0.05;
            run(&self.ctx, crate::test_support::at(self.time, events), |ui| {
                rt.pass(ui, |_| {
                    WindowSpec::new("w", "Window")
                        .size(vec2(320.0, 140.0))
                        .show(rt, open, |ui| {
                            ui.label("body");
                        });
                });
            });
        }

        fn close_rect(&self) -> Rect {
            let tree = self.rt.tree(ViewportId::ROOT).expect("tree");
            tree.find_key("w-close").expect("close node").rect
        }

        fn glyph(&self) -> f32 {
            f32::from_bits(self.glyph.load(Ordering::Relaxed))
        }

        fn window(&self) -> Rect {
            self.ctx
                .memory(|m| {
                    m.layer_ids()
                        .find_map(|l| m.area_rect(l.id).filter(|_| l.order == egui::Order::Middle))
                })
                .expect("window area")
        }
    }

    fn touch(pos: Pos2, phase: TouchPhase) -> Event {
        synth::touch(pos, phase)
    }

    #[test]
    fn touch_target_grows_around_a_normal_glyph() {
        let mut f = Fixture::new();
        let icon = f.ctx.global_style().spacing.icon_width;
        let mouse = f.close_rect();
        assert!(
            mouse.width() < f.rt.core().policies().target.touch,
            "mouse target {mouse:?}"
        );
        f.frame(vec![touch(pos2(5.0, 5.0), TouchPhase::Start)]);
        f.frame(vec![touch(pos2(5.0, 5.0), TouchPhase::End)]);
        let target = f.rt.core().policies().target.touch;
        let hit = f.close_rect();
        assert!(hit.width() >= target && hit.height() >= target, "touch target {hit:?}");
        assert_eq!(f.glyph(), icon);
        assert!(hit.contains_rect(Rect::from_center_size(mouse.center(), Vec2::splat(icon))));
    }

    #[test]
    fn first_touch_taps_the_edge_of_the_touch_target() {
        let mut touched = Fixture::new();
        touched.frame(vec![touch(pos2(5.0, 5.0), TouchPhase::Start)]);
        touched.frame(vec![touch(pos2(5.0, 5.0), TouchPhase::End)]);
        let edge = touched.close_rect().min + vec2(1.0, 1.0);

        let mut f = Fixture::new();
        assert!(!f.close_rect().contains(edge), "edge lies outside the mouse target");
        // Web order: the press precedes the touchstart, and the release shares its pass.
        f.frame(vec![synth::pointer_down(edge, egui::PointerButton::Primary)]);
        f.frame(vec![
            touch(edge, TouchPhase::Start),
            synth::pointer_up(edge, egui::PointerButton::Primary),
            touch(edge, TouchPhase::End),
        ]);
        assert!(!f.open, "tap in the pass that sets the modality closes");
    }

    #[test]
    fn title_drag_moves_the_window() {
        let mut f = Fixture::new();
        let before = f.window();
        let grab = pos2(before.left() + 20.0, before.top() + 10.0);
        f.frame(crate::test_support::press(grab));
        f.frame(vec![Event::PointerMoved(grab + vec2(40.0, 30.0))]);
        f.frame(vec![Event::PointerMoved(grab + vec2(40.0, 30.0))]);
        f.frame(vec![synth::pointer_up(
            grab + vec2(40.0, 30.0),
            egui::PointerButton::Primary,
        )]);
        f.frame(vec![]);
        assert_eq!(f.window().min - before.min, vec2(40.0, 30.0));
        assert!(f.open);
    }

    #[test]
    fn top_and_shortcut_scope_agree() {
        let ctx = ctx();
        let rt = crate::test_support::runtime(&ctx);
        let (a, b) = (Id::unique("modal-a"), Id::unique("modal-b"));
        let mut stack = ModalStack::default();
        stack.push(a);
        stack.push(b);
        run(&ctx, input(vec![]), |_| stack.show(&rt, |_, _| false));
        let layer = |id| LayerId::new(Order::Foreground, id);
        assert_eq!(stack.top(&ctx), Some(b));
        assert_eq!(scope_of(&ctx, layer(b)), Scope::Modal(b));
        assert_eq!(scope_of(&ctx, layer(a)), Scope::Global, "covered modal");
        stack.pop();
        run(&ctx, input(vec![]), |_| stack.show(&rt, |_, _| false));
        assert_eq!(stack.top(&ctx), Some(a));
        assert_eq!(scope_of(&ctx, layer(a)), Scope::Modal(a));
    }
}
