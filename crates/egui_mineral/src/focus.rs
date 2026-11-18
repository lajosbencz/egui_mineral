//! <https://www.w3.org/WAI/ARIA/apg/practices/keyboard-interface>

use std::time::Duration;

use egui::{Context, Event, Id, Key, Modifiers, Sense, TouchPhase};

use crate::{
    i18n::Direction,
    kit::{
        focus::{Arrows, lock_arrows, remember},
        input::plain_key_presses,
    },
    layout::Axis,
};

/// Input that last moved or could move focus; drives [`visible`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Modality {
    /// Key press without Ctrl, Alt or Command; also the state before any input.
    #[default]
    Keyboard,
    /// Pointer button press or touch start.
    Pointer,
}

const MODALITY: &str = "mineral::focus_modality";

pub(crate) fn track_modality(ctx: &Context) {
    let latest = ctx.input(|i| {
        i.events.iter().rev().find_map(|e| match e {
            Event::PointerButton { pressed: true, .. }
            | Event::Touch {
                phase: TouchPhase::Start,
                ..
            } => Some(Modality::Pointer),
            Event::Key {
                pressed: true,
                modifiers,
                ..
            } if !(modifiers.ctrl || modifiers.alt || modifiers.command || modifiers.mac_cmd) => {
                Some(Modality::Keyboard)
            }
            _ => None,
        })
    });
    if let Some(m) = latest {
        crate::setting::set(ctx, MODALITY, m);
    }
}

/// Last [`Modality`] seen by [`Runtime::pass`](crate::runtime::Runtime::pass).
pub fn modality(ctx: &Context) -> Modality {
    crate::setting::get(ctx, MODALITY)
}

/// `id` has focus and it should be indicated, per the `:focus-visible` heuristics: focus reached by
/// keyboard (or set by code after keyboard input), or a focused text edit.
/// <https://drafts.csswg.org/selectors/#the-focus-visible-pseudo>
pub fn visible(ctx: &Context, id: Id) -> bool {
    ctx.memory(|m| m.has_focus(id)) && (modality(ctx) == Modality::Keyboard || ctx.text_edit_focused())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Orientation {
    Horizontal,
    Vertical,
    Grid { columns: usize },
}

/// APG roving tabindex model over a linear or grid item set.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RovingFocus {
    orientation: Orientation,
    wrap: bool,
}

impl RovingFocus {
    /// Linear group along `axis`; wraps at the ends by default.
    pub fn new(axis: Axis) -> Self {
        Self {
            orientation: match axis {
                Axis::Horizontal => Orientation::Horizontal,
                Axis::Vertical => Orientation::Vertical,
            },
            wrap: true,
        }
    }

    /// Row-major grid: left/right step by one, up/down by `columns`; 0 columns count as 1.
    pub fn grid(columns: usize) -> Self {
        Self {
            orientation: Orientation::Grid {
                columns: crate::kit::sanitize::at_least(columns, 1.., "RovingFocus grid columns"),
            },
            wrap: true,
        }
    }

    /// Wrap from last to first and back; grids never wrap.
    #[must_use]
    pub fn wrap(mut self, wrap: bool) -> Self {
        self.wrap = wrap;
        self
    }

    fn step(&self, key: Key, dir: Direction, current: usize, len: usize) -> Option<usize> {
        if len == 0 {
            return None;
        }
        let last = len - 1;
        // A stored index can outlive a shrunk list.
        let current = current.min(last);
        let fwd = dir.forward_key();
        let horizontal = |k: Key| match k {
            Key::ArrowLeft | Key::ArrowRight => Some((k == fwd, 1)),
            _ => None,
        };
        let vertical = |k: Key| match k {
            Key::ArrowDown => Some((true, 1)),
            Key::ArrowUp => Some((false, 1)),
            _ => None,
        };
        let (forward, n) = match (key, self.orientation) {
            (Key::Home, _) => return Some(0),
            (Key::End, _) => return Some(last),
            (k, Orientation::Horizontal) => horizontal(k)?,
            (k, Orientation::Vertical) => vertical(k)?,
            (k, Orientation::Grid { columns }) => horizontal(k).or_else(|| vertical(k).map(|(f, _)| (f, columns)))?,
        };
        Some(match self.orientation {
            Orientation::Grid { .. } => {
                let target = if forward {
                    current.checked_add(n)
                } else {
                    current.checked_sub(n)
                };
                return target.filter(|t| *t < len);
            }
            _ if self.wrap && forward => (current + 1) % len,
            _ if self.wrap => current.checked_sub(1).unwrap_or(last),
            _ if forward => (current + 1).min(last),
            _ => current.saturating_sub(1),
        })
    }

