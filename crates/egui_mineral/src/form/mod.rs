//! Form state, validation and submission. Headless: stylekits render fields and the footer through
//! [`Form::show_with`].

mod validators;

use std::{
    borrow::Cow,
    collections::BTreeMap,
    hash::{DefaultHasher, Hash as _, Hasher as _},
    sync::Arc,
};

use serde::{Deserialize, Serialize};

use crate::{
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

    /// Message id resolved through [`I18n::tr`](i18n::I18n::tr) at display time.
    pub fn localized(id: impl Into<Cow<'static, str>>) -> Self {
        Self {
            code: id.into(),
            args: Vec::new(),
            text: None,
        }
    }

    /// Appends a Fluent argument used when resolving `code`.
    #[must_use]
    pub fn arg(mut self, name: &'static str, value: impl Into<Arg<'static>>) -> Self {
        self.args.push((Cow::Borrowed(name), value.into()));
        self
    }

    /// Display text: `text` if set, else `code` translated by [`I18n::tr`](i18n::I18n::tr) with `args`.
    pub fn message(&self, i18n: &i18n::I18n) -> String {
        if let Some(t) = &self.text {
            return t.to_string();
        }
        let args: Vec<(&str, Arg<'_>)> = self.args.iter().map(|(k, v)| (k.as_ref(), v.clone())).collect();
        i18n.tr(&self.code, &args)
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
    /// Never sees an empty value when called through [`FieldSpec::check`].
    ///
    /// # Errors
    /// [`ValidationError`] describing why `value` is rejected.
    fn validate(&self, value: &str, locale: &dyn LocaleFormat) -> Result<(), ValidationError>;
}

impl<F: Fn(&str, &dyn LocaleFormat) -> Result<(), ValidationError> + Send + Sync> Validator for F {
    fn validate(&self, value: &str, locale: &dyn LocaleFormat) -> Result<(), ValidationError> {
        self(value, locale)
    }
}

/// When a field re-validates on its own (submit always validates everything).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
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
    value: String,
    dirty: bool,
    touched: bool,
    #[serde(skip)]
    pending: bool,
    #[serde(skip)]
    error: Option<ValidationError>,
}

impl FieldState {
    /// Raw text as entered; validators do the locale parsing.
    pub fn value(&self) -> &str {
        &self.value
    }

    /// Renderer access to the text; call [`FieldCtx::changed`] after editing.
    pub fn value_mut(&mut self) -> &mut String {
        &mut self.value
    }

    /// Replaces the text from code (initial values); does not mark the field dirty.
    pub fn set_value(&mut self, value: impl Into<String>) {
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

    /// Empty for unknown fields.
    pub fn value(&self, name: &str) -> &str {
        self.fields.get(name).map_or("", FieldState::value)
    }

    /// Snapshot of all values by field name.
    pub fn values(&self) -> BTreeMap<String, String> {
        self.fields.iter().map(|(k, v)| (k.clone(), v.value.clone())).collect()
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
    /// Masked single line.
    Password,
    /// Enter inserts a newline instead of submitting.
    Multiline,
}

/// Declaration of one [`Form`] field: name, label, kind, validators.
#[must_use]
pub struct FieldSpec {
    name: String,
    label: String,
    supporting: Option<String>,
    required: bool,
    kind: FieldKind,
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

    /// Input form; [`FieldKind::Multiline`] disables Enter-to-submit.
    pub fn input_kind(&self) -> FieldKind {
        self.kind
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
            validate_on: ValidateOn::default(),
            validators: Vec::new(),
        }
    }

    /// Helper text shown with the field.
    pub fn supporting(mut self, supporting: impl Into<String>) -> Self {
        self.supporting = Some(supporting.into());
        self
    }

    /// Empty values fail with [`Required`]; when `false`, empty values skip validators.
    pub fn required(mut self, required: bool) -> Self {
        self.required = required;
        self
    }

    /// Defaults to [`FieldKind::Text`].
    pub fn kind(mut self, kind: FieldKind) -> Self {
        self.kind = kind;
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
    pub fn check(&self, value: &str, locale: &dyn LocaleFormat) -> Result<(), ValidationError> {
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
    /// Renderers edit [`FieldState::value_mut`], then call [`Self::changed`].
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
    pub submitted: Option<BTreeMap<String, String>>,
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

    // Drops state without a spec and adds missing entries; a no-op while specs and state are unchanged.
    fn sync_fields(&mut self) {
        let mut hasher = DefaultHasher::new();
        self.fields.iter().for_each(|s| s.name.hash(&mut hasher));
        let sig = (hasher.finish(), self.state.fields.len());
        if self.state.synced == Some(sig) {
            return;
        }
        let specs = &self.fields;
        self.state
            .fields
            .retain(|name, _| specs.iter().any(|s| &s.name == name));
        for spec in specs {
            if !self.state.fields.contains_key(&spec.name) {
                self.state.fields.insert(spec.name.clone(), FieldState::default());
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
    pub fn submit(&mut self, i18n: &i18n::I18n) -> Option<BTreeMap<String, String>> {
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
                enter |= spec.kind != FieldKind::Multiline
                    && r.lost_focus()
                    && ui.input(|i| i.key_pressed(egui::Key::Enter));
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
                f.required(true).validator(|v: &str, _: &dyn LocaleFormat| {
                    if v.contains('@') {
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
        assert_eq!(values["email"], "a@b.co");
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
        assert_eq!(state.value("a"), "x");
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
        ctx.state.value_mut().clear();
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
