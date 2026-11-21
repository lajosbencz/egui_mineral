use std::borrow::Cow;

use egui::{Align, Rangef, Rect, Response, Sense, WidgetInfo, pos2, vec2};
use egui_mineral::{
    form::{FieldPlacement, FormRow, FormUi, LabelSlot, PlaceRender},
    layout::{FormRows, StackBelow},
    semantic::{KeySalt, KeyedSalt, StableId},
    widget::{AsResponse, MineralUi},
};

use crate::spec::TypeScale;
use crate::text_inputs::{TextFieldVariant, text_field};
use crate::widget::parts;
use crate::{Material, MaterialExt, MaterialUiExt as _, adaptive};

// No MD3 counterpart
const LABEL_FONT: TypeScale = TypeScale::BodyLarge;

/// Two-column form: labels in a column as wide as the widest label, controls beside them, section headings
/// and full-width rows; below the first [`layout::breakpoint`](egui_mineral::layout::breakpoint) labels stack
/// above their controls. Each control is labelled by its row label in the accessibility tree. Non-MD3.
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
    pub fn show<R>(self, ui: &mut MineralUi<'_, Material>, add: impl FnOnce(&mut FormGridUi<'_, '_>) -> R) -> R {
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

    /// Places the fields `add` shows in `form` as grid rows; [`FormUiExt::section`](crate::text_inputs::FormUiExt::section)
    /// adds headings.
    pub fn show_form<R>(
        self,
        form: &mut FormUi<'_, '_, Material>,
        add: impl FnOnce(&mut FormUi<'_, '_, Material>) -> R,
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
    ui: &'u mut MineralUi<'s, Material>,
    salt: &'u KeyedSalt,
    place: GridPlacement,
}

impl<'s> FormGridUi<'_, 's> {
    /// Content between rows, outside the columns.
    pub fn ui(&mut self) -> &mut MineralUi<'s, Material> {
        self.ui
    }

    /// Title small heading spanning both columns; a [`Role::Heading`](egui_mineral::semantic::Role::Heading) node keyed `<grid salt>-<salt>`.
    pub fn section(&mut self, salt: impl KeySalt, title: &str) -> Response {
        let key = StableId::of(self.salt, salt);
        self.place.lay(self.ui, FormRow::Section, |ui, _| {
            crate::text_inputs::form::heading(ui, key, title)
        })
    }

    /// `label` in the label column, `control` beside it (below when stacked), labelled by it. `control` gets the
    /// label: hide the control's own visible label (`label_visible(false)`) and keep it as its name for agents.
    /// The label centers on a text field's height from the control's top.
    pub fn row<R: AsResponse>(
        &mut self,
        label: &str,
        control: impl FnOnce(&mut MineralUi<'_, Material>, &str) -> R,
    ) -> R {
        let row = FormRow::Field { label, required: false };
        self.place.lay(self.ui, row, |ui, _| control(ui, label))
    }

    /// Row spanning both columns.
    pub fn full<R>(&mut self, add: impl FnOnce(&mut MineralUi<'_, Material>) -> R) -> R {
        self.place.space(self.ui, FormRow::Full);
        add(self.ui)
    }
}

#[derive(Debug)]
struct GridPlacement(FormRows);

impl GridPlacement {
    fn lay<R: AsResponse>(
        &mut self,
        ui: &mut MineralUi<'_, Material>,
        row: FormRow<'_>,
        render: impl FnOnce(&mut MineralUi<'_, Material>, LabelSlot) -> R,
    ) -> R {
        self.space(ui, row);
        match row {
            FormRow::Section => {
                let r = render(ui, LabelSlot::Own);
                ui.add_space(adaptive::grid(1));
                r
            }
            FormRow::Field { label, required } => {
                let label = if required {
                    Cow::Owned(format!("{label}*"))
                } else {
                    Cow::Borrowed(label)
                };
                self.field(ui, &label, render)
            }
            _ => render(ui, LabelSlot::Own),
        }
    }

    fn space(&mut self, ui: &mut MineralUi<'_, Material>, row: FormRow<'_>) {
        let gap = if row == FormRow::Section { 6 } else { 2 };
        ui.add_space(self.0.next(adaptive::grid(gap)));
    }

    fn field<R: AsResponse>(
        &mut self,
        ui: &mut MineralUi<'_, Material>,
        label: &str,
        render: impl FnOnce(&mut MineralUi<'_, Material>, LabelSlot) -> R,
    ) -> R {
        let color = ui.tokens().color.on_surface;
        let natural = parts::type_wrapped(ui.painter(), label, LABEL_FONT, color, f32::INFINITY, 0)
            .size()
            .x;
        self.0.measure(natural);
        if self.0.stacked() {
            let name = parts::type_label(ui, label, LABEL_FONT, color, 0);
            let r = render(ui, LabelSlot::External);
            link(&name, label, &r);
            return r;
        }
        let gap = adaptive::grid(4);
        let total = ui.available_width();
        let label_w = self.0.label_width(total, gap);
        let dir = ui.i18n().direction();
        let band = text_field::field_h(ui.density(), TextFieldVariant::Filled, true);
        let rt = ui.rt();
        let row = ui.allocate_ui_with_layout(vec2(total, 0.0), dir.row().with_cross_align(Align::Min), |ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
            let (slot, _) = ui.allocate_exact_size(vec2(label_w + gap, 0.0), Sense::hover());
            let r = ui
                .allocate_ui_with_layout(vec2(total - label_w - gap, 0.0), dir.column(), |ui| {
                    render(&mut rt.ui(ui), LabelSlot::External)
                })
                .inner;
            (slot, r)
        });
        let (slot, r) = row.inner;
        let g = parts::type_wrapped(ui.painter(), label, LABEL_FONT, color, label_w, 0);
        let height = row.response.rect.height().min(band);
        let top = slot.top() + ((height - g.size().y) / 2.0).max(0.0);
        let column = Rect::from_x_y_ranges(slot.x_range(), top..=top);
        let x = dir.start_x(column, Rangef::new(0.0, g.size().x));
        let rect = Rect::from_min_size(pos2(x, top), g.size());
        ui.painter().galley(rect.min, g, egui::Color32::PLACEHOLDER);
        let name = ui.interact(rect, row.response.id.with("label"), Sense::hover());
        link(&name, label, &r);
        r
    }
}

// The painted label becomes an accessibility label node and names the control.
fn link(name: &Response, label: &str, control: &impl AsResponse) {
    name.widget_info(|| WidgetInfo::labeled(egui::Role::Label, true, label));
    control.as_response().clone().labelled_by(name.id);
}

impl FieldPlacement<Material> for GridPlacement {
    fn place(
        &mut self,
        ui: &mut MineralUi<'_, Material>,
        row: FormRow<'_>,
        render: PlaceRender<'_, Material>,
    ) -> Response {
        self.lay(ui, row, render)
    }
}
