use std::sync::Arc;

use egui::{Context, Event, FullOutput, Pos2, RawInput, Rect, Ui, Vec2};
use egui_mineral::{i18n::I18n, runtime::Runtime, widget::MineralUi};

use crate::{Material, Rt};

pub(crate) fn runtime(ctx: &Context) -> Arc<Rt> {
    #[cfg(feature = "icu")]
    let format = egui_mineral::i18n::IcuFormat::new(&egui_mineral::i18n::locale!("en-US")).expect("en-US ICU data");
    #[cfg(not(feature = "icu"))]
    let format = egui_mineral::i18n::AsciiFormat::new(&egui_mineral::i18n::locale!("en-US"));
    let i18n = I18n::with_format(ctx, egui_mineral::i18n::locale!("en-US"), format, &[]).expect("i18n");
    let rt = Runtime::with_i18n(ctx, Material::default(), i18n).expect("material runtime");
    rt.core().set_preference(egui::ThemePreference::Light);
    rt
}

pub(crate) fn run(ctx: &Context, raw: RawInput, add: impl FnMut(&mut Ui)) -> FullOutput {
    let mut out = ctx.run_ui(raw, add);
    out.textures_delta.clear();
    out
}

pub(crate) fn pass(
    ctx: &Context,
    rt: &Rt,
    size: Vec2,
    events: Vec<Event>,
    mut add: impl FnMut(&mut MineralUi<'_, Material>),
) {
    let raw = RawInput {
        screen_rect: Some(Rect::from_min_size(Pos2::ZERO, size)),
        events,
        ..RawInput::default()
    };
    run(ctx, raw, |ui| rt.pass(ui, &mut add));
}
