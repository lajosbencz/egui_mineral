use egui::{Context, Id};

use super::LabelColumn;
use crate::runtime::Core;

// Share of the row the label column and its gap may take; longer labels wrap.
const MAX_LABEL_SHARE: f32 = 0.5;

/// Width below which [`FormRows`] stacks labels above their controls.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[non_exhaustive]
pub enum StackBelow {
    /// First layout [`breakpoint`](super::breakpoint).
    #[default]
    FirstBreakpoint,
    /// Points.
    Width(f32),
    /// Always two columns.
    Never,
}

/// Row layout of a two-column form: the shared [`LabelColumn`], the gaps between rows, and labels stacked above
/// their controls when narrow ([`StackBelow`]).
#[derive(Clone, Copy, Debug)]
pub struct FormRows {
    column: LabelColumn,
    rows: usize,
    stacked: bool,
}

impl FormRows {
    /// Rows under `id` across `width`, stacked below `stack`.
    pub fn begin(core: &Core, ctx: &Context, id: Id, width: f32, stack: StackBelow) -> Self {
        let stacked = match stack {
            StackBelow::FirstBreakpoint => super::breakpoint(core, width) == 0,
            StackBelow::Width(w) => width < w,
            StackBelow::Never => false,
        };
        Self {
            column: LabelColumn::begin(ctx, id.with("labels")),
            rows: 0,
            stacked,
        }
    }

    /// Labels above their controls.
    pub fn stacked(&self) -> bool {
        self.stacked
    }

    /// Space before the next row: `gap`, 0 before the first.
    pub fn next(&mut self, gap: f32) -> f32 {
        self.rows += 1;
        if self.rows > 1 { gap } else { 0.0 }
    }

    /// Counts a label of natural (unwrapped) width `width` into the column ([`LabelColumn::measure`]).
    pub fn measure(&mut self, width: f32) {
        self.column.measure(width);
    }

    /// Label column width in a row `total` wide, `gap` before the control excluded: the widest label of the
    /// previous pass, capped so column and gap take half the row.
    pub fn label_width(&self, total: f32, gap: f32) -> f32 {
        self.column.width().min(total * MAX_LABEL_SHARE - gap).max(0.0)
    }

    /// Stores the column ([`LabelColumn::end`]).
    pub fn end(self, ctx: &Context) {
        self.column.end(ctx);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn label_column_caps_at_half_the_row_and_rows_gap_after_the_first() {
        let ctx = crate::test_support::ctx();
        let rt = crate::test_support::runtime(&ctx);
        let id = Id::unique("rows");
        let mut rows = FormRows::begin(rt.core(), &ctx, id, 2000.0, StackBelow::default());
        assert!(!rows.stacked());
        assert_eq!((rows.next(8.0), rows.next(8.0)), (0.0, 8.0));
        rows.measure(300.0);
        rows.end(&ctx);
        let rows = FormRows::begin(rt.core(), &ctx, id, 2000.0, StackBelow::default());
        assert_eq!(rows.label_width(1000.0, 16.0), 300.0);
        assert_eq!(rows.label_width(400.0, 16.0), 184.0);
        assert!(FormRows::begin(rt.core(), &ctx, id, 100.0, StackBelow::default()).stacked());
        assert!(!FormRows::begin(rt.core(), &ctx, id, 100.0, StackBelow::Never).stacked());
        assert!(FormRows::begin(rt.core(), &ctx, id, 900.0, StackBelow::Width(901.0)).stacked());
    }
}
