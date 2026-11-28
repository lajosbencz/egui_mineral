use std::{fmt, sync::Arc};

use serde::{Deserialize, Serialize};

use super::{
    Choices, FieldInput, FieldPath, FieldSpec, FieldWidget, FormFields, FormModel, FormUi, Label, MaxLength, MinLength,
    NumberOptions, NumberRange, Required, TextOptions, ValidateOn, Validation, ValidationError, Validator, kind,
};
use crate::{date::Date, i18n::LocaleFormat, select::Selection};

/// Value of a [`DynField`]; the variant follows its [`DynKind`].
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum DynValue {
    /// [`DynKind::Text`].
    Text(String),
    /// [`DynKind::Check`] and [`DynKind::Switch`].
    Flag(bool),
    /// [`DynKind::Number`] and [`DynKind::Slider`].
    Number(f64),
    /// [`DynKind::Date`].
    Date(Option<Date>),
    /// [`DynKind::Pick`]: key of the picked choice.
    Pick(Option<String>),
    /// [`DynKind::MultiPick`]: keys of the picked choices.
    Picks(Selection<String>),
}

impl DynValue {
    /// Blank text, unchecked, no date, nothing picked ([`FieldInput::is_blank`]); a number never is.
    pub fn is_blank(&self) -> bool {
        match self {
            Self::Text(v) => v.is_blank(),
            Self::Flag(v) => v.is_blank(),
            Self::Number(_) => false,
            Self::Date(v) => v.is_blank(),
            Self::Pick(v) => v.is_blank(),
            Self::Picks(v) => v.is_blank(),
        }
    }
}

/// Widget kind of a [`DynField`] with its options, one per core [`kind`] marker.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum DynKind {
    /// [`kind::Text`].
    Text(TextOptions),
    /// [`kind::Check`].
    Check,
    /// [`kind::Switch`].
    Switch,
    /// [`kind::Number`].
    Number(NumberOptions),
    /// [`kind::Slider`].
    Slider(NumberOptions),
    /// [`kind::Date`], optional.
    Date,
    /// [`kind::Pick`] over choice keys.
    Pick(Choices<String>),
    /// [`kind::MultiPick`] over choice keys.
    MultiPick(Choices<String>),
}

impl DynKind {
    /// Value of a new field: empty text, unchecked, the range minimum (else 0), no date, nothing picked.
    pub fn default_value(&self) -> DynValue {
        match self {
            Self::Text(_) => DynValue::Text(String::new()),
            Self::Check | Self::Switch => DynValue::Flag(false),
            Self::Number(o) | Self::Slider(o) => DynValue::Number(o.range.map_or(0.0, |r| r.min())),
            Self::Date => DynValue::Date(None),
            Self::Pick(_) => DynValue::Pick(None),
            Self::MultiPick(_) => DynValue::Picks(Selection::default()),
        }
    }

    /// `value` is the variant this kind holds; a non-finite number is not held.
    pub fn holds(&self, value: &DynValue) -> bool {
        !matches!(value, DynValue::Number(n) if !n.is_finite())
            && matches!(
                (self, value),
                (Self::Text(_), DynValue::Text(_))
                    | (Self::Check | Self::Switch, DynValue::Flag(_))
                    | (Self::Number(_) | Self::Slider(_), DynValue::Number(_))
                    | (Self::Date, DynValue::Date(_))
                    | (Self::Pick(_), DynValue::Pick(_))
                    | (Self::MultiPick(_), DynValue::Picks(_))
            )
    }
}

// Prebuilt spec per kind, so a pass borrows label and options instead of cloning them.
#[derive(Clone)]
enum Spec {
    Text(FieldSpec<String, kind::Text>),
    Check(FieldSpec<bool, kind::Check>),
    Switch(FieldSpec<bool, kind::Switch>),
    Number(FieldSpec<f64, kind::Number>),
    Slider(FieldSpec<f64, kind::Slider>),
    Date(FieldSpec<Option<Date>, kind::Date>),
    Pick(FieldSpec<Option<String>, kind::Pick>),
    MultiPick(FieldSpec<Selection<String>, kind::MultiPick>),
}

