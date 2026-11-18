// Compose Strategy.kt
use super::{
    CarouselPadding, MIN_PADDED_SIZE,
    keyline_list::{Keyline, KeylineBuilder, KeylineList},
};

// Compose lerp(outputMin, outputMax, inputMin, inputMax, value)
pub(super) fn lerp_range(out_min: f32, out_max: f32, in_min: f32, in_max: f32, value: f32) -> f32 {
    if value <= in_min {
        out_min
    } else if value >= in_max {
        out_max
    } else {
        egui::lerp(out_min..=out_max, (value - in_min) / (in_max - in_min))
    }
}

pub(super) struct Strategy {
    pub(super) default: KeylineList,
    pub(super) start_steps: Vec<KeylineList>,
    pub(super) end_steps: Vec<KeylineList>,
    pub(super) available: f32,
    pub(super) spacing: f32,
    pub(super) start_shift: f32,
    pub(super) end_shift: f32,
    pub(super) start_points: Vec<f32>,
    pub(super) end_points: Vec<f32>,
}

impl Strategy {
    pub(super) fn new(default: KeylineList, available: f32, spacing: f32, padding: CarouselPadding) -> Self {
        let CarouselPadding {
            start: before,
            end: after,
        } = padding;
        let start_steps = start_steps(&default, available, spacing, before);
        let end_steps = end_steps(&default, available, spacing, after);
        let start_shift = start_steps.first().zip(start_steps.last()).map_or(0.0, |(f, l)| {
            (l.first().unadjusted_offset - f.first().unadjusted_offset).max(before)
        });
        let end_shift = end_steps.first().zip(end_steps.last()).map_or(0.0, |(f, l)| {
            (f.last().unadjusted_offset - l.last().unadjusted_offset).max(after)
        });
        let start_points = step_points(start_shift, &start_steps, true);
        let end_points = step_points(end_shift, &end_steps, false);
        Self {
            default,
            start_steps,
            end_steps,
            available,
            spacing,
            start_shift,
            end_shift,
            start_points,
            end_points,
        }
    }

    pub(super) fn item_size(&self) -> f32 {
        self.default.first_focal().size
    }

    pub(super) fn is_valid(&self) -> bool {
        !self.default.0.is_empty() && self.available != 0.0 && self.item_size() != 0.0
    }

    pub(super) fn pitch(&self) -> f32 {
        self.item_size() + self.spacing
    }

    // Compose calculateMaxScrollOffset
    pub(super) fn max_scroll(&self, count: usize) -> f32 {
        let n = count as f32;
        (self.item_size() * n + self.spacing * (n - 1.0) - self.available).max(0.0)
    }

    pub(super) fn step<'a>(&'a self, steps: &'a [KeylineList], i: usize) -> &'a KeylineList {
        steps.get(i).unwrap_or(&self.default)
    }

    // Compose getSnapPositionOffset
    pub(super) fn snap_offset(&self, index: usize, count: usize) -> f32 {
        let at = |k: Keyline| (k.unadjusted_offset - self.item_size() / 2.0).round();
        let mut offset = at(self.default.first_focal());
        let start_last = self.start_steps.len().saturating_sub(1);
        if index <= start_last {
            offset = at(self.step(&self.start_steps, start_last - index).first_focal());
        }
        let end_last = self.end_steps.len().saturating_sub(1);
        let last = count.saturating_sub(1);
        if index + end_last >= last && count > self.default.focal_count() {
            let i = end_last.saturating_sub(last.saturating_sub(index));
            offset = at(self.step(&self.end_steps, i).last_focal());
        }
        offset
    }

    pub(super) fn snap_points(&self, count: usize) -> Vec<f32> {
        let max = self.max_scroll(count);
        (0..count)
            .map(|i| (i as f32 * self.pitch() - self.snap_offset(i, count)).clamp(0.0, max))
            .collect()
    }

    // Compose getKeylineListForScrollOffset
    pub(super) fn keylines_at(&self, scroll: f32, max_scroll: f32) -> KeylineList {
        let positive = scroll.max(0.0);
        let start_offset = self.start_shift;
        let end_offset = (max_scroll - self.end_shift).max(0.0);
        if start_offset <= positive && positive <= end_offset {
            return self.default.clone();
        }
        let mut interpolation = lerp_range(1.0, 0.0, 0.0, start_offset, positive);
        let mut points = &self.start_points;
        let mut steps: Vec<&KeylineList> = self.start_steps.iter().collect();
        if positive > end_offset {
            interpolation = lerp_range(0.0, 1.0, end_offset, max_scroll, positive);
            points = &self.end_points;
            steps = self.end_steps.iter().collect();
            if end_offset < 0.01 && self.start_steps.len() == 2 && self.end_steps.len() == 2 {
                steps = vec![&self.start_steps[1], &self.end_steps[1]];
            }
        }
        let (mut from, mut to, mut stepped) = (0, 0, 0.0);
        let mut lower = points.first().copied().unwrap_or_default();
        for (i, &upper) in points.iter().enumerate().take(steps.len()).skip(1) {
            if interpolation <= upper {
                (from, to) = (i - 1, i);
                stepped = lerp_range(0.0, 1.0, lower, upper, interpolation);
                break;
            }
            lower = upper;
        }
        match (steps.get(from), steps.get(to)) {
            (Some(a), Some(b)) => a.lerp(b, stepped),
            _ => self.default.clone(),
        }
    }
}

pub(super) fn pad_last(
    steps: &mut [KeylineList],
    main: f32,
    spacing: f32,
    padding: f32,
    focal: fn(&KeylineList) -> usize,
) {
    let Some(last) = steps.last_mut() else { return };
    let i = focal(last);
    if let Some(shifted) = shift_for_padding(last, main, spacing, padding, last.at(i), i) {
        *last = shifted;
    }
}

