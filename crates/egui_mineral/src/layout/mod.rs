//! In-UI flexbox/grid uses `egui_taffy` (re-exported as [`flex`], with its `taffy`; wrapping rows of widgets:
//! [`Flow`]).

mod axis;
mod child;
mod constraints;
mod flow;
mod zones;

pub use axis::{Axis, Edge};
pub use child::child;
pub use constraints::Constraints;
pub use flow::{Flow, FlowItems};
pub use zones::{Measure, MinTarget, TargetPolicy, Zones, breakpoint, safe_area};

pub use egui_taffy as flex;
