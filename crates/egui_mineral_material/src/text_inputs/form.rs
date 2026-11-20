use crate::Material;
use crate::MaterialExt;
use crate::msg;
use egui_mineral::form::{FieldCtx, FieldKind, FieldValue, Form, FormFooter, FormResponse};
use egui_mineral::select::Selection;
use egui_mineral::widget::MineralUi;

use crate::actions::PushButton;
use crate::selection::{CheckButton, DatePicker, DisplayMode, Select};
use crate::spec::tokens::filled_text_field as m;
use crate::text_inputs::TextField;
use crate::widget::parts;

mod sealed {
    pub trait Sealed {}
    impl Sealed for egui_mineral::form::Form<'_> {}
}

/// Shows a core [`Form`] with stylekit fields; sealed.
pub trait FormExt: sealed::Sealed {
    /// Paints text kinds as [`TextField`]s, checkboxes as [`CheckButton`]s, selects as [`Select`]s and dates as
    /// docked [`DatePicker`]s, and the submit as a filled [`PushButton`]; form errors use the field error style.
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
        .map_or_else(|| crate::tr(rt.i18n(), msg::SUBMIT), str::to_owned);
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
    let salt = f.form_salt.child(spec.name().to_owned());
    let options = || spec.field_options().iter().map(|o| (o.key.clone(), o.label.clone()));
    let (response, changed, lost_focus) = match f.state.value_mut() {
        FieldValue::Text(text) => {
            let mut tf = TextField::new(salt, label, text)
                .masked(spec.input_kind() == FieldKind::Password)
                .rows(if spec.input_kind() == FieldKind::Multiline {
                    3
                } else {
                    1
                })
                .maybe_error(error);
            if let Some(h) = spec.supporting_text() {
                tf = tf.supporting(h);
            }
            let r = rt.ui(ui).mineral(tf);
            return text_result(f, r.changed(), r.lost_focus(), r.into_response());
        }
        FieldValue::Bool(checked) => {
            let has_error = error.is_some();
            let r = ui
                .scope(|ui| {
                    let r = rt
                        .ui(ui)
                        .mineral(CheckButton::new(salt, label, checked).maybe_error(error));
                    if let Some(h) = spec.supporting_text().filter(|_| !has_error) {
                        let t = rt.tokens();
                        parts::type_label(ui, h, m::SUPPORTING_FONT, t.role(m::SUPPORTING_COLOR), 0);
                    }
                    r
                })
                .inner;
            (r.response().clone(), r.changed(), r.lost_focus())
        }
        FieldValue::Choice(picked) => {
            let mut select = Select::optional(salt, label, picked)
                .items(options())
                .maybe_error(error);
            if let Some(h) = spec.supporting_text() {
                select = select.supporting(h);
            }
            let r = rt.ui(ui).mineral(select);
            (r.response().clone(), r.changed(), r.lost_focus())
        }
        FieldValue::Choices(keys) => {
            let mut picked: Selection<String> = keys.iter().cloned().collect();
            let mut select = Select::multi(salt, label, &mut picked)
                .items(options())
                .maybe_error(error);
            if let Some(h) = spec.supporting_text() {
                select = select.supporting(h);
            }
            let r = rt.ui(ui).mineral(select);
            if r.changed() {
                *keys = spec
                    .field_options()
                    .iter()
                    .filter(|o| picked.is_selected(&o.key))
                    .map(|o| o.key.clone())
                    .collect();
            }
            (r.response().clone(), r.changed(), r.lost_focus())
        }
        FieldValue::Date(day) => {
            let mut picker = DatePicker::new(salt, label, day)
                .input_mode(DisplayMode::Input)
                .maybe_error(error);
            if let Some(h) = spec.supporting_text() {
                picker = picker.supporting(h);
            }
            let r = picker.show(&mut rt.ui(ui));
            (r.response, r.changed, false)
        }
        // Kinds this renderer does not know take no space.
        _ => return ui.allocate_response(egui::Vec2::ZERO, egui::Sense::hover()),
    };
    // A pick commits the field, as a blur does for text.
    if changed {
        f.changed();
        f.blurred();
    } else if lost_focus {
        f.blurred();
    }
    response
}

fn text_result(f: &mut FieldCtx<'_>, changed: bool, lost_focus: bool, response: egui::Response) -> egui::Response {
    if changed {
        f.changed();
    }
    if lost_focus {
        f.blurred();
    }
    response
}
