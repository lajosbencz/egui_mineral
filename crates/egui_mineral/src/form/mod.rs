//! Form state, validation and submission. Headless: stylekits render fields and the footer through
//! [`Form::show_with`].
//!
//! Each field holds a typed [`FieldValue`] whose variant follows its [`FieldKind`]: text kinds hold the raw text
//! (validators do the locale parsing), checkboxes a `bool`, selects option keys, dates a civil date. A submit hands
//! back the validated [`FormValues`].

mod validators;

use std::{
    borrow::Cow,
    collections::BTreeMap,
    hash::{DefaultHasher, Hash as _, Hasher as _},
    sync::Arc,
};

use serde::{Deserialize, Serialize};

use crate::{
    date::Date,
    i18n::{self, Arg, LocaleFormat},
    semantic::KeyedSalt,
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

/// `locale` parses locale-formatted input (numbers, dates).
pub trait Validator: Send + Sync {
    /// Never sees an empty value when called through [`FieldSpec::check`]. Built-in text validators pass values
    /// of other kinds.
    ///
    /// # Errors
    /// [`ValidationError`] describing why `value` is rejected.
    fn validate(&self, value: &FieldValue, locale: &dyn LocaleFormat) -> Result<(), ValidationError>;
}

impl<F: Fn(&FieldValue, &dyn LocaleFormat) -> Result<(), ValidationError> + Send + Sync> Validator for F {
    fn validate(&self, value: &FieldValue, locale: &dyn LocaleFormat) -> Result<(), ValidationError> {
        self(value, locale)
    }
}

/// Value of a form field; the variant follows the field's [`FieldKind`] ([`FieldKind::empty_value`]).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum FieldValue {
    /// Text kinds: the raw text as entered.
    Text(String),
    /// [`FieldKind::Checkbox`]: checked.
    Bool(bool),
    /// [`FieldKind::Select`]: key of the picked option.
    Choice(Option<String>),
    /// [`FieldKind::MultiSelect`]: keys of the picked options, in option order.
    Choices(Vec<String>),
    /// [`FieldKind::Date`]: picked day.
    Date(Option<Date>),
}

impl Default for FieldValue {
    fn default() -> Self {
        Self::Text(String::new())
    }
}

impl FieldValue {
    /// Blank text, unchecked, nothing picked; what a non-required field may submit.
    pub fn is_empty(&self) -> bool {
        match self {
            Self::Text(t) => t.trim().is_empty(),
            Self::Bool(b) => !b,
            Self::Choice(c) => c.is_none(),
            Self::Choices(c) => c.is_empty(),
            Self::Date(d) => d.is_none(),
        }
    }

    /// Text of a text kind.
    pub fn as_text(&self) -> Option<&str> {
        match self {
            Self::Text(t) => Some(t),
            _ => None,
        }
    }

    /// State of a checkbox.
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Self::Bool(b) => Some(*b),
            _ => None,
        }
    }

    /// Picked option key of a select.
    pub fn as_choice(&self) -> Option<&str> {
        match self {
            Self::Choice(c) => c.as_deref(),
            _ => None,
        }
    }

    /// Picked option keys of a multi-select.
    pub fn as_choices(&self) -> Option<&[String]> {
        match self {
            Self::Choices(c) => Some(c),
            _ => None,
        }
    }

    /// Picked day of a date field.
    pub fn as_date(&self) -> Option<Date> {
        match self {
            Self::Date(d) => *d,
            _ => None,
        }
    }

    fn fits(&self, kind: FieldKind) -> bool {
        matches!(
            (self, kind),
            (
                Self::Text(_),
                FieldKind::Text | FieldKind::Password | FieldKind::Multiline
            ) | (Self::Bool(_), FieldKind::Checkbox)
                | (Self::Choice(_), FieldKind::Select)
                | (Self::Choices(_), FieldKind::MultiSelect)
                | (Self::Date(_), FieldKind::Date)
        )
    }
}

