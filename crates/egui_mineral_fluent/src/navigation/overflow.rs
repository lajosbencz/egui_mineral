use egui::{Context, Id};
use egui_mineral::semantic::KeyedSalt;

use crate::actions::PushButton;

/// Which end of a row [`Overflow::fit`] hides first (`overflowDirection`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum OverflowDirection {
    /// Trailing items hide first.
    #[default]
    End,
    /// Leading items hide first.
    Start,
}

/// One entry of an [`Overflow`] row.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct OverflowItem {
    /// Natural main-axis size.
    pub width: f32,
    /// Higher stays visible longer (`priority`); ties hide from [`OverflowDirection`]'s end.
    pub priority: i32,
}

impl OverflowItem {
    /// `width` in points, priority 0.
    pub const fn new(width: f32) -> Self {
        Self { width, priority: 0 }
    }
}

/// Main-axis space of an [`Overflow`] row ([`Overflow::fit`]).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct OverflowSpace {
    /// Space for the items.
    pub available: f32,
    /// Size of the overflow button, reserved only when something hides.
    pub menu: f32,
}

/// Fluent `Overflow`: decides which items of a row fit; the rest go into an overflow [`Menu`](super::Menu)
/// that [`Toolbar`](super::Toolbar) and [`Breadcrumb`](super::Breadcrumb) show.
#[derive(Clone, Debug)]
#[must_use]
pub struct Overflow {
    label: String,
    direction: OverflowDirection,
    min_visible: usize,
    padding: f32,
}

impl Overflow {
    /// `label` names the overflow menu button for assistive technology.
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            direction: OverflowDirection::End,
            min_visible: 0,
            padding: 0.0,
        }
    }

    /// Default [`OverflowDirection::End`].
    pub fn direction(mut self, direction: OverflowDirection) -> Self {
        self.direction = direction;
        self
    }

    /// Items that never overflow; default 0.
    pub fn min_visible(mut self, min_visible: usize) -> Self {
        self.min_visible = min_visible;
        self
    }

    /// Main-axis space kept free beside the items; default 0.
    #[track_caller]
    pub fn padding(mut self, padding: f32) -> Self {
        self.padding = egui_mineral::kit::sanitize::at_least(padding, 0.0.., "Overflow padding");
        self
    }

    pub(crate) fn label(&self) -> &str {
        &self.label
    }

    pub(crate) const fn at_start(&self) -> bool {
        matches!(self.direction, OverflowDirection::Start)
    }

    /// Which `items` stay visible in `space`. Items hide by ascending priority, from the configured end, until
    /// the rest fit or `min_visible` remain.
    #[must_use]
    pub fn fit(&self, items: &[OverflowItem], space: OverflowSpace) -> Vec<bool> {
        let OverflowSpace { available, menu } = space;
        let total: f32 = items.iter().map(|i| i.width).sum();
        let mut visible = vec![true; items.len()];
        if total <= available {
            return visible;
        }
        let budget = available - menu - self.padding;
        let mut order: Vec<usize> = (0..items.len()).collect();
        let start = self.at_start();
        order.sort_by_key(|i| {
            let priority = items.get(*i).map_or(0, |it| it.priority);
            (priority, if start { *i } else { usize::MAX - *i })
        });
        let (mut used, mut shown) = (total, items.len());
        for i in order {
            if used <= budget || shown <= self.min_visible {
                break;
            }
            if let (Some(v), Some(item)) = (visible.get_mut(i), items.get(i)) {
                *v = false;
                used -= item.width;
                shown -= 1;
            }
        }
        visible
    }
}

// useOverflowStyles.styles.ts `overflowMenu`: the menu button never shrinks.
pub(crate) fn trigger(salt: KeyedSalt, label: &str, icons: &crate::IconSet) -> PushButton<'static> {
    PushButton::new(salt, label)
        .appearance(crate::actions::ButtonAppearance::Subtle)
        .icon(icons.more_horizontal)
        .label_visible(false)
}

// Open state of the overflow menu owned by the widget `owner`.
fn open_id(owner: Id) -> Id {
    owner.with("mineral::overflow-open")
}

pub(crate) fn is_open(ctx: &Context, owner: Id) -> bool {
    ctx.data(|d| d.get_temp::<bool>(open_id(owner))).unwrap_or(false)
}

pub(crate) fn set_open(ctx: &Context, owner: Id, open: bool) {
    ctx.data_mut(|d| d.insert_temp(open_id(owner), open));
}

#[cfg(test)]
mod tests {
    use super::*;

    const fn space(available: f32) -> OverflowSpace {
        OverflowSpace { available, menu: 32.0 }
    }

    fn row(widths: &[f32]) -> Vec<OverflowItem> {
        widths.iter().copied().map(OverflowItem::new).collect()
    }

    #[test]
    fn everything_fits_without_the_menu() {
        let o = Overflow::new("More");
        assert_eq!(o.fit(&row(&[40.0, 40.0]), space(80.0)), [true, true]);
    }

    #[test]
    fn trailing_items_hide_first_and_leave_room_for_the_menu() {
        let o = Overflow::new("More");
        assert_eq!(o.fit(&row(&[40.0, 40.0, 40.0]), space(100.0)), [true, false, false]);
        assert_eq!(o.fit(&row(&[40.0, 40.0, 40.0]), space(112.0)), [true, true, false]);
    }

    #[test]
    fn start_direction_hides_leading_items() {
        let o = Overflow::new("More").direction(OverflowDirection::Start);
        assert_eq!(o.fit(&row(&[40.0, 40.0, 40.0]), space(112.0)), [false, true, true]);
    }

    #[test]
    fn lower_priority_hides_before_position() {
        let o = Overflow::new("More");
        let mut items = row(&[40.0, 40.0, 40.0]);
        items[0].priority = -1;
        assert_eq!(o.fit(&items, space(112.0)), [false, true, true]);
    }

    #[test]
    fn min_visible_items_stay() {
        let o = Overflow::new("More").min_visible(2);
        assert_eq!(o.fit(&row(&[40.0, 40.0, 40.0]), space(10.0)), [true, true, false]);
    }
}
