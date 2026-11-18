//! One runtime per native window in a shared `Context`: state, style, semantics and agent input stay per window.

use std::{sync::Arc, time::Instant};

use egui::{Context, RawInput, Rect, ViewportId, pos2};
use egui_mineral::{
    agent::{AgentAction, AgentError},
    toast::ToastQueue,
    window::WindowSpec,
};
use egui_mineral_material::{Rt, communication::Snackbar, prelude::*};

use crate::support::{AGENT_DEADLINE, MAX_STEPS, SCREEN, run, runtime};

fn screen(viewport_id: ViewportId, time: f64) -> RawInput {
    RawInput {
        viewport_id,
        time: Some(time),
        viewports: std::iter::once((viewport_id, egui::ViewportInfo::default())).collect(),
        screen_rect: Some(Rect::from_min_size(pos2(0.0, 0.0), SCREEN)),
        ..RawInput::default()
    }
}

fn native_windows() -> Context {
    let ctx = Context::default();
    ctx.set_embed_viewports(false);
    Context::set_immediate_viewport_renderer(|ctx, mut viewport| {
        let time = ctx.input(|i| i.time);
        run(ctx, screen(viewport.ids.this, time), |ui| (viewport.viewport_ui_cb)(ui));
    });
    ctx
}

#[derive(Default)]
struct Window {
    dark: Option<bool>,
    clicks: u32,
    viewport: Option<ViewportId>,
}

struct Two {
    a: Arc<Rt>,
    b: Arc<Rt>,
    win: [Window; 2],
    toasts: [ToastQueue<Snackbar>; 2],
    second: bool,
    frames: u32,
}

impl Two {
    fn new(ctx: &Context) -> Self {
        Self {
            a: runtime(ctx, egui::Theme::Light),
            b: runtime(ctx, egui::Theme::Dark),
            win: Default::default(),
            toasts: [snackbar_queue(), snackbar_queue()],
            second: true,
            frames: 0,
        }
    }

    fn frame(&mut self, ctx: &Context) {
        self.frames += 1;
        let time = f64::from(self.frames) / 60.0;
        let Self {
            a,
            b,
            win,
            toasts,
            second,
            ..
        } = self;
        let [wa, wb] = win;
        let [qa, qb] = toasts;
        run(ctx, screen(ViewportId::ROOT, time), |ui| {
            a.pass(ui, |ui| window(ui, wa, qa));
            if *second {
                WindowSpec::new("second", "Second").show(b, &mut true, |ui| window(ui, wb, qb));
            }
        });
    }

    fn frames_until(&mut self, ctx: &Context, what: &str, mut done: impl FnMut(&Self) -> bool) {
        for _ in 0..MAX_STEPS {
            self.frame(ctx);
            if done(self) {
                return;
            }
        }
        panic!("{what}: not reached in {MAX_STEPS} frames");
    }

    fn b_viewport(&self) -> ViewportId {
        self.win[1].viewport.expect("second window ran")
    }
}

fn window(ui: &mut MineralUi<'_, Material>, w: &mut Window, toasts: &mut ToastQueue<Snackbar>) {
    w.dark = Some(ui.visuals().dark_mode);
    w.viewport = Some(ui.ctx().viewport_id());
    if ui.mineral(PushButton::new("save", "Save")).clicked() {
        w.clicks += 1;
    }
    SnackbarHost::new(toasts).show(ui.rt());
}

fn settled_toast(second: bool) -> Rect {
    const STILL: usize = 3;
    let ctx = native_windows();
    let mut two = Two::new(&ctx);
    two.second = second;
    two.toasts[0].push(Snackbar::new("Saved in A"));
    let toast = |two: &Two| {
        let tree = two.a.tree(ViewportId::ROOT)?;
        tree.nodes
            .iter()
            .find(|n| n.name.contains("Saved in A"))
            .map(|n| n.rect)
    };
    let mut recent = Vec::new();
    two.frames_until(&ctx, "toast settles", |two| {
        recent.push(toast(two));
        recent.len() > STILL
            && recent[recent.len() - STILL..]
                .windows(2)
                .all(|w| w[0].is_some() && w[0] == w[1])
    });
    toast(&two).expect("toast shown")
}

