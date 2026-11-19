//! `react-toast/library/src/components/{Toast,ToastTitle,ToastBody,ToastFooter,ToastContainer,Toaster}/use*Styles.styles.ts`.

use std::time::Duration;

/// `Toaster` `width`.
pub const WIDTH: f32 = 292.0;
/// `Toast` `padding`.
pub const PADDING: f32 = 12.0;
/// `ToastContainer` `marginTop`: gap between stacked toasts.
pub const GAP: f32 = 16.0;
/// `ToastTitle` `media` `fontSize`.
pub const MEDIA_SIZE: f32 = 16.0;
/// `ToastTitle` `media` `paddingTop`.
pub const MEDIA_PADDING_TOP: f32 = 2.0;
/// `ToastTitle` `media` `paddingRight`.
pub const MEDIA_GAP: f32 = 8.0;
/// `ToastTitle` `action` `paddingLeft`.
pub const ACTION_PADDING_START: f32 = 12.0;
/// `Toast` `lineHeight` of the title.
pub const TITLE_LINE_HEIGHT: f32 = 20.0;
/// `ToastBody` `paddingTop`.
pub const BODY_PADDING_TOP: f32 = 6.0;
/// `ToastBody` `subtitle` `paddingTop`.
pub const SUBTITLE_PADDING_TOP: f32 = 4.0;
/// `ToastFooter` `paddingTop`.
pub const FOOTER_PADDING_TOP: f32 = 16.0;
/// `ToastFooter` `gap`.
pub const FOOTER_GAP: f32 = 14.0;
/// `timeout` default of `dispatchToast` (`useToastController`).
pub const TIMEOUT: Duration = Duration::from_millis(3000);
/// Toasts shown at once by [`toast_queue`](crate::feedback::toast_queue); React's `limit` is unbounded.
pub const LIMIT: usize = 20;
/// Distance from the screen edge (`offset`, `getPositionStyles`); not in the vendored styles file.
pub const EDGE_OFFSET: f32 = 16.0;
/// Enter and exit transition; not in the vendored styles file.
pub const MOTION: Duration = Duration::from_millis(300);
/// Slide distance of the enter and exit transition; not in the vendored styles file.
pub const SLIDE: f32 = 16.0;
