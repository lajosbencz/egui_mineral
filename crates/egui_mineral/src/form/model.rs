use std::{collections::HashSet, fmt, sync::Arc};

use egui::Id;

use super::{Blank, FieldInput, FieldKind, FieldSpec, Required, ValidationError, Validator, field_id};
use crate::{i18n::LocaleFormat, semantic::KeyedSalt};

/// Leaf field `V` of the form value `T` shown as kind `Kd`: its key, accessors and presentation. `#[derive(Form)]`
/// generates one const per field.
pub struct FieldKey<T, V: FieldInput, Kd: FieldKind<V> = <V as FieldInput>::Kind> {
    name: &'static str,
    index: usize,
    get: fn(&T) -> &V,
    get_mut: fn(&mut T) -> &mut V,
    spec: fn(&T) -> FieldSpec<V, Kd>,
}

/// Field of `T` holding a nested form value `U`, validated and shown with `U`'s own fields.
pub struct NestedKey<T, U> {
    name: &'static str,
    index: usize,
    get: fn(&T) -> &U,
    get_mut: fn(&mut T) -> &mut U,
}

/// Field of `T` holding a list of form values `U`; item `i` lives under `KEY.at(i)`, so the meta state of an item
/// (dirty, touched, server errors) follows its index, not the item, when items are removed or reordered.
pub struct ListKey<T, U> {
    name: &'static str,
    index: usize,
    get: fn(&T) -> &Vec<U>,
    get_mut: fn(&mut T) -> &mut Vec<U>,
}

impl<T, V: FieldInput, Kd: FieldKind<V>> FieldKey<T, V, Kd> {
    /// `name` keys the field (salt and agent key part); `index` is its declaration order for
    /// [`FormUi::rest`](super::FormUi::rest).
    pub const fn new(
        name: &'static str,
        index: usize,
        get: fn(&T) -> &V,
        get_mut: fn(&mut T) -> &mut V,
        spec: fn(&T) -> FieldSpec<V, Kd>,
    ) -> Self {
        Self {
            name,
            index,
            get,
            get_mut,
            spec,
        }
    }

    pub(super) fn get(self, value: &T) -> &V {
        (self.get)(value)
    }

    pub(super) fn get_mut(self, value: &mut T) -> &mut V {
        (self.get_mut)(value)
    }

    pub(super) fn spec(self, value: &T) -> FieldSpec<V, Kd> {
        (self.spec)(value)
    }
}

impl<T, U> NestedKey<T, U> {
    /// `name` keys the field; `index` is its declaration order.
    pub const fn new(name: &'static str, index: usize, get: fn(&T) -> &U, get_mut: fn(&mut T) -> &mut U) -> Self {
        Self {
            name,
            index,
            get,
            get_mut,
        }
    }

    /// Field `key` of the nested value.
    pub fn then(self, key: impl KeyName<Owner = U>) -> FieldPath {
        FieldPath::from(self).then(key)
    }

    pub(super) fn get(self, value: &T) -> &U {
        (self.get)(value)
    }

    pub(super) fn get_mut(self, value: &mut T) -> &mut U {
        (self.get_mut)(value)
    }
}

impl<T, U> ListKey<T, U> {
    /// `name` keys the field; `index` is its declaration order.
    pub const fn new(
        name: &'static str,
        index: usize,
        get: fn(&T) -> &Vec<U>,
        get_mut: fn(&mut T) -> &mut Vec<U>,
    ) -> Self {
        Self {
            name,
            index,
            get,
            get_mut,
        }
    }

    /// Item `index` of the list.
    pub fn at(self, index: usize) -> FieldPath {
        FieldPath::from(self).at(index)
    }

    pub(super) fn get(self, value: &T) -> &Vec<U> {
        (self.get)(value)
    }

    pub(super) fn get_mut(self, value: &mut T) -> &mut Vec<U> {
        (self.get_mut)(value)
    }
}

/// Path segment of a [`FieldKey`], [`NestedKey`] or [`ListKey`]; sealed.
pub trait KeyName: sealed::Sealed + Copy {
    /// Type the field belongs to.
    type Owner;

    /// Key text.
    fn name(self) -> &'static str;

    /// Declaration order in its type.
    fn index(self) -> usize;
}

mod sealed {
    pub trait Sealed {}
}

macro_rules! key_common {
    ($key:ident<$t:ident $(, $p:ident $(: $bound:path)?)*>) => {
        impl<$t $(, $p $(: $bound)?)*> sealed::Sealed for $key<$t $(, $p)*> {}

        impl<$t $(, $p $(: $bound)?)*> KeyName for $key<$t $(, $p)*> {
            type Owner = $t;

            fn name(self) -> &'static str {
                self.name
            }

            fn index(self) -> usize {
                self.index
            }
        }

        impl<$t $(, $p $(: $bound)?)*> Clone for $key<$t $(, $p)*> {
            fn clone(&self) -> Self {
                *self
            }
        }

        impl<$t $(, $p $(: $bound)?)*> Copy for $key<$t $(, $p)*> {}

        impl<$t $(, $p $(: $bound)?)*> fmt::Debug for $key<$t $(, $p)*> {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.debug_tuple(stringify!($key)).field(&self.name).finish()
            }
        }

        impl<$t $(, $p $(: $bound)?)*> From<$key<$t $(, $p)*>> for FieldPath {
            fn from(key: $key<$t $(, $p)*>) -> Self {
                Self::root(key.name)
            }
        }
    };
}

