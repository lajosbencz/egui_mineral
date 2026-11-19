//! `react-dialog/library/src/components/{DialogSurface,DialogTitle,DialogBody,DialogContent,DialogActions}/use*Styles.styles.ts`
//! and `contexts/constants.ts`, `DialogSurfaceMotion.ts` and `DialogBackdropMotion.ts`.

use std::time::Duration;

/// `SURFACE_PADDING`.
pub const PADDING: f32 = 24.0;
/// `DialogSurface` `maxWidth`.
pub const MAX_WIDTH: f32 = 600.0;
/// `DIALOG_GAP`: `DialogBody` and `DialogActions` `gap`.
pub const GAP: f32 = 8.0;
/// `DialogContent` `minHeight`.
pub const CONTENT_MIN_HEIGHT: f32 = 32.0;
/// `DIALOG_MEDIA_QUERY_BREAKPOINT`: up to it the surface is full width and the actions stack.
pub const BREAKPOINT: f32 = 480.0;
/// `DialogSurfaceMotion` and `DialogBackdropMotion` duration (`durationGentle`).
pub const MOTION: Duration = Duration::from_millis(250);