impl From<String> for FieldValue {
    fn from(text: String) -> Self {
        Self::Text(text)
    }
}

impl From<&str> for FieldValue {
    fn from(text: &str) -> Self {
        Self::Text(text.to_owned())
    }
}

impl From<bool> for FieldValue {
    fn from(checked: bool) -> Self {
        Self::Bool(checked)
    }
}

impl From<Date> for FieldValue {
    fn from(date: Date) -> Self {
        Self::Date(Some(date))
    }
}

/// Validated values of a submitted [`Form`], by field name.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct FormValues(BTreeMap<String, FieldValue>);

impl FormValues {
    /// Value of field `name`.
    pub fn get(&self, name: &str) -> Option<&FieldValue> {
        self.0.get(name)
    }

    /// Text of text field `name`.
    pub fn text(&self, name: &str) -> Option<&str> {
        self.get(name).and_then(FieldValue::as_text)
    }

    /// State of checkbox `name`.
    pub fn bool(&self, name: &str) -> Option<bool> {
        self.get(name).and_then(FieldValue::as_bool)
    }

    /// Picked option key of select `name`.
    pub fn choice(&self, name: &str) -> Option<&str> {
        self.get(name).and_then(FieldValue::as_choice)
    }

    /// Picked option keys of multi-select `name`.
    pub fn choices(&self, name: &str) -> Option<&[String]> {
        self.get(name).and_then(FieldValue::as_choices)
    }

    /// Picked day of date field `name`.
    pub fn date(&self, name: &str) -> Option<Date> {
        self.get(name).and_then(FieldValue::as_date)
    }

    /// Fields by name.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &FieldValue)> {
        self.0.iter().map(|(k, v)| (k.as_str(), v))
    }

    /// Field count.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// No fields.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

/// When a field re-validates on its own (submit always validates everything).
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

/// Runtime state of one form field.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct FieldState {
    value: FieldValue,
    dirty: bool,
    touched: bool,
    #[serde(skip)]
    pending: bool,
    #[serde(skip)]
    error: Option<ValidationError>,
}

impl FieldState {
    /// Current value; its variant follows the field's kind once the form has shown or validated it.
    pub fn value(&self) -> &FieldValue {
        &self.value
    }

    /// Text of a text kind, else empty.
    pub fn text(&self) -> &str {
        self.value.as_text().unwrap_or("")
    }

    /// Renderer access to the value; call [`FieldCtx::changed`] after editing.
    pub fn value_mut(&mut self) -> &mut FieldValue {
        &mut self.value
    }

    /// Replaces the value from code (initial values); does not mark the field dirty. A value of another kind than
    /// the field's is reported and replaced by the kind's empty value.
    pub fn set_value(&mut self, value: impl Into<FieldValue>) {
        self.value = value.into();
    }

    /// Edited since the last successful submit.
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

    /// Latest validation result; not persisted.
    pub fn error(&self) -> Option<&ValidationError> {
        self.error.as_ref()
    }
}

/// Runtime state of a [`Form`].
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct FormState {
    fields: BTreeMap<String, FieldState>,
    #[serde(skip)]
    submitting: bool,
    #[serde(skip)]
    submit_count: u32,
    #[serde(skip)]
    form_error: Option<ValidationError>,
    // Hash of the spec names and field count at the last `Form::sync_fields`.
    #[serde(skip)]
    synced: Option<(u64, usize)>,
}

impl FormState {
    /// Keyed by field name; [`Form::show_with`] prunes names without a spec.
    pub fn fields(&self) -> &BTreeMap<String, FieldState> {
        &self.fields
    }

    /// `None` for unknown fields.
    pub fn field_state(&self, name: &str) -> Option<&FieldState> {
        self.fields.get(name)
    }

    /// `true` from [`Form::submit`] until [`FormState::finish_submit`]; blocks re-submit.
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

