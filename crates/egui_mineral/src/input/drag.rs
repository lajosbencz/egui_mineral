use egui::{Response, Vec2};

/// While `r` is dragged: `value` as it was when the drag started, and the pointer travel since the press.
/// A click-and-drag widget's drag starts past the click distance, so summing per-frame deltas loses that
/// travel; derive the dragged quantity from these instead.
pub fn drag_from_start<T: Clone + Send + Sync + 'static>(r: &Response, value: T) -> Option<(T, Vec2)> {
    let id = r.id.with("mineral::drag_start");
    if r.drag_started() {
        r.ctx.data_mut(|d| d.insert_temp(id, value.clone()));
    }
    let travel = r.total_drag_delta()?;
    Some((r.ctx.data(|d| d.get_temp(id)).unwrap_or(value), travel))
}
