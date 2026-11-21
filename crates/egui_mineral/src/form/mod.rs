//! Form validation and submission over app-owned values. Headless: a [`Form`] binds the app's value `T`, each field
//! binds a `&mut V` part of it through [`FormUi::field`] with typed [`Validator`]s, and stylekits render the fields
//! and the footer through [`Form::show_with`]. [`FormState`] keeps only per-field meta state (dirty, touched,
//! errors) and the submission; the values stay in `T`, so a submit hands back nothing to extract.

mod validators;

use std::{borrow::Cow, collections::HashMap, sync::Arc};

use egui::Id;
use serde::{Deserialize, Serialize};

use crate::{
    date::Date,
    i18n::{self, Arg, LocaleFormat},
    select::Selection,
    semantic::KeyedSalt,
    widget::MineralUi,
};
pub use validators::{MaxLength, MinLength, NumberRange, Required};

/// Field or form error; [`Self::message`] resolves the text for display. Built with [`Self::new`] or
/// [`Self::localized`]; the fields are read-only data.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ValidationError {
    /// Stable code; also the Fluent message id unless `text` is set.
    pub code: Cow<'static, str>,
    /// Fluent arguments for `code`; ignored when `text` is set.
    pub args: Vec<(Cow<'static, str>, Arg<'static>)>,
    /// Literal, already localized text.
    pub text: Option<Cow<'static, str>>,
}

impl ValidationError {
    /// Literal text.
    pub fn new(code: impl Into<Cow<'static, str>>, text: impl Into<Cow<'static, str>>) -> Self {
        Self {
            code: code.into(),
            args: Vec::new(),
            text: Some(text.into()),
        }
    }

    /// Catalog message resolved at display time; its id is the code.
    pub fn localized<'a, const N: usize>(message: impl i18n::IntoMessage<'a, N>) -> Self {
        let message = message.into_message();
        Self {
            code: Cow::Borrowed(message.id()),
            args: message
                .args()
                .iter()
                .map(|(name, value)| (Cow::Owned((*name).to_owned()), value.clone().into_owned()))
                .collect(),
            text: None,
        }
    }

    /// Display text: `text` if set, else the message `code` in the active locale with `args`.
    pub fn message(&self, i18n: &i18n::I18n) -> String {
        if let Some(t) = &self.text {
            return t.to_string();
        }
        let args: Vec<(&str, Arg<'_>)> = self.args.iter().map(|(k, v)| (k.as_ref(), v.clone())).collect();
        i18n.lookup(&self.code, &args)
    }
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.text.as_deref().unwrap_or(&self.code))
    }
}

impl std::error::Error for ValidationError {}

/// Value a field binds; [`Required`] fails and optional fields skip their validators while it is blank. Numbers and
/// dates are never blank.
pub trait Blank {
    /// Blank text, unchecked, nothing picked.
    fn is_blank(&self) -> bool;
}

impl Blank for str {
    fn is_blank(&self) -> bool {
        self.trim().is_empty()
    }
}

impl Blank for String {
    fn is_blank(&self) -> bool {
        self.as_str().is_blank()
    }
}

impl Blank for bool {
    fn is_blank(&self) -> bool {
        !self
    }
}

impl<T> Blank for Option<T> {
    fn is_blank(&self) -> bool {
        self.is_none()
    }
}

impl<T> Blank for Vec<T> {
    fn is_blank(&self) -> bool {
        self.is_empty()
    }
}

impl<K: Ord + Clone> Blank for Selection<K> {
    fn is_blank(&self) -> bool {
        self.is_empty()
    }
}

impl Blank for Date {
    fn is_blank(&self) -> bool {
        false
    }
}

// Numbers always hold a value; a field that may be empty binds `Option<N>`.
macro_rules! never_blank {
    ($($t:ty),*) => {$(
        impl Blank for $t {
            fn is_blank(&self) -> bool {
                false
            }
        }
    )*};
}

never_blank!(i8, i16, i32, i64, i128, isize, u8, u16, u32, u64, u128, usize, f32, f64);

