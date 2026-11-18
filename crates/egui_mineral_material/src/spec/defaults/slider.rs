//! `Slider.kt` private values, the MDC `m3_comp_slider_*` size tokens Compose has no API for, and
//! the `SliderSamples.kt` track icon and `PlainTooltip` value label.

/// One slider size: track thickness, handle length, outer track corner, inset icon.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SliderMetrics {
    /// Track thickness in dp.
    pub track: f32,
    /// Handle height in dp, across the track.
    pub handle: f32,
    /// `None`: half the track (`Track` without `trackCornerSize`).
    pub corner: Option<f32>,
    /// Inset track icon size in dp; `None` shows no icon.
    pub icon: Option<f32>,
}

/// `m3_comp_slider_xsmall_*`, the default `Slider`.
pub const XS: SliderMetrics = SliderMetrics {
    track: 16.0,
    handle: 44.0,
    corner: None,
    icon: None,
};
/// `m3_comp_slider_small_*`.
pub const S: SliderMetrics = SliderMetrics {
    track: 24.0,
    handle: 44.0,
    corner: Some(8.0),
    icon: None,
};
/// `m3_comp_slider_medium_*`.
pub const M: SliderMetrics = SliderMetrics {
    track: 40.0,
    handle: 44.0,
    corner: Some(12.0),
    icon: Some(24.0),
};
/// `m3_comp_slider_large_*`.
pub const L: SliderMetrics = SliderMetrics {
    track: 56.0,
    handle: 68.0,
    corner: Some(16.0),
    icon: Some(24.0),
};
/// `m3_comp_slider_xlarge_*`.
pub const XL: SliderMetrics = SliderMetrics {
    track: 96.0,
    handle: 108.0,
    corner: Some(28.0),
    icon: Some(32.0),
};
/// `TrackInsideCornerSize`.
pub const INSIDE_CORNER: f32 = 2.0;
/// `SliderWithTrackIconsSample` `iconPadding`.
pub const ICON_PADDING: f32 = 10.0;
/// `SliderWithCustomThumbSample` value label `sizeIn`.
pub const VALUE_MIN_SIZE: [f32; 2] = [45.0, 25.0];
/// `PlainTooltipContentPadding` (horizontal, vertical).
pub const VALUE_PADDING: [f32; 2] = [8.0, 4.0];
/// `SpacingBetweenTooltipAndAnchor`.
pub const VALUE_SPACE: f32 = 4.0;