    /// Moves focus within `ids` on arrow keys, skipping items where `enabled` is false.
    pub fn update(&self, core: &crate::runtime::Core, ids: &[Id], enabled: impl Fn(usize) -> bool) -> Option<usize> {
        let ctx = core.ctx();
        let focused = ctx.memory(egui::Memory::focused)?;
        let current = ids.iter().position(|id| *id == focused)?;
        let idx = self.on_key(core, focused, current, ids.len(), enabled)?;
        ctx.memory_mut(|m| m.request_focus(ids[idx]));
        Some(idx)
    }

    /// Index this pass's arrow keys move the item `current` of `len` to, skipping items where
    /// `enabled` is false; `None` when it stays. Keeps egui's focus navigation off the arrows while
    /// `focused`, the item's widget, has focus. Moving focus is the caller's.
    pub fn on_key(
        &self,
        core: &crate::runtime::Core,
        focused: Id,
        current: usize,
        len: usize,
        enabled: impl Fn(usize) -> bool,
    ) -> Option<usize> {
        let ctx = core.ctx();
        let arrows = match self.orientation {
            Orientation::Horizontal => Arrows::Axis(Axis::Horizontal),
            Orientation::Vertical => Arrows::Axis(Axis::Vertical),
            Orientation::Grid { .. } => Arrows::Both,
        };
        lock_arrows(ctx, focused, arrows);
        let dir = core.i18n().direction();
        let mut idx = current;
        for key in plain_key_presses(ctx) {
            if let Some(next) = self.target(key, dir, idx, len, &enabled) {
                idx = next;
            }
        }
        (idx != current).then_some(idx)
    }

    /// Next index for `key`, stepping past disabled items; `None` when no enabled item is reachable.
    pub fn target(
        &self,
        key: Key,
        dir: Direction,
        current: usize,
        len: usize,
        enabled: impl Fn(usize) -> bool,
    ) -> Option<usize> {
        let mut cand = self.step(key, dir, current, len)?;
        match key {
            Key::Home => return (0..len).find(|i| enabled(*i)),
            Key::End => return (0..len).rev().find(|i| enabled(*i)),
            _ => {}
        }
        for _ in 0..len {
            if enabled(cand) {
                return Some(cand);
            }
            let next = self.step(key, dir, cand, len)?;
            if next == cand {
                return None;
            }
            cand = next;
        }
        None
    }
}

impl RovingFocus {
    /// [`Self::update`], then remembers `ids` so [`tab_stop`] can return to the last focused item.
    pub fn update_group(
        &self,
        core: &crate::runtime::Core,
        group: Id,
        ids: &[Id],
        enabled: impl Fn(usize) -> bool,
    ) -> Option<usize> {
        let ctx = core.ctx();
        let moved = self.update(core, ids, enabled);
        remember(ctx, group, ids.iter().copied().enumerate());
        moved
    }
}

/// Index of the group's single Tab stop: the focused item, else `selected`, else the item focused
/// last, else 0; falls to the first enabled item when that one is disabled.
pub fn tab_stop(
    ctx: &Context,
    group: Id,
    selected: Option<usize>,
    len: usize,
    enabled: impl Fn(usize) -> bool,
) -> usize {
    let last: Option<(usize, Id)> = ctx.data(|d| d.get_temp(tab_stop_key(group)));
    let focused = last.filter(|(_, id)| ctx.memory(|m| m.has_focus(*id)));
    let stop = resolve_tab_stop(focused.map(|f| f.0), selected, last.map(|l| l.0), len);
    if len == 0 || enabled(stop) {
        stop
    } else {
        (0..len).find(|i| enabled(*i)).unwrap_or(stop)
    }
}

pub(crate) fn tab_stop_key(group: Id) -> Id {
    group.with("mineral::tab_stop")
}

fn resolve_tab_stop(focused: Option<usize>, selected: Option<usize>, last: Option<usize>, len: usize) -> usize {
    focused.or(selected).or(last).unwrap_or(0).min(len.saturating_sub(1))
}

/// Role of a composite group item in Tab order: one stop per group (APG roving tabindex).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TabStop {
    /// Focusable; takes the group's single Tab stop.
    Stop,
    /// Reached by arrows only; drops [`Sense::FOCUSABLE`].
    Skip,
}

