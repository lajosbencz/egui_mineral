use std::ops::Range;

/// Row heights with O(log n) offset queries (Fenwick tree).
#[derive(Clone, Debug, Default)]
pub struct HeightIndex {
    tree: Vec<f32>,
    heights: Vec<f32>,
    estimate: f32,
}

impl HeightIndex {
    /// `len` rows of `estimate` points each until measured.
    pub fn new(len: usize, estimate: f32) -> Self {
        let mut s = Self {
            tree: Vec::new(),
            heights: Vec::new(),
            estimate,
        };
        s.resize(len);
        s
    }

    /// Row count.
    pub fn len(&self) -> usize {
        self.heights.len()
    }

    /// `true` without rows.
    pub fn is_empty(&self) -> bool {
        self.heights.is_empty()
    }

    /// Points; `None` past the last row.
    pub fn height(&self, row: usize) -> Option<f32> {
        self.heights.get(row).copied()
    }

    /// Added rows start at the estimate; O(n) rebuild.
    pub fn resize(&mut self, len: usize) {
        self.heights.resize(len, self.estimate);
        self.tree = vec![0.0; len + 1];
        for j in 1..=len {
            self.tree[j] += self.heights[j - 1];
            let parent = j + j.isolate_lowest_one();
            if parent <= len {
                self.tree[parent] += self.tree[j];
            }
        }
    }

    /// Records a measured height in points; O(log n). Ignored past the last row.
    pub fn set(&mut self, row: usize, height: f32) {
        let Some(h) = self.heights.get_mut(row) else {
            return;
        };
        let delta = height - *h;
        if delta == 0.0 {
            return;
        }
        *h = height;
        let mut j = row + 1;
        while j < self.tree.len() {
            self.tree[j] += delta;
            j += j.isolate_lowest_one();
        }
    }

    /// Top of `row` in points from the first row; `row >= len` gives the total.
    pub fn offset(&self, row: usize) -> f32 {
        let mut sum = 0.0;
        let mut j = row.min(self.len());
        while j > 0 {
            sum += self.tree[j];
            j -= j.isolate_lowest_one();
        }
        sum
    }

    /// Sum of all heights in points.
    pub fn total(&self) -> f32 {
        self.offset(self.len())
    }

    /// Row containing `y` points; clamped to the last row, 0 when empty.
    pub fn row_at(&self, y: f32) -> usize {
        let n = self.len();
        let mut pos = 0;
        let mut rem = y;
        let mut step = n.next_power_of_two();
        while step > 0 {
            let next = pos + step;
            if next <= n && self.tree[next] <= rem {
                pos = next;
                rem -= self.tree[next];
            }
            step >>= 1;
        }
        pos.min(n.saturating_sub(1))
    }

    /// Rows intersecting `span` (y in points); empty when there are no rows.
    pub fn range(&self, span: egui::Rangef) -> Range<usize> {
        if self.is_empty() {
            return 0..0;
        }
        self.row_at(span.min.max(0.0))..(self.row_at(span.max) + 1).min(self.len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mixed_measured_and_estimated_rows() {
        let mut idx = HeightIndex::new(100, 10.0);
        assert_eq!(idx.total(), 1000.0);
        idx.set(0, 50.0);
        idx.set(3, 30.0);
        assert_eq!(idx.offset(4), 100.0);
        assert_eq!(idx.total(), 1060.0);
        idx.set(100, 1.0);
        assert_eq!((idx.height(100), idx.offset(500), idx.total()), (None, 1060.0, 1060.0));
        assert_eq!(idx.row_at(49.0), 0);
        assert_eq!(idx.row_at(50.0), 1);
        assert_eq!(idx.row_at(75.0), 3);
        assert_eq!(idx.range(egui::Rangef::new(55.0, 100.0)), 1..5);
        assert_eq!(idx.range(egui::Rangef::new(5000.0, 6000.0)), 99..100);
        idx.resize(5);
        assert_eq!(idx.total(), 110.0);
    }
}
