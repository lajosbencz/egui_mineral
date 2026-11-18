//! `Checkbox.kt` private values, with `ComposeMaterial3Flags.isCheckboxStylingFixEnabled` at its
//! default (off), and the labeled row of `CheckboxWithTextSample`.

use std::time::Duration;

use crate::spec::TypeScale;

/// `CheckboxSize`, the drawn box; `CheckboxTokens.ContainerSize` applies only with the styling fix.
pub const BOX_SIZE: f32 = 20.0;
/// `CheckboxDefaultPadding`, around the box.
pub const PADDING: f32 = 2.0;
/// `RadiusSize`.
pub const RADIUS: f32 = 2.0;
/// `CheckboxDefaults.StrokeWidth`, of the outline and the check mark.
pub const STROKE_WIDTH: f32 = 2.0;
/// `drawCheck` `leftX`, `leftY`, as fractions of the box.
pub const CHECK_START: [f32; 2] = [0.2, 0.5];
/// `drawCheck` `checkCrossX`, `checkCrossY`.
pub const CHECK_CROSS: [f32; 2] = [0.4, 0.7];
/// `drawCheck` `rightX`, `rightY`.
pub const CHECK_END: [f32; 2] = [0.8, 0.3];
/// `drawCheck` center the indeterminate dash gravitates to.
pub const CHECK_CENTER: f32 = 0.5;
/// `SnapAnimationDelay`, before an unchecked mark snaps away.
pub const SNAP_DELAY: Duration = Duration::from_millis(100);
/// `CheckboxWithTextSample` `Text(Modifier.padding(start))`, after the control; the radio and
/// switch samples use the same row.
pub const LABEL_SPACE: f32 = 16.0;
/// `CheckboxWithTextSample` label style.
pub const LABEL_FONT: TypeScale = TypeScale::BodyLarge;
