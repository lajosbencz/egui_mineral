//! Form validation and submission over app-owned values. `#[derive(Form)]` declares a form on the value type `T`:
//! each field's key, accessors, label, kind options and rules live on the field, so the UI names a field only to
//! place it. A [`Form`](struct@Form) runs the rules ([`FormModel`]) every pass over the whole value, renders fields through
//! the stylekit's [`FieldWidget`]s in [`FormUi::rest`] or wherever [`FormUi::show`] places them, and drives
//! submission. [`FormState`] keeps only per-field meta state (dirty, touched, shown errors) and the submission;
//! the values stay in `T`, so a submit hands back nothing to extract.
//!
//! ```
//! use egui_mineral::form::{Form, FormModel, MinLength, Validation, ValidationError};
//!
//! #[derive(Default, Form)]
//! #[form(check = Self::passwords_match)]
//! struct Signup {
//!     #[form(label = "Name", required, check = MinLength(3))]
//!     name: String,
//!     #[form(label = "Password", required, masked)]
//!     password: String,
//!     #[form(label = "Confirm password", masked)]
//!     confirm: String,
//! }
//!
//! impl Signup {
//!     fn passwords_match(&self, v: &mut Validation<'_, Self>) {
//!         if self.confirm != self.password {
//!             v.error(Self::CONFIRM, ValidationError::new("mismatch", "Passwords differ"));
//!         }
//!     }
//! }
//!
//! let report = Signup::default().report(&egui_mineral::i18n::AsciiFormat::default());
//! assert_eq!(report.first_invalid(), Some(&Signup::NAME.into()));
//! ```
//!
//! Labels and help texts take literals or message ids (`label = msg::SIGNUP_NAME`); `#[form(messages = msg)]` on
//! the type looks up `<type>-<field>` for every label and its `.supporting` attribute for the help text. Options of
//! a field's kind pass through to [`FieldSpec`] methods (`masked`, `rows = 3`, `choices = ROLES`,
//! `range = 0.0..=50.0`), with `self` in scope for options computed from the value.

mod input;
mod model;
mod placement;
#[cfg(test)]
mod tests;
mod ui;
mod validators;

use std::{borrow::Cow, collections::HashMap, sync::Arc};

use egui::Id;
use serde::{Deserialize, Serialize};

use crate::{
    i18n::{self, Arg, LocaleFormat},
    semantic::KeyedSalt,
    widget::MineralUi,
};
/// Declares a form on a struct with named fields: a key const per field (`start_date` -> `START_DATE`, keyed
/// `start-date`) and the [`FormModel`] and [`FormFields`] impls.
///
/// On the type:
/// - `check = path`: `fn(&self, &mut Validation<Self>)` run after the field rules, for cross-field rules;
///   repeatable.
/// - `messages = path`: the `messages!` module labels come from: message `<prefix>-<field>` labels a field, its
///   `.supporting` attribute is the help text.
/// - `prefix = "name"`: message prefix; default the kebab-case type name.
/// - `crate = path`: path of `egui_mineral`.
///
/// On a field:
/// - `label = expr`, `supporting = expr`: literal or message id ([`Label`]); the default label is the humanized
///   field name.
/// - `required`: blank fails ([`Blank`] types only); `check = expr`: a [`Validator`], repeatable, run in order.
/// - `on = Change | Blur | Submit`: [`ValidateOn`].
/// - `kind = Name`: widget kind, a [`kind`] marker or, given as a path (`kind = my::Stars`), an app's
///   [`FieldKind`]; default the value type's [`FieldInput::Kind`].
/// - `nested`: a nested `Form` value; `list`: a `Vec` of `Form` values.
/// - `rename = "key"`, `skip`.
/// - any other `name` or `name = expr`: the [`FieldSpec`] option method of the kind, `.name(true)` or
///   `.name(expr)` (`masked`, `rows = 3`, `choices = ROLES`, `range = 0.0..=50.0`); `self` is the form value.
pub use egui_mineral_macros::Form;
pub use input::{Blank, Choices, FieldInput, FieldKind, Label, NumberOptions, TextOptions, kind};
pub use model::{FieldCheck, FieldKey, FieldPath, FormModel, KeyName, ListKey, NestedKey, Report, Validation};
pub use placement::{FieldPlacement, FormRow, LabelSlot, PlaceRender};
pub use ui::{FieldCtx, FieldSpec, FieldWidget, FormFields, FormUi};
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