    /// Inserts a default state on first access.
    pub fn field(&mut self, name: &str) -> &mut FieldState {
        self.fields.entry(name.to_owned()).or_default()
    }

    /// `None` for unknown fields.
    pub fn value(&self, name: &str) -> Option<&FieldValue> {
        self.fields.get(name).map(FieldState::value)
    }

    /// Text of text field `name`; empty for unknown and non-text fields.
    pub fn text(&self, name: &str) -> &str {
        self.fields.get(name).map_or("", FieldState::text)
    }

    /// Snapshot of all values by field name.
    pub fn values(&self) -> FormValues {
        FormValues(self.fields.iter().map(|(k, v)| (k.clone(), v.value.clone())).collect())
    }

    /// `true` when any field is dirty.
    pub fn is_dirty(&self) -> bool {
        self.fields.values().any(|f| f.dirty)
    }

    /// No field or form error and no pending check.
    pub fn is_valid(&self) -> bool {
        self.form_error.is_none() && self.fields.values().all(|f| f.error.is_none() && !f.pending)
    }

    /// Server-side or async validation result.
    pub fn set_error(&mut self, name: &str, error: Option<ValidationError>) {
        let f = self.field(name);
        f.error = error;
        f.pending = false;
    }

    /// Ends a submission started by [`Form`]; `Err` lands in `form_error`.
    pub fn finish_submit(&mut self, result: Result<(), ValidationError>) {
        self.submitting = false;
        match result {
            Ok(()) => self.fields.values_mut().for_each(|f| f.dirty = false),
            Err(e) => self.form_error = Some(e),
        }
    }

    /// Clears all fields and submission state.
    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

/// Failed cross-field rule, attached to a field or to the form.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct CheckError {
    /// Field the error shows on; `None` for the form.
    pub field: Option<String>,
    /// What failed.
    pub error: ValidationError,
}

impl CheckError {
    /// Error shown on field `name`.
    pub fn on_field(name: impl Into<String>, error: ValidationError) -> Self {
        Self {
            field: Some(name.into()),
            error,
        }
    }

    /// Error shown on the form.
    pub fn on_form(error: ValidationError) -> Self {
        Self { field: None, error }
    }
}

type FormCheck = Box<dyn Fn(&FormState) -> Result<(), CheckError> + Send + Sync>;

/// Input a renderer shows for a field.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
pub enum FieldKind {
    /// Single line; Enter submits.
    #[default]
    Text,
    /// Masked single line; Enter submits.
    Password,
    /// Enter inserts a newline instead of submitting.
    Multiline,
    /// Checkbox labeled by the field label; required means checked.
    Checkbox,
    /// One of the [`FieldSpec::option`]s, or none.
    Select,
    /// Any of the [`FieldSpec::option`]s.
    MultiSelect,
    /// Civil date.
    Date,
}

impl FieldKind {
    /// Value of a fresh field of this kind.
    pub fn empty_value(self) -> FieldValue {
        match self {
            Self::Text | Self::Password | Self::Multiline => FieldValue::Text(String::new()),
            Self::Checkbox => FieldValue::Bool(false),
            Self::Select => FieldValue::Choice(None),
            Self::MultiSelect => FieldValue::Choices(Vec::new()),
            Self::Date => FieldValue::Date(None),
        }
    }

    fn submits_on_enter(self) -> bool {
        matches!(self, Self::Text | Self::Password)
    }
}

/// Option of a [`FieldKind::Select`] or [`FieldKind::MultiSelect`] field.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct FieldOption {
    /// Stable key, the submitted value.
    pub key: String,
    /// Visible text.
    pub label: String,
}

/// Declaration of one [`Form`] field: name, label, kind, validators.
#[must_use]
pub struct FieldSpec {
    name: String,
    label: String,
    supporting: Option<String>,
    required: bool,
    kind: FieldKind,
    options: Vec<FieldOption>,
    validate_on: ValidateOn,
    validators: Vec<Box<dyn Validator>>,
}

