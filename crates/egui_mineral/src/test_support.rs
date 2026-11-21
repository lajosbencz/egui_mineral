use std::sync::Arc;

use egui::{Context, Event, FullOutput, Key, Modifiers, PointerButton, Pos2, RawInput, Ui};

use crate::{
    i18n::{I18n, LocaleFormat},
    input::{GestureConfig, synth},
    motion::Fling,
    policy::Policies,
    runtime::{Runtime, Stylekit},
    widget::MineralUi,
};

pub(crate) const FLING: Fling = Fling {
    deceleration: 0.998,
    dismiss_fraction: crate::layout::Fraction::HALF,
    dismiss_velocity: 1000.0,
};

pub(crate) const GESTURES: GestureConfig = GestureConfig {
    slop: 8.0,
    long_press: std::time::Duration::from_millis(500),
    swipe_min_velocity: 600.0,
};

pub(crate) fn policies() -> Policies {
    Policies::baseline()
}

pub(crate) struct TestKit(pub(crate) Policies);

impl Stylekit for TestKit {
    fn policies(&self, _: &Context) -> Policies {
        self.0.clone()
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
}

pub(crate) fn ctx() -> Context {
    Context::default()
}

#[cfg(feature = "icu")]
pub(crate) fn en_us() -> impl LocaleFormat + 'static {
    crate::i18n::IcuFormat::new(&crate::i18n::locale!("en-US")).unwrap()
}

#[cfg(not(feature = "icu"))]
pub(crate) fn en_us() -> impl LocaleFormat + 'static {
    crate::i18n::AsciiFormat::new(&crate::i18n::locale!("en-US"))
}

pub(crate) fn runtime(ctx: &Context) -> Arc<Runtime<TestKit>> {
    runtime_with(ctx, policies())
}

pub(crate) fn runtime_with(ctx: &Context, policies: Policies) -> Arc<Runtime<TestKit>> {
    let i18n = I18n::with_format(ctx, crate::i18n::locale!("en-US"), en_us(), &[]).unwrap();
    Runtime::with_i18n(ctx, TestKit(policies), i18n).unwrap()
}

pub(crate) fn input(events: Vec<Event>) -> RawInput {
    RawInput {
        events,
        ..RawInput::default()
    }
}

pub(crate) fn at(time: f64, events: Vec<Event>) -> RawInput {
    RawInput {
        time: Some(time),
        ..input(events)
    }
}

pub(crate) fn run(ctx: &Context, raw: RawInput, add: impl FnMut(&mut Ui)) -> FullOutput {
    let mut out = ctx.run_ui(raw, add);
    out.textures_delta.clear();
    out
}

pub(crate) struct Run<S> {
    rt: Option<Arc<Runtime<TestKit>>>,
    pub(crate) value: S,
}

pub(crate) fn harness<'a, S>(
    mut app: impl FnMut(&mut MineralUi<'_, TestKit>, &mut S) + 'a,
    value: S,
) -> egui_kittest::Harness<'a, Run<S>> {
    egui_kittest::Harness::builder()
        .with_step_dt(RawInput::default().predicted_dt)
        .build_ui_state(
            move |ui, run: &mut Run<S>| {
                // The harness owns the context, so the runtime starts in the first pass.
                let rt = run.rt.get_or_insert_with(|| runtime(ui.ctx()));
                rt.pass(ui, |ui| app(ui, &mut run.value));
            },
            Run { rt: None, value },
        )
}

pub(crate) fn tap(key: Key, modifiers: Modifiers) -> Vec<Event> {
    synth::key_tap(key, modifiers).into_iter().collect()
}

pub(crate) fn press(pos: Pos2) -> Vec<Event> {
    vec![
        Event::PointerMoved(pos),
        synth::pointer_down(pos, PointerButton::Primary),
    ]
}