/// Check of a field value of type `V`; `locale` parses locale-formatted input (numbers, dates).
pub trait Validator<V: ?Sized>: Send + Sync {
    /// Never sees a blank value when called through [`FieldCheck::check`].
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
    // Rule result of the last pass, shown on submit.
    #[serde(skip)]
    latest: Option<ValidationError>,
    // `error` came from `set_error` or a submission: kept until an edit of the field.
    #[serde(skip)]
    external: bool,
    #[serde(skip)]
    path: Option<FieldPath>,
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

    fn show(&mut self, error: ValidationError) {
        self.touched = true;
        self.error = Some(error);
        self.external = true;
        self.pending = false;
    }

    fn reveal(&mut self) {
        self.touched = true;
        if !self.pending && !self.external {
            self.error.clone_from(&self.latest);
        }
    }
}

pub(crate) fn field_id(salt: &KeyedSalt) -> Id {
    Id::unique(salt.id_salt())
}

/// Meta state of a [`Form`](struct@Form): fields by their path, and the submission.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct FormState {
    fields: HashMap<Id, FieldState>,
    #[serde(skip)]
    submitting: bool,
    #[serde(skip)]
    submit_requested: bool,
    #[serde(skip)]
    submit_count: u32,
    #[serde(skip)]
    form_error: Option<ValidationError>,
    #[serde(skip)]
    form_error_source: FormErrorSource,
}

// What set `FormState::form_error`, which decides when it updates.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum FormErrorSource {
    // A rule: follows the rules once shown.
    #[default]
    Rules,
    // Errors on fields not shown: clears once none is left.
    Hidden,
    // A submission: kept until the next submit.
    Server,
}

impl FormState {
    /// `None` for a field the form neither checks nor shows; [`Form::show_with`] drops the others each pass.
    pub fn field(&self, path: impl Into<FieldPath>) -> Option<&FieldState> {
        self.fields.get(&path.into().id())
    }

    /// Inserts a default state on first access.
    pub fn field_mut(&mut self, path: impl Into<FieldPath>) -> &mut FieldState {
        let path = path.into();
        let f = self.fields.entry(path.id()).or_default();
        f.path.get_or_insert(path);
        f
    }

    /// `true` from a submit of a [`Form::pending_submit`] form until [`Self::finish_submit`]; blocks re-submit.
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

    /// Every rule of the last pass passed (shown or not), no shown field or form error, no pending check.
    pub fn is_valid(&self) -> bool {
        self.form_error.is_none()
            && self
                .fields
                .values()
                .all(|f| f.error.is_none() && f.latest.is_none() && !f.pending)
    }

    /// Server-side or async validation result of the field at `path`; an error shows at once and stays until the
    /// field is edited.
    pub fn set_error(&mut self, path: impl Into<FieldPath>, error: Option<ValidationError>) {
        let f = self.field_mut(path);
        if let Some(e) = error {
            f.show(e);
        } else {
            f.error = None;
            f.external = false;
            f.pending = false;
        }
    }

    /// Shows the errors of the fields at or under `prefix` (a wizard step's fields or nested value) as a submit
    /// would; `true` when they all pass and none is pending, `false` when no field the form checked or showed
    /// matches.
    pub fn touch(&mut self, prefix: impl Into<FieldPath>) -> bool {
        let prefix = prefix.into();
        let (mut matched, mut valid) = (false, true);
        for f in self.fields.values_mut() {
            if f.path.as_ref().is_some_and(|p| p.starts_with(&prefix)) {
                f.reveal();
                matched = true;
                valid &= f.error.is_none() && f.latest.is_none() && !f.pending;
            }
        }
        matched && valid
    }

    /// Submits on the next pass of the form, as its submit control would.
    pub fn request_submit(&mut self) {
        self.submit_requested = true;
    }

    /// Ends a submission: `Ok` clears every dirty flag; each `Err` entry shows on its field until the field is
    /// edited, or on the form when it targets the form or a field neither checked nor shown (the last such entry
    /// wins).
    pub fn finish_submit(&mut self, result: Result<(), Vec<SubmitError>>) {
        self.submitting = false;
        match result {
            Ok(()) => self.fields.values_mut().for_each(|f| f.dirty = false),
            Err(errors) => errors.into_iter().for_each(|e| self.show_error(e)),
        }
    }

