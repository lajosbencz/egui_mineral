use std::{fmt, sync::Arc};

use serde::ser::{Serialize, SerializeMap, Serializer};

use super::{
    Blank, FieldCtx, FieldInput, FieldKind, FieldPath, FieldSpec, FormFields, FormModel, FormUi, Report, Required,
    ValidationError, Validator, kind,
};
use crate::{date::Date, i18n::LocaleFormat, select::Selection, widget::MineralUi};

type Check<V> = fn(&V, &dyn LocaleFormat) -> Result<(), ValidationError>;

/// Field of kind `Kd` over value `V` described at runtime: the static form's [`FieldSpec`], the value and its rules.
/// Built by the per-kind constructors ([`Self::text`], [`Self::switch`], ...).
#[must_use]
pub struct DynCell<V: FieldInput, Kd: FieldKind<V>> {
    spec: FieldSpec<V, Kd>,
    value: V,
    required: Option<Check<V>>,
    checks: Vec<Arc<dyn Validator<V>>>,
}

impl<V: FieldInput + Clone, Kd: FieldKind<V>> Clone for DynCell<V, Kd>
where
    Kd::Options: Clone,
{
    fn clone(&self) -> Self {
        Self {
            spec: self.spec.clone(),
            value: self.value.clone(),
            required: self.required,
            checks: self.checks.clone(),
        }
    }
}

impl<V: FieldInput + fmt::Debug, Kd: FieldKind<V>> fmt::Debug for DynCell<V, Kd> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DynCell")
            .field("value", &self.value)
            .finish_non_exhaustive()
    }
}

impl<V: FieldInput, Kd: FieldKind<V>> DynCell<V, Kd> {
    fn of(spec: FieldSpec<V, Kd>, value: V) -> Self {
        Self {
            spec,
            value,
            required: None,
            checks: Vec::new(),
        }
    }

    /// Starting value instead of the kind's empty one.
    pub fn initial(mut self, value: V) -> Self {
        self.value = value;
        self
    }

    /// Rule run in order on a non-blank value; the first failure wins.
    pub fn check(mut self, validator: impl Validator<V> + 'static) -> Self {
        self.checks.push(Arc::new(validator));
        self
    }

    /// Presentation of the field.
    pub fn spec(&self) -> &FieldSpec<V, Kd> {
        &self.spec
    }

    /// Current value.
    pub fn value(&self) -> &V {
        &self.value
    }

    /// Value to edit from code; such edits are not user edits.
    pub fn value_mut(&mut self) -> &mut V {
        &mut self.value
    }

    fn validate(&self, path: FieldPath, report: &mut Report, locale: &dyn LocaleFormat) {
        report.check(&path);
        if self.required.is_some() {
            report.require(&path);
        }
        let failed = if self.value.is_blank() {
            self.required.and_then(|r| r(&self.value, locale).err())
        } else {
            self.checks.iter().find_map(|c| c.validate(&self.value, locale).err())
        };
        if let Some(e) = failed {
            report.fail(path, e);
        }
    }
}

impl<V: Blank, Kd: FieldKind<V>> DynCell<V, Kd> {
    /// Blank fails with [`Required`] and the label is marked.
    pub fn required(mut self) -> Self {
        self.required = Some(|v, locale| Required.validate(v, locale));
        self
    }
}

impl DynCell<f64, kind::Number> {
    /// Number input; starts at the range minimum, else 0.
    pub fn number(spec: FieldSpec<f64, kind::Number>) -> Self {
        let start = spec.options().range.map_or(0.0, |r| r.min());
        Self::of(spec, start)
    }
}

impl DynCell<f64, kind::Slider> {
    /// Slider; starts at the range minimum, else 0.
    pub fn slider(spec: FieldSpec<f64, kind::Slider>) -> Self {
        let start = spec.options().range.map_or(0.0, |r| r.min());
        Self::of(spec, start)
    }
}

