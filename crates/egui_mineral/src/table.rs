// docs/rules.md rule 10: indices here outlive the frame.
#![cfg_attr(not(test), deny(clippy::indexing_slicing))]
#![cfg_attr(not(test), warn(clippy::arithmetic_side_effects))]

use crate::focus::Cell;

/// Column sort direction.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum SortOrder {
    /// Smallest first.
    Ascending,
    /// Largest first.
    Descending,
}

/// Sorted column and its direction.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct SortColumn {
    /// Column index.
    pub index: usize,
    /// Direction.
    pub order: SortOrder,
}

/// Active sort column and direction.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SortState {
    /// `None` keeps data order.
    pub column: Option<SortColumn>,
}

impl SortState {
    /// Header activation: none -> ascending -> descending -> none; another column starts ascending.
    pub fn cycle(&mut self, column: usize) {
        self.column = match self.column {
            Some(SortColumn {
                index,
                order: SortOrder::Ascending,
            }) if index == column => Some(SortColumn {
                index,
                order: SortOrder::Descending,
            }),
            Some(SortColumn {
                index,
                order: SortOrder::Descending,
            }) if index == column => None,
            _ => Some(SortColumn {
                index: column,
                order: SortOrder::Ascending,
            }),
        };
    }

    /// `Some` only for the sorted column.
    pub fn order(&self, column: usize) -> Option<SortOrder> {
        self.column.filter(|c| c.index == column).map(|c| c.order)
    }
}

/// Data indices `0..len` in display order; `key` is the sort key of a data cell. Stable.
pub fn sort_order<K: Ord>(len: usize, sort: SortState, mut key: impl FnMut(Cell) -> K) -> Vec<usize> {
    let mut order: Vec<usize> = (0..len).collect();
    match sort.column {
        Some(SortColumn {
            index: column,
            order: SortOrder::Ascending,
        }) => order.sort_by_cached_key(|&row| key(Cell { row, column })),
        Some(SortColumn {
            index: column,
            order: SortOrder::Descending,
        }) => order.sort_by_cached_key(|&row| std::cmp::Reverse(key(Cell { row, column }))),
        None => {}
    }
    order
}

/// Initial width and minimum of one column.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ColumnSize {
    /// Starting width in points.
    pub width: f32,
    /// Smallest width in points.
    pub min: f32,
}

/// Resizable column widths with minimums.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ColumnWidths {
    widths: Vec<f32>,
    min: Vec<f32>,
}

impl ColumnWidths {
    /// One [`ColumnSize`] per column. A negative or non-finite `min` counts as 0, a `width`
    /// below `min` as `min`.
    pub fn new(columns: impl IntoIterator<Item = ColumnSize>) -> Self {
        let (widths, min): (Vec<f32>, Vec<f32>) = columns
            .into_iter()
            .map(|ColumnSize { width, min }| {
                let m = crate::kit::sanitize::at_least(min, 0.0.., "ColumnWidths min");
                (crate::kit::sanitize::at_least(width, m.., "ColumnWidths width"), m)
            })
            .unzip();
        Self { widths, min }
    }

    /// Column count.
    pub fn len(&self) -> usize {
        self.widths.len()
    }

    /// `true` without columns.
    pub fn is_empty(&self) -> bool {
        self.widths.is_empty()
    }

    /// Points; 0 for a column out of range.
    #[track_caller]
    pub fn width(&self, column: usize) -> f32 {
        Self::at(&self.widths, column)
    }

    /// Points; 0 for a column out of range.
    #[track_caller]
    pub fn min(&self, column: usize) -> f32 {
        Self::at(&self.min, column)
    }

    /// Raises `width` to the column minimum, a non-finite one counts as the minimum; returns the stored
    /// width, 0 for a column out of range.
    #[track_caller]
    pub fn set(&mut self, column: usize, width: f32) -> f32 {
        let min = self.min(column);
        let Some(slot) = self.widths.get_mut(column) else {
            return 0.0;
        };
        *slot = crate::kit::sanitize::finite(width, "ColumnWidths width")
            .unwrap_or(min)
            .max(min);
        *slot
    }

    /// Sum of the widths left of `column`, in points; past the last column, all of them.
    pub fn offset(&self, column: usize) -> f32 {
        self.widths.iter().take(column).sum()
    }

    #[track_caller]
    fn at(values: &[f32], column: usize) -> f32 {
        values.get(column).copied().unwrap_or_else(|| {
            crate::kit::sanitize::report(format_args!(
                "ColumnWidths column {column} is out of range for {}",
                values.len()
            ));
            0.0
        })
    }

    /// Sum of all widths in points.
    pub fn total(&self) -> f32 {
        self.widths.iter().sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cs(width: f32, min: f32) -> ColumnSize {
        ColumnSize { width, min }
    }

    #[test]
    fn stale_column_indexes_read_zero_and_change_nothing() {
        let mut w = ColumnWidths::new([cs(100.0, 40.0), cs(50.0, 20.0)]);
        assert_eq!((w.width(5), w.min(5)), (0.0, 0.0));
        assert_eq!(w.set(5, 80.0), 0.0);
        assert_eq!(w.offset(5), 150.0, "past the last column: all widths");
        assert_eq!(w.set(1, 10.0), 20.0, "raised to the minimum");
    }

    #[test]
    fn sort_cycle_and_stable_order() {
        let mut s = SortState::default();
        s.cycle(1);
        assert_eq!(s.order(1), Some(SortOrder::Ascending));
        s.cycle(1);
        assert_eq!(s.order(1), Some(SortOrder::Descending));
        s.cycle(1);
        assert_eq!(s.column, None);
        s.cycle(1);
        s.cycle(0);
        assert_eq!(s.order(0), Some(SortOrder::Ascending));

        let data = [3, 1, 2, 1];
        let asc = sort_order(4, s, |cell| data[cell.row]);
        assert_eq!(asc, [1, 3, 2, 0]);
        s.cycle(0);
        let desc = sort_order(4, s, |cell| data[cell.row]);
        assert_eq!(desc, [0, 2, 1, 3]);
    }

    #[test]
    fn widths() {
        let mut w = ColumnWidths::new([cs(100.0, 40.0), cs(80.0, 40.0)]);
        assert_eq!(w.set(0, 10.0), 40.0);
        assert_eq!(w.offset(1), 40.0);
        assert_eq!(w.total(), 120.0);
    }

    #[test]
    fn invalid_widths_are_raised_to_the_nearest_valid() {
        let w = ColumnWidths::new([cs(10.0, 40.0), cs(f32::NAN, -5.0)]);
        assert_eq!(
            w.total(),
            40.0,
            "width below min rises to min, bad min is 0, NaN width is min"
        );
    }

    #[test]
    fn non_finite_set_widths_count_as_the_minimum() {
        let mut w = ColumnWidths::new([cs(100.0, 40.0)]);
        assert_eq!(w.set(0, f32::INFINITY), 40.0);
        assert_eq!(w.set(0, f32::NAN), 40.0);
        assert_eq!(w.set(0, f32::NEG_INFINITY), 40.0);
    }
}