    fn show_error(&mut self, SubmitError { field, error }: SubmitError) {
        if let Some(f) = field.and_then(|id| self.fields.get_mut(&id)) {
            f.show(error);
        } else {
            self.form_error = Some(error);
            self.form_error_source = FormErrorSource::Server;
        }
    }

    fn hidden_errors(&self, seen: &[Id]) -> bool {
        self.fields
            .iter()
            .any(|(id, f)| !seen.contains(id) && (f.error.is_some() || f.pending))
    }

    /// Clears the meta state of every field and the submission; the app resets its own value.
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    // Keeps the fields the rules check or the pass showed, stores the rule results, and updates shown errors: on
    // a `refreshed` field, and on every field showing a rule error so a fix elsewhere (a cross-field rule) clears it.
    fn sync(&mut self, report: &Report, seen: &[Id], refreshed: &[Id]) {
        let checked = report.checked();
        self.fields
            .retain(|id, f| seen.contains(id) || checked.iter().any(|p| p.id() == *id) || f.external || f.pending);
        for path in checked {
            self.fields.entry(path.id()).or_default().path = Some(path.clone());
        }
        for (id, f) in &mut self.fields {
            f.latest = report.error_of(*id).cloned();
            let live = f.touched && f.error.is_some() && !f.external;
            if !f.pending && !f.external && (live || refreshed.contains(id)) {
                f.error.clone_from(&f.latest);
            }
        }
        match self.form_error_source {
            FormErrorSource::Rules if self.form_error.is_some() => self.form_error = report.form_error().cloned(),
            FormErrorSource::Hidden if !self.hidden_errors(seen) => self.form_error = None,
            _ => {}
        }
    }
}

/// Failed submission, attached to a field or to the form.
#[derive(Clone, Debug, PartialEq)]
pub struct SubmitError {
    field: Option<Id>,
    error: ValidationError,
}

impl SubmitError {
    /// Error shown on the field at `path`.
    pub fn on_field(path: impl Into<FieldPath>, error: ValidationError) -> Self {
        Self {
            field: Some(path.into().id()),
            error,
        }
    }

    /// Error shown on the form.
    pub fn on_form(error: ValidationError) -> Self {
        Self { field: None, error }
    }

    /// Shows on the field at `path`; `false` for a form error.
    pub fn targets(&self, path: impl Into<FieldPath>) -> bool {
        self.field == Some(path.into().id())
    }

    /// What failed.
    pub fn error(&self) -> &ValidationError {
        &self.error
    }
}

type FormRule<'a, T> = Box<dyn Fn(&T, &mut Validation<'_, T>) + 'a>;

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
    /// On a rejected submit: the first field in check order with an error, e.g. to switch to its wizard step.
    pub first_invalid: Option<FieldPath>,
}

/// Form driver over the app's value `T`: validation timing, submit and error state in a [`FormState`].
#[must_use = "call .show(..)"]
pub struct Form<'a, T> {
    salt: KeyedSalt,
    state: &'a mut FormState,
    value: &'a mut T,
    rules: Vec<FormRule<'a, T>>,
    submit_label: Option<String>,
    locale: Option<Arc<dyn LocaleFormat>>,
    pending: bool,
}

