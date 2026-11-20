use egui_mineral::{
    i18n::I18n,
    kit,
    layout::{Constraints, Measure},
    semantic::{KeyedSalt, SemanticNode},
    widget::{MineralUi, MineralWidget, WidgetFrame, WidgetResponse},
};

use super::dropdown::{Dropdown, Kind};
use super::field::Validation;
use super::listbox::{self, ListEntry, ListOption};
use super::textbox::InputAppearance;
use super::value::{ListState, Value};
use crate::{
    Fluent, Size,
    widget::choice::{self, Choice},
};

/// Fluent `Select`: the field of a native `<select>` bound to the picked option value; the list is the Fluent
/// listbox.
#[derive(Debug)]
#[must_use = "show with ui.mineral(..)"]
pub struct Select<'a>(Dropdown<'a>);

impl<'a> Select<'a> {
    /// `label` is the accessible name; `entries` are options and groups. Agent key `<id_salt>-select`.
    pub fn new(
        id_salt: impl Into<KeyedSalt>,
        label: impl Into<String>,
        entries: impl IntoIterator<Item = impl Into<ListEntry>>,
        value: &'a mut Option<String>,
    ) -> Self {
        Self(Dropdown::with(
            Kind::Select,
            id_salt.into(),
            label.into(),
            listbox::entries(entries),
            Value::One(value),
        ))
    }

    /// Binds the value of the picked option instead of its string: each of `options` comes with its value
    /// (the option's own string value is not read), a pick writes it into `value`, and nothing is selected
    /// while `value` matches none.
    pub fn bound<T: PartialEq + Clone + 'a>(
        id_salt: impl Into<KeyedSalt>,
        label: impl Into<String>,
        options: impl IntoIterator<Item = (T, impl Into<ListOption>)>,
        value: &'a mut T,
    ) -> Self {
        let (values, options): (Vec<T>, Vec<ListOption>) = choice::split(options);
        let entries = options
            .into_iter()
            .enumerate()
            .map(|(i, mut o)| {
                o.value = i.to_string();
                ListEntry::from(o)
            })
            .collect();
        Self(Dropdown::with(
            Kind::Select,
            id_salt.into(),
            label.into(),
            entries,
            Value::Bound(Choice::by_value(value, values)),
        ))
    }

    /// Default [`InputAppearance::Outline`].
    pub fn appearance(mut self, appearance: InputAppearance) -> Self {
        self.0 = self.0.appearance(appearance);
        self
    }

    /// Default [`Size::Medium`].
    pub fn size(mut self, size: Size) -> Self {
        self.0 = self.0.size(size);
        self
    }

    /// Hint shown while nothing is selected.
    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.0 = self.0.placeholder(placeholder);
        self
    }

    /// `contentBefore`: icon leading the text.
    pub fn icon_before(mut self, icon: impl Into<egui_mineral::icon::Icon>) -> Self {
        self.0 = self.0.icon_before(icon);
        self
    }

    /// Validation state, as [`FieldContext::validation`](super::FieldContext::validation) hands it over;
    /// [`Validation::Error`] is `aria-invalid`: red border until focused.
    pub fn validation(mut self, validation: Option<Validation>) -> Self {
        self.0 = self.0.validation(validation);
        self
    }

    /// Greys out the field; the list cannot open.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.0 = self.0.disabled(disabled);
        self
    }

    /// Replaces the `<id_salt>-select` agent key.
    pub fn agent_key(mut self, key: impl Into<egui_mineral::semantic::StableId>) -> Self {
        self.0 = self.0.agent_key(key);
        self
    }
}

impl MineralWidget for Select<'_> {
    type Kit = Fluent;
    type State = ListState;
    type Output = ();

    kit::delegate_widget_meta!(0);

    fn layout(&self, ui: &MineralUi<'_, Fluent>, c: Constraints, st: &ListState) -> Measure {
        self.0.layout(ui, c, st)
    }

    fn update(
        &mut self,
        ui: &mut MineralUi<'_, Fluent>,
        frame: &WidgetFrame<'_>,
        st: &mut ListState,
    ) -> WidgetResponse<()> {
        self.0.update(ui, frame, st)
    }

    fn semantics(&self, i18n: &I18n, r: &WidgetResponse<()>, st: &ListState) -> Option<SemanticNode> {
        self.0.semantics(i18n, r, st)
    }
}
