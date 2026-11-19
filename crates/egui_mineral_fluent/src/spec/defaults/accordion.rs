//! `react-accordion/library/src/components/{AccordionHeader,AccordionPanel}/use*Styles.styles.ts`,
//! `AccordionHeader/useAccordionHeader.tsx` and `react-motion-components-preview` `Collapse/Collapse.ts`.

use std::time::Duration;

/// `useStyles.button` `minHeight`.
pub const HEADER_MIN_HEIGHT: f32 = 44.0;
/// `useStyles.buttonSmall` `minHeight`.
pub const HEADER_MIN_HEIGHT_SMALL: f32 = 32.0;
/// `useStyles.button` `padding` start (`spacingHorizontalMNudge`).
pub const HEADER_PADDING_START: f32 = 10.0;
/// `useStyles.button` `padding` end (`spacingHorizontalM`).
pub const HEADER_PADDING_END: f32 = 12.0;
/// `useStyles.expandIconStart` `paddingRight` and `icon` `paddingRight` (`spacingHorizontalS`).
pub const ICON_GAP: f32 = 8.0;
/// `useStyles.expandIcon` and `icon` `fontSize` (`fontSizeBase500`).
pub const ICON_SIZE: f32 = 20.0;
/// `useStyles.root` `borderRadius` is `borderRadiusMedium`; `useAccordionPanelStyles.root` `margin` inline (`spacingHorizontalM`).
pub const PANEL_MARGIN: f32 = 12.0;
/// `useAccordionHeader` chevron `transition` (`durationNormal`).
pub const ROTATE: Duration = Duration::from_millis(200);
/// `Collapse` `duration` default (`durationNormal`).
pub const COLLAPSE: Duration = Duration::from_millis(200);
