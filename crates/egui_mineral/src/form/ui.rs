use egui::Id;

use super::{
    FieldInput, FieldKey, FieldKind, FieldPath, FieldPlacement, FieldState, FormModel, FormRow, FormState, KeyName,
    Label, LabelSlot, ListKey, NestedKey, NumberOptions, Report, TextOptions, ValidateOn, ValidationError,
    input::Choices,
};
use crate::{range::RangeModel, semantic::KeyedSalt, widget::MineralUi};

/// Presentation of one field of type `V` shown as kind `Kd`: label, help text, when its shown error updates, and the
/// options of its kind. Its rules live on the [`FormModel`].
#[must_use]
pub struct FieldSpec<V: FieldInput, Kd: FieldKind<V> = <V as FieldInput>::Kind> {
    label: Label,
    supporting: Option<Label>,
    validate_on: ValidateOn,
    options: Kd::Options,
}

impl<V: FieldInput, Kd: FieldKind<V>> std::fmt::Debug for FieldSpec<V, Kd>
where
    Kd::Options: std::fmt::Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FieldSpec")
            .field("label", &self.label)
            .field("supporting", &self.supporting)
            .field("validate_on", &self.validate_on)
            .field("options", &self.options)
            .finish()
    }
}

impl<V: FieldInput, Kd: FieldKind<V>> FieldSpec<V, Kd> {
    /// `label` is the visible, accessible name.
    pub fn new(label: impl Into<Label>) -> Self {
        Self {
            label: label.into(),
            supporting: None,
            validate_on: ValidateOn::default(),
            options: Kd::Options::default(),
        }
    }

    /// Visible text; also the accessible name.
    pub fn label(&self) -> &Label {
        &self.label
    }

    /// Value of [`Self::supporting`].
    pub fn supporting_label(&self) -> Option<&Label> {
        self.supporting.as_ref()
    }

    /// Value of [`Self::validate_on`].
    pub fn validates_on(&self) -> ValidateOn {
        self.validate_on
    }

    /// Options of the field's kind.
    pub fn options(&self) -> &Kd::Options {
        &self.options
    }

    /// Helper text shown with the field.
    pub fn supporting(mut self, supporting: impl Into<Label>) -> Self {
        self.supporting = Some(supporting.into());
        self
    }

    /// Defaults to [`ValidateOn::Blur`].
    pub fn validate_on(mut self, validate_on: ValidateOn) -> Self {
        self.validate_on = validate_on;
        self
    }
}

impl<V: FieldInput, Kd: FieldKind<V, Options = TextOptions>> FieldSpec<V, Kd> {
    /// Masked input (passwords); default `false`.
    pub fn masked(mut self, masked: bool) -> Self {
        self.options.masked = masked;
        self
    }

    /// Visible text lines; more than one makes Enter insert a newline instead of submitting. Default 1.
    pub fn rows(mut self, rows: usize) -> Self {
        self.options.rows = rows.max(1);
        self
    }

    /// Shown while empty.
    pub fn placeholder(mut self, placeholder: impl Into<Label>) -> Self {
        self.options.placeholder = Some(placeholder.into());
        self
    }
}

impl<V: FieldInput, T, Kd: FieldKind<V, Options = Choices<T>>> FieldSpec<V, Kd> {
    /// `(value, label)` options in display order.
    pub fn choices<L: Into<Label>>(mut self, items: impl IntoIterator<Item = (T, L)>) -> Self {
        self.options = Choices(items.into_iter().map(|(v, l)| (v, l.into())).collect());
        self
    }
}

impl<V: FieldInput, Kd: FieldKind<V, Options = NumberOptions>> FieldSpec<V, Kd> {
    /// Bounds and step of the number.
    pub fn range(mut self, range: impl Into<RangeModel>) -> Self {
        self.options.range = Some(range.into());
        self
    }
}

/// Per-field handle for stylekit widgets: the bound value, its spec with resolved texts, and edit, blur and Enter
/// reports.
pub struct FieldCtx<'a, V: FieldInput, Kd: FieldKind<V> = <V as FieldInput>::Kind> {
    salt: KeyedSalt,
    spec: &'a FieldSpec<V, Kd>,
    label: &'a str,
    supporting: Option<&'a str>,
    value: &'a mut V,
    state: &'a mut FieldState,
    required: bool,
    refresh: bool,
    changed: bool,
    submit: bool,
    slot: LabelSlot,
}

impl<V: FieldInput, Kd: FieldKind<V>> std::fmt::Debug for FieldCtx<'_, V, Kd> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FieldCtx").finish_non_exhaustive()
    }
}

impl<'a, V: FieldInput, Kd: FieldKind<V>> FieldCtx<'a, V, Kd> {
    /// Widget salt of the field: the form salt joined with the field path.
    pub fn salt(&self) -> KeyedSalt {
        self.salt.clone()
    }