impl TabStop {
    /// `Stop` when `item` is the group's stop (an index from [`tab_stop`], or any item key).
    pub fn at<T: PartialEq + Copy>(item: T, stop: T) -> Self {
        if item == stop { Self::Stop } else { Self::Skip }
    }
}

/// `sense` without [`Sense::FOCUSABLE`] unless the item is its group's Tab stop.
pub fn roving_sense(sense: Sense, tab_stop: TabStop) -> Sense {
    match tab_stop {
        TabStop::Stop => sense,
        TabStop::Skip => sense - Sense::FOCUSABLE,
    }
}

/// Type-to-select: printable keys within [`policy::typeahead`](crate::policy::Policies::typeahead) of each
/// other form a prefix query.
#[derive(Clone, Debug, Default)]
pub struct Typeahead {
    query: String,
    last: f64,
}

impl crate::kit::state::TempState for Typeahead {}

impl Typeahead {
    /// `time` in seconds; keys further than `timeout` apart start a new query.
    pub fn feed<'a>(
        &mut self,
        text: &str,
        time: f64,
        timeout: Duration,
        current: Option<usize>,
        labels: impl ExactSizeIterator<Item = &'a str> + Clone,
    ) -> Option<usize> {
        if time - self.last > timeout.as_secs_f64() {
            self.query.clear();
        }
        self.last = time;
        self.query.extend(text.chars().flat_map(char::to_lowercase));
        let len = labels.len();
        if len == 0 || self.query.is_empty() {
            return None;
        }
        // A repeated single char cycles through items starting with it.
        let first_len = self.query.chars().next().map_or(0, char::len_utf8);
        let repeated = self.query.chars().all(|c| self.query.starts_with(c));
        let (needle, start) = if repeated {
            (self.query.split_at(first_len).0, current.map_or(0, |c| c + 1))
        } else {
            (self.query.as_str(), current.unwrap_or(0))
        };
        let starts_with = |label: &str| {
            let mut lower = label.chars().flat_map(char::to_lowercase);
            needle.chars().all(|c| lower.next() == Some(c))
        };
        labels
            .enumerate()
            .cycle()
            .skip(start % len)
            .take(len)
            .find(|&(_, l)| starts_with(l))
            .map(|(i, _)| i)
    }

    /// Feeds this pass's text input with the [`policy::typeahead`](crate::policy::Policies::typeahead) timeout; `Some`
    /// with the matched index. Always `None` while the policy is unset.
    pub fn update<'a>(
        &mut self,
        core: &crate::runtime::Core,
        current: Option<usize>,
        labels: impl ExactSizeIterator<Item = &'a str> + Clone,
    ) -> Option<usize> {
        let ctx = core.ctx();
        let timeout = core.policies().typeahead;
        let (text, time) = ctx.input(|i| {
            let t: String = i
                .events
                .iter()
                .filter_map(|e| match e {
                    Event::Text(t) => Some(t.as_str()),
                    _ => None,
                })
                .collect();
            (t, i.time)
        });
        if text.is_empty() {
            None
        } else {
            self.feed(&text, time, timeout, current, labels)
        }
    }
}

/// Grid cell position.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Cell {
    /// Row index.
    pub row: usize,
    /// Column index within the row.
    pub column: usize,
}

/// <https://www.w3.org/WAI/ARIA/apg/patterns/grid/>; horizontal arrows flip in RTL, no wrapping.
/// Cell grid a key press moves in; an empty grid has no target.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Grid {
    rows: usize,
    columns: usize,
    jump_rows: usize,
}

/// Extent of a [`Grid`] in cells.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct GridSize {
    /// Row count.
    pub rows: usize,
    /// Column count.
    pub columns: usize,
}

impl Grid {
    /// `size` cells; PageUp/PageDown move one row until [`Self::jump_rows`] sets more.
    pub fn new(size: GridSize) -> Self {
        let GridSize { rows, columns } = size;
        Self {
            rows,
            columns,
            jump_rows: 1,
        }
    }

    /// Rows PageUp/PageDown move; 0 counts as 1.
    #[must_use]
    pub fn jump_rows(mut self, jump_rows: usize) -> Self {
        self.jump_rows = crate::kit::sanitize::at_least(jump_rows, 1.., "grid jump_rows");
        self
    }