impl std::fmt::Debug for FieldSpec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FieldSpec")
            .field("name", &self.name)
            .field("label", &self.label)
            .field("supporting", &self.supporting)
            .field("required", &self.required)
            .field("kind", &self.kind)
            .field("options", &self.options)
            .field("validate_on", &self.validate_on)
            .field("validators", &self.validators.len())
            .finish()
    }
}

impl FieldSpec {
    /// Key in [`FormState::fields`] and in the submitted map.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Visible text; also the accessible name.
    pub fn label(&self) -> &str {
        &self.label
    }

    /// Helper text shown with the field.
    pub fn supporting_text(&self) -> Option<&str> {
        self.supporting.as_deref()
    }

    /// Empty values fail before validators run; when `false`, empty values skip validation.
    pub fn is_required(&self) -> bool {
        self.required
    }

    /// Input form; only [`FieldKind::Text`] and [`FieldKind::Password`] submit on Enter.
    pub fn input_kind(&self) -> FieldKind {
        self.kind
    }

    /// Options of a select kind, in display order.
    pub fn field_options(&self) -> &[FieldOption] {
        &self.options
    }

    /// When edits and focus loss re-validate.
    pub fn validates_on(&self) -> ValidateOn {
        self.validate_on
    }

    fn new(name: &str, label: String) -> Self {
        Self {
            name: name.to_owned(),
            label,
            supporting: None,
            required: false,
            kind: FieldKind::default(),
            options: Vec::new(),
            validate_on: ValidateOn::default(),
            validators: Vec::new(),
        }
    }

    /// Helper text shown with the field.
    pub fn supporting(mut self, supporting: impl Into<String>) -> Self {
        self.supporting = Some(supporting.into());
        self
    }

    /// Empty values ([`FieldValue::is_empty`]) fail with [`Required`]; when `false`, empty values skip validators.
    pub fn required(mut self, required: bool) -> Self {
        self.required = required;
        self
    }

    /// Defaults to [`FieldKind::Text`].
    pub fn kind(mut self, kind: FieldKind) -> Self {
        self.kind = kind;
        self
    }

    /// Appends an option of a select kind; `key` is what the field submits.
    pub fn option(mut self, key: impl Into<String>, label: impl Into<String>) -> Self {
        self.options.push(FieldOption {
            key: key.into(),
            label: label.into(),
        });
        self
    }

    /// Appends `(key, label)` options.
    pub fn options(mut self, options: impl IntoIterator<Item = (impl Into<String>, impl Into<String>)>) -> Self {
        for (key, label) in options {
            self = self.option(key, label);
        }
        self
    }

    /// Defaults to [`ValidateOn::Blur`].
    pub fn validate_on(mut self, validate_on: ValidateOn) -> Self {
        self.validate_on = validate_on;
        self
    }

    /// Runs after the validators added before it.
    pub fn validator(mut self, validator: impl Validator + 'static) -> Self {
        self.validators.push(Box::new(validator));
        self
    }

    /// Required check, then validators in order; empty optional values pass.
    ///
    /// # Errors
    /// Error of the required check, else of the first validator that rejects `value`.
    pub fn check(&self, value: &FieldValue, locale: &dyn LocaleFormat) -> Result<(), ValidationError> {
        if self.required {
            Required.validate(value, locale)?;
        } else if value.is_empty() {
            return Ok(());
        }
        self.validators.iter().try_for_each(|v| v.validate(value, locale))
    }
}

/// Per-field handle for renderers: report edits and focus loss, read state.
#[non_exhaustive]
pub struct FieldCtx<'a> {
    /// Form salt; renderers derive field salts with [`KeyedSalt::child`].
    pub form_salt: &'a KeyedSalt,
    /// Declaration of the rendered field.
    pub spec: &'a FieldSpec,
    /// Renderers edit [`FieldState::value_mut`], then call [`Self::changed`]; its variant matches the spec's kind.
    pub state: &'a mut FieldState,
    /// Locale validators parse with.
    pub locale: &'a dyn LocaleFormat,
}

