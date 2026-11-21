use crate::widget::MineralUi;

/// Where a field shows its label, as its [`FieldPlacement`] decides.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[non_exhaustive]
pub enum LabelSlot {
    /// Field shows its own label, as outside any placement.
    #[default]
    Own,
    /// Beside the control in a label column this wide, label gap included; kits without a horizontal field
    /// layout show their own label.
    Column(f32),
    /// Shown by the placement: the field hides its visible label and keeps it as its accessible name.
    External,
}

/// What a row of a [`FieldPlacement`] holds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum FormRow<'a> {
    /// Field or control named by `label`; `required` adds the kit's required mark where the placement draws it.
    Field {
        /// Visible label.
        label: &'a str,
        /// Required field.
        required: bool,
    },
    /// Heading of the rows that follow.
    Section,
    /// Content spanning the form.
    Full,
}

/// Row renderer handed to [`FieldPlacement::place`].
pub type PlaceRender<'r, K> = Box<dyn FnOnce(&mut MineralUi<'_, K>, LabelSlot) -> egui::Response + 'r>;

/// Lays out the rows of a [`FormUi`](super::FormUi), e.g. a kit's two-column form grid; installed with
/// [`FormUi::placed`](super::FormUi::placed).
pub trait FieldPlacement<K> {
    /// Places one `row` in `ui`: `render` shows it in the ui the placement gives it, told where its label goes.
    fn place(&mut self, ui: &mut MineralUi<'_, K>, row: FormRow<'_>, render: PlaceRender<'_, K>) -> egui::Response;
}
