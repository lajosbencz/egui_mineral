//! Star rating hit testing shared by stylekits.

use egui::{Pos2, Rect};

use crate::{
    i18n::Direction,
    range::{RangeModel, Track},
};

/// Rating under pointer `p` over `max` stars of width `cell` laid out in `rect`; `half` allows half stars.
/// The result is snapped to the whole or half step.
pub fn value_at(rect: Rect, p: Pos2, dir: Direction, cell: f32, max: u8, half: bool) -> f64 {
    let dx = RangeModel::new(0.0, f64::from(rect.width())).value_at(rect.x_range(), p.x, Track::horizontal(dir)) as f32;
    let i = (dx / cell).floor().clamp(0.0, f32::from(max) - 1.0);
    let frac = ((dx - i * cell) / cell).clamp(0.0, 1.0);
    let model = RangeModel::new(0.0, f64::from(max)).step(if half { 0.5 } else { 1.0 });
    model.snap(f64::from(i) + if half && frac < 0.5 { 0.5 } else { 1.0 })
}
