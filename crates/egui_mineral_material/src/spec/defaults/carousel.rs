//! `CarouselDefaults` and the keyline strategy constants (`carousel/Carousel.kt`,
//! `carousel/Keylines.kt`, `carousel/Arrangement.kt`).

/// `CarouselDefaults.MinSmallItemSize`.
pub const MIN_SMALL_ITEM_SIZE: f32 = 40.0;
/// `CarouselDefaults.MaxSmallItemSize`.
pub const MAX_SMALL_ITEM_SIZE: f32 = 56.0;
/// `CarouselDefaults.ItemSpacing`.
pub const ITEM_SPACING: f32 = 0.0;
/// `CarouselDefaults.AnchorSize`: the off-screen keylines at both ends.
pub const ANCHOR_SIZE: f32 = 10.0;
/// `CarouselDefaults.MediumLargeItemDiffThreshold`.
pub const MEDIUM_LARGE_ITEM_DIFF_THRESHOLD: f32 = 0.85;
/// `Arrangement.MediumItemFlexPercentage`.
pub const MEDIUM_ITEM_FLEX_PERCENTAGE: f32 = 0.1;
/// `CarouselDefaults.singleAdvanceFlingBehavior` snap spring stiffness (`Spring.StiffnessMediumLow`).
pub const SNAP_STIFFNESS: f32 = 400.0;
/// `CarouselDefaults.singleAdvanceFlingBehavior` snap spring damping (`Spring.DampingRatioNoBouncy`).
pub const SNAP_DAMPING: f32 = 1.0;
