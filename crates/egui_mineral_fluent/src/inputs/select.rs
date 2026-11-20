use egui_mineral::{
    i18n::I18n,
    kit,
    layout::{Constraints, Measure},
    semantic::{KeyedSalt, SemanticNode},
    widget::{MineralUi, MineralWidget, WidgetFrame, WidgetResponse},
};

use super::dropdown::{Dropdown, Kind};
use super::field::Validation;
use super::listbox::{ListEntry, ListOption, OptionGroup};
use super::textbox::InputAppearance;
use super::value::ListState;
use crate::{Fluent, Size};
use egui_mineral::kit::choice::Choice;

/// Fluent `Select`: the field of a native `<select>` bound to the picked option value; the list is the Fluent
/// listbox.
#[derive(Debug)]
#[must_use = "show with ui.mineral(..)"]
pub struct Select<'a, K>(Dropdown<'a, K>);

impl<'a, K: PartialEq + Clone> Select<'a, K> {
    /// Shows the option whose value equals `selected`; `label` is the accessible name. Agent key
    /// `<id_salt>-select`.
    pub fn new(id_salt: impl Into<KeyedSalt>, label: impl Into<String>, selected: &'a mut K) -> Self {
        Self(Dropdown::with(
            Kind::Select,
            id_salt.into(),
            label.into(),
            Choice::new(selected),
        ))
    }

    /// [`Self::new`] that shows no option while `selected` is `None`.
    pub fn optional(id_salt: impl Into<KeyedSalt>, label: impl Into<String>, selected: &'a mut Option<K>) -> Self {
        Self(Dropdown::with(
            Kind::Select,
            id_salt.into(),
            label.into(),
            Choice::optional(selected),
        ))
    }

    /// Appends one option carrying `value`.
    pub fn item(self, value: K, option: impl Into<ListOption>) -> Self {
        Self(self.0.item(value, option))
    }

    /// Appends `(value, option)` pairs and [`OptionGroup`]s.
    pub fn items(self, entries: impl IntoIterator<Item = impl Into<ListEntry<K>>>) -> Self {
        Self(self.0.items(entries))
    }

    /// Appends a group of options.
    pub fn group(self, group: OptionGroup<K>) -> Self {
        Self(self.0.group(group))
    }
}

impl<K: PartialEq + Clone> Select<'_, K> {
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

impl<K: PartialEq + Clone> MineralWidget for Select<'_, K> {
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
