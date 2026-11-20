use egui::emath::Numeric;
use egui_mineral::date::Date;
use egui_mineral::form::{Blank, FieldCtx, FieldSpec, Form, FormFooter, FormResponse, FormUi};
use egui_mineral::range::RangeModel;
use egui_mineral::select::{ItemKey, Selection};
use egui_mineral::semantic::KeyedSalt;
use egui_mineral::widget::MineralUi;

use crate::Fluent;
use crate::actions::{ButtonAppearance, PushButton};
use crate::feedback::{MessageBar, MessageBarIntent};
use crate::inputs::{Checkbox, DatePicker, Dropdown, Field, FieldContext, Input, SpinButton, Textarea, Validation};
use crate::msg;

mod sealed {
    pub trait Sealed {}
    impl<T> Sealed for egui_mineral::form::Form<'_, T> {}
    impl Sealed for egui_mineral::form::FormUi<'_, '_, crate::Fluent> {}
}

/// Shows a core [`Form`] with stylekit inputs; sealed.
pub trait FormExt<T>: sealed::Sealed {
    /// `add` places the fields with [`FormUiExt`]; the submit is a primary [`PushButton`] after them, form errors
    /// show in an error [`MessageBar`].
    fn show<R>(
        self,
        ui: &mut MineralUi<'_, Fluent>,
        add: impl FnOnce(&mut FormUi<'_, '_, Fluent>, &mut T) -> R,
    ) -> FormResponse<R>;
}

impl<T> FormExt<T> for Form<'_, T> {
    fn show<R>(
        self,
        ui: &mut MineralUi<'_, Fluent>,
        add: impl FnOnce(&mut FormUi<'_, '_, Fluent>, &mut T) -> R,
    ) -> FormResponse<R> {
        self.show_with(ui, add, footer)
    }
}

/// Typed fields of a Fluent form, each control in a [`Field`] carrying its label, hint and error; sealed. `build`
/// adds the spec's required flag, help text and validators.
pub trait FormUiExt: sealed::Sealed {
    /// [`Input`], or a [`Textarea`] past one [`FieldSpec::rows`]; [`FieldSpec::masked`] makes it a password input.
    /// Enter in an [`Input`] submits.
    fn text(
        &mut self,
        salt: impl Into<KeyedSalt>,
        label: impl Into<String>,
        value: &mut String,
        build: impl FnOnce(FieldSpec<String>) -> FieldSpec<String>,
    ) -> egui::Response;

    /// [`Checkbox`] labeled by itself; required means checked.
    fn check(
        &mut self,
        salt: impl Into<KeyedSalt>,
        label: impl Into<String>,
        value: &mut bool,
        build: impl FnOnce(FieldSpec<bool>) -> FieldSpec<bool>,
    ) -> egui::Response;

    /// Optional [`Dropdown`] of `(value, label)` options.
    fn select<K: ItemKey>(
        &mut self,
        salt: impl Into<KeyedSalt>,
        label: impl Into<String>,
        value: &mut Option<K>,
        options: impl IntoIterator<Item = (K, impl Into<String>)>,
        build: impl FnOnce(FieldSpec<Option<K>>) -> FieldSpec<Option<K>>,
    ) -> egui::Response;

    /// Multi [`Dropdown`] of `(value, label)` options.
    fn multi_select<K: ItemKey>(
        &mut self,
        salt: impl Into<KeyedSalt>,
        label: impl Into<String>,
        value: &mut Selection<K>,
        options: impl IntoIterator<Item = (K, impl Into<String>)>,
        build: impl FnOnce(FieldSpec<Selection<K>>) -> FieldSpec<Selection<K>>,
    ) -> egui::Response;

    /// [`DatePicker`].
    fn date(
        &mut self,
        salt: impl Into<KeyedSalt>,
        label: impl Into<String>,
        value: &mut Option<Date>,
        build: impl FnOnce(FieldSpec<Option<Date>>) -> FieldSpec<Option<Date>>,
    ) -> egui::Response;

    /// [`SpinButton`] over `range`; a step or committed text commits the field.
    fn number<N: Numeric + Blank>(
        &mut self,
        salt: impl Into<KeyedSalt>,
        label: impl Into<String>,
        value: &mut N,
        range: impl Into<RangeModel>,
        build: impl FnOnce(FieldSpec<N>) -> FieldSpec<N>,
    ) -> egui::Response;
}

