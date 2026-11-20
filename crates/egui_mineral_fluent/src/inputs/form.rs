use egui_mineral::form::{FieldCtx, FieldKind, Form, FormFooter, FormResponse};
use egui_mineral::widget::MineralUi;

use crate::Fluent;
use crate::actions::{ButtonAppearance, PushButton};
use crate::feedback::{MessageBar, MessageBarIntent};
use crate::inputs::{Field, Input, Textarea, Validation};
use crate::msg;

mod sealed {
    pub trait Sealed {}
    impl Sealed for egui_mineral::form::Form<'_> {}
}

/// Shows a core [`Form`] with stylekit inputs; sealed.
pub trait FormExt: sealed::Sealed {
    /// Wraps each input in a [`Field`] carrying its label, hint and error, and the submit in a primary
    /// [`PushButton`]; form errors show in an error [`MessageBar`].
    fn show(self, ui: &mut MineralUi<'_, Fluent>) -> FormResponse;
}

impl FormExt for Form<'_> {
    fn show(self, ui: &mut MineralUi<'_, Fluent>) -> FormResponse {
        self.show_with(ui, field, footer)
    }
}

fn footer(ui: &mut MineralUi<'_, Fluent>, f: &FormFooter<'_>) -> bool {
    let rt = ui.rt();
    if let Some(e) = f.error {
        let message = e.message(ui.i18n());
        MessageBar::new(f.form_salt.child("error"), message)
            .intent(MessageBarIntent::Error)
            .show(ui);
    }
    let label = f
        .submit_label
        .map_or_else(|| crate::tr(rt.i18n(), msg::SUBMIT), str::to_owned);
    ui.mineral(
        PushButton::new(f.form_salt.child("submit"), label)
            .appearance(ButtonAppearance::Primary)
            .disabled(!f.enabled),
    )
    .clicked()
}

fn field(ui: &mut MineralUi<'_, Fluent>, f: &mut FieldCtx<'_>) -> egui::Response {
    let error = f.visible_error().map(|e| e.message(ui.i18n()));
    let spec = f.spec;
    let salt = f.form_salt.child(spec.name().to_owned());
    let mut field = Field::new(salt.child("field"))
        .label(spec.label())
        .required(spec.is_required());
    if let Some(h) = spec.supporting_text() {
        field = field.hint(h);
    }
    if let Some(e) = error {
        field = field.validation(Validation::Error, e);
    }
    let value = f.state.value_mut();
    let r = field
        .show(ui, |ui, cx| match spec.input_kind() {
            FieldKind::Multiline => ui.mineral(
                Textarea::new(salt, spec.label(), value)
                    .rows(3)
                    .validation(cx.validation),
            ),
            kind => ui.mineral(
                Input::new(salt, spec.label(), value)
                    .password(kind == FieldKind::Password)
                    .validation(cx.validation),
            ),
        })
        .inner;
    if r.changed() {
        f.changed();
    }
    if r.lost_focus() {
        f.blurred();
    }
    r.into_response()
}
