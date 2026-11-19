//! `react-avatar/library/src/components/{Avatar/useAvatarStyles.styles.ts,
//! AvatarGroupItem/useAvatarGroupItemStyles.styles.ts,AvatarGroupPopover/useAvatarGroupPopoverStyles.styles.ts,
//! AvatarGroup/useAvatarGroupStyles.styles.ts}` and `Avatar/useAvatar.tsx`.

/// `useStyles.inactive` `transform: scale`.
pub const INACTIVE_SCALE: f32 = 0.875;
/// `useStyles.inactive` `opacity`.
pub const INACTIVE_OPACITY: f32 = 0.8;
/// `getInitials` input of `useAvatar`: sizes up to this show the first initial only.
pub const FIRST_INITIAL_ONLY_MAX: u16 = 16;
/// `useAvatarGroup` default `maxInlineItems` (`partitionAvatarGroupItems`).
pub const MAX_INLINE_ITEMS: usize = 5;
/// `partitionAvatarGroupItems` slices shown in the `pie` layout.
pub const PIE_SLICES: usize = 3;
/// `useAvatarGroupPopover` `indicator` default: the count from this size, else the icon.
pub const COUNT_INDICATOR_MIN_SIZE: u16 = 24;
/// `useAvatarGroupPopover` counts above this show as `99+`.
pub const COUNT_CAP: i64 = 99;
/// `usePopoverSurfaceStyles` `width`.
pub const POPOVER_WIDTH: f32 = 220.0;
/// `usePopoverSurfaceStyles` `maxHeight`.
pub const POPOVER_MAX_HEIGHT: f32 = 220.0;
/// `usePopoverSurfaceStyles` `minHeight`.
pub const POPOVER_MIN_HEIGHT: f32 = 80.0;

/// `useStyles.text*` `fontSize`: `Caption2Strong` to 24, `Caption1Strong` to 28, `Base300` to 40, `Subtitle2` to 56,
/// `Subtitle1` to 96, `Title3` above.
pub const fn text_size(size: u16) -> f32 {
    match size {
        0..=24 => 10.0,
        25..=28 => 12.0,
        29..=40 => 14.0,
        41..=56 => 16.0,
        57..=96 => 20.0,
        _ => 24.0,
    }
}

/// `useAvatarStyles_unstable` icon `fontSize` classes.
pub const fn icon_size(size: u16) -> f32 {
    match size {
        0..=16 => 12.0,
        17..=24 => 16.0,
        25..=40 => 20.0,
        41..=48 => 24.0,
        49..=56 => 28.0,
        57..=72 => 32.0,
        _ => 48.0,
    }
}

/// `useStyles.square*` `borderRadius`: small to 24, medium to 48, large to 72, extra large above.
pub const fn square_radius(size: u16) -> f32 {
    match size {
        0..=24 => 2.0,
        25..=48 => 4.0,
        49..=72 => 6.0,
        _ => 8.0,
    }
}

/// `useStyles.ringThick`, `ringThicker` and `ringThickest` `ringWidth`.
pub const fn ring_width(size: u16) -> f32 {
    match size {
        0..=48 => 2.0,
        49..=64 => 3.0,
        _ => 4.0,
    }
}

/// `useStyles.shadow4` to `shadow28`: the shadow level.
pub const fn shadow_level(size: u16) -> u8 {
    match size {
        0..=28 => 4,
        29..=48 => 8,
        49..=64 => 16,
        _ => 28,
    }
}

/// `useStyles` `badgeGap` of the badge sizes: thin up to `small`, thick from `large`.
pub const fn badge_gap(badge_radius: f32) -> f32 {
    if badge_radius >= 10.0 { 2.0 } else { 1.0 }
}

/// `useStackStyles` `thick`, `thicker`, `thickest` outline of the `stack` layout, and the
/// `usePieStyles` divider width (`strokeWidthThick`, `Thicker`, `Thickest`).
pub const fn group_outline(size: u16) -> f32 {
    match size {
        0..=55 => 2.0,
        56..=71 => 3.0,
        _ => 4.0,
    }
}

/// `useStackStyles` negative `marginLeft` of items after the first.
pub const fn stack_overlap(size: u16) -> f32 {
    match size {
        0..=23 => 2.0,
        24..=47 => 4.0,
        48..=95 => 8.0,
        _ => 16.0,
    }
}

/// `useSpreadStyles` `marginLeft` of items after the first.
pub const fn spread_gap(size: u16) -> f32 {
    match size {
        0..=19 => 8.0,
        20..=31 => 10.0,
        32..=63 => 16.0,
        _ => 20.0,
    }
}

/// `useTriggerButtonStyles.borderThin` to `borderThickest` width.
pub const fn trigger_border(size: u16) -> f32 {
    match size {
        0..=35 => 1.0,
        36..=55 => 2.0,
        56..=71 => 3.0,
        _ => 4.0,
    }
}