/// Check of a field value of type `V`; `locale` parses locale-formatted input (numbers, dates).
pub trait Validator<V: ?Sized>: Send + Sync {
    /// Never sees a blank value when called through [`FieldSpec::check`].
    ///
    /// # Errors
    /// [`ValidationError`] describing why `value` is rejected.
    fn validate(&self, value: &V, locale: &dyn LocaleFormat) -> Result<(), ValidationError>;
}

impl<V: ?Sized, F: Fn(&V, &dyn LocaleFormat) -> Result<(), ValidationError> + Send + Sync> Validator<V> for F {
    fn validate(&self, value: &V, locale: &dyn LocaleFormat) -> Result<(), ValidationError> {
        self(value, locale)
    }
}

/// When a field's shown error updates on its own (submit always updates every field).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
pub enum ValidateOn {
    /// On every edit and on focus loss.
    Change,
    /// On focus loss.
    #[default]
    Blur,
    /// Only on submit; a field showing an error ([`FieldCtx::visible_error`]) re-validates on each edit until it passes.
    Submit,
}

/// Meta state of one form field; its value lives in the app's `T`.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct FieldState {
    dirty: bool,
    touched: bool,
    #[serde(skip)]
    pending: bool,
    #[serde(skip)]
    error: Option<ValidationError>,
    // Validator result of the last pass, shown on submit.
    #[serde(skip)]
    latest: Option<ValidationError>,
}

impl FieldState {
    /// Edited through its input since the last successful submit; values set from code are not edits.
    pub fn dirty(&self) -> bool {
        self.dirty
    }

    /// Lost focus or went through a submit; gates [`FieldCtx::visible_error`].
    pub fn touched(&self) -> bool {
        self.touched
    }

    /// Awaiting an async (e.g. server-side) check.
    pub fn pending(&self) -> bool {
        self.pending
    }

    /// Marks the field as awaiting an async check; [`FormState::set_error`] clears it.
    pub fn set_pending(&mut self, pending: bool) {
        self.pending = pending;
    }

    /// Shown validation result; not persisted.
    pub fn error(&self) -> Option<&ValidationError> {
        self.error.as_ref()
    }
}

fn field_id(salt: &KeyedSalt) -> Id {
    Id::unique(salt.id_salt())
}

/// Meta state of a [`Form`]: fields by their salt, and the submission.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct FormState {
    fields: HashMap<Id, FieldState>,
    #[serde(skip)]
    submitting: bool,
    #[serde(skip)]
    submit_count: u32,
    #[serde(skip)]
    form_error: Option<ValidationError>,
}

impl FormState {
    /// `None` for a field the form has not shown; [`Form::show_with`] drops fields a pass does not show.
    pub fn field(&self, salt: impl Into<KeyedSalt>) -> Option<&FieldState> {
        self.fields.get(&field_id(&salt.into()))
    }

    /// Inserts a default state on first access.
    pub fn field_mut(&mut self, salt: impl Into<KeyedSalt>) -> &mut FieldState {
        self.fields.entry(field_id(&salt.into())).or_default()
    }

    /// `true` from a submit until [`FormState::finish_submit`]; blocks re-submit.
    pub fn submitting(&self) -> bool {
        self.submitting
    }

    /// Submit attempts, valid or not.
    pub fn submit_count(&self) -> u32 {
        self.submit_count
    }

    /// Error from a cross-field check or a failed submission.
    pub fn form_error(&self) -> Option<&ValidationError> {
        self.form_error.as_ref()
    }

    /// `true` when any field is dirty.
    pub fn is_dirty(&self) -> bool {
        self.fields.values().any(|f| f.dirty)
    }

    /// No shown field or form error and no pending check.
    pub fn is_valid(&self) -> bool {
        self.form_error.is_none() && self.fields.values().all(|f| f.error.is_none() && !f.pending)
    }

    /// Server-side or async validation result of the field `salt`.
    pub fn set_error(&mut self, salt: impl Into<KeyedSalt>, error: Option<ValidationError>) {
        let f = self.field_mut(salt);
        f.error = error;
        f.pending = false;
    }