macro_rules! dyn_inputs {
    ($($variant:ident($v:ty, $kd:ident) $(= $ctor:ident: $doc:literal)?),* $(,)?) => {
        /// Field of a [`DynForm`] with its kind's value type, one variant per core [`kind`] marker.
        #[derive(Clone, Debug)]
        #[non_exhaustive]
        pub enum DynInput {
            $(#[doc = concat!("[`kind::", stringify!($kd), "`].")] $variant(DynCell<$v, kind::$kd>),)*
        }

        $(
            impl From<DynCell<$v, kind::$kd>> for DynInput {
                fn from(cell: DynCell<$v, kind::$kd>) -> Self {
                    Self::$variant(cell)
                }
            }

            $(
                impl DynCell<$v, kind::$kd> {
                    #[doc = $doc]
                    pub fn $ctor(spec: FieldSpec<$v, kind::$kd>) -> Self {
                        Self::of(spec, <$v>::default())
                    }
                }
            )?
        )*

        // `$c` is the cell of whichever variant `$input` holds.
        macro_rules! on_cell {
            ($input:expr, $c:ident => $body:expr) => {
                match $input {
                    $(DynInput::$variant($c) => $body,)*
                }
            };
        }
    };
}

dyn_inputs!(
    Text(String, Text) = text: "Text input; starts empty.",
    Check(bool, Check) = checkbox: "Checkbox; starts unchecked.",
    Switch(bool, Switch) = switch: "On/off switch; starts off.",
    Number(f64, Number),
    Slider(f64, Slider),
    Date(Option<Date>, Date) = date: "Optional date; starts unset.",
    Pick(Option<String>, Pick) = pick: "Pick one choice key; starts with none.",
    MultiPick(Selection<String>, MultiPick) = multi_pick: "Pick any choice keys; starts with none.",
);

/// Field of a [`DynForm`]: its key and typed input.
#[derive(Clone, Debug)]
pub struct DynField {
    key: String,
    input: DynInput,
}

impl DynField {
    /// `key` names the field: its [`FieldPath::named`] path, salt and agent key part.
    pub fn new(key: impl Into<String>, input: impl Into<DynInput>) -> Self {
        Self {
            key: key.into(),
            input: input.into(),
        }
    }

    /// Key of the field.
    pub fn key(&self) -> &str {
        &self.key
    }

    /// Typed input; match it to read the value.
    pub fn input(&self) -> &DynInput {
        &self.input
    }

    /// Typed input, for code edits of the value.
    pub fn input_mut(&mut self) -> &mut DynInput {
        &mut self.input
    }
}

/// Form value whose fields arrive as data at runtime, e.g. a plugin's or a server's form. Implements [`FormModel`]
/// and [`FormFields`], so a [`Form`](struct@super::Form) drives it with the same [`FormState`](super::FormState),
/// validators and stylekit [`FieldWidget`](super::FieldWidget)s as a `#[derive(Form)]` type; field `key` is at
/// [`FieldPath::named`]`(key)`. Cross-field rules go in [`Form::rule`](super::Form::rule) through
/// [`Validation::error_named`](super::Validation::error_named). Serializes as a map of key to value, a multi-pick as
/// its picked keys.
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

    /// Field `key`.
    pub fn field(&self, key: &str) -> Option<&DynField> {
        self.fields.iter().find(|f| f.key == key)
    }

    /// Field `key`, for code edits of its value.
    pub fn field_mut(&mut self, key: &str) -> Option<&mut DynField> {
        self.fields.iter_mut().find(|f| f.key == key)
    }

    /// Fields of kinds stylekit `K` does not render ([`DynWidgets::WIDGETS`]), in display order; check a form with it
    /// when it arrives, since such a field is not shown.
    pub fn unsupported<K: DynWidgets>(&self) -> impl Iterator<Item = &DynField> {
        self.fields.iter().filter(|f| !rendered_by::<K>(&f.input))
    }
}

impl Serialize for DynForm {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(Some(self.fields.len()))?;
        for f in &self.fields {
            match &f.input {
                // The picked keys only, not the selection's anchor and mode.
                DynInput::MultiPick(c) => {
                    let picked: Vec<&String> = c.value.selected().collect();
                    map.serialize_entry(&f.key, &picked)?;
                }
                input => on_cell!(input, c => map.serialize_entry(&f.key, &c.value)?),
            }
        }
        map.end()
    }
}

impl FormModel for DynForm {
    fn validate(&self, v: &mut super::Validation<'_, Self>) {
        for f in &self.fields {
            let (path, report, locale) = v.named(&f.key);
            on_cell!(&f.input, c => c.validate(path, report, locale));
        }
    }
}

/// Widget fn of a kind a stylekit renders: its [`FieldWidget::show`](super::FieldWidget::show).
pub type DynRender<K, V, Kd> = fn(&mut MineralUi<'_, K>, &mut FieldCtx<'_, V, Kd>) -> egui::Response;