crate::kit::opaque_debug!(FieldCtx<'_>);

impl FieldCtx<'_> {
    /// Call after editing the value: marks dirty; re-validates under [`ValidateOn::Change`], and under every mode while
    /// [`Self::visible_error`] is `Some`, so a shown error clears on the edit that fixes it.
    pub fn changed(&mut self) {
        self.state.dirty = true;
        if self.spec.validate_on == ValidateOn::Change || (self.state.touched && self.state.error.is_some()) {
            self.state.error = self.spec.check(&self.state.value, self.locale).err();
        }
    }

    /// Call on focus loss: marks touched, re-validates unless [`ValidateOn::Submit`].
    pub fn blurred(&mut self) {
        self.state.touched = true;
        if self.spec.validate_on != ValidateOn::Submit {
            self.state.error = self.spec.check(&self.state.value, self.locale).err();
        }
    }

    /// Error to display: only after the field was touched or a submit was attempted.
    pub fn visible_error(&self) -> Option<&ValidationError> {
        self.state.error.as_ref().filter(|_| self.state.touched)
    }
}

/// Returned by [`Form::show_with`]; `submitted` carries validated values.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct FormResponse {
    /// Form container.
    pub response: egui::Response,
    /// `Some` with all values on the pass a submission starts.
    pub submitted: Option<FormValues>,
}

/// Form driver: validation timing, submit and error state over [`FormState`].
#[must_use = "call .show(..)"]
pub struct Form<'a> {
    salt: KeyedSalt,
    state: &'a mut FormState,
    fields: Vec<FieldSpec>,
    checks: Vec<FormCheck>,
    submit_label: Option<String>,
    locale: Option<Arc<dyn LocaleFormat>>,
}

