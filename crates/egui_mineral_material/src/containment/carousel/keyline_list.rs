// Compose KeylineList.kt
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(super) struct Keyline {
    pub(super) size: f32,
    pub(super) offset: f32,
    pub(super) unadjusted_offset: f32,
    pub(super) is_focal: bool,
    pub(super) is_anchor: bool,
    pub(super) is_pivot: bool,
    pub(super) cutoff: f32,
}

pub(super) fn lerp_keyline(a: Keyline, b: Keyline, f: f32) -> Keyline {
    let pick = |x: bool, y: bool| if f < 0.5 { x } else { y };
    Keyline {
        size: egui::lerp(a.size..=b.size, f),
        offset: egui::lerp(a.offset..=b.offset, f),
        unadjusted_offset: egui::lerp(a.unadjusted_offset..=b.unadjusted_offset, f),
        is_focal: pick(a.is_focal, b.is_focal),
        is_anchor: pick(a.is_anchor, b.is_anchor),
        is_pivot: pick(a.is_pivot, b.is_pivot),
        cutoff: egui::lerp(a.cutoff..=b.cutoff, f),
    }
}

// Lookups are total: an empty list yields index 0 and default keylines
#[derive(Clone, Debug, Default, PartialEq)]
pub(super) struct KeylineList(pub(super) Vec<Keyline>);

impl KeylineList {
    pub(super) fn at(&self, i: usize) -> Keyline {
        self.0.get(i).copied().unwrap_or_default()
    }

    pub(super) fn index_of(&self, f: impl Fn(&Keyline) -> bool) -> usize {
        self.0.iter().position(f).unwrap_or_default()
    }

    pub(super) fn last_index_of(&self, f: impl Fn(&Keyline) -> bool) -> usize {
        self.0.iter().rposition(f).unwrap_or_default()
    }

    pub(super) fn first(&self) -> Keyline {
        self.at(0)
    }

    pub(super) fn last(&self) -> Keyline {
        self.at(self.0.len().saturating_sub(1))
    }

    pub(super) fn pivot_index(&self) -> usize {
        self.index_of(|k| k.is_pivot)
    }

    pub(super) fn first_non_anchor_index(&self) -> usize {
        self.index_of(|k| !k.is_anchor)
    }

    pub(super) fn last_non_anchor_index(&self) -> usize {
        self.last_index_of(|k| !k.is_anchor)
    }

    pub(super) fn first_focal_index(&self) -> usize {
        self.index_of(|k| k.is_focal)
    }

    pub(super) fn last_focal_index(&self) -> usize {
        self.last_index_of(|k| k.is_focal)
    }

    pub(super) fn first_focal(&self) -> Keyline {
        self.at(self.first_focal_index())
    }

    pub(super) fn last_focal(&self) -> Keyline {
        self.at(self.last_focal_index())
    }

    pub(super) fn focal_count(&self) -> usize {
        self.last_focal_index() - self.first_focal_index() + 1
    }

    pub(super) fn is_first_focal_item_at_start_of_container(&self) -> bool {
        let first = self.first_focal();
        first.offset - first.size / 2.0 >= 0.0 && first == self.at(self.first_non_anchor_index())
    }

    pub(super) fn is_last_focal_item_at_end_of_container(&self, main: f32) -> bool {
        let last = self.last_focal();
        last.offset + last.size / 2.0 <= main && last == self.at(self.last_non_anchor_index())
    }

    #[expect(clippy::float_cmp, reason = "keyline sizes are copied, not computed")]
    pub(super) fn first_index_after_focal_range_with_size(&self, size: f32) -> usize {
        let last = self.0.len().saturating_sub(1);
        (self.last_focal_index()..=last)
            .find(|&i| self.at(i).size == size)
            .unwrap_or(last)
    }

    #[expect(clippy::float_cmp, reason = "keyline sizes are copied, not computed")]
    pub(super) fn last_index_before_focal_range_with_size(&self, size: f32) -> usize {
        (0..self.first_focal_index())
            .rev()
            .find(|&i| self.at(i).size == size)
            .unwrap_or(0)
    }

    pub(super) fn keyline_before(&self, unadjusted_offset: f32) -> Keyline {
        self.0
            .iter()
            .rev()
            .find(|k| k.unadjusted_offset < unadjusted_offset)
            .copied()
            .unwrap_or_else(|| self.first())
    }

    pub(super) fn keyline_after(&self, unadjusted_offset: f32) -> Keyline {
        self.0
            .iter()
            .find(|k| k.unadjusted_offset >= unadjusted_offset)
            .copied()
            .unwrap_or_else(|| self.last())
    }

