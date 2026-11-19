//! Star rating hit testing shared by stylekits.

use egui::{Pos2, Rect};

use crate::{
    i18n::Direction,
    range::{RangeModel, Track},
};

/// Smallest rating increment.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Step {
    /// Whole stars.
    #[default]
    Whole,
    /// Half stars.
    Half,
}

impl Step {
    /// [`Self::Half`] for `true`.
    pub const fn half_if(half: bool) -> Self {
        if half { Self::Half } else { Self::Whole }
    }
}

/// Rating under pointer `p` over `max` stars of width `cell` laid out in `rect`, snapped to `step`.
pub fn value_at(rect: Rect, p: Pos2, dir: Direction, cell: f32, max: u8, step: Step) -> f64 {
    let half = step == Step::Half;
    let dx = RangeModel::new(0.0, f64::from(rect.width())).value_at(rect.x_range(), p.x, Track::horizontal(dir)) as f32;
    let i = (dx / cell).floor().clamp(0.0, f32::from(max) - 1.0);
    let frac = ((dx - i * cell) / cell).clamp(0.0, 1.0);
    let model = RangeModel::new(0.0, f64::from(max)).step(if half { 0.5 } else { 1.0 });
    model.snap(f64::from(i) + if half && frac < 0.5 { 0.5 } else { 1.0 })
}