macro_rules! each_spec {
    ($spec:expr, $s:ident => $e:expr) => {
        match $spec {
            Spec::Text($s) => Spec::Text($e),
            Spec::Check($s) => Spec::Check($e),
            Spec::Switch($s) => Spec::Switch($e),
            Spec::Number($s) => Spec::Number($e),
            Spec::Slider($s) => Spec::Slider($e),
            Spec::Date($s) => Spec::Date($e),
            Spec::Pick($s) => Spec::Pick($e),
            Spec::MultiPick($s) => Spec::MultiPick($e),
        }
    };
}

impl Spec {
    fn new(kind: DynKind, label: Label) -> Self {
        let on = ValidateOn::default();
        match kind {
            DynKind::Text(o) => Self::Text(FieldSpec::of(label, None, on, o)),
            DynKind::Check => Self::Check(FieldSpec::of(label, None, on, ())),
            DynKind::Switch => Self::Switch(FieldSpec::of(label, None, on, ())),
            DynKind::Number(o) => Self::Number(FieldSpec::of(label, None, on, o)),
            DynKind::Slider(o) => Self::Slider(FieldSpec::of(label, None, on, o)),
            DynKind::Date => Self::Date(FieldSpec::of(label, None, on, ())),
            DynKind::Pick(o) => Self::Pick(FieldSpec::of(label, None, on, o)),
            DynKind::MultiPick(o) => Self::MultiPick(FieldSpec::of(label, None, on, o)),
        }
    }

    fn holds(&self, value: &DynValue) -> bool {
        !matches!(value, DynValue::Number(n) if !n.is_finite())
            && matches!(
                (self, value),
                (Self::Text(_), DynValue::Text(_))
                    | (Self::Check(_) | Self::Switch(_), DynValue::Flag(_))
                    | (Self::Number(_) | Self::Slider(_), DynValue::Number(_))
                    | (Self::Date(_), DynValue::Date(_))
                    | (Self::Pick(_), DynValue::Pick(_))
                    | (Self::MultiPick(_), DynValue::Picks(_))
            )
    }
}

/// Field of a [`DynForm`] described by data: key, kind, label, help text, rules and value.
#[derive(Clone)]
#[must_use]
pub struct DynField {
    key: String,
    spec: Spec,
    required: bool,
    checks: Vec<Arc<dyn Validator<DynValue>>>,
    value: DynValue,
}

impl fmt::Debug for DynField {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DynField")
            .field("key", &self.key)
            .field("value", &self.value)
            .finish_non_exhaustive()
    }
}

impl DynField {
    /// `key` names the field: its [`FieldPath::named`] path, salt and agent key part. The value starts at
    /// [`DynKind::default_value`].
    pub fn new(key: impl Into<String>, kind: DynKind, label: impl Into<Label>) -> Self {
        Self {
            key: key.into(),
            value: kind.default_value(),
            spec: Spec::new(kind, label.into()),
            required: false,
            checks: Vec::new(),
        }
    }

    /// Helper text shown with the field.
    pub fn supporting(mut self, supporting: impl Into<Label>) -> Self {
        let supporting = supporting.into();
        self.spec = each_spec!(self.spec, s => s.supporting(supporting));
        self
    }

    /// Defaults to [`ValidateOn::Blur`].
    pub fn validate_on(mut self, validate_on: ValidateOn) -> Self {
        self.spec = each_spec!(self.spec, s => s.validate_on(validate_on));
        self
    }

    /// Blank fails with [`Required`] and the label is marked; default `false`.
    pub fn required(mut self, required: bool) -> Self {
        self.required = required;
        self
    }

    /// Rule run in order on a non-blank value; the first failure wins. [`MinLength`], [`MaxLength`] and
    /// [`NumberRange`] check the variants they apply to and pass the others.
    pub fn check(mut self, validator: impl Validator<DynValue> + 'static) -> Self {
        self.checks.push(Arc::new(validator));
        self
    }

