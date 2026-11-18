// Compose Keylines.kt
use super::{
    arrangement::{Arrangement, ArrangementInput},
    cx,
    keyline_list::{Alignment, KeylineBuilder, KeylineList},
};

// Compose largeCounts
pub(super) fn large_counts(min_available: f32, main: f32, target_large: f32) -> Vec<usize> {
    #[expect(clippy::cast_sign_loss, reason = "clamped to >= 1 and main > 0")]
    let (fewest, most) = (
        ((min_available / target_large).floor() as usize).max(1),
        (main / target_large).ceil() as usize,
    );
    (fewest..=most.max(fewest)).rev().collect()
}

pub(super) fn multi_browse_keylines(main: f32, preferred: f32, spacing: f32, count: usize) -> KeylineList {
    let (min_small, max_small) = (cx::MIN_SMALL_ITEM_SIZE, cx::MAX_SMALL_ITEM_SIZE);
    if main == 0.0 || preferred == 0.0 {
        return KeylineList::default();
    }
    let mut small_counts: &[usize] = &[1];
    let medium_counts: &[usize] = &[1, 0];
    let target_large = preferred.min(main);
    let target_small = (target_large / 3.0).clamp(min_small, max_small);
    let target_medium = f32::midpoint(target_large, target_small);
    if main < min_small * 2.0 {
        small_counts = &[0];
    }
    let min_available = main - target_medium - max_small * small_counts.iter().copied().max().unwrap_or(0) as f32;
    let large = large_counts(min_available, main, target_large);
    let input = |small: &'static [usize], medium: &'static [usize]| ArrangementInput {
        available: main,
        spacing,
        target_small,
        min_small,
        max_small,
        small_counts: small,
        target_medium,
        medium_counts: medium,
        target_large,
        large_counts: &large,
    };
    let mut arrangement = Arrangement::find_lowest_cost(&input(small_counts, medium_counts));
    if let Some(a) = arrangement
        && a.item_count() > count
    {
        let (mut small, mut medium) = (a.small_count, a.medium_count);
        for _ in 0..a.item_count() - count {
            if small > 0 {
                small -= 1;
            } else if medium > 1 {
                medium -= 1;
            }
        }
        let (smalls, mediums) = ([small], [medium]);
        arrangement = Arrangement::find_lowest_cost(&ArrangementInput {
            small_counts: &smalls,
            medium_counts: &mediums,
            ..input(small_counts, medium_counts)
        });
    }
    arrangement.map_or_else(KeylineList::default, |a| {
        start_aligned(main, spacing, cx::ANCHOR_SIZE, cx::ANCHOR_SIZE, &a)
    })
}

pub(super) fn uncontained_keylines(main: f32, item: f32, spacing: f32) -> KeylineList {
    if main == 0.0 || item == 0.0 {
        return KeylineList::default();
    }
    let large_size = (item + spacing).min(main);
    #[expect(clippy::cast_sign_loss, reason = "clamped to >= 1")]
    let large_count = ((main / large_size).floor() as usize).max(1);
    let remaining = main - large_count as f32 * large_size;
    let medium_count = usize::from(remaining > 0.0);
    let medium_size = medium_child_size(cx::ANCHOR_SIZE, large_size, remaining);
    let a = Arrangement {
        priority: 0,
        small_size: 0.0,
        small_count: 0,
        medium_size,
        medium_count,
        large_size,
        large_count,
    };
    let left_anchor = cx::ANCHOR_SIZE.min(item).max(medium_size * 0.5);
    start_aligned(main, spacing, left_anchor, cx::ANCHOR_SIZE, &a)
}

pub(super) fn hero_keylines(
    main: f32,
    preferred: Option<f32>,
    spacing: f32,
    count: usize,
    centered: bool,
) -> KeylineList {
    let (min_small, max_small) = (cx::MIN_SMALL_ITEM_SIZE, cx::MAX_SMALL_ITEM_SIZE);
    if main == 0.0 {
        return KeylineList::default();
    }
    let center = centered && count >= 3;
    let mut small_count = match count {
        0 | 1 => 0,
        _ if center => 2,
        _ => 1,
    };
    let target_large = preferred.unwrap_or(main).min(main);
    let target_small = (target_large / 3.0).clamp(min_small, max_small);
    let fullscreen_threshold = min_small * small_count as f32 + min_small * 1.25;
    if main < fullscreen_threshold {
        small_count = 0;
    }
    let min_available = main - min_small * small_count as f32;
    let large = large_counts(min_available, main, target_large);
    let smalls = [small_count];
    let Some(a) = Arrangement::find_lowest_cost(&ArrangementInput {
        available: main,
        spacing,
        target_small,
        min_small,
        max_small,
        small_counts: &smalls,
        target_medium: 0.0,
        medium_counts: &[0],
        target_large,
        large_counts: &large,
    }) else {
        return KeylineList::default();
    };
    if center && count >= a.item_count() {
        center_aligned(main, spacing, cx::ANCHOR_SIZE, cx::ANCHOR_SIZE, &a)
    } else {
        start_aligned(main, spacing, cx::ANCHOR_SIZE, cx::ANCHOR_SIZE, &a)
    }
}

pub(super) fn start_aligned(
    main: f32,
    spacing: f32,
    left_anchor: f32,
    right_anchor: f32,
    a: &Arrangement,
) -> KeylineList {
    let mut b = KeylineBuilder::default();
    b.add(left_anchor, true);
    (0..a.large_count).for_each(|_| b.add(a.large_size, false));
    (0..a.medium_count).for_each(|_| b.add(a.medium_size, false));
    (0..a.small_count).for_each(|_| b.add(a.small_size, false));
    b.add(right_anchor, true);
    b.with_alignment(main, spacing, Alignment::Start)
}

pub(super) fn center_aligned(
    main: f32,
    spacing: f32,
    left_anchor: f32,
    right_anchor: f32,
    a: &Arrangement,
) -> KeylineList {
    let mut b = KeylineBuilder::default();
    b.add(left_anchor, true);
    (0..a.small_count / 2).for_each(|_| b.add(a.small_size, false));
    (0..a.medium_count / 2).for_each(|_| b.add(a.medium_size, false));
    (0..a.large_count).for_each(|_| b.add(a.large_size, false));
    (0..a.medium_count / 2).for_each(|_| b.add(a.medium_size, false));
    (0..a.small_count / 2).for_each(|_| b.add(a.small_size, false));
    b.add(right_anchor, true);
    b.with_alignment(main, spacing, Alignment::Center)
}

pub(super) fn medium_child_size(min_medium: f32, large: f32, remaining: f32) -> f32 {
    let mut medium = (remaining * 1.5).max(min_medium);
    let threshold = large * cx::MEDIUM_LARGE_ITEM_DIFF_THRESHOLD;
    if medium > threshold {
        medium = threshold.max(remaining * 1.2).min(large);
    }
    medium
}
