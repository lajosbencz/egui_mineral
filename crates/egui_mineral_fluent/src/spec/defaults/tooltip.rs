//! `react-tooltip/library/src/components/Tooltip/useTooltipStyles.styles.ts` and `useTooltipBase.tsx`.

use std::time::Duration;

/// `useStyles.root` `maxWidth`.
pub const MAX_WIDTH: f32 = 240.0;
/// `useStyles.root` `padding` top, border excluded.
pub const PADDING_TOP: f32 = 4.0;
/// `useStyles.root` `padding` bottom, border excluded.
pub const PADDING_BOTTOM: f32 = 6.0;
/// `useStyles.root` `padding` start and end, border excluded.
pub const PADDING_X: f32 = 11.0;
/// `arrowHeight`.
pub const ARROW_HEIGHT: f32 = 6.0;
/// `useTooltipBase.tsx` positioning `offset`.
pub const OFFSET: f32 = 4.0;
/// `useTooltipBase.tsx` `showDelay`.
pub const SHOW_DELAY: Duration = Duration::from_millis(250);
/// `useTooltipBase.tsx` `hideDelay`.
pub const HIDE_DELAY: Duration = Duration::from_millis(250);