    /// Row count.
    pub fn rows(&self) -> usize {
        self.rows
    }

    /// Cells per row.
    pub fn columns(&self) -> usize {
        self.columns
    }

    /// Rows PageUp/PageDown move; at least 1.
    pub fn jump(&self) -> usize {
        self.jump_rows
    }
}

/// APG grid navigation: target cell for `key`, `None` when the key does not navigate; a `cell` outside `grid`
/// counts as its nearest cell.
pub fn on_grid_key(grid: Grid, cell: Cell, key: Key, modifiers: Modifiers, dir: Direction) -> Option<Cell> {
    let Grid {
        rows,
        columns,
        jump_rows,
    } = grid;
    if rows == 0 || columns == 0 {
        return None;
    }
    let (r, c) = (cell.row.min(rows - 1), cell.column.min(columns - 1));
    let (last_r, last_c) = (rows - 1, columns - 1);
    let fwd = dir.forward_key();
    let (row, column) = match key {
        Key::ArrowRight | Key::ArrowLeft if key == fwd => (r, (c + 1).min(last_c)),
        Key::ArrowRight | Key::ArrowLeft => (r, c.saturating_sub(1)),
        Key::ArrowDown => ((r + 1).min(last_r), c),
        Key::ArrowUp => (r.saturating_sub(1), c),
        Key::Home if modifiers.command => (0, 0),
        Key::End if modifiers.command => (last_r, last_c),
        Key::Home => (r, 0),
        Key::End => (r, last_c),
        Key::PageDown => ((r + jump_rows).min(last_r), c),
        Key::PageUp => (r.saturating_sub(jump_rows), c),
        _ => return None,
    };
    Some(Cell { row, column })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        input::synth::{key_down, pointer_up},
        test_support::{harness, press},
    };

    #[test]
    fn press_focuses_and_only_keyboard_shows_it() {
        let rect = egui::Rect::from_min_size(egui::pos2(10.0, 10.0), egui::vec2(40.0, 20.0));
        let mut h = harness(
            |ui, s: &mut (bool, bool)| {
                let r = crate::kit::input::interact(
                    ui,
                    rect,
                    Id::unique("pressable"),
                    Sense::click(),
                    crate::widget::Availability::Enabled,
                );
                *s = (
                    r.has_focus(),
                    crate::widget::Interaction::of(&r, crate::widget::Availability::Enabled).focused,
                );
            },
            (false, false),
        );
        h.input_mut().events.extend(press(rect.center()));
        h.step();
        h.event(pointer_up(rect.center(), egui::PointerButton::Primary));
        h.step();
        assert_eq!(h.state().value, (true, false), "a press focuses without indicating it");
        h.event(key_down(Key::S, Modifiers::COMMAND));
        h.step();
        assert_eq!(h.state().value, (true, false), "shortcuts keep the pointer modality");
        h.event(key_down(Key::ArrowDown, Modifiers::NONE));
        h.step();
        assert_eq!(h.state().value, (true, true), "keyboard input shows it");
    }

    #[test]
    fn roving_wraps_flips_and_grids() {
        let h = RovingFocus::new(Axis::Horizontal);
        assert_eq!(h.target(Key::ArrowRight, Direction::Ltr, 2, 3, |_| true), Some(0));
        assert_eq!(h.target(Key::ArrowRight, Direction::Rtl, 0, 3, |_| true), Some(2));
        assert_eq!(h.target(Key::ArrowDown, Direction::Ltr, 0, 3, |_| true), None);
        let g = RovingFocus::grid(7);
        assert_eq!(g.target(Key::ArrowDown, Direction::Ltr, 3, 42, |_| true), Some(10));
        assert_eq!(g.target(Key::ArrowUp, Direction::Ltr, 3, 42, |_| true), None);
        let g3 = RovingFocus::grid(3);
        assert_eq!(g3.target(Key::ArrowDown, Direction::Ltr, 6, 9, |_| true), None);
        assert_eq!(g3.target(Key::ArrowRight, Direction::Ltr, 8, 9, |_| true), None);
    }

    #[test]
    fn roving_skips_disabled() {
        let h = RovingFocus::new(Axis::Horizontal);
        let on = |i: usize| i != 1;
        assert_eq!(h.target(Key::ArrowRight, Direction::Ltr, 0, 3, on), Some(2));
        assert_eq!(h.target(Key::ArrowLeft, Direction::Ltr, 2, 3, on), Some(0));
        assert_eq!(h.target(Key::End, Direction::Ltr, 0, 3, |i| i < 2), Some(1));
        let nowrap = h.wrap(false);
        assert_eq!(nowrap.target(Key::ArrowRight, Direction::Ltr, 2, 3, |i| i != 2), None);
    }

    #[test]
    fn roving_clamps_a_stale_index_into_the_list() {
        let enabled = [true; 3];
        for wrap in [true, false] {
            let h = RovingFocus::new(Axis::Horizontal).wrap(wrap);
            assert_eq!(h.target(Key::ArrowLeft, Direction::Ltr, 9, 3, |i| enabled[i]), Some(1));
        }
    }

    #[test]
    fn tab_stop_precedence() {
        assert_eq!(resolve_tab_stop(Some(2), Some(1), Some(0), 3), 2);
        assert_eq!(resolve_tab_stop(None, Some(1), Some(0), 3), 1);
        assert_eq!(resolve_tab_stop(None, None, Some(5), 3), 2);
        assert_eq!(resolve_tab_stop(None, None, None, 0), 0);
        assert!(!roving_sense(Sense::click(), TabStop::Skip).is_focusable());
        assert!(roving_sense(Sense::click(), TabStop::Skip).senses_click());
    }

    #[test]
    fn typeahead_prefix_and_cycle() {
        let labels = ["Apple", "Banana", "Blueberry", "Cherry"];
        let timeout = Duration::from_secs(1);
        let mut t = Typeahead::default();
        assert_eq!(t.feed("b", 0.0, timeout, None, labels.iter().copied()), Some(1));
        assert_eq!(t.feed("l", 0.5, timeout, Some(1), labels.iter().copied()), Some(2));
        let mut t = Typeahead::default();
        assert_eq!(t.feed("b", 0.0, timeout, Some(1), labels.iter().copied()), Some(2));
        assert_eq!(t.feed("b", 0.2, timeout, Some(2), labels.iter().copied()), Some(1));
    }

    #[test]
    fn typeahead_jumps_to_the_typed_prefix() {
        let labels = ["Apple", "Banana"];
        let ctx = crate::test_support::ctx();
        let rt = crate::test_support::runtime(&ctx);
        let mut t = Typeahead::default();
        let mut got = None;
        crate::test_support::run(&ctx, crate::test_support::input(vec![Event::Text("b".into())]), |_| {
            got = t.update(rt.core(), None, labels.iter().copied());
        });
        assert_eq!(got, Some(1));
    }

    #[test]
    fn zero_grid_counts_clamp_to_one() {
        assert_eq!(RovingFocus::grid(0), RovingFocus::grid(1));
        let grid = Grid::new(GridSize { rows: 4, columns: 2 }).jump_rows(0);
        assert_eq!(grid.jump(), 1);
        assert_eq!(
            on_grid_key(grid, cell(0, 0), Key::PageDown, Modifiers::NONE, Direction::Ltr),
            Some(cell(1, 0))
        );
    }

    fn cell(row: usize, column: usize) -> Cell {
        Cell { row, column }
    }

    #[test]
    fn grid_keys() {
        let grid = Grid::new(GridSize { rows: 10, columns: 4 }).jump_rows(5);
        let k = |at, code, m, dir| on_grid_key(grid, at, code, m, dir);
        assert_eq!(
            k(cell(0, 0), Key::ArrowRight, Modifiers::NONE, Direction::Ltr),
            Some(cell(0, 1))
        );
        assert_eq!(
            k(cell(0, 0), Key::ArrowRight, Modifiers::NONE, Direction::Rtl),
            Some(cell(0, 0))
        );
        assert_eq!(
            k(cell(0, 3), Key::ArrowRight, Modifiers::NONE, Direction::Ltr),
            Some(cell(0, 3))
        );
        assert_eq!(
            k(cell(2, 2), Key::Home, Modifiers::NONE, Direction::Ltr),
            Some(cell(2, 0))
        );
        assert_eq!(
            k(cell(2, 2), Key::End, Modifiers::COMMAND, Direction::Ltr),
            Some(cell(9, 3))
        );
        assert_eq!(
            k(cell(8, 1), Key::PageDown, Modifiers::NONE, Direction::Ltr),
            Some(cell(9, 1))
        );
        assert_eq!(k(cell(0, 0), Key::A, Modifiers::NONE, Direction::Ltr), None);
    }
}