    /// Declaration of the field.
    pub fn spec(&self) -> &'a FieldSpec<V, Kd> {
        self.spec
    }

    /// Label in the active locale.
    pub fn label(&self) -> &'a str {
        self.label
    }

    /// Helper text in the active locale.
    pub fn supporting(&self) -> Option<&'a str> {
        self.supporting
    }

    /// Model rules call [`FieldCheck::required`](super::FieldCheck::required) on this field; kits mark the label.
    pub fn required(&self) -> bool {
        self.required
    }

    /// Where the field shows its label; [`LabelSlot::Own`] outside a [`FieldPlacement`].
    pub fn label_slot(&self) -> LabelSlot {
        self.slot
    }

    /// Bound value; call [`Self::changed`] after an edit.
    pub fn value(&mut self) -> &mut V {
        self.value
    }

    /// Call after an edit: marks dirty and drops a server error, which the edit answers; the shown error updates
    /// under [`ValidateOn::Change`]. A shown rule error follows the rules under every mode, so it clears on the
    /// edit that fixes it.
    pub fn changed(&mut self) {
        self.state.dirty = true;
        self.changed = true;
        if self.state.external {
            self.state.external = false;
            self.state.error = None;
        }
        if self.spec.validate_on == ValidateOn::Change {
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

/// Stylekit widget of kind `Kd` for value `V`, implemented on the stylekit for each pair it renders.
pub trait FieldWidget<Kd: FieldKind<V>, V: FieldInput>: Sized {
    /// Shows the input in `ui` and reports edits through `field`.
    fn show(ui: &mut MineralUi<'_, Self>, field: &mut FieldCtx<'_, V, Kd>) -> egui::Response;
}

/// Declared fields of a form value, rendered in order by [`FormUi::rest`]; implemented by `#[derive(Form)]`.
pub trait FormFields<K>: FormModel {
    /// Number of declared fields.
    const FIELDS: usize;

    /// Shows the field declared at `index` through `f`.
    fn show_index(f: &mut FormUi<'_, '_, '_, K, Self>, index: usize);
}

pub(super) struct Pass<'p, 's, K> {
    pub(super) ui: &'p mut MineralUi<'s, K>,
    pub(super) salt: &'p KeyedSalt,
    pub(super) state: &'p mut FormState,
    pub(super) report: &'p Report,
    pub(super) seen: Vec<Id>,
    pub(super) refreshed: Vec<Id>,
    pub(super) changed: bool,
    pub(super) submit: bool,
    pub(super) placement: Option<&'p mut dyn FieldPlacement<K>>,
}

impl<K> Pass<'_, '_, K> {
    fn field<V: FieldInput, Kd: FieldKind<V>>(
        &mut self,
        path: &FieldPath,
        spec: &FieldSpec<V, Kd>,
        value: &mut V,
        render: impl FnOnce(&mut MineralUi<'_, K>, &mut FieldCtx<'_, V, Kd>) -> egui::Response,
    ) -> egui::Response {
        let id = path.id();
        if self.seen.contains(&id) {
            crate::kit::sanitize::report(format_args!("form field {path:?} shown twice in one pass"));
        }
        self.seen.push(id);
        let i18n = self.ui.i18n();
        let label = spec.label().text(i18n);
        let supporting = spec.supporting_label().and_then(|l| l.optional(i18n));
        let required = self.report.required_id(id);
        let state = self.state.fields.entry(id).or_default();
        state.path = Some(path.clone());
        let mut ctx = FieldCtx {
            salt: self.salt.join(path.salt()),
            spec,
            label: &label,
            supporting: supporting.as_deref(),
            value,
            state,
            required,
            refresh: false,
            changed: false,
            submit: false,
            slot: LabelSlot::Own,
        };
        let row = FormRow::Field {
            label: &label,
            required,
        };
        let response = match self.placement.as_deref_mut() {
            Some(p) => p.place(
                self.ui,
                row,
                Box::new(|ui, slot| {
                    ctx.slot = slot;
                    render(ui, &mut ctx)
                }),
            ),
            None => render(self.ui, &mut ctx),
        };
        if ctx.refresh {
            self.refreshed.push(id);
        }
        self.changed |= ctx.changed;
        self.submit |= ctx.submit;
        response
    }
}

/// Places the fields of a form value `T` (the whole form, or a nested value or list item of it).
pub struct FormUi<'u, 'p, 's, K, T> {
    pub(super) pass: &'u mut Pass<'p, 's, K>,
    pub(super) value: &'u mut T,
    pub(super) prefix: Option<FieldPath>,
    pub(super) placed: Vec<usize>,
}

crate::kit::opaque_debug!(FormUi<'_, '_, '_, K, T>);

impl<'p, 's, K, T> FormUi<'_, 'p, 's, K, T> {
    /// Ui the fields are placed in, for content between them.
    pub fn ui(&mut self) -> &mut MineralUi<'s, K> {
        self.pass.ui
    }

    /// Form salt; field widgets derive their salts from it.
    pub fn form_salt(&self) -> &KeyedSalt {
        self.pass.salt
    }

    /// Rule results of the value at the start of this pass.
    pub fn report(&self) -> &Report {
        self.pass.report
    }

    /// Value these fields edit; edits through it are code edits, not user edits.
    pub fn value(&mut self) -> &mut T {
        self.value
    }

    /// Path of the value these fields edit; `None` at the form root.
    pub fn prefix(&self) -> Option<&FieldPath> {
        self.prefix.as_ref()
    }

    fn path(&self, key: impl KeyName) -> FieldPath {
        FieldPath::under(self.prefix.as_ref(), key.name())
    }

    pub(super) fn mark(&mut self, key: impl KeyName) {
        let index = key.index();
        if !self.placed.contains(&index) {
            self.placed.push(index);
        }
    }

    /// Field `key` in the kit widget of its kind.
    pub fn show<V: FieldInput, Kd: FieldKind<V>>(&mut self, key: FieldKey<T, V, Kd>) -> egui::Response
    where
        K: FieldWidget<Kd, V>,
    {
        self.custom(key, K::show)
    }

    /// Field `key` in a widget `render` draws, e.g. a kit widget variant or an app control.
    pub fn custom<V: FieldInput, Kd: FieldKind<V>>(
        &mut self,
        key: FieldKey<T, V, Kd>,
        render: impl FnOnce(&mut MineralUi<'_, K>, &mut FieldCtx<'_, V, Kd>) -> egui::Response,
    ) -> egui::Response {
        self.mark(key);
        let path = self.path(key);
        let spec = key.spec(self.value);
        self.pass.field(&path, &spec, key.get_mut(self.value), render)
    }

    /// Field at the absolute `path` bound to `value` outside the declared fields, e.g. an enum variant's data or app
    /// state (build it from [`Self::prefix`] inside [`Self::nested`] and [`Self::list`]); the rules see it only when
    /// they check the same path.
    pub fn bind<V: FieldInput, Kd: FieldKind<V>>(
        &mut self,
        path: impl Into<FieldPath>,
        spec: &FieldSpec<V, Kd>,
        value: &mut V,
        render: impl FnOnce(&mut MineralUi<'_, K>, &mut FieldCtx<'_, V, Kd>) -> egui::Response,
    ) -> egui::Response {
        self.pass.field(&path.into(), spec, value, render)
    }

    /// Fields of the nested value of `key`.
    pub fn nested<U, R>(&mut self, key: NestedKey<T, U>, add: impl FnOnce(&mut FormUi<'_, 'p, 's, K, U>) -> R) -> R {
        self.mark(key);
        let prefix = Some(self.path(key));
        add(&mut FormUi {
            pass: &mut *self.pass,
            value: key.get_mut(self.value),
            prefix,
            placed: Vec::new(),
        })
    }

    /// Fields of each item of list `key`, with the item index.
    pub fn list<U>(&mut self, key: ListKey<T, U>, mut add: impl FnMut(&mut FormUi<'_, 'p, 's, K, U>, usize)) {
        self.mark(key);
        let path = self.path(key);
        for (i, item) in key.get_mut(self.value).iter_mut().enumerate() {
            add(
                &mut FormUi {
                    pass: &mut *self.pass,
                    value: item,
                    prefix: Some(path.at(i)),
                    placed: Vec::new(),
                },
                i,
            );
        }
    }

    /// Every declared field not placed yet, in declaration order; nested values and lists show all their fields.
    pub fn rest(&mut self)
    where
        T: FormFields<K>,
    {
        for i in 0..T::FIELDS {
            if !self.placed.contains(&i) {
                T::show_index(self, i);
            }
        }
    }

    /// Lays out the rows `add` places through `placement`, e.g. a kit form grid; fields keep their state.
    pub fn placed<R>(
        &mut self,
        placement: &mut dyn FieldPlacement<K>,
        add: impl FnOnce(&mut FormUi<'_, '_, 's, K, T>) -> R,
    ) -> R {
        let mut pass = Pass {
            ui: &mut *self.pass.ui,
            salt: self.pass.salt,
            state: &mut *self.pass.state,
            report: self.pass.report,
            seen: std::mem::take(&mut self.pass.seen),
            refreshed: std::mem::take(&mut self.pass.refreshed),
            changed: false,
            submit: false,
            placement: Some(placement),
        };
        let mut nested = FormUi {
            pass: &mut pass,
            value: &mut *self.value,
            prefix: self.prefix.clone(),
            placed: std::mem::take(&mut self.placed),
        };
        let r = add(&mut nested);
        self.placed = nested.placed;
        let Pass {
            seen,
            refreshed,
            changed,
            submit,
            ..
        } = pass;
        self.pass.seen = seen;
        self.pass.refreshed = refreshed;
        self.pass.changed |= changed;
        self.pass.submit |= submit;
        r
    }

    /// Non-field `row` laid out by the placement, e.g. a heading or content spanning the form; `add` shows it,
    /// told where a [`FormRow::Field`] label goes.
    pub fn row(
        &mut self,
        row: FormRow<'_>,
        add: impl FnOnce(&mut MineralUi<'_, K>, LabelSlot) -> egui::Response,
    ) -> egui::Response {
        match self.pass.placement.as_deref_mut() {
            Some(p) => p.place(self.pass.ui, row, Box::new(add)),
            None => add(self.pass.ui, LabelSlot::Own),
        }
    }
}