    /// Starting value; ignored unless the field's [`DynKind::holds`] it.
    pub fn initial(mut self, value: DynValue) -> Self {
        if self.spec.holds(&value) {
            self.value = value;
        }
        self
    }

    /// Key of the field.
    pub fn key(&self) -> &str {
        &self.key
    }

    /// Current value.
    pub fn value(&self) -> &DynValue {
        &self.value
    }
}

/// Form value whose fields arrive as data at runtime, e.g. a plugin's form. Implements [`FormModel`] and
/// [`FormFields`], so a [`Form`](struct@super::Form) drives it with the same [`FormState`](super::FormState),
/// validators and stylekit [`FieldWidget`]s as a `#[derive(Form)]` type; field `key` is at
/// [`FieldPath::named`]`(key)`. Cross-field rules go in [`Form::rule`](super::Form::rule) through
/// [`Validation::error_named`].
#[derive(Clone, Debug, Default)]
pub struct DynForm {
    fields: Vec<DynField>,
}

impl DynForm {
    /// Fields in display order; a field whose key repeats an earlier one is dropped.
    pub fn new(fields: impl IntoIterator<Item = DynField>) -> Self {
        let mut out: Vec<DynField> = Vec::new();
        for f in fields {
            if out.iter().any(|o| o.key == f.key) {
                crate::kit::sanitize::report(format_args!("dynamic form field {:?} repeats a key", f.key));
            } else {
                out.push(f);
            }
        }
        Self { fields: out }
    }

    /// Fields in display order.
    pub fn fields(&self) -> &[DynField] {
        &self.fields
    }

    /// Value of field `key`.
    pub fn value(&self, key: &str) -> Option<&DynValue> {
        self.field(key).map(DynField::value)
    }

    /// Keys and values in display order.
    pub fn values(&self) -> impl Iterator<Item = (&str, &DynValue)> {
        self.fields.iter().map(|f| (f.key.as_str(), &f.value))
    }

    /// Replaces the value of field `key`, as a code edit; `false` for an unknown key or a value its kind does not
    /// hold ([`DynKind::holds`]).
    pub fn set(&mut self, key: &str, value: DynValue) -> bool {
        match self.fields.iter_mut().find(|f| f.key == key) {
            Some(f) if f.spec.holds(&value) => {
                f.value = value;
                true
            }
            _ => false,
        }
    }

    fn field(&self, key: &str) -> Option<&DynField> {
        self.fields.iter().find(|f| f.key == key)
    }
}

impl FormModel for DynForm {
    fn validate(&self, v: &mut Validation<'_, Self>) {
        for f in &self.fields {
            let (path, report, locale) = v.named(&f.key);
            report.check(&path);
            if f.required {
                report.require(&path);
            }
            let failed = if f.value.is_blank() {
                f.required.then(|| Required.validate(&f.value, locale).err()).flatten()
            } else {
                f.checks.iter().find_map(|c| c.validate(&f.value, locale).err())
            };
            if let Some(e) = failed {
                report.fail(path, e);
            }
        }
    }
}

/// Stylekit with a [`FieldWidget`] for every [`DynKind`]; implemented for each such kit.
pub trait DynWidgets:
    FieldWidget<kind::Text, String>
    + FieldWidget<kind::Check, bool>
    + FieldWidget<kind::Switch, bool>
    + FieldWidget<kind::Number, f64>
    + FieldWidget<kind::Slider, f64>
    + FieldWidget<kind::Date, Option<Date>>
    + FieldWidget<kind::Pick, Option<String>>
    + FieldWidget<kind::MultiPick, Selection<String>>
{
}

impl<K> DynWidgets for K where
    K: FieldWidget<kind::Text, String>
        + FieldWidget<kind::Check, bool>
        + FieldWidget<kind::Switch, bool>
        + FieldWidget<kind::Number, f64>
        + FieldWidget<kind::Slider, f64>
        + FieldWidget<kind::Date, Option<Date>>
        + FieldWidget<kind::Pick, Option<String>>
        + FieldWidget<kind::MultiPick, Selection<String>>
{
}

