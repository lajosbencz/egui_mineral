//! `react-color-picker/library/src/components/{ColorPicker,ColorArea,ColorSlider,AlphaSlider}/use*Styles.styles.ts`.

/// `useRootStyles` `minWidth` and `minHeight` of `ColorArea`; the sliders share its width.
pub const AREA_MIN: f32 = 300.0;
/// `--fui-Slider__thumb--size`.
pub const THUMB_SIZE: f32 = 20.0;
/// `railSizeVar`.
pub const RAIL_SIZE: f32 = 20.0;
/// `useRootStyles` `minHeight` of `ColorSlider`.
pub const SLIDER_MIN_HEIGHT: f32 = 32.0;
/// Arrow key step as a fraction of the range (1 percent).
pub const KEY_STEP: f32 = 0.01;
/// Shift+arrow key step (10 percent).
pub const KEY_STEP_LARGE: f32 = 0.1;
/// Edge of a transparency checker cell: the styles load an image, no size is specified.
pub const CHECKER_CELL: f32 = 8.0;
/// `hueBackground` color stops, in `linear-gradient` order.
pub const HUE_STOPS: [[u8; 3]; 7] = [
    [255, 0, 0],
    [255, 0, 255],
    [0, 0, 255],
    [0, 255, 255],
    [0, 255, 0],
    [255, 255, 0],
    [255, 0, 0],
];