/// Widget fn per [`DynInput`] kind of a stylekit; `None` for a kind it does not render. Kits fill the kinds they
/// have over [`Self::NONE`]: `text: Some(FieldWidget::show), ..DynTable::NONE`.
#[derive(Debug)]
pub struct DynTable<K> {
    /// [`DynInput::Text`].
    pub text: Option<DynRender<K, String, kind::Text>>,
    /// [`DynInput::Check`].
    pub check: Option<DynRender<K, bool, kind::Check>>,
    /// [`DynInput::Switch`].
    pub switch: Option<DynRender<K, bool, kind::Switch>>,
    /// [`DynInput::Number`].
    pub number: Option<DynRender<K, f64, kind::Number>>,
    /// [`DynInput::Slider`].
    pub slider: Option<DynRender<K, f64, kind::Slider>>,
    /// [`DynInput::Date`].
    pub date: Option<DynRender<K, Option<Date>, kind::Date>>,
    /// [`DynInput::Pick`].
    pub pick: Option<DynRender<K, Option<String>, kind::Pick>>,
    /// [`DynInput::MultiPick`].
    pub multi_pick: Option<DynRender<K, Selection<String>, kind::MultiPick>>,
}

impl<K> DynTable<K> {
    /// No kind rendered.
    pub const NONE: Self = Self {
        text: None,
        check: None,
        switch: None,
        number: None,
        slider: None,
        date: None,
        pick: None,
        multi_pick: None,
    };
}

/// Stylekit that renders [`DynForm`] fields: the kinds in [`Self::WIDGETS`].
pub trait DynWidgets: Sized {
    /// Widget fn per kind; the kit's [`FieldWidget`](super::FieldWidget)s.
    const WIDGETS: DynTable<Self>;
}

fn rendered_by<K: DynWidgets>(input: &DynInput) -> bool {
    let t = &K::WIDGETS;
    match input {
        DynInput::Text(_) => t.text.is_some(),
        DynInput::Check(_) => t.check.is_some(),
        DynInput::Switch(_) => t.switch.is_some(),
        DynInput::Number(_) => t.number.is_some(),
        DynInput::Slider(_) => t.slider.is_some(),
        DynInput::Date(_) => t.date.is_some(),
        DynInput::Pick(_) => t.pick.is_some(),
        DynInput::MultiPick(_) => t.multi_pick.is_some(),
    }
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
    /// Field `key` in the kit widget of its kind; `None` for an unknown key or a kind the kit does not render.
    pub fn field(&mut self, key: &str) -> Option<egui::Response> {
        let index = self.value.fields.iter().position(|f| f.key == key)?;
        self.show_dyn(index)
    }

    fn show_dyn(&mut self, index: usize) -> Option<egui::Response> {
        if index < self.value.fields.len() {
            self.mark_index(index);
        }
        let DynField { key, input } = self.value.fields.get_mut(index)?;
        let path = FieldPath::under(self.prefix.as_ref(), key);
        let pass = &mut *self.pass;
        let t = &K::WIDGETS;
        let shown = match input {
            DynInput::Text(c) => t.text.map(|r| pass.field(&path, &c.spec, &mut c.value, r)),
            DynInput::Check(c) => t.check.map(|r| pass.field(&path, &c.spec, &mut c.value, r)),
            DynInput::Switch(c) => t.switch.map(|r| pass.field(&path, &c.spec, &mut c.value, r)),
            DynInput::Number(c) => t.number.map(|r| pass.field(&path, &c.spec, &mut c.value, r)),
            DynInput::Slider(c) => t.slider.map(|r| pass.field(&path, &c.spec, &mut c.value, r)),
            DynInput::Date(c) => t.date.map(|r| pass.field(&path, &c.spec, &mut c.value, r)),
            DynInput::Pick(c) => t.pick.map(|r| pass.field(&path, &c.spec, &mut c.value, r)),
            DynInput::MultiPick(c) => t.multi_pick.map(|r| pass.field(&path, &c.spec, &mut c.value, r)),
        };
        if shown.is_none() {
            crate::kit::sanitize::report(format_args!(
                "dynamic form field {path:?} is of a kind the kit does not render; check DynForm::unsupported"
            ));
        }
        shown
    }
}