crate::kit::opaque_debug!(Form<'_>);

impl<'a> Form<'a> {
    /// `id_salt` salts field ids and derives the agent key; `state` persists across passes.
    pub fn new(id_salt: impl Into<KeyedSalt>, state: &'a mut FormState) -> Self {
        Self {
            salt: id_salt.into(),
            state,
            fields: Vec::new(),
            checks: Vec::new(),
            submit_label: None,
            locale: None,
        }
    }

    /// Parses locale-formatted input in validators instead of [`I18n::format`](i18n::I18n::format).
    pub fn locale(mut self, locale: Arc<dyn LocaleFormat>) -> Self {
        self.locale = Some(locale);
        self
    }

    fn locale_for(&self, i18n: &i18n::I18n) -> Arc<dyn LocaleFormat> {
        self.locale.clone().unwrap_or_else(|| i18n.format())
    }

    /// `name` keys the value; `label` is the visible, accessible name.
    pub fn field(mut self, name: &str, label: impl Into<String>, build: impl FnOnce(FieldSpec) -> FieldSpec) -> Self {
        self.fields.push(build(FieldSpec::new(name, label.into())));
        self
    }

    /// Cross-field rule; [`CheckError::field`] attaches the error to a field, `None` to the form.
    pub fn check(mut self, check: impl Fn(&FormState) -> Result<(), CheckError> + Send + Sync + 'static) -> Self {
        self.checks.push(Box::new(check));
        self
    }

    /// Overrides the renderer's own submit label.
    pub fn submit_label(mut self, submit_label: impl Into<String>) -> Self {
        self.submit_label = Some(submit_label.into());
        self
    }

    // Drops state without a spec, adds missing entries and replaces values of another kind.
    fn sync_fields(&mut self) {
        let mut hasher = DefaultHasher::new();
        self.fields.iter().for_each(|s| (&s.name, s.kind).hash(&mut hasher));
        let sig = (hasher.finish(), self.state.fields.len());
        let specs = &self.fields;
        if self.state.synced != Some(sig) {
            self.state
                .fields
                .retain(|name, _| specs.iter().any(|s| &s.name == name));
        }
        for spec in specs {
            if !self.state.fields.contains_key(&spec.name) {
                let fresh = FieldState {
                    value: spec.kind.empty_value(),
                    ..FieldState::default()
                };
                self.state.fields.insert(spec.name.clone(), fresh);
            }
            let Some(f) = self.state.fields.get_mut(&spec.name) else {
                continue;
            };
            if !f.value.fits(spec.kind) {
                crate::kit::sanitize::report(format_args!(
                    "form field `{}` holds {:?}, not a {:?} value",
                    spec.name, f.value, spec.kind
                ));
                f.value = spec.kind.empty_value();
            }
            if let FieldValue::Choice(Some(key)) = &f.value
                && !spec.options.iter().any(|o| &o.key == key)
            {
                f.value = FieldValue::Choice(None);
            }
            if let FieldValue::Choices(keys) = &mut f.value {
                let position = |k: &String| spec.options.iter().position(|o| &o.key == k);
                let ordered = keys.iter().map(position).try_fold(None, |prev, at| match at {
                    Some(at) if prev < Some(at) => Some(Some(at)),
                    _ => None,
                });
                if ordered.is_none() {
                    let picked = std::mem::take(keys);
                    keys.extend(
                        spec.options
                            .iter()
                            .filter(|o| picked.contains(&o.key))
                            .map(|o| o.key.clone()),
                    );
                }
            }
        }
        self.state.synced = Some((sig.0, self.state.fields.len()));
    }

    /// Validates every field and form rule, marking all fields touched. A check targeting a field
    /// that is not in the form reports its error on the form.
    pub fn validate(&mut self, i18n: &i18n::I18n) -> bool {
        let locale = self.locale_for(i18n);
        self.sync_fields();
        self.state.form_error = None;
        for spec in &self.fields {
            let f = self.state.field(&spec.name);
            f.touched = true;
            if !f.pending {
                f.error = spec.check(&f.value, locale.as_ref()).err();
            }
        }
        for check in &self.checks {
            if let Err(CheckError { field, error: e }) = check(self.state) {
                match field {
                    Some(name) if self.fields.iter().any(|s| s.name == name) => {
                        self.state.field(&name).error = Some(e);
                    }
                    Some(name) => {
                        crate::kit::sanitize::report(format_args!("form check targets unknown field `{name}`"));
                        self.state.form_error = Some(e);
                    }
                    None => self.state.form_error = Some(e),
                }
            }
        }
        self.state.is_valid()
    }

    /// Validates and, if valid, starts a submission; the app ends it with [`FormState::finish_submit`].
    pub fn submit(&mut self, i18n: &i18n::I18n) -> Option<FormValues> {
        self.state.submit_count += 1;
        if self.state.submitting || !self.validate(i18n) {
            return None;
        }
        self.state.submitting = true;
        Some(self.state.values())
    }

    /// `field` renders one field and must call [`FieldCtx::changed`]/[`FieldCtx::blurred`];
    /// `footer` renders the form error and submit control and returns `true` when activated. Stylekits wrap it in
    /// their own `show`.
    pub fn show_with<K>(
        mut self,
        ui: &mut crate::widget::MineralUi<'_, K>,
        mut field: impl FnMut(&mut crate::widget::MineralUi<'_, K>, &mut FieldCtx<'_>) -> egui::Response,
        footer: impl FnOnce(&mut crate::widget::MineralUi<'_, K>, &FormFooter<'_>) -> bool,
    ) -> FormResponse {
        self.sync_fields();
        let rt = ui.rt();
        let locale = self.locale_for(rt.core().i18n());
        let scope = ui.scope(|ui| {
            let ui = &mut rt.ui(ui);
            let mut enter = false;
            for spec in &self.fields {
                let Some(state) = self.state.fields.get_mut(&spec.name) else {
                    continue;
                };
                let r = field(
                    ui,
                    &mut FieldCtx {
                        form_salt: &self.salt,
                        spec,
                        state,
                        locale: locale.as_ref(),
                    },
                );
                enter |=
                    spec.kind.submits_on_enter() && r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
            }
            let clicked = footer(
                ui,
                &FormFooter {
                    form_salt: &self.salt,
                    error: self.state.form_error.as_ref(),
                    submit_label: self.submit_label.as_deref(),
                    enabled: !self.state.submitting,
                },
            );
            if clicked || enter {
                self.submit(rt.core().i18n())
            } else {
                None
            }
        });
        FormResponse {
            response: scope.response,
            submitted: scope.inner,
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

    #[test]
    fn validate_and_submit_flow() {
        let mut state = FormState::default();
        state.field("email").set_value("nope");
        state.field("pw").set_value("abc");
        state.field("pw2").set_value("xyz");
        let en = Arc::new(crate::test_support::en_us());
        let ctx = egui::Context::default();
        let rt = crate::test_support::runtime(&ctx);
        let mut form = Form::new("f", &mut state)
            .locale(en)
            .field("email", "Email", |f| {
                f.required(true).validator(|v: &FieldValue, _: &dyn LocaleFormat| {
                    if v.as_text().is_some_and(|t| t.contains('@')) {
                        Ok(())
                    } else {
                        Err(ValidationError::new("email", "Not an email"))
                    }
                })
            })
            .field("pw", "Password", |f| f.required(true).validator(MinLength(3)))
            .field("pw2", "Repeat", |f| f)
            .check(|s| {
                if s.value("pw") == s.value("pw2") {
                    Ok(())
                } else {
                    Err(CheckError::on_field(
                        "pw2",
                        ValidationError::new("mismatch", "Passwords differ"),
                    ))
                }
            });
        assert!(form.submit(rt.core().i18n()).is_none());
        assert_eq!(form.state.fields["email"].error.as_ref().unwrap().code, "email");
        assert_eq!(form.state.fields["pw2"].error.as_ref().unwrap().code, "mismatch");

        form.state.field("email").set_value("a@b.co");
        form.state.field("pw2").set_value("abc");
        let values = form.submit(rt.core().i18n()).expect("valid");
        assert_eq!(values.text("email"), Some("a@b.co"));
        assert!(form.submit(rt.core().i18n()).is_none(), "no double submit");
        assert!(state.submitting());
        assert_eq!(state.submit_count(), 3);
        state.finish_submit(Ok(()));
        assert!(!state.submitting());
    }

    #[test]
    fn field_state_is_changed_through_accessors() {
        let mut state = FormState::default();
        state.field("a").set_value("x");
        assert_eq!(state.text("a"), "x");
        assert!(!state.is_dirty(), "programmatic values are not edits");
        let spec = FieldSpec::new("a", "A".into()).required(true);
        assert_eq!((spec.name(), spec.label(), spec.is_required()), ("a", "A", true));
        let en = crate::test_support::en_us();
        let mut ctx = FieldCtx {
            form_salt: &"f".into(),
            spec: &spec,
            state: state.field("a"),
            locale: &en,
        };
        *ctx.state.value_mut() = FieldValue::Text(String::new());
        ctx.changed();
        ctx.blurred();
        assert!(ctx.state.dirty() && ctx.state.touched());
        assert_eq!(ctx.visible_error().map(|e| e.code.as_ref()), Some("mineral-required"));
        state.set_error("a", None);
        assert!(
            state
                .field_state("a")
                .is_some_and(|f| f.error().is_none() && !f.pending())
        );
        state.field("a").set_pending(true);
        assert!(!state.is_valid(), "a pending check blocks validity");
    }

    #[test]
    fn typed_kinds_hold_their_values_and_submit_them() {
        let mut state = FormState::default();
        state.field("terms").set_value("stale text");
        state.field("size").set_value(FieldValue::Choice(Some("gone".into())));
        state
            .field("tags")
            .set_value(FieldValue::Choices(vec!["b".into(), "x".into(), "a".into()]));
        let en: Arc<dyn LocaleFormat> = Arc::new(crate::test_support::en_us());
        let ctx = egui::Context::default();
        let rt = crate::test_support::runtime(&ctx);
        let day = Date::constant(2026, 11, 20);
        let mut form = Form::new("f", &mut state)
            .locale(en)
            .field("terms", "Accept", |f| f.kind(FieldKind::Checkbox).required(true))
            .field("size", "Size", |f| {
                f.kind(FieldKind::Select).options([("s", "Small"), ("m", "Medium")])
            })
            .field("tags", "Tags", |f| {
                f.kind(FieldKind::MultiSelect)
                    .options([("a", "A"), ("b", "B")])
                    .validator(MaxLength(1))
            })
            .field("day", "Day", |f| f.kind(FieldKind::Date).required(true));
        assert!(form.submit(rt.core().i18n()).is_none());
        assert_eq!(
            form.state.value("terms"),
            Some(&FieldValue::Bool(false)),
            "wrong kind replaced"
        );
        assert_eq!(
            form.state.value("size"),
            Some(&FieldValue::Choice(None)),
            "unknown key dropped"
        );
        assert_eq!(
            form.state.value("tags").and_then(FieldValue::as_choices),
            Some(&["a".to_owned(), "b".to_owned()][..]),
            "known keys in option order"
        );
        let errors = ["terms", "tags", "day"].map(|n| form.state.fields[n].error.as_ref().map(|e| e.code.clone()));
        assert_eq!(
            errors,
            [
                Some("mineral-required".into()),
                Some("mineral-max-picks".into()),
                Some("mineral-required".into())
            ]
        );
        form.state.field("terms").set_value(true);
        form.state
            .field("tags")
            .set_value(FieldValue::Choices(vec!["b".into()]));
        form.state.field("day").set_value(day);
        let values = form.submit(rt.core().i18n()).expect("valid");
        assert_eq!(
            (values.bool("terms"), values.choice("size"), values.date("day")),
            (Some(true), None, Some(day))
        );
        assert_eq!(values.choices("tags"), Some(&["b".to_owned()][..]));
    }

    #[test]
    fn removed_field_does_not_block_submit() {
        let mut state = FormState::default();
        let en: Arc<dyn LocaleFormat> = Arc::new(crate::test_support::en_us());
        let ctx = egui::Context::default();
        let rt = crate::test_support::runtime(&ctx);
        let mut form = Form::new("f", &mut state)
            .locale(en.clone())
            .field("extra", "Extra", |f| f.required(true));
        assert!(form.submit(rt.core().i18n()).is_none());
        let mut form = Form::new("f", &mut state).locale(en).field("name", "Name", |f| f);
        assert_eq!(form.submit(rt.core().i18n()).expect("valid").len(), 1);
    }

    #[test]
    fn check_on_an_unknown_field_reports_on_the_form() {
        let mut state = FormState::default();
        let en: Arc<dyn LocaleFormat> = Arc::new(crate::test_support::en_us());
        let ctx = egui::Context::default();
        let rt = crate::test_support::runtime(&ctx);
        let mut form = Form::new("f", &mut state)
            .locale(en)
            .field("name", "Name", |f| f)
            .check(|_| Err(CheckError::on_field("gone", ValidationError::new("x", "Nope"))));
        assert!(!form.validate(rt.core().i18n()));
        assert_eq!(state.form_error().map(|e| e.code.as_ref()), Some("x"));
    }

    #[test]
    fn sync_prunes_state_added_between_passes() {
        let mut state = FormState::default();
        Form::new("f", &mut state).field("name", "Name", |f| f).sync_fields();
        assert!(state.field_state("name").is_some(), "specs get their state");
        state.field("stale");
        Form::new("f", &mut state).field("name", "Name", |f| f).sync_fields();
        assert_eq!(state.fields().keys().collect::<Vec<_>>(), ["name"]);
    }
}