    pub(super) fn lerp(&self, to: &Self, f: f32) -> Self {
        Self(self.0.iter().zip(&to.0).map(|(a, b)| lerp_keyline(*a, *b, f)).collect())
    }
}

#[derive(Clone, Copy, Debug)]
pub(super) enum Alignment {
    Start,
    Center,
}

// Compose KeylineListScopeImpl
#[derive(Default)]
pub(super) struct KeylineBuilder {
    pub(super) tmp: Vec<(f32, bool)>,
    pub(super) first_focal_index: Option<usize>,
    pub(super) focal_size: f32,
}

impl KeylineBuilder {
    pub(super) fn add(&mut self, size: f32, is_anchor: bool) {
        self.tmp.push((size, is_anchor));
        if !is_anchor && size > self.focal_size {
            self.first_focal_index = Some(self.tmp.len() - 1);
            self.focal_size = size;
        }
    }

    #[expect(clippy::float_cmp, reason = "focal sizes are copied, not computed")]
    pub(super) fn last_focal_index(&self) -> Option<usize> {
        let mut last = self.first_focal_index?;
        while last + 1 < self.tmp.len() && self.tmp[last + 1].0 == self.focal_size {
            last += 1;
        }
        Some(last)
    }

    pub(super) fn with_alignment(self, main: f32, spacing: f32, alignment: Alignment) -> KeylineList {
        let (Some(first), Some(last)) = (self.first_focal_index, self.last_focal_index()) else {
            return KeylineList::default();
        };
        let focal_count = last - first;
        let pivot_offset = match alignment {
            Alignment::Center => {
                let split = if spacing == 0.0 || focal_count % 2 == 0 {
                    0.0
                } else {
                    spacing / 2.0
                };
                let spaces = (focal_count / 2) as f32 * spacing;
                main / 2.0 - self.focal_size / 2.0 * focal_count as f32 - split - spaces
            }
            Alignment::Start => self.focal_size / 2.0,
        };
        self.with_pivot(main, spacing, first, pivot_offset)
    }

    pub(super) fn with_pivot(self, main: f32, spacing: f32, pivot_index: usize, pivot_offset: f32) -> KeylineList {
        let (Some(first), Some(last)) = (self.first_focal_index, self.last_focal_index()) else {
            return KeylineList::default();
        };
        if pivot_index >= self.tmp.len() {
            return KeylineList::default();
        }
        let item = self.focal_size;
        let focal = |i: usize| (first..=last).contains(&i);
        let cut_left = |size: f32, offset: f32| offset - size / 2.0 < 0.0 && offset + size / 2.0 > 0.0;
        let cut_right = |size: f32, offset: f32| offset - size / 2.0 < main && offset + size / 2.0 > main;
        let (pivot_size, pivot_anchor) = self.tmp[pivot_index];
        let pivot_cutoff = if cut_left(pivot_size, pivot_offset) {
            pivot_offset - pivot_size / 2.0
        } else if cut_right(pivot_size, pivot_offset) {
            pivot_offset + pivot_size / 2.0 - main
        } else {
            0.0
        };
        let mut lines = vec![Keyline {
            size: pivot_size,
            offset: pivot_offset,
            unadjusted_offset: pivot_offset,
            is_focal: focal(pivot_index),
            is_anchor: pivot_anchor,
            is_pivot: true,
            cutoff: pivot_cutoff,
        }];
        let mut offset = pivot_offset - item / 2.0 - spacing;
        let mut unadjusted = offset;
        for (i, &(size, is_anchor)) in self.tmp.iter().enumerate().take(pivot_index).rev() {
            let at = offset - size / 2.0;
            lines.insert(
                0,
                Keyline {
                    size,
                    offset: at,
                    unadjusted_offset: unadjusted - item / 2.0,
                    is_focal: focal(i),
                    is_anchor,
                    is_pivot: false,
                    cutoff: if cut_left(size, at) {
                        (at - size / 2.0).abs()
                    } else {
                        0.0
                    },
                },
            );
            offset -= size + spacing;
            unadjusted -= item + spacing;
        }
        offset = pivot_offset + item / 2.0 + spacing;
        unadjusted = offset;
        for (i, &(size, is_anchor)) in self.tmp.iter().enumerate().skip(pivot_index + 1) {
            let at = offset + size / 2.0;
            lines.push(Keyline {
                size,
                offset: at,
                unadjusted_offset: unadjusted + item / 2.0,
                is_focal: focal(i),
                is_anchor,
                is_pivot: false,
                cutoff: if cut_right(size, at) {
                    at + size / 2.0 - main
                } else {
                    0.0
                },
            });
            offset += size + spacing;
            unadjusted += item + spacing;
        }
        KeylineList(lines)
    }
}