    /// Ends a submission: `Ok` clears every dirty flag; each `Err` entry shows on its field, or on the form when it
    /// targets the form or a field not shown (the last such entry wins).
    pub fn finish_submit(&mut self, result: Result<(), Vec<CheckError>>) {
        self.submitting = false;
        match result {
            Ok(()) => self.fields.values_mut().for_each(|f| f.dirty = false),
            Err(errors) => errors.into_iter().for_each(|e| self.show_check(e)),
        }
    }

    fn show_check(&mut self, CheckError { field, error }: CheckError) {
        match field.and_then(|id| self.fields.get_mut(&id)) {
            Some(f) => {
                f.touched = true;
                f.error = Some(error);
            }
            None => self.form_error = Some(error),
        }
    }

    /// Clears the meta state of every field and the submission; the app resets its own value.
    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

/// Failed form rule or submission, attached to a field or to the form.
#[derive(Clone, Debug, PartialEq)]
pub struct CheckError {
    field: Option<Id>,
    error: ValidationError,
}

impl CheckError {
    /// Error shown on the field with salt `salt`.
    pub fn on_field(salt: impl Into<KeyedSalt>, error: ValidationError) -> Self {
        Self {
            field: Some(field_id(&salt.into())),
            error,
        }
    }

    /// Error shown on the form.
    pub fn on_form(error: ValidationError) -> Self {
        Self { field: None, error }
    }

    /// Shows on the field with salt `salt`; `false` for a form error.
    pub fn targets(&self, salt: impl Into<KeyedSalt>) -> bool {
        self.field == Some(field_id(&salt.into()))
    }

    /// What failed.
    pub fn error(&self) -> &ValidationError {
        &self.error
    }
}

type FormRule<T> = Box<dyn Fn(&T) -> Result<(), CheckError> + Send + Sync>;

/// Declaration of one field: label, help text, required flag, validation timing and typed validators.
#[must_use]
pub struct FieldSpec<V: ?Sized> {
    label: String,
    supporting: Option<String>,
    required: bool,
    validate_on: ValidateOn,
    masked: bool,
    rows: usize,
    validators: Vec<Box<dyn Validator<V>>>,
}

impl<V: ?Sized> std::fmt::Debug for FieldSpec<V> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FieldSpec")
            .field("label", &self.label)
            .field("supporting", &self.supporting)
            .field("required", &self.required)
            .field("validate_on", &self.validate_on)
            .field("masked", &self.masked)
            .field("rows", &self.rows)
            .field("validators", &self.validators.len())
            .finish()
    }
}

impl<V: ?Sized> FieldSpec<V> {
    /// `label` is the visible, accessible name.
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            supporting: None,
            required: false,
            validate_on: ValidateOn::default(),
            masked: false,
            rows: 1,
            validators: Vec::new(),
        }
    }

    /// Visible text; also the accessible name.
    pub fn label(&self) -> &str {
        &self.label
    }

    /// Value of [`Self::supporting`].
    pub fn supporting_text(&self) -> Option<&str> {
        self.supporting.as_deref()
    }

    /// Value of [`Self::required`].
    pub fn is_required(&self) -> bool {
        self.required
    }

    /// Value of [`Self::validate_on`].
    pub fn validates_on(&self) -> ValidateOn {
        self.validate_on
    }

    /// Helper text shown with the field.
    pub fn supporting(mut self, supporting: impl Into<String>) -> Self {
        self.supporting = Some(supporting.into());
        self
    }

    /// Blank values ([`Blank`]) fail with [`Required`]; when `false`, blank values skip the validators.
    pub fn required(mut self, required: bool) -> Self {
        self.required = required;
        self
    }

    /// Defaults to [`ValidateOn::Blur`].
    pub fn validate_on(mut self, validate_on: ValidateOn) -> Self {
        self.validate_on = validate_on;
        self
    }

    /// Runs after the validators added before it.
    pub fn validator(mut self, validator: impl Validator<V> + 'static) -> Self {
        self.validators.push(Box::new(validator));
        self
    }
}

