use egui::Response;
use egui_mineral::{
    form::{FieldPlacement, FormRow, FormUi, LabelSlot, PlaceRender},
    layout::{FormRows, StackBelow},
    semantic::{KeySalt, KeyedSalt, StableId},
    widget::MineralUi,
};

use crate::Fluent;
use crate::inputs::{Field, FieldContext, FieldResponse};
use crate::spec::tokens::geometry as g;

/// Two-column form: horizontal [`Field`]s whose label column is as wide as the widest label, section
/// headings and full-width rows; below the first [`layout::breakpoint`](egui_mineral::layout::breakpoint)
/// fields turn vertical. Not in Fluent (docs/divergences.md).
#[derive(Clone, Debug)]
#[must_use = "show with FormGrid::show or FormGrid::show_form"]
pub struct FormGrid {
    salt: KeyedSalt,
    stack: StackBelow,
}

impl FormGrid {
    /// Section heading node keys `<id_salt>-<section salt>`.
    pub fn new(id_salt: impl Into<KeyedSalt>) -> Self {
        Self {
            salt: id_salt.into(),
            stack: StackBelow::default(),
        }
    }

    /// Width below which labels stack above their controls; default [`StackBelow::FirstBreakpoint`].
    pub fn stack_below(mut self, stack: StackBelow) -> Self {
        self.stack = stack;
        self
    }

    /// Lays out the rows `add` makes, top to bottom.
    pub fn show<R>(self, ui: &mut MineralUi<'_, Fluent>, add: impl FnOnce(&mut FormGridUi<'_, '_>) -> R) -> R {
        let rt = ui.rt();
        let id = ui.make_persistent_id(self.salt.id_salt());
        let rows = FormRows::begin(rt.core(), rt.ctx(), id, ui.available_width(), self.stack);
        let (r, rows) = ui
            .push_id(self.salt.id_salt(), |ui| {
                let mut grid = FormGridUi {
                    ui: &mut rt.ui(ui),
                    salt: &self.salt,
                    place: GridPlacement(rows),
                };
                let r = add(&mut grid);
                (r, grid.place.0)
            })
            .inner;
        rows.end(rt.ctx());
        r
    }

    /// Places the fields `add` shows in `form` as grid rows; [`FormUiExt::section`](crate::inputs::FormUiExt::section)
    /// adds headings.
    pub fn show_form<'s, T, R>(
        self,
        form: &mut FormUi<'_, '_, 's, Fluent, T>,
        add: impl FnOnce(&mut FormUi<'_, '_, 's, Fluent, T>) -> R,
    ) -> R {
        let ui = form.ui();
        let rt = ui.rt();
        let id = ui.make_persistent_id(self.salt.id_salt());
        let mut place = GridPlacement(FormRows::begin(
            rt.core(),
            rt.ctx(),
            id,
            ui.available_width(),
            self.stack,
        ));
        let r = form.placed(&mut place, add);
        place.0.end(rt.ctx());
        r
    }
}

/// Rows of a [`FormGrid`].
#[derive(Debug)]
pub struct FormGridUi<'u, 's> {
    ui: &'u mut MineralUi<'s, Fluent>,
    salt: &'u KeyedSalt,
    place: GridPlacement,
}

impl<'s> FormGridUi<'_, 's> {
    /// Content between rows, outside the columns.
    pub fn ui(&mut self) -> &mut MineralUi<'s, Fluent> {
        self.ui
    }

    /// `subtitle2` heading spanning both columns; a [`Role::Heading`](egui_mineral::semantic::Role::Heading) node
    /// keyed `<grid salt>-<salt>`.
    pub fn section(&mut self, salt: impl KeySalt, title: &str) -> Response {
        let key = StableId::of(self.salt, salt);
        self.place.lay(self.ui, FormRow::Section, |ui, _| {
            crate::inputs::form::heading(ui, key, title)
        })
    }

    /// `field` laid out horizontally in the grid's label column, vertically when stacked.
    pub fn field<R>(
        &mut self,
        field: Field,
        add: impl FnOnce(&mut MineralUi<'_, Fluent>, FieldContext) -> R,
    ) -> FieldResponse<R> {
        let (label, required) = field.row_label();
        let row = FormRow::Field {
            label: &label,
            required,
        };
        self.place
            .lay(self.ui, row, |ui, slot| field.placed(slot).show(ui, add))
    }

    /// Row spanning both columns.
    pub fn full<R>(&mut self, add: impl FnOnce(&mut MineralUi<'_, Fluent>) -> R) -> R {
        self.place.lay(self.ui, FormRow::Full, |ui, _| add(ui))
    }
}

#[derive(Debug)]
struct GridPlacement(FormRows);

impl GridPlacement {
    fn lay<R>(
        &mut self,
        ui: &mut MineralUi<'_, Fluent>,
        row: FormRow<'_>,
        render: impl FnOnce(&mut MineralUi<'_, Fluent>, LabelSlot) -> R,
    ) -> R {
        let gap = if row == FormRow::Section {
            g::SPACING_VERTICAL_XXL
        } else {
            g::SPACING_VERTICAL_M
        };
        ui.add_space(self.0.next(gap));
        let FormRow::Field { label, required } = row else {
            return render(ui, LabelSlot::Own);
        };
        self.0.measure(Field::natural_label_width(ui, label, required));
        if self.0.stacked() {
            return render(ui, LabelSlot::Own);
        }
        let gap = Field::label_gap(ui);
        let width = self.0.label_width(ui.available_width(), gap) + gap;
        render(ui, LabelSlot::Column(width))
    }
}

impl FieldPlacement<Fluent> for GridPlacement {
    fn place(&mut self, ui: &mut MineralUi<'_, Fluent>, row: FormRow<'_>, render: PlaceRender<'_, Fluent>) -> Response {
        self.lay(ui, row, render)
    }
}