key_common!(FieldKey<T, V: FieldInput, Kd: FieldKind<V>>);
key_common!(NestedKey<T, U>);
key_common!(ListKey<T, U>);

/// Location of a field in the form value: field keys and list indices from the root.
#[derive(Clone)]
pub struct FieldPath {
    salt: KeyedSalt,
    id: Id,
    // Ids of the strict prefixes, root first.
    ancestors: Arc<[Id]>,
}

impl FieldPath {
    fn root(name: &str) -> Self {
        let salt = KeyedSalt::from(name);
        Self {
            id: field_id(&salt),
            salt,
            ancestors: Arc::from([]),
        }
    }

    fn child(&self, salt: KeyedSalt) -> Self {
        let ancestors = self.ancestors.iter().copied().chain([self.id]).collect();
        Self {
            id: field_id(&salt),
            salt,
            ancestors,
        }
    }

    /// Path of the root field `name`, for fields named at runtime such as a [`DynForm`](super::DynForm)'s.
    pub fn named(name: &str) -> Self {
        Self::root(name)
    }

    pub(super) fn under(prefix: Option<&Self>, name: &str) -> Self {
        prefix.map_or_else(|| Self::root(name), |p| p.child(p.salt.child(name)))
    }

    /// Item `index` of the list at this path.
    #[must_use]
    pub fn at(&self, index: usize) -> Self {
        self.child(self.salt.child(index))
    }

    /// Field `key` of the value at this path.
    #[must_use]
    pub fn then(&self, key: impl KeyName) -> Self {
        self.child(self.salt.child(key.name()))
    }

    /// `self` is `prefix` or lies under it, e.g. a field of a wizard step's nested value.
    pub fn starts_with(&self, prefix: &Self) -> bool {
        self.id == prefix.id || self.ancestors.contains(&prefix.id)
    }

    /// Salt of the path; fields join it to the form salt.
    pub fn salt(&self) -> &KeyedSalt {
        &self.salt
    }

    pub(super) const fn id(&self) -> Id {
        self.id
    }
}

impl From<&Self> for FieldPath {
    fn from(path: &Self) -> Self {
        path.clone()
    }
}

impl PartialEq for FieldPath {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for FieldPath {}

impl fmt::Debug for FieldPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("FieldPath").field(&self.salt.key()).finish()
    }
}

/// Result of validating a form value: errors by field in check order, required fields and the form error.
#[derive(Clone, Debug, Default)]
pub struct Report {
    checked: Vec<FieldPath>,
    errors: Vec<(FieldPath, ValidationError)>,
    required: HashSet<Id>,
    form: Option<ValidationError>,
}

impl Report {
    /// No field or form error.
    pub fn is_valid(&self) -> bool {
        self.errors.is_empty() && self.form.is_none()
    }

    /// First error of the field at `path`.
    pub fn error(&self, path: impl Into<FieldPath>) -> Option<&ValidationError> {
        self.error_of(path.into().id)
    }

    pub(super) fn error_of(&self, id: Id) -> Option<&ValidationError> {
        self.errors.iter().find(|(p, _)| p.id == id).map(|(_, e)| e)
    }

    /// Field errors in check order.
    pub fn errors(&self) -> impl Iterator<Item = (&FieldPath, &ValidationError)> {
        self.errors.iter().map(|(p, e)| (p, e))
    }

    /// First field in check order that has an error, e.g. the wizard step to show.
    pub fn first_invalid(&self) -> Option<&FieldPath> {
        self.errors.first().map(|(p, _)| p)
    }

    /// Error of the form as a whole.
    pub fn form_error(&self) -> Option<&ValidationError> {
        self.form.as_ref()
    }

    /// Field at `path` was checked with [`FieldCheck::required`].
    pub fn is_required(&self, path: impl Into<FieldPath>) -> bool {
        self.required.contains(&path.into().id)
    }

    pub(super) fn required_id(&self, id: Id) -> bool {
        self.required.contains(&id)
    }

    pub(super) fn checked(&self) -> &[FieldPath] {
        &self.checked
    }

    pub(super) fn check(&mut self, path: &FieldPath) {
        if !self.checked.contains(path) {
            self.checked.push(path.clone());
        }
    }

    pub(super) fn require(&mut self, path: &FieldPath) {
        self.required.insert(path.id);
    }

    pub(super) fn fail(&mut self, path: FieldPath, error: ValidationError) {
        self.check(&path);
        if self.error_of(path.id).is_none() {
            self.errors.push((path, error));
        }
    }
}

