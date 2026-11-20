//! In-UI flexbox/grid uses `egui_taffy` (re-exported as [`flex`], with its `taffy`; wrapping rows of widgets:
//! [`Flow`]).

mod axis;
mod child;
mod constraints;
mod flow;
mod fraction;
mod zones;

pub use axis::{Axis, Edge};
pub use child::child;
pub use constraints::Constraints;
pub use flow::{Flow, FlowItems};
pub use fraction::Fraction;
pub use zones::{Measure, MinTarget, TargetPolicy, Zones, breakpoint, safe_area};

pub use egui_taffy as flex;

/// Part of an overflowing one-line text replaced by an ellipsis.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Elide {
    /// Cuts the end: `/home/user/pro...`.
    #[default]
    End,
    /// Cuts the middle: `/home/...ts/a.png`.
    Middle,
    /// Cuts the start, keeping e.g. a file name: `...shots/a.png`.
    Start,
}