impl FormUiExt for FormUi<'_, '_, Fluent> {
    fn text(
        &mut self,
        salt: impl Into<KeyedSalt>,
        label: impl Into<String>,
        value: &mut String,
        build: impl FnOnce(FieldSpec<String>) -> FieldSpec<String>,
    ) -> egui::Response {
        self.field(salt, &build(FieldSpec::new(label)), value, |ui, f| {
            let spec = f.spec();
            let single = spec.row_count() == 1;
            let r = wrapped(ui, f, true, |ui, salt, text, cx| {
                if single {
                    ui.mineral(
                        Input::new(salt, spec.label(), text)
                            .password(spec.is_masked())
                            .validation(cx.validation),
                    )
                    .into_response()
                } else {
                    ui.mineral(
                        Textarea::new(salt, spec.label(), text)
                            .rows(u8::try_from(spec.row_count()).unwrap_or(u8::MAX))
                            .validation(cx.validation),
                    )
                    .into_response()
                }
            });
            if r.changed() {
                f.changed();
            }
            if r.lost_focus() {
                f.blurred();
                if single && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                    f.submit();
                }
            }
            r
        })
    }

    fn check(
        &mut self,
        salt: impl Into<KeyedSalt>,
        label: impl Into<String>,
        value: &mut bool,
        build: impl FnOnce(FieldSpec<bool>) -> FieldSpec<bool>,
    ) -> egui::Response {
        self.field(salt, &build(FieldSpec::new(label)), value, |ui, f| {
            let label = f.spec().label();
            let r = wrapped(ui, f, false, |ui, salt, checked, _| {
                ui.mineral(Checkbox::new(salt, label, checked)).into_response()
            });
            pick(f, &r);
            r
        })
    }

    fn select<K: ItemKey>(
        &mut self,
        salt: impl Into<KeyedSalt>,
        label: impl Into<String>,
        value: &mut Option<K>,
        options: impl IntoIterator<Item = (K, impl Into<String>)>,
        build: impl FnOnce(FieldSpec<Option<K>>) -> FieldSpec<Option<K>>,
    ) -> egui::Response {
        self.field(salt, &build(FieldSpec::new(label)), value, |ui, f| {
            let label = f.spec().label();
            let options = options.into_iter().map(|(k, l)| (k, l.into()));
            let r = wrapped(ui, f, true, |ui, salt, picked, cx| {
                ui.mineral(
                    Dropdown::optional(salt, label, picked)
                        .items(options)
                        .validation(cx.validation),
                )
                .into_response()
            });
            pick(f, &r);
            r
        })
    }

    fn multi_select<K: ItemKey>(
        &mut self,
        salt: impl Into<KeyedSalt>,
        label: impl Into<String>,
        value: &mut Selection<K>,
        options: impl IntoIterator<Item = (K, impl Into<String>)>,
        build: impl FnOnce(FieldSpec<Selection<K>>) -> FieldSpec<Selection<K>>,
    ) -> egui::Response {
        self.field(salt, &build(FieldSpec::new(label)), value, |ui, f| {
            let label = f.spec().label();
            let options = options.into_iter().map(|(k, l)| (k, l.into()));
            let r = wrapped(ui, f, true, |ui, salt, picked, cx| {
                ui.mineral(
                    Dropdown::multi(salt, label, picked)
                        .items(options)
                        .validation(cx.validation),
                )
                .into_response()
            });
            pick(f, &r);
            r
        })
    }

    fn date(
        &mut self,
        salt: impl Into<KeyedSalt>,
        label: impl Into<String>,
        value: &mut Option<Date>,
        build: impl FnOnce(FieldSpec<Option<Date>>) -> FieldSpec<Option<Date>>,
    ) -> egui::Response {
        self.field(salt, &build(FieldSpec::new(label)), value, |ui, f| {
            let label = f.spec().label();
            let r = wrapped(ui, f, true, |ui, salt, day, cx| {
                ui.mineral(DatePicker::new(salt, label, day).validation(cx.validation))
                    .into_response()
            });
            pick(f, &r);
            r
        })
    }

    fn number<N: Numeric + Blank>(
        &mut self,
        salt: impl Into<KeyedSalt>,
        label: impl Into<String>,
        value: &mut N,
        range: impl Into<RangeModel>,
        build: impl FnOnce(FieldSpec<N>) -> FieldSpec<N>,
    ) -> egui::Response {
        let range = range.into();
        self.field(salt, &build(FieldSpec::new(label)), value, |ui, f| {
            let label = f.spec().label();
            let r = wrapped(ui, f, true, |ui, salt, n, cx| {
                ui.mineral(SpinButton::new(salt, label, range, n).validation(cx.validation))
                    .into_response()
            });
            pick(f, &r);
            r
        })
    }
}

// The control in a `Field` with the spec's label (unless the control labels itself), hint and shown error.
fn wrapped<V: ?Sized>(
    ui: &mut MineralUi<'_, Fluent>,
    f: &mut FieldCtx<'_, V>,
    labeled: bool,
    control: impl FnOnce(&mut MineralUi<'_, Fluent>, KeyedSalt, &mut V, FieldContext) -> egui::Response,
) -> egui::Response {
    let spec = f.spec();
    let salt = f.salt();
    let mut field = Field::new(salt.child("field")).required(spec.is_required());
    if labeled {
        field = field.label(spec.label());
    }
    if let Some(h) = spec.supporting_text() {
        field = field.hint(h);
    }
    if let Some(e) = f.visible_error() {
        field = field.validation(Validation::Error, e.message(ui.i18n()));
    }
    let value = f.value();
    field.show(ui, |ui, cx| control(ui, salt, value, cx)).inner
}

// A pick commits the field, as a blur does for text.
fn pick<V: ?Sized>(f: &mut FieldCtx<'_, V>, r: &egui::Response) {
    if r.changed() {
        f.committed();
    } else if r.lost_focus() {
        f.blurred();
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