/// Form value with its validation rules, usually from `#[derive(Form)]`. [`Form`](struct@super::Form) runs
/// [`Self::validate`] every pass over the whole value, so fields not shown (other wizard steps, closed tabs) still
/// count.
pub trait FormModel: Sized {
    /// Checks fields through `v`; the first error of a field wins.
    fn validate(&self, v: &mut Validation<'_, Self>);

    /// [`Self::validate`] outside a form: on load, in tests, server side.
    fn report(&self, locale: &dyn LocaleFormat) -> Report {
        let mut report = Report::default();
        self.validate(&mut Validation::new(self, locale, None, &mut report));
        report
    }
}

/// Collects the checks of a [`FormModel`] `T` into a [`Report`]; keys read their values from `T`.
pub struct Validation<'a, T> {
    value: &'a T,
    locale: &'a dyn LocaleFormat,
    prefix: Option<&'a FieldPath>,
    report: &'a mut Report,
}

crate::kit::opaque_debug!(Validation<'_, T>);

impl<'a, T> Validation<'a, T> {
    pub(super) fn new(
        value: &'a T,
        locale: &'a dyn LocaleFormat,
        prefix: Option<&'a FieldPath>,
        report: &'a mut Report,
    ) -> Self {
        Self {
            value,
            locale,
            prefix,
            report,
        }
    }

    fn path(&self, key: impl KeyName) -> FieldPath {
        FieldPath::under(self.prefix, key.name())
    }

    /// Value under validation.
    pub fn value(&self) -> &'a T {
        self.value
    }

    /// Parses locale-formatted input in validators.
    pub fn locale(&self) -> &'a dyn LocaleFormat {
        self.locale
    }

    /// Checks of field `key`; an unchecked field still counts as part of the form.
    pub fn field<V: FieldInput, Kd: FieldKind<V>>(&mut self, key: FieldKey<T, V, Kd>) -> FieldCheck<'_, V> {
        let path = self.path(key);
        self.report.check(&path);
        FieldCheck {
            value: key.get(self.value),
            report: self.report,
            locale: self.locale,
            path,
            failed: false,
        }
    }

    // Path of the field `name` under this value, the report and the locale, for fields named at runtime.
    pub(super) fn named(&mut self, name: &str) -> (FieldPath, &mut Report, &'a dyn LocaleFormat) {
        (FieldPath::under(self.prefix, name), self.report, self.locale)
    }

    /// Error on the field `name` of this value, e.g. from a cross-field rule of a [`DynForm`](super::DynForm).
    pub fn error_named(&mut self, name: &str, error: ValidationError) {
        let path = FieldPath::under(self.prefix, name);
        self.report.fail(path, error);
    }

    /// Error on field `key`, e.g. from a cross-field rule.
    pub fn error(&mut self, key: impl KeyName<Owner = T>, error: ValidationError) {
        let path = self.path(key);
        self.report.fail(path, error);
    }

    /// Error of the form as a whole; the first one wins.
    pub fn form_error(&mut self, error: ValidationError) {
        self.report.form.get_or_insert(error);
    }

    /// Validates the nested value of `key` with its own rules, under the path of `key`.
    pub fn nested<U: FormModel>(&mut self, key: NestedKey<T, U>) {
        let path = self.path(key);
        let value = key.get(self.value);
        value.validate(&mut Validation::new(value, self.locale, Some(&path), self.report));
    }

    /// Validates each item of list `key` with its own rules, under `key.at(index)`.
    pub fn each<U: FormModel>(&mut self, key: ListKey<T, U>) {
        let path = self.path(key);
        self.report.check(&path);
        for (i, item) in key.get(self.value).iter().enumerate() {
            let at = path.at(i);
            item.validate(&mut Validation::new(item, self.locale, Some(&at), self.report));
        }
    }
}

/// Checks of one field; the first failing check sets its error. Optional blank values skip [`Self::check`].
pub struct FieldCheck<'v, V> {
    value: &'v V,
    report: &'v mut Report,
    locale: &'v dyn LocaleFormat,
    path: FieldPath,
    failed: bool,
}

crate::kit::opaque_debug!(FieldCheck<'_, V>);

impl<V: FieldInput> FieldCheck<'_, V> {
    /// Runs `validator` unless blank or failed already.
    #[expect(
        clippy::needless_pass_by_value,
        reason = "validators are built inline: `.check(MinLength(3))`"
    )]
    pub fn check(&mut self, validator: impl Validator<V>) -> &mut Self {
        if !self.value.is_blank() {
            self.run(&validator);
        }
        self
    }

    fn run(&mut self, validator: &dyn Validator<V>) {
        if self.failed {
            return;
        }
        if let Err(e) = validator.validate(self.value, self.locale) {
            self.failed = true;
            self.report.fail(self.path.clone(), e);
        }
    }
}

impl<V: Blank> FieldCheck<'_, V> {
    /// Blank fails with [`Required`]; marks the field required for its label.
    pub fn required(&mut self) -> &mut Self {
        self.report.required.insert(self.path.id);
        self.run(&Required);
        self
    }
}
