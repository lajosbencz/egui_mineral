use egui::{Pos2, Response, Sense, text::LayoutJob, vec2};
use egui_mineral::{
    layout::{Axis, LabelColumn},
    semantic::{self, KeySalt, KeyedSalt, Role, SemanticNode, StableId},
    widget::MineralUi,
};

use crate::inputs::{Field, FieldContext, FieldResponse};
use crate::spec::{ColorToken::ColorNeutralForeground1, tokens::geometry as g};
use crate::typography::TypeRamp;
use crate::{Fluent, FluentExt};

// Share of the width the label column may take; longer labels wrap.
const MAX_LABEL_SHARE: f32 = 0.5;

/// Two-column form: horizontal [`Field`]s whose label column is as wide as the widest label, section
/// headings and full-width rows. Not in Fluent (docs/divergences.md).
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
    pub fn show<R>(self, ui: &mut MineralUi<'_, Fluent>, add: impl FnOnce(&mut FormGridUi<'_, '_>) -> R) -> R {
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
    ui: &'u mut MineralUi<'s, Fluent>,
    salt: &'u KeyedSalt,
    column: LabelColumn,
    rows: usize,
}

impl<'s> FormGridUi<'_, 's> {
    /// Content between rows, outside the columns.
    pub fn ui(&mut self) -> &mut MineralUi<'s, Fluent> {
        self.ui
    }

    fn gap(&mut self, before: f32) {
        if self.rows > 0 {
            self.ui.add_space(before);
        }
        self.rows += 1;
    }

    /// `subtitle2` heading spanning both columns; a [`Role::Heading`] node keyed `<grid salt>-<salt>`.
    pub fn section(&mut self, salt: impl KeySalt, title: &str) -> Response {
        self.gap(g::SPACING_VERTICAL_XXL);
        let ui = &mut *self.ui;
        let color = ui.tokens().color(ColorNeutralForeground1);
        let mut job = LayoutJob::single_section(title.to_owned(), TypeRamp::Subtitle2.format(ui.ctx(), color));
        job.wrap.max_width = ui.available_width();
        let key = StableId::of(self.salt, salt);
        let galley = ui.painter().layout_job(job);
        let width = ui.available_width();
        let (rect, r) = ui.allocate_exact_size(vec2(width, galley.size().y), Sense::hover());
        let x = ui
            .i18n()
            .direction()
            .start_x(rect, egui::Rangef::new(0.0, galley.size().x));
        ui.painter().galley(Pos2::new(x, rect.top()), galley, color);
        semantic::emit(ui.ctx(), Some(r.id), r.rect, || {
            SemanticNode::new(key, Role::Heading, title)
        });
        r
    }

    /// `field` laid out horizontally in the grid's label column.
    pub fn field<R>(
        &mut self,
        field: Field,
        add: impl FnOnce(&mut MineralUi<'_, Fluent>, FieldContext) -> R,
    ) -> FieldResponse<R> {
        self.gap(g::SPACING_VERTICAL_M);
        self.column.measure(field.label_natural_width(self.ui));
        let max = self.ui.available_width() * MAX_LABEL_SHARE;
        let share = Field::label_share(self.ui, self.column.width()).min(max);
        field.axis(Axis::Horizontal).label_width(share).show(self.ui, add)
    }

    /// Row spanning both columns.
    pub fn full<R>(&mut self, add: impl FnOnce(&mut MineralUi<'_, Fluent>) -> R) -> R {
        self.gap(g::SPACING_VERTICAL_M);
        add(self.ui)
    }
}
