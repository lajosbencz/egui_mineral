//! `react-tags/library/src/components/{Tag/useTagStyles.styles.ts,InteractionTag/useInteractionTagStyles.styles.ts,
//! InteractionTagPrimary/useInteractionTagPrimaryStyles.styles.ts,
//! InteractionTagSecondary/useInteractionTagSecondaryStyles.styles.ts,TagGroup/useTagGroupStyles.styles.ts,
//! Tag/useTag.tsx}`.

/// Per-`size` values of the tag styles.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Metrics {
    /// `useRootStyles` `height`.
    pub height: f32,
    /// `tagSpacing*`: the inner left and right space.
    pub side_padding: f32,
    /// `useIconStyles` `width` and `fontSize` (also the dismiss icon's).
    pub icon: f32,
    /// `useIconStyles` `paddingRight`.
    pub icon_padding_end: f32,
    /// `tagAvatarSizeMap`.
    pub media: f32,
    /// `useMediaStyles` `paddingRight`.
    pub media_padding_end: f32,
    /// `useDismissIconStyles` `paddingLeft`.
    pub dismiss_padding_start: f32,
    /// `useRootWithSecondaryActionStyles` `paddingRight` of the primary part.
    pub primary_padding_end: f32,
    /// `useInteractionTagSecondaryStyles` `paddingLeft` and `paddingRight`.
    pub secondary_padding: f32,
    /// `useTagGroupStyles` `columnGap`.
    pub group_gap: f32,
}

/// `medium`.
pub const MEDIUM: Metrics = Metrics {
    height: 32.0,
    side_padding: 7.0,
    icon: 20.0,
    icon_padding_end: 4.0,
    media: 28.0,
    media_padding_end: 8.0,
    dismiss_padding_start: 4.0,
    primary_padding_end: 8.0,
    secondary_padding: 5.0,
    group_gap: 8.0,
};
/// `small`.
pub const SMALL: Metrics = Metrics {
    height: 24.0,
    side_padding: 5.0,
    icon: 16.0,
    icon_padding_end: 2.0,
    media: 20.0,
    media_padding_end: 6.0,
    dismiss_padding_start: 2.0,
    primary_padding_end: 6.0,
    secondary_padding: 3.0,
    group_gap: 6.0,
};
/// `extra-small`.
pub const EXTRA_SMALL: Metrics = Metrics {
    height: 20.0,
    side_padding: 5.0,
    icon: 12.0,
    icon_padding_end: 2.0,
    media: 16.0,
    media_padding_end: 6.0,
    dismiss_padding_start: 2.0,
    primary_padding_end: 6.0,
    secondary_padding: 5.0,
    group_gap: 4.0,
};
/// `useMediaStyles.base` `paddingLeft`.
pub const MEDIA_PADDING_START: f32 = 1.0;
/// `usePrimaryTextStyles.base` and `useSecondaryTextBaseClassName` inline padding (`spacingHorizontalXXS`).
pub const TEXT_PADDING: f32 = 2.0;
/// `usePrimaryTextStyles.withoutSecondaryText` `paddingBottom` (`spacingHorizontalXXS`).
pub const PRIMARY_PADDING_BOTTOM: f32 = 2.0;
/// `usePrimaryTextStyles.withSecondaryText` `marginTop`.
pub const SECONDARY_OFFSET: f32 = -2.0;
/// `baseStyles` `border` width (`strokeWidthThin`).
pub const BORDER: f32 = 1.0;