pub(super) fn start_steps(default: &KeylineList, main: f32, spacing: f32, before: f32) -> Vec<KeylineList> {
    if default.0.is_empty() {
        return Vec::new();
    }
    let mut steps = vec![default.clone()];
    if default.is_first_focal_item_at_start_of_container() {
        if before != 0.0 {
            steps.extend(shift_for_padding(
                default,
                main,
                spacing,
                before,
                default.first_focal(),
                default.first_focal_index(),
            ));
        }
        return steps;
    }
    let start = default.first_non_anchor_index();
    let end = default.first_focal_index();
    if end <= start && default.first_focal().cutoff > 0.0 {
        steps.push(move_keyline(default, 0, 0, main, spacing));
        return steps;
    }
    for i in 0..end.saturating_sub(start) {
        let prev = steps.last().unwrap_or(default);
        let original = start + i;
        let dst = if original > 0 {
            prev.first_index_after_focal_range_with_size(default.at(original - 1).size)
                .saturating_sub(1)
        } else {
            default.0.len().saturating_sub(1)
        };
        let next = move_keyline(prev, default.first_non_anchor_index(), dst, main, spacing);
        steps.push(next);
    }
    if before != 0.0 {
        pad_last(&mut steps, main, spacing, before, KeylineList::first_focal_index);
    }
    steps
}

pub(super) fn end_steps(default: &KeylineList, main: f32, spacing: f32, after: f32) -> Vec<KeylineList> {
    if default.0.is_empty() {
        return Vec::new();
    }
    let mut steps = vec![default.clone()];
    if default.is_last_focal_item_at_end_of_container(main) {
        if after != 0.0 {
            steps.extend(shift_for_padding(
                default,
                main,
                spacing,
                -after,
                default.last_focal(),
                default.last_focal_index(),
            ));
        }
        return steps;
    }
    let start = default.last_focal_index();
    let end = default.last_non_anchor_index();
    if end <= start && default.last_focal().cutoff > 0.0 {
        steps.push(move_keyline(default, 0, 0, main, spacing));
        return steps;
    }
    for i in 0..end.saturating_sub(start) {
        let prev = steps.last().unwrap_or(default);
        let original = end - i;
        let dst = if original < default.0.len().saturating_sub(1) {
            prev.last_index_before_focal_range_with_size(default.at(original + 1).size) + 1
        } else {
            0
        };
        let next = move_keyline(prev, default.last_non_anchor_index(), dst, main, spacing);
        steps.push(next);
    }
    if after != 0.0 {
        pad_last(&mut steps, main, spacing, -after, KeylineList::last_focal_index);
    }
    steps
}

// Compose createShiftedKeylineListForContentPadding; every line keeps a size of at least 1
pub(super) fn shift_for_padding(
    from: &KeylineList,
    main: f32,
    spacing: f32,
    padding: f32,
    pivot: Keyline,
    pivot_index: usize,
) -> Option<KeylineList> {
    let non_anchor = from.0.iter().filter(|k| !k.is_anchor).count() as f32;
    let smallest = from.0.iter().map(|k| k.size).fold(f32::INFINITY, f32::min);
    let max_reduction = smallest - MIN_PADDED_SIZE;
    if non_anchor == 0.0 || max_reduction <= 0.0 {
        return None;
    }
    let limit = max_reduction * non_anchor;
    let padding = padding.clamp(-limit, limit);
    let reduction = padding / non_anchor;
    let mut b = KeylineBuilder::default();
    for k in &from.0 {
        b.add(k.size - reduction.abs(), k.is_anchor);
    }
    let mut shifted = b.with_pivot(main, spacing, pivot_index, pivot.offset - reduction / 2.0 + padding);
    if shifted.0.len() != from.0.len() {
        return None;
    }
    for (k, f) in shifted.0.iter_mut().zip(&from.0) {
        k.unadjusted_offset = f.unadjusted_offset;
    }
    Some(shifted)
}

// Compose moveKeylineAndCreateShiftedKeylineList
pub(super) fn move_keyline(from: &KeylineList, src: usize, dst: usize, main: f32, spacing: f32) -> KeylineList {
    if src >= from.0.len() {
        return from.clone();
    }
    let dir: isize = if src > dst { 1 } else { -1 };
    let delta = (from.at(src).size - from.at(src).cutoff + spacing) * dir as f32;
    let pivot = from.pivot_index().saturating_add_signed(dir);
    let pivot_offset = from.at(from.pivot_index()).offset + delta;
    let mut moved = from.0.clone();
    let k = moved.remove(src);
    moved.insert(dst.min(moved.len()), k);
    let mut b = KeylineBuilder::default();
    for k in &moved {
        b.add(k.size, k.is_anchor);
    }
    b.with_pivot(main, spacing, pivot, pivot_offset)
}

// Compose getStepInterpolationPoints
pub(super) fn step_points(total: f32, steps: &[KeylineList], shifting_left: bool) -> Vec<f32> {
    let mut points = vec![0.0];
    if total == 0.0 || steps.is_empty() {
        return points;
    }
    for i in 1..steps.len() {
        let (prev, curr) = (&steps[i - 1], &steps[i]);
        let shifted = if shifting_left {
            curr.first().unadjusted_offset - prev.first().unadjusted_offset
        } else {
            prev.last().unadjusted_offset - curr.last().unadjusted_offset
        };
        let point = if i == steps.len() - 1 {
            1.0
        } else {
            points[i - 1] + shifted / total
        };
        points.push(point);
    }
    points
}
