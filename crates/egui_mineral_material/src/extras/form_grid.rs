use egui::{Align, Rangef, Rect, Response, Sense, pos2, vec2};
use egui_mineral::{
    layout::LabelColumn,
    semantic::{self, KeySalt, KeyedSalt, Role, SemanticNode, StableId},
    widget::MineralUi,
};

use crate::spec::TypeScale;
use crate::text_inputs::{TextFieldVariant, text_field};
use crate::widget::parts;
use crate::{Material, MaterialExt, MaterialUiExt as _, adaptive};

// No MD3 counterpart
const LABEL_FONT: TypeScale = TypeScale::BodyLarge;
const SECTION_FONT: TypeScale = TypeScale::TitleSmall;
// Share of the width the label column may take; longer labels wrap.
const MAX_LABEL_SHARE: f32 = 0.5;

/// Two-column form: labels in a column as wide as the widest label, controls beside them, section headings
/// and full-width rows; non-MD3.
#[derive(Clone, Debug)]
#[must_use = "show with FormGrid::show"]
pub struct FormGrid {
    salt: KeyedSalt,
}

impl FormGrid {
    /// Section heading node keys `<id_salt>-<section salt>`.
    pub fn new(id_salt: impl Into<KeyedSalt>) -> Self {
        Self { salt: id_salt.into() }
    }

    /// Lays out the rows `add` makes, top to bottom.
    pub fn show<R>(self, ui: &mut MineralUi<'_, Material>, add: impl FnOnce(&mut FormGridUi<'_, '_>) -> R) -> R {
        let rt = ui.rt();
        let id = ui.make_persistent_id(self.salt.id_salt());
        let column = LabelColumn::begin(rt.ctx(), id.with("labels"));
        let (r, column) = ui
            .push_id(self.salt.id_salt(), |ui| {
                let mut grid = FormGridUi {
                    ui: &mut rt.ui(ui),
                    salt: &self.salt,
                    column,
                    rows: 0,
                };
                let r = add(&mut grid);
                (r, grid.column)
            })
            .inner;
        column.end(rt.ctx());
        r
    }
}

/// Rows of a [`FormGrid`].
#[derive(Debug)]
pub struct FormGridUi<'u, 's> {
    ui: &'u mut MineralUi<'s, Material>,
    salt: &'u KeyedSalt,
    column: LabelColumn,
    rows: usize,
}

impl<'s> FormGridUi<'_, 's> {
    /// Content between rows, outside the columns.
    pub fn ui(&mut self) -> &mut MineralUi<'s, Material> {
        self.ui
    }

    fn gap(&mut self, before: f32) {
        if self.rows > 0 {
            self.ui.add_space(before);
        }
        self.rows += 1;
    }

    /// Title small heading spanning both columns; a [`Role::Heading`] node keyed `<grid salt>-<salt>`.
    pub fn section(&mut self, salt: impl KeySalt, title: &str) -> Response {
        self.gap(adaptive::grid(6));
        let ui = &mut *self.ui;
        let key = StableId::of(self.salt, salt);
        let color = ui.tokens().color.on_surface_variant;
        let g = parts::type_wrapped(ui.painter(), title, SECTION_FONT, color, ui.available_width(), 0);
        let width = ui.available_width();
        let (rect, r) = ui.allocate_exact_size(vec2(width, g.size().y), Sense::hover());
        let x = ui.i18n().direction().start_x(rect, Rangef::new(0.0, g.size().x));
        ui.painter().galley(pos2(x, rect.top()), g, egui::Color32::PLACEHOLDER);
        semantic::emit(ui.ctx(), Some(r.id), r.rect, || {
            SemanticNode::new(key, Role::Heading, title)
        });
        ui.add_space(adaptive::grid(1));
        r
    }

    /// `label` in the label column, `control` beside it. `control` gets the label: pass it as the control's
    /// accessible name and hide the control's own visible label (`label_visible(false)`). The label centers on
    /// a text field's height from the control's top.
    pub fn row<R>(&mut self, label: &str, control: impl FnOnce(&mut MineralUi<'_, Material>, &str) -> R) -> R {
        self.gap(adaptive::grid(2));
        let ui = &mut *self.ui;
        let color = ui.tokens().color.on_surface;
        let natural = parts::type_wrapped(ui.painter(), label, LABEL_FONT, color, f32::INFINITY, 0)
            .size()
            .x;
        self.column.measure(natural);
        let gap = adaptive::grid(4);
        let total = ui.available_width();
        let label_w = self.column.width().min(total * MAX_LABEL_SHARE - gap).max(0.0);
        let dir = ui.i18n().direction();
        let band = text_field::field_h(ui.density(), TextFieldVariant::Filled, true);
        let rt = ui.rt();
        let row = ui.allocate_ui_with_layout(vec2(total, 0.0), dir.row().with_cross_align(Align::Min), |ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
            let (slot, _) = ui.allocate_exact_size(vec2(label_w + gap, 0.0), Sense::hover());
            let r = ui
                .allocate_ui_with_layout(vec2(total - label_w - gap, 0.0), dir.column(), |ui| {
                    control(&mut rt.ui(ui), label)
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
        ui.painter().galley(pos2(x, top), g, egui::Color32::PLACEHOLDER);
        r
    }

    /// Row spanning both columns.
    pub fn full<R>(&mut self, add: impl FnOnce(&mut MineralUi<'_, Material>) -> R) -> R {
        self.gap(adaptive::grid(2));
        add(self.ui)
    }
}
