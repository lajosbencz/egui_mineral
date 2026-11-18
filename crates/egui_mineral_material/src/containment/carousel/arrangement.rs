// Compose Arrangement.kt
use super::cx;

#[derive(Clone, Copy, Debug)]
pub(super) struct Arrangement {
    pub(super) priority: u32,
    pub(super) small_size: f32,
    pub(super) small_count: usize,
    pub(super) medium_size: f32,
    pub(super) medium_count: usize,
    pub(super) large_size: f32,
    pub(super) large_count: usize,
}

pub(super) struct ArrangementInput<'a> {
    pub(super) available: f32,
    pub(super) spacing: f32,
    pub(super) target_small: f32,
    pub(super) min_small: f32,
    pub(super) max_small: f32,
    pub(super) small_counts: &'a [usize],
    pub(super) target_medium: f32,
    pub(super) medium_counts: &'a [usize],
    pub(super) target_large: f32,
    pub(super) large_counts: &'a [usize],
}

impl Arrangement {
    pub(super) fn is_valid(&self) -> bool {
        if self.large_count > 0 && self.small_count > 0 && self.medium_count > 0 {
            self.large_size > self.medium_size && self.medium_size > self.small_size
        } else if self.large_count > 0 && self.small_count > 0 {
            self.large_size > self.small_size
        } else {
            true
        }
    }

    pub(super) fn cost(&self, target_large: f32) -> f32 {
        if self.is_valid() {
            (target_large - self.large_size).abs() * self.priority as f32
        } else {
            f32::MAX
        }
    }

    pub(super) fn item_count(&self) -> usize {
        self.large_count + self.medium_count + self.small_count
    }

    pub(super) fn find_lowest_cost(input: &ArrangementInput<'_>) -> Option<Self> {
        let mut lowest: Option<Self> = None;
        let mut priority = 1;
        for &large in input.large_counts {
            for &medium in input.medium_counts {
                for &small in input.small_counts {
                    let a = Self::fit(input, priority, small, medium, large);
                    if lowest.is_none_or(|l| a.cost(input.target_large) < l.cost(input.target_large)) {
                        lowest = Some(a);
                        if a.cost(input.target_large) == 0.0 {
                            return lowest;
                        }
                    }
                    priority += 1;
                }
            }
        }
        lowest
    }

    #[expect(clippy::float_cmp, reason = "Compose compares the computed large size exactly")]
    pub(super) fn fit(input: &ArrangementInput<'_>, priority: u32, small: usize, medium: usize, large: usize) -> Self {
        let total = (large + medium + small) as f32;
        let space = input.available - (total - 1.0) * input.spacing;
        let mut small_size = input.target_small.clamp(input.min_small, input.max_small);
        let taken = input.target_large * large as f32 + input.target_medium * medium as f32 + small_size * small as f32;
        let delta = space - taken;
        if small > 0 && delta > 0.0 {
            small_size += (delta / small as f32).min(input.max_small - small_size);
        } else if small > 0 && delta < 0.0 {
            small_size += (delta / small as f32).max(input.min_small - small_size);
        }
        if small == 0 {
            small_size = 0.0;
        }
        let mut large_size =
            (space - (small as f32 + medium as f32 / 2.0) * small_size) / (large as f32 + medium as f32 / 2.0);
        let mut medium_size = f32::midpoint(large_size, small_size);
        if medium > 0 && large_size != input.target_large {
            let adjustment = (input.target_large - large_size) * large as f32;
            let flex = medium_size * cx::MEDIUM_ITEM_FLEX_PERCENTAGE * medium as f32;
            let distribute = adjustment.abs().min(flex);
            if adjustment > 0.0 {
                medium_size -= distribute / medium as f32;
                large_size += distribute / large as f32;
            } else {
                medium_size += distribute / medium as f32;
                large_size -= distribute / large as f32;
            }
        }
        Self {
            priority,
            small_size,
            small_count: small,
            medium_size,
            medium_count: medium,
            large_size,
            large_count: large,
        }
    }
}
