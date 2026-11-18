// docs/rules.md rule 10: indices here outlive the frame.
#![cfg_attr(not(test), deny(clippy::indexing_slicing))]
#![warn(clippy::arithmetic_side_effects)]

use std::ops::Range;

/// Page index and size over a total.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(try_from = "RawPaging")]
pub struct Paging {
    index: usize,
    size: usize,
}

#[derive(serde::Deserialize)]
struct RawPaging {
    index: usize,
    size: usize,
}

impl TryFrom<RawPaging> for Paging {
    type Error = String;

    fn try_from(r: RawPaging) -> Result<Self, String> {
        if r.size == 0 {
            return Err("Paging: size must be > 0".into());
        }
        if r.index.checked_mul(r.size).is_none() {
            return Err(format!("Paging: index {} * size {} overflows", r.index, r.size));
        }
        Ok(Self {
            index: r.index,
            size: r.size,
        })
    }
}

impl Paging {
    /// First page of `size` rows; 0 counts as 1.
    pub fn new(size: usize) -> Self {
        Self {
            index: 0,
            size: crate::kit::sanitize::at_least(size, 1.., "Paging size"),
        }
    }

    /// 0-based; call [`Self::clamp_index`] when the row count changes.
    pub fn index(&self) -> usize {
        self.index
    }

    /// Rows per page; at least 1.
    pub fn size(&self) -> usize {
        self.size
    }

    /// `rows` is the current row count; an `index` past the last page of `rows` selects the last page.
    pub fn set_index(&mut self, index: usize, rows: usize) {
        self.index = crate::kit::sanitize::index(index, self.page_count(rows), "Paging index").unwrap_or_default();
    }

    /// At least 1, also for zero rows.
    pub fn page_count(&self, rows: usize) -> usize {
        rows.div_ceil(self.size).max(1)
    }

    /// Moves the index onto the last page when it is past it.
    pub fn clamp_index(&mut self, rows: usize) {
        self.index = self.index.min(self.page_count(rows).saturating_sub(1));
    }

    /// Empty when the index is past the last page of `rows` (rows shrank without [`Self::clamp_index`]).
    pub fn range(&self, rows: usize) -> Range<usize> {
        let start = self.index.saturating_mul(self.size).min(rows);
        start..start.saturating_add(self.size).min(rows)
    }

    /// Keeps the first row of the current page visible, so an in-range index stays in range; 0 counts as 1.
    pub fn set_size(&mut self, size: usize) {
        let first = self.index.saturating_mul(self.size);
        self.size = crate::kit::sanitize::at_least(size, 1.., "Paging size");
        self.index = first.checked_div(self.size).unwrap_or_default();
    }
}

/// Slot of a pagination control: a page or an ellipsis gap.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PageItem {
    /// 1-based page number.
    Page(usize),
    /// Ellipsis for skipped pages.
    Gap,
}

/// Pages shown around the current page and at each end.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Spread {
    /// Pages on each side of the current one.
    pub siblings: usize,
    /// Pages always shown at each end.
    pub boundaries: usize,
}

impl Spread {
    /// 1-based `page` of `pages` pages as the MUI `usePagination` layout (constant item count);
    /// a `page` outside `1..=pages` counts as the nearest page.
    pub fn items(self, page: usize, pages: usize) -> Vec<PageItem> {
        let Self { siblings, boundaries } = self;
        if pages == 0 {
            return Vec::new();
        }
        let current = if (1..=pages).contains(&page) {
            page
        } else {
            crate::kit::sanitize::report(format_args!("Spread: page {page} out of range 1..={pages}"));
            page.clamp(1, pages)
        };
        // Saturated slots exceed any `pages`: every page fits.
        let slots = boundaries
            .saturating_mul(2)
            .saturating_add(siblings.saturating_mul(2))
            .saturating_add(3);
        if pages <= slots {
            return (1..=pages).map(PageItem::Page).collect();
        }
        window(current, pages, self)
    }
}

#[expect(
    clippy::arithmetic_side_effects,
    reason = "1 <= current <= total and total > 2 * boundaries + 2 * siblings + 3 bound every term"
)]
fn window(current: usize, total: usize, Spread { siblings, boundaries }: Spread) -> Vec<PageItem> {
    let start = current
        .saturating_sub(siblings)
        .min(total - boundaries - 2 * siblings - 1)
        .max(boundaries + 2);
    let end = current
        .saturating_add(siblings)
        .max(boundaries + 2 * siblings + 2)
        .min(total - boundaries - 1);
    let mut out: Vec<_> = (1..=boundaries).map(PageItem::Page).collect();
    out.push(if start > boundaries + 2 {
        PageItem::Gap
    } else {
        PageItem::Page(boundaries + 1)
    });
    out.extend((start..=end).map(PageItem::Page));
    out.push(if end < total - boundaries - 1 {
        PageItem::Gap
    } else {
        PageItem::Page(total - boundaries)
    });
    out.extend((total - boundaries..total).map(|page| PageItem::Page(page + 1)));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paging_math() {
        let mut p = Paging::new(10);
        assert_eq!(p.page_count(0), 1);
        assert_eq!(p.page_count(25), 3);
        p.set_index(2, 25);
        assert_eq!(p.range(25), 20..25);
        p.set_size(5);
        assert_eq!(p.index(), 4);
        p.clamp_index(12);
        assert_eq!(p.index(), 2);
        assert_eq!(p.range(5), 5..5, "stale index past the end");
        assert!(
            Paging::try_from(RawPaging {
                index: usize::MAX,
                size: 2
            })
            .is_err()
        );
    }

    #[test]
    fn stale_and_zero_arguments_are_clamped() {
        let mut p = Paging::new(0);
        assert_eq!(p.size(), 1);
        p.set_index(99, 25);
        assert_eq!(p.index(), 24, "last page of 25 one-row pages");
        p.set_size(0);
        assert_eq!((p.size(), p.index()), (1, 24));
        let s = Spread {
            siblings: 1,
            boundaries: 1,
        };
        assert_eq!(s.items(99, 20), s.items(20, 20), "stale page snaps to the last");
        assert_eq!(s.items(0, 20), s.items(1, 20));
    }

    #[test]
    fn pagination_layout() {
        use PageItem::{Gap, Page};
        const S: Spread = Spread {
            siblings: 1,
            boundaries: 1,
        };
        assert_eq!(S.items(1, 5), (1..=5).map(Page).collect::<Vec<_>>());
        assert_eq!(
            S.items(1, 10),
            [Page(1), Page(2), Page(3), Page(4), Page(5), Gap, Page(10)]
        );
        assert_eq!(S.items(5, 10), [Page(1), Gap, Page(4), Page(5), Page(6), Gap, Page(10)]);
        assert_eq!(
            S.items(10, 10),
            [Page(1), Gap, Page(6), Page(7), Page(8), Page(9), Page(10)]
        );
    }

    #[test]
    fn huge_page_counts_do_not_overflow() {
        for (siblings, boundaries) in [(0, 0), (1, 0), (0, 1), (2, 2)] {
            let items = Spread { siblings, boundaries }.items(usize::MAX, usize::MAX);
            assert_eq!(items.last(), Some(&PageItem::Page(usize::MAX)));
            assert_eq!(items.len(), 2 * boundaries + 2 * siblings + 3);
        }
    }
}
