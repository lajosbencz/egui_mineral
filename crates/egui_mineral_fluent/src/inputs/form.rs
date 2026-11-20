use egui_mineral::form::{FieldCtx, FieldKind, FieldValue, Form, FormFooter, FormResponse};
use egui_mineral::select::Selection;
use egui_mineral::widget::MineralUi;

use crate::Fluent;
use crate::actions::{ButtonAppearance, PushButton};
use crate::feedback::{MessageBar, MessageBarIntent};
use crate::inputs::{Checkbox, DatePicker, Dropdown, Field, Input, Textarea, Validation};
use crate::msg;

mod sealed {
    pub trait Sealed {}
    impl Sealed for egui_mineral::form::Form<'_> {}
}

/// Shows a core [`Form`] with stylekit inputs; sealed.
pub trait FormExt: sealed::Sealed {
    /// Wraps each control in a [`Field`] carrying its label, hint and error: text kinds as [`Input`] or
    /// [`Textarea`], checkboxes as [`Checkbox`] (labeled by itself), selects as [`Dropdown`]s and dates as
    /// [`DatePicker`]s; the submit is a primary [`PushButton`], form errors show in an error [`MessageBar`].
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
    let checkbox = spec.input_kind() == FieldKind::Checkbox;
    let mut field = Field::new(salt.child("field")).required(spec.is_required());
    // A checkbox is labeled by its own text.
    if !checkbox {
        field = field.label(spec.label());
    }
    if let Some(h) = spec.supporting_text() {
        field = field.hint(h);
    }
    if let Some(e) = error {
        field = field.validation(Validation::Error, e);
    }
    let options = || spec.field_options().iter().map(|o| (o.key.clone(), o.label.clone()));
    let value = f.state.value_mut();
    let (r, text) = field
        .show(ui, |ui, cx| match value {
            FieldValue::Text(text) => {
                let r = match spec.input_kind() {
                    FieldKind::Multiline => ui.mineral(
                        Textarea::new(salt, spec.label(), text)
                            .rows(3)
                            .validation(cx.validation),
                    ),
                    kind => ui.mineral(
                        Input::new(salt, spec.label(), text)
                            .password(kind == FieldKind::Password)
                            .validation(cx.validation),
                    ),
                };
                (r.into_response(), true)
            }
            FieldValue::Bool(checked) => (
                ui.mineral(Checkbox::new(salt, spec.label(), checked)).into_response(),
                false,
            ),
            FieldValue::Choice(picked) => (
                ui.mineral(
                    Dropdown::optional(salt, spec.label(), picked)
                        .items(options())
                        .validation(cx.validation),
                )
                .into_response(),
                false,
            ),
            FieldValue::Choices(keys) => {
                let mut picked: Selection<String> = keys.iter().cloned().collect();
                let r = ui
                    .mineral(
                        Dropdown::multi(salt, spec.label(), &mut picked)
                            .items(options())
                            .validation(cx.validation),
                    )
                    .into_response();
                if r.changed() {
                    *keys = spec
                        .field_options()
                        .iter()
                        .filter(|o| picked.is_selected(&o.key))
                        .map(|o| o.key.clone())
                        .collect();
                }
                (r, false)
            }
            FieldValue::Date(day) => (
                ui.mineral(DatePicker::new(salt, spec.label(), day).validation(cx.validation))
                    .into_response(),
                false,
            ),
            // Kinds this renderer does not know take no space.
            _ => (ui.allocate_response(egui::Vec2::ZERO, egui::Sense::hover()), false),
        })
        .inner;
    if r.changed() {
        f.changed();
        // A pick commits the field, as a blur does for text.
        if !text {
            f.blurred();
        }
    }
    if r.lost_focus() {
        f.blurred();
    }
    r
}