impl<V: Blank + ?Sized> FieldSpec<V> {
    /// Required check, then validators in order; blank optional values pass.
    ///
    /// # Errors
    /// Error of the required check, else of the first validator that rejects `value`.
    pub fn check(&self, value: &V, locale: &dyn LocaleFormat) -> Result<(), ValidationError> {
        if self.required {
            Required.validate(value, locale)?;
        } else if value.is_blank() {
            return Ok(());
        }
        self.validators.iter().try_for_each(|v| v.validate(value, locale))
    }
}

impl FieldSpec<String> {
    /// Masked input (passwords); default `false`.
    pub fn masked(mut self, masked: bool) -> Self {
        self.masked = masked;
        self
    }

    /// Visible text lines; more than one makes Enter insert a newline instead of submitting. Default 1.
    pub fn rows(mut self, rows: usize) -> Self {
        self.rows = rows.max(1);
        self
    }

    /// Value of [`Self::masked`].
    pub fn is_masked(&self) -> bool {
        self.masked
    }

    /// Value of [`Self::rows`].
    pub fn row_count(&self) -> usize {
        self.rows
    }
}

/// Per-field handle for renderers: the bound value, its spec, and edit, blur and Enter reports.
pub struct FieldCtx<'a, V: ?Sized> {
    salt: KeyedSalt,
    spec: &'a FieldSpec<V>,
    value: &'a mut V,
    state: &'a mut FieldState,
    refresh: bool,
    submit: bool,
}

impl<V: ?Sized> std::fmt::Debug for FieldCtx<'_, V> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FieldCtx").finish_non_exhaustive()
    }
}

impl<'a, V: ?Sized> FieldCtx<'a, V> {
    /// Widget salt of the field: the form salt joined with the field salt.
    pub fn salt(&self) -> KeyedSalt {
        self.salt.clone()
    }

    /// Declaration of the field.
    pub fn spec(&self) -> &'a FieldSpec<V> {
        self.spec
    }

    /// Bound value; call [`Self::changed`] after an edit.
    pub fn value(&mut self) -> &mut V {
        self.value
    }

    /// Call after an edit: marks dirty; the shown error updates under [`ValidateOn::Change`], and under every mode
    /// while [`Self::visible_error`] is `Some`, so a shown error clears on the edit that fixes it.
    pub fn changed(&mut self) {
        self.state.dirty = true;
        if self.spec.validate_on == ValidateOn::Change || (self.state.touched && self.state.error.is_some()) {
            self.refresh = true;
        }
    }

    /// Call on focus loss: marks touched; the shown error updates unless [`ValidateOn::Submit`].
    pub fn blurred(&mut self) {
        self.state.touched = true;
        if self.spec.validate_on != ValidateOn::Submit {
            self.refresh = true;
        }
    }

    /// Edit and focus loss in one: pickers and checkboxes have no blur step, so a pick shows its error at once.
    pub fn committed(&mut self) {
        self.changed();
        self.blurred();
    }

    /// Call when Enter in a single-line input asks the form to submit.
    pub fn submit(&mut self) {
        self.submit = true;
    }

    /// Error to display: only after the field was touched or a submit was attempted.
    pub fn visible_error(&self) -> Option<&ValidationError> {
        self.state.error.as_ref().filter(|_| self.state.touched)
    }
}

/// Places the fields of a [`Form`]; stylekits add typed field methods on top of [`Self::field`].
pub struct FormUi<'u, 's, K> {
    ui: &'u mut MineralUi<'s, K>,
    salt: &'u KeyedSalt,
    state: &'u mut FormState,
    locale: &'u dyn LocaleFormat,
    seen: Vec<Id>,
    submit: bool,
}

crate::kit::opaque_debug!(FormUi<'_, '_, K>);