#[test]
fn windows_keep_their_own_style_and_semantics() {
    let ctx = native_windows();
    let global = ctx.global_style();
    let mut two = Two::new(&ctx);
    two.frames_until(&ctx, "both windows publish trees", |two| {
        two.win[1]
            .viewport
            .is_some_and(|vb| two.a.tree(ViewportId::ROOT).is_some() && two.b.tree(vb).is_some())
    });
    let vb = two.b_viewport();
    assert_ne!(vb, ViewportId::ROOT, "second window is a native viewport");
    assert_eq!([two.win[0].dark, two.win[1].dark], [Some(false), Some(true)]);
    assert_eq!(
        ctx.global_style().visuals,
        global.visuals,
        "egui's app-wide style untouched"
    );
    let (ta, tb) = (
        two.a.tree(ViewportId::ROOT).expect("tree of a"),
        two.b.tree(vb).expect("tree of b"),
    );
    assert!(ta.find_key("save-button").is_some() && tb.find_key("save-button").is_some());
    assert!(two.a.tree(vb).is_none() && two.b.tree(ViewportId::ROOT).is_none());
}

#[test]
fn shared_widget_state_stays_in_its_window() {
    // Same widget ids in both windows: a shared presence would restart a's enter animation every pass.
    assert_eq!(settled_toast(true), settled_toast(false));
}

#[test]
fn agent_input_reaches_only_its_window() {
    let ctx = native_windows();
    let mut two = Two::new(&ctx);
    two.frames_until(&ctx, "second window runs", |two| two.win[1].viewport.is_some());
    let agent = two.b.agent(two.b_viewport());
    #[expect(clippy::disallowed_methods, reason = "agent client thread, joined below")]
    let session = std::thread::spawn(move || {
        futures_executor::block_on(async {
            agent.snapshot().await?;
            agent.act("save-button", AgentAction::Click).await
        })
    });
    let deadline = Instant::now() + AGENT_DEADLINE;
    while !session.is_finished() && Instant::now() < deadline {
        two.frame(&ctx);
        std::thread::yield_now();
    }
    assert!(session.is_finished(), "agent session still running after the deadline");
    session
        .join()
        .expect("agent thread")
        .unwrap_or_else(|e: AgentError| panic!("{e}"));
    two.frames_until(&ctx, "the click lands", |two| two.win[1].clicks > 0);
    two.frame(&ctx);
    assert_eq!([two.win[0].clicks, two.win[1].clicks], [0, 1]);
}

#[test]
fn embedded_window_runtimes_keep_their_trees() {
    // Default `Context`: viewports embed, so b's pass nests inside a's in one viewport.
    let ctx = Context::default();
    let two = Two::new(&ctx);
    let published = || two.a.tree(ViewportId::ROOT).is_some() && two.b.tree(ViewportId::ROOT).is_some();
    for _ in 0..MAX_STEPS {
        if published() {
            break;
        }
        run(&ctx, screen(ViewportId::ROOT, 0.0), |ui| {
            two.a.pass(ui, |ui| {
                ui.mineral(PushButton::new("a-first", "A1"));
                WindowSpec::new("second", "Second").show(&two.b, &mut true, |ui| {
                    ui.mineral(PushButton::new("b-only", "B"));
                });
                ui.mineral(PushButton::new("a-last", "A2"));
            });
        });
    }
    assert!(published(), "both runtimes publish a tree");
    let keys = |rt: &Rt| -> Vec<String> {
        rt.tree(ViewportId::ROOT)
            .map_or_default(|t| t.nodes.iter().map(|n| n.id.key().to_owned()).collect())
    };
    assert_eq!(keys(&two.a), ["a-first-button", "a-last-button"]);
    assert_eq!(keys(&two.b), ["b-only-button"]);
}
