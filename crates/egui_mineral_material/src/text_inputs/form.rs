use std::num::NonZeroUsize;

use crate::Material;
use crate::MaterialExt;
use egui_mineral::form::{FieldCtx, FieldKind, Form, FormFooter, FormResponse};
use egui_mineral::widget::MineralUi;

use crate::actions::PushButton;
use crate::spec::tokens::filled_text_field as m;
use crate::text_inputs::TextField;
use crate::widget::parts;

/// Shows a core [`Form`] with stylekit text fields.
pub trait FormExt {
    /// Paints the fields as [`TextField`]s and the submit as a filled [`PushButton`]; form errors use the field
    /// error style.
    fn show(self, ui: &mut MineralUi<'_, crate::Material>) -> FormResponse;
}

impl FormExt for Form<'_> {
    fn show(self, ui: &mut MineralUi<'_, crate::Material>) -> FormResponse {
        self.show_with(ui, field, footer)
    }
}

fn footer(ui: &mut MineralUi<'_, Material>, f: &FormFooter<'_>) -> bool {
    let rt = ui.rt();
    // Form-level errors read as a text field's error supporting text.
    if let Some(e) = f.error {
        let t = rt.tokens();
        let color = t.role(m::ERROR_SUPPORTING_COLOR);
        let message = e.message(ui.i18n());
        parts::type_label(ui, &message, m::SUPPORTING_FONT, color, 0);
    }
    let label = f
        .submit_label
        .map_or_else(|| crate::tr(rt.i18n(), "mineral-submit"), str::to_owned);
    rt.ui(ui)
        .mineral(PushButton::new(f.form_salt.child("submit"), label).disabled(!f.enabled))
        .clicked()
}

fn field(ui: &mut MineralUi<'_, Material>, f: &mut FieldCtx<'_>) -> egui::Response {
    let rt = ui.rt();
    let error = f.visible_error().map(|e| e.message(ui.i18n()));
    let spec = f.spec;
    let label = if spec.is_required() {
        format!("{}*", spec.label())
    } else {
        spec.label().to_owned()
    };
    let mut tf = TextField::new(f.form_salt.child(spec.name().to_owned()), label, f.state.value_mut())
        .masked(spec.input_kind() == FieldKind::Password)
        .rows(if spec.input_kind() == FieldKind::Multiline {
            NonZeroUsize::MIN.saturating_add(2)
        } else {
            NonZeroUsize::MIN
        })
        .maybe_error(error);
    if let Some(h) = spec.supporting_text() {
        tf = tf.supporting(h);
    }
    let r = rt.ui(ui).mineral(tf);
    if r.changed() {
        f.changed();
    }
    if r.lost_focus() {
        f.blurred();
    }
    r.into_response()
}