impl<'s, K> FormUi<'_, 's, K> {
    /// Ui the fields are placed in, for content between them.
    pub fn ui(&mut self) -> &mut MineralUi<'s, K> {
        self.ui
    }

    /// Form salt; field widgets derive their salts from it.
    pub fn form_salt(&self) -> &KeyedSalt {
        self.salt
    }

    /// Field `salt` bound to `value`: `render` shows its input and reports edits through the [`FieldCtx`], then
    /// `spec` validates the value of this pass.
    pub fn field<V: Blank + ?Sized>(
        &mut self,
        salt: impl Into<KeyedSalt>,
        spec: &FieldSpec<V>,
        value: &mut V,
        render: impl FnOnce(&mut MineralUi<'s, K>, &mut FieldCtx<'_, V>) -> egui::Response,
    ) -> egui::Response {
        let salt = salt.into();
        let id = field_id(&salt);
        self.seen.push(id);
        let state = self.state.fields.entry(id).or_default();
        let mut ctx = FieldCtx {
            salt: self.salt.join(&salt),
            spec,
            value,
            state,
            refresh: false,
            submit: false,
        };
        let response = render(self.ui, &mut ctx);
        let FieldCtx {
            value,
            state,
            refresh,
            submit,
            ..
        } = ctx;
        let latest = spec.check(value, self.locale).err();
        if refresh && !state.pending {
            state.error.clone_from(&latest);
        }
        state.latest = latest;
        self.submit |= submit;
        response
    }
}

/// Returned by [`Form::show_with`].
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct FormResponse<R = ()> {
    /// Form container.
    pub response: egui::Response,
    /// What the fields closure returned.
    pub inner: R,
    /// `true` on the pass a valid submission starts; the values are in the bound `T`.
    pub submitted: bool,
}

/// Form driver over the app's value `T`: validation timing, submit and error state in a [`FormState`].
#[must_use = "call .show(..)"]
pub struct Form<'a, T> {
    salt: KeyedSalt,
    state: &'a mut FormState,
    value: &'a mut T,
    rules: Vec<FormRule<T>>,
    submit_label: Option<String>,
    locale: Option<Arc<dyn LocaleFormat>>,
}