impl<K: DynWidgets> FormFields<K> for DynForm {
    fn field_count(&self) -> usize {
        self.fields.len()
    }

    fn show_index(f: &mut FormUi<'_, '_, '_, K, Self>, index: usize) {
        f.show_dyn(index);
    }
}

impl<K: DynWidgets> FormUi<'_, '_, '_, K, DynForm> {
    /// Field `key` in the kit widget of its kind; `None` for an unknown key.
    pub fn field(&mut self, key: &str) -> Option<egui::Response> {
        let index = self.value.fields.iter().position(|f| f.key == key)?;
        self.show_dyn(index)
    }

    fn show_dyn(&mut self, index: usize) -> Option<egui::Response> {
        if index < self.value.fields.len() {
            self.mark_index(index);
        }
        let DynField { key, spec, value, .. } = self.value.fields.get_mut(index)?;
        let path = FieldPath::under(self.prefix.as_ref(), key);
        let pass = &mut *self.pass;
        let response = match (&*spec, value) {
            (Spec::Text(s), DynValue::Text(v)) => pass.field(&path, s, v, <K as FieldWidget<kind::Text, String>>::show),
            (Spec::Check(s), DynValue::Flag(v)) => pass.field(&path, s, v, <K as FieldWidget<kind::Check, bool>>::show),
            (Spec::Switch(s), DynValue::Flag(v)) => {
                pass.field(&path, s, v, <K as FieldWidget<kind::Switch, bool>>::show)
            }
            (Spec::Number(s), DynValue::Number(v)) => {
                pass.field(&path, s, v, <K as FieldWidget<kind::Number, f64>>::show)
            }
            (Spec::Slider(s), DynValue::Number(v)) => {
                pass.field(&path, s, v, <K as FieldWidget<kind::Slider, f64>>::show)
            }
            (Spec::Date(s), DynValue::Date(v)) => {
                pass.field(&path, s, v, <K as FieldWidget<kind::Date, Option<Date>>>::show)
            }
            (Spec::Pick(s), DynValue::Pick(v)) => {
                pass.field(&path, s, v, <K as FieldWidget<kind::Pick, Option<String>>>::show)
            }
            (Spec::MultiPick(s), DynValue::Picks(v)) => pass.field(
                &path,
                s,
                v,
                <K as FieldWidget<kind::MultiPick, Selection<String>>>::show,
            ),
            // `DynField` keeps the value in its kind's variant.
            _ => {
                crate::kit::sanitize::report(format_args!("dynamic form field {path:?} holds another kind's value"));
                return None;
            }
        };
        Some(response)
    }
}

impl Validator<DynValue> for Required {
    fn validate(&self, value: &DynValue, _locale: &dyn LocaleFormat) -> Result<(), ValidationError> {
        if value.is_blank() {
            Err(ValidationError::localized(crate::i18n::msg::REQUIRED))
        } else {
            Ok(())
        }
    }
}

impl Validator<DynValue> for MinLength {
    fn validate(&self, value: &DynValue, locale: &dyn LocaleFormat) -> Result<(), ValidationError> {
        match value {
            DynValue::Text(v) => self.validate(v, locale),
            DynValue::Picks(v) => self.validate(v, locale),
            _ => Ok(()),
        }
    }
}

impl Validator<DynValue> for MaxLength {
    fn validate(&self, value: &DynValue, locale: &dyn LocaleFormat) -> Result<(), ValidationError> {
        match value {
            DynValue::Text(v) => self.validate(v, locale),
            DynValue::Picks(v) => self.validate(v, locale),
            _ => Ok(()),
        }
    }
}

impl Validator<DynValue> for NumberRange {
    fn validate(&self, value: &DynValue, locale: &dyn LocaleFormat) -> Result<(), ValidationError> {
        match value {
            DynValue::Number(v) => self.validate(v, locale),
            _ => Ok(()),
        }
    }
}
