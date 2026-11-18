use egui::{Align, IdSalt, Rect, ScrollArea, Ui, UiBuilder, pos2, vec2};

use super::HeightIndex;
use crate::semantic::KeyedSalt;

enum Mode {
    Uniform { row_height: f32 },
    Variable { estimate: f32 },
}

const MIN_HEIGHT: f32 = 1.0;

/// Virtualized scrolling list; paints only visible rows.
#[must_use = "call .show(..)"]
pub struct VirtualList {
    id_salt: IdSalt,
    mode: Mode,
    scroll_to: Option<(usize, Option<Align>)>,
}

crate::kit::opaque_debug!(VirtualList);

impl VirtualList {
    /// Every row is exactly `row_height` (excluding item spacing); delegates to `ScrollArea::show_rows`.
    /// Non-finite or sub-1 `row_height` counts as 1.
    pub fn uniform(id_salt: impl Into<KeyedSalt>, row_height: f32) -> Self {
        let row_height = crate::kit::sanitize::at_least(row_height, MIN_HEIGHT.., "VirtualList row_height");
        Self {
            id_salt: id_salt.into().id_salt(),
            mode: Mode::Uniform { row_height },
            scroll_to: None,
        }
    }

    /// Rows measure themselves on first show; offsets come from a [`HeightIndex`] kept in egui memory.
    /// Non-finite or sub-1 `estimate` counts as 1.
    pub fn variable(id_salt: impl Into<KeyedSalt>, estimate: f32) -> Self {
        let estimate = crate::kit::sanitize::at_least(estimate, MIN_HEIGHT.., "VirtualList estimate");
        Self {
            id_salt: id_salt.into().id_salt(),
            mode: Mode::Variable { estimate },
            scroll_to: None,
        }
    }

    /// Scrolls `row` to the `align` edge of the viewport this pass, or just into view with `None`
    /// (as [`egui::Response::scroll_to_me`]). Unmeasured rows above it in a variable list use the
    /// estimate, so the target settles once they are measured.
    pub fn scroll_to(mut self, row: usize, align: Option<Align>) -> Self {
        self.scroll_to = Some((row, align));
        self
    }

    /// `row` paints row index `i` of `total_rows`; only visible rows are called.
    #[tracing::instrument(level = "trace", skip_all)]
    pub fn show(self, ui: &mut Ui, total_rows: usize, mut row: impl FnMut(&mut Ui, usize)) {
        let scroll = ScrollArea::vertical().id_salt(self.id_salt).auto_shrink([false; 2]);
        let viewport_h = ui.available_height();
        let current = egui::scroll_area::State::load(ui.ctx(), ui.make_persistent_id(self.id_salt))
            .map_or(0.0, |st| st.clamped_offset().y);
        let target = |top: f32, h: f32, align: Option<Align>| match align {
            Some(Align::Min) => top,
            Some(Align::Center) => top - (viewport_h - h) / 2.0,
            Some(Align::Max) => top - (viewport_h - h),
            None => current.clamp(top + h - viewport_h, top.max(top + h - viewport_h)),
        };
        match self.mode {
            Mode::Uniform { row_height } => {
                let pitch = row_height + ui.spacing().item_spacing.y;
                let scroll = match self.scroll_to {
                    Some((r, a)) => scroll.vertical_scroll_offset(target(r as f32 * pitch, row_height, a).max(0.0)),
                    None => scroll,
                };
                scroll.show_rows(ui, row_height, total_rows, |ui, range| range.for_each(|i| row(ui, i)));
            }
            Mode::Variable { estimate } => {
                let mem_id = ui.make_persistent_id(self.id_salt);
                let mut index = ui
                    .data_mut(|d| d.remove_temp::<HeightIndex>(mem_id))
                    .unwrap_or_else(|| HeightIndex::new(total_rows, estimate));
                if index.len() != total_rows {
                    index.resize(total_rows);
                }
                let scroll = match self.scroll_to.and_then(|(r, a)| Some((r, index.height(r)?, a))) {
                    Some((r, h, a)) => scroll.vertical_scroll_offset(target(index.offset(r), h, a).max(0.0)),
                    None => scroll,
                };
                let mut changed = false;
                scroll.show_viewport(ui, |ui, viewport| {
                    ui.set_height(index.total());
                    let origin = ui.max_rect().min;
                    let width = ui.max_rect().width();
                    for i in index.range(viewport.y_range()) {
                        let rect =
                            Rect::from_min_size(pos2(origin.x, origin.y + index.offset(i)), vec2(width, estimate));
                        let height = ui
                            .scope_builder(UiBuilder::new().max_rect(rect).id_salt(i), |ui| row(ui, i))
                            .response
                            .rect
                            .height();
                        if index.height(i).is_some_and(|h| (height - h).abs() > 0.5) {
                            index.set(i, height);
                            changed = true;
                        }
                    }
                });
                ui.data_mut(|d| d.insert_temp(mem_id, index));
                if changed {
                    #[expect(
                        clippy::disallowed_methods,
                        reason = "one pass when measured row heights differ from the index; stops once they settle"
                    )]
                    ui.ctx().request_repaint();
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use egui::{Align, Rect, Sense, vec2};

    use super::{Mode, VirtualList};
    use crate::test_support::harness;

    const ROWS: usize = 1000;
    const ESTIMATE: f32 = 30.0;

    fn height(i: usize) -> f32 {
        if i.is_multiple_of(2) {
            ESTIMATE / 2.0
        } else {
            ESTIMATE * 2.0
        }
    }

    #[test]
    fn variable_rows_render_the_viewport_and_settle_on_the_scroll_target() {
        const TARGET: usize = 500;
        let h = harness(
            |ui, (shown, viewport): &mut (BTreeMap<usize, Rect>, Rect)| {
                shown.clear();
                *viewport = ui.available_rect_before_wrap();
                VirtualList::variable("rows", ESTIMATE)
                    .scroll_to(TARGET, Some(Align::Min))
                    .show(ui, ROWS, |ui, i| {
                        let (rect, _) = ui.allocate_exact_size(vec2(10.0, height(i)), Sense::hover());
                        shown.insert(i, rect);
                    });
            },
            (BTreeMap::new(), Rect::NOTHING),
        );
        let (shown, viewport) = &h.state().value;
        let rows = shown.keys().copied().collect::<Vec<_>>();
        assert!(rows.len() < ROWS / 10, "only rows near the viewport render: {rows:?}");
        assert_eq!(
            shown[&TARGET].top(),
            viewport.top(),
            "measured offsets put the target at the top"
        );
        for (i, r) in shown {
            assert_eq!(r.height(), height(*i), "row {i} keeps its measured height");
        }
    }

    #[test]
    fn invalid_heights_count_as_one() {
        for list in [VirtualList::uniform("a", 0.0), VirtualList::variable("b", f32::NAN)] {
            let one = matches!(
                list.mode,
                Mode::Uniform { row_height: 1.0 } | Mode::Variable { estimate: 1.0 }
            );
            assert!(one);
        }
    }
}