crate::kit::opaque_debug!(Form<'_, T>);

impl<'a, T: FormModel> Form<'a, T> {
    /// `id_salt` salts field ids and derives the agent key; `state` persists across passes; `value` holds the
    /// field values and their rules.
    pub fn new(id_salt: impl Into<KeyedSalt>, state: &'a mut FormState, value: &'a mut T) -> Self {
        Self {
            salt: id_salt.into(),
            state,
            value,
            rules: Vec::new(),
            submit_label: None,
            locale: None,
            pending: false,
        }
    }

    /// Parses locale-formatted input in validators instead of [`I18n::format`](i18n::I18n::format).
    pub fn locale(mut self, locale: Arc<dyn LocaleFormat>) -> Self {
        self.locale = Some(locale);
        self
    }

    /// Check run after [`FormModel::validate`] every pass, for rules that depend on app state outside `T`.
    pub fn rule(mut self, rule: impl Fn(&T, &mut Validation<'_, T>) + 'a) -> Self {
        self.rules.push(Box::new(rule));
        self
    }

    /// Overrides the renderer's own submit label.
    pub fn submit_label(mut self, submit_label: impl Into<String>) -> Self {
        self.submit_label = Some(submit_label.into());
        self
    }

    /// Keeps a valid submit in flight ([`FormState::submitting`], no re-submit) until
    /// [`FormState::finish_submit`], e.g. for a server round trip; otherwise it finishes at once, clearing the
    /// dirty flags.
    pub fn pending_submit(mut self) -> Self {
        self.pending = true;
        self
    }

    fn report(&self, locale: &dyn LocaleFormat) -> Report {
        let mut report = Report::default();
        let mut v = Validation::new(&*self.value, locale, None, &mut report);
        self.value.validate(&mut v);
        for rule in &self.rules {
            rule(self.value, &mut v);
        }
        report
    }

    fn submit(&mut self, report: &Report, seen: &[Id]) -> bool {
        self.state.submit_count += 1;
        if self.state.submitting {
            return false;
        }
        self.state.form_error = report.form_error().cloned();
        self.state.form_error_source = FormErrorSource::Rules;
        self.state.fields.values_mut().for_each(FieldState::reveal);
        let valid = self.state.is_valid();
        if !valid && self.state.form_error.is_none() && self.state.hidden_errors(seen) {
            self.state.form_error = Some(ValidationError::localized(i18n::msg::FORM_HIDDEN_ERRORS));
            self.state.form_error_source = FormErrorSource::Hidden;
        }
        if valid && !self.pending {
            self.state.finish_submit(Ok(()));
        } else {
            self.state.submitting = valid;
        }
        valid
    }

    /// `add` places the fields through [`FormUi`]; `footer` renders the form error and submit control and returns
    /// `true` when activated. Stylekits wrap it in their own `show`.
    pub fn show_with<K, R>(
        mut self,
        ui: &mut MineralUi<'_, K>,
        add: impl FnOnce(&mut FormUi<'_, '_, '_, K, T>) -> R,
        footer: impl FnOnce(&mut MineralUi<'_, K>, &FormFooter<'_>) -> bool,
    ) -> FormResponse<R> {
        let rt = ui.rt();
        let locale = self.locale.clone().unwrap_or_else(|| rt.core().i18n().format());
        let scope = ui.scope(|ui| {
            let ui = &mut rt.ui(ui);
            let requested = std::mem::take(&mut self.state.submit_requested);
            let start = self.report(locale.as_ref());
            let mut pass = ui::Pass {
                ui: &mut *ui,
                salt: &self.salt,
                state: &mut *self.state,
                report: &start,
                seen: Vec::new(),
                refreshed: Vec::new(),
                changed: false,
                submit: false,
                placement: None,
            };
            let inner = add(&mut FormUi {
                pass: &mut pass,
                value: &mut *self.value,
                prefix: None,
                placed: Vec::new(),
            });
            let ui::Pass {
                seen,
                refreshed,
                changed,
                submit,
                ..
            } = pass;
            let mut report = if changed { self.report(locale.as_ref()) } else { start };
            self.state.sync(&report, &seen, &refreshed);
            let clicked = footer(
                ui,
                &FormFooter {
                    form_salt: &self.salt,
                    error: self.state.form_error.as_ref(),
                    submit_label: self.submit_label.as_deref(),
                    enabled: !self.state.submitting,
                },
            );
            let attempted = clicked || submit || requested;
            if attempted {
                // Edits through `FormUi::value` or a render that skipped `FieldCtx::changed` reach the rules here.
                if !changed {
                    report = self.report(locale.as_ref());
                    self.state.sync(&report, &seen, &[]);
                }
                repaint(ui.ctx());
            }
            let submitted = attempted && self.submit(&report, &seen);
            let first_invalid = (attempted && !submitted)
                .then(|| report.first_invalid().cloned())
                .flatten();
            (inner, submitted, first_invalid)
        });
        let (inner, submitted, first_invalid) = scope.inner;
        FormResponse {
            response: scope.response,
            inner,
            submitted,
            first_invalid,
        }
    }
}

// Errors shown by a submit appear after the fields and footer drew this pass.
#[expect(clippy::disallowed_methods, reason = "one pass after a submit attempt")]
fn repaint(ctx: &egui::Context) {
    ctx.request_repaint();
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