crate::kit::opaque_debug!(Form<'_, T>);

impl<'a, T> Form<'a, T> {
    /// `id_salt` salts field ids and derives the agent key; `state` persists across passes; `value` holds the
    /// field values.
    pub fn new(id_salt: impl Into<KeyedSalt>, state: &'a mut FormState, value: &'a mut T) -> Self {
        Self {
            salt: id_salt.into(),
            state,
            value,
            rules: Vec::new(),
            submit_label: None,
            locale: None,
        }
    }

    /// Parses locale-formatted input in validators instead of [`I18n::format`](i18n::I18n::format).
    pub fn locale(mut self, locale: Arc<dyn LocaleFormat>) -> Self {
        self.locale = Some(locale);
        self
    }

    /// Cross-field rule over the whole value, run on submit; the error shows on its field
    /// ([`CheckError::on_field`]), or on the form.
    pub fn rule(mut self, rule: impl Fn(&T) -> Result<(), CheckError> + Send + Sync + 'static) -> Self {
        self.rules.push(Box::new(rule));
        self
    }

    /// Overrides the renderer's own submit label.
    pub fn submit_label(mut self, submit_label: impl Into<String>) -> Self {
        self.submit_label = Some(submit_label.into());
        self
    }

    fn submit(&mut self) -> bool {
        self.state.submit_count += 1;
        if self.state.submitting {
            return false;
        }
        self.state.form_error = None;
        for f in self.state.fields.values_mut() {
            f.touched = true;
            if !f.pending {
                f.error.clone_from(&f.latest);
            }
        }
        for rule in &self.rules {
            if let Err(e) = rule(self.value) {
                self.state.show_check(e);
            }
        }
        self.state.submitting = self.state.is_valid();
        self.state.submitting
    }

    /// `add` places the fields through [`FormUi::field`]; `footer` renders the form error and submit control and
    /// returns `true` when activated. Stylekits wrap it in their own `show`.
    pub fn show_with<K, R>(
        mut self,
        ui: &mut MineralUi<'_, K>,
        add: impl FnOnce(&mut FormUi<'_, '_, K>, &mut T) -> R,
        footer: impl FnOnce(&mut MineralUi<'_, K>, &FormFooter<'_>) -> bool,
    ) -> FormResponse<R> {
        let rt = ui.rt();
        let locale = self.locale.clone().unwrap_or_else(|| rt.core().i18n().format());
        let scope = ui.scope(|ui| {
            let ui = &mut rt.ui(ui);
            let mut form = FormUi {
                ui: &mut *ui,
                salt: &self.salt,
                state: self.state,
                locale: locale.as_ref(),
                seen: Vec::new(),
                submit: false,
            };
            let inner = add(&mut form, self.value);
            let FormUi { seen, submit, .. } = form;
            self.state.fields.retain(|id, _| seen.contains(id));
            let clicked = footer(
                ui,
                &FormFooter {
                    form_salt: &self.salt,
                    error: self.state.form_error.as_ref(),
                    submit_label: self.submit_label.as_deref(),
                    enabled: !self.state.submitting,
                },
            );
            let submitted = (clicked || submit) && self.submit();
            (inner, submitted)
        });
        let (inner, submitted) = scope.inner;
        FormResponse {
            response: scope.response,
            inner,
            submitted,
        }
    }
}

/// Form-level state for the footer renderer.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct FormFooter<'a> {
    /// Form salt; renderers derive footer salts with [`KeyedSalt::child`].
    pub form_salt: &'a KeyedSalt,
    /// Form-level error to display, from [`FormState::form_error`].
    pub error: Option<&'a ValidationError>,
    /// App-supplied label; renderers fall back to their own.
    pub submit_label: Option<&'a str>,
    /// `false` while a submission is in flight.
    pub enabled: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{TestKit, harness};

    #[derive(Default)]
    struct Signup {
        email: String,
        pw: String,
        pw2: String,
        extra: bool,
    }

    #[derive(Default)]
    struct App {
        state: FormState,
        value: Signup,
        press: bool,
        edit: bool,
        show_extra: bool,
        submitted: bool,
    }

    fn blank<V: ?Sized>(ui: &mut MineralUi<'_, TestKit>, _: &mut FieldCtx<'_, V>) -> egui::Response {
        ui.allocate_response(egui::Vec2::ZERO, egui::Sense::hover())
    }

    fn show(ui: &mut MineralUi<'_, TestKit>, app: &mut App) {
        let press = std::mem::take(&mut app.press);
        let edit = std::mem::take(&mut app.edit);
        let show_extra = app.show_extra;
        let r = Form::new("f", &mut app.state, &mut app.value)
            .locale(Arc::new(crate::test_support::en_us()))
            .rule(|v: &Signup| {
                if v.pw == v.pw2 {
                    Ok(())
                } else {
                    Err(CheckError::on_field(
                        "pw2",
                        ValidationError::new("mismatch", "Passwords differ"),
                    ))
                }
            })
            .show_with(
                ui,
                |f, v| {
                    f.field(
                        "email",
                        &FieldSpec::new("Email")
                            .required(true)
                            .validator(|v: &String, _: &dyn LocaleFormat| {
                                if v.contains('@') {
                                    Ok(())
                                } else {
                                    Err(ValidationError::new("email", "Not an email"))
                                }
                            }),
                        &mut v.email,
                        |ui, c| {
                            if edit {
                                c.value().push('x');
                                c.changed();
                            }
                            blank(ui, c)
                        },
                    );
                    let pw = FieldSpec::new("Password").required(true).validator(MinLength(3));
                    f.field("pw", &pw, &mut v.pw, blank);
                    f.field("pw2", &FieldSpec::new("Repeat"), &mut v.pw2, blank);
                    if show_extra {
                        f.field("extra", &FieldSpec::new("Extra").required(true), &mut v.extra, blank);
                    }
                },
                |_, _| press,
            );
        app.submitted |= r.submitted;
    }

    fn code(app: &App, field: &'static str) -> Option<String> {
        let f = app.state.field(field)?;
        f.error().map(|e| e.code.to_string())
    }

    #[test]
    fn submit_validates_typed_fields_and_form_rules() {
        let mut h = harness(show, App::default());
        h.state_mut().value.value = Signup {
            email: "nope".into(),
            pw: "abc".into(),
            pw2: "xyz".into(),
            extra: false,
        };
        h.run();
        assert_eq!(
            code(&h.state().value, "email"),
            None,
            "errors show after touch or submit"
        );
        h.state_mut().value.press = true;
        h.run();
        let app = &h.state().value;
        assert!(!app.submitted);
        assert_eq!(code(app, "email").as_deref(), Some("email"));
        assert_eq!(code(app, "pw2").as_deref(), Some("mismatch"));

        let v = &mut h.state_mut().value;
        "a@b.co".clone_into(&mut v.value.email);
        "abc".clone_into(&mut v.value.pw2);
        v.press = true;
        h.run();
        assert!(h.state().value.submitted, "valid values submit");
        assert!(h.state().value.state.submitting());
        h.state_mut().value.submitted = false;
        h.state_mut().value.press = true;
        h.run();
        let app = &mut h.state_mut().value;
        assert!(!app.submitted, "no double submit");
        assert_eq!(app.state.submit_count(), 3);
        app.state.finish_submit(Ok(()));
        assert!(!app.state.submitting());
    }

    #[test]
    fn edits_mark_dirty_and_code_values_do_not() {
        let mut h = harness(show, App::default());
        h.state_mut().value.value.email = "a@b".into();
        h.run();
        assert!(!h.state().value.state.is_dirty(), "values set from code are not edits");
        h.state_mut().value.edit = true;
        h.run();
        let app = &mut h.state_mut().value;
        assert_eq!(app.value.email, "a@bx");
        assert!(app.state.is_dirty());
        app.state.finish_submit(Ok(()));
        assert!(!app.state.is_dirty());
    }

    #[test]
    fn hidden_field_drops_its_state_and_stops_blocking() {
        let mut h = harness(show, App::default());
        let v = &mut h.state_mut().value;
        v.value = Signup {
            email: "a@b".into(),
            pw: "abc".into(),
            pw2: "abc".into(),
            extra: false,
        };
        v.show_extra = true;
        v.press = true;
        h.run();
        assert!(!h.state().value.submitted, "the required extra blocks");
        h.state_mut().value.show_extra = false;
        h.run();
        assert!(h.state().value.state.field("extra").is_none());
        h.state_mut().value.press = true;
        h.run();
        assert!(h.state().value.submitted);
    }

    #[test]
    fn async_errors_and_pending_block_validity() {
        let mut state = FormState::default();
        state.field_mut("name").set_pending(true);
        assert!(!state.is_valid(), "a pending check blocks validity");
        state.set_error("name", Some(ValidationError::new("taken", "Name taken")));
        let f = state.field("name").expect("field");
        assert!(!f.pending() && f.error().is_some());
        assert!(!state.is_valid());
    }

    #[test]
    fn rejected_submission_shows_on_fields_and_form() {
        let mut h = harness(show, App::default());
        h.run();
        let taken = CheckError::on_field("email", ValidationError::new("taken", "Email taken"));
        assert!(taken.targets("email") && !taken.targets("pw"));
        let state = &mut h.state_mut().value.state;
        state.finish_submit(Err(vec![
            CheckError::on_field("email", ValidationError::new("taken", "Email taken")),
            CheckError::on_field("gone", ValidationError::new("x", "Unshown")),
        ]));
        assert_eq!(code(&h.state().value, "email").as_deref(), Some("taken"));
        let state = &h.state().value.state;
        assert!(state.field("email").is_some_and(FieldState::touched));
        assert_eq!(
            state.form_error().map(|e| e.code.as_ref()),
            Some("x"),
            "unshown field falls back"
        );
    }
}
