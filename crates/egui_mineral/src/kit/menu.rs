//! Cascading menu model: the open submenu path, the hover intent that moves it, the arrow-key walk and the
//! focus a level takes on opening. Kits paint the levels and report rows to it.
//!
//! Level 0 is the root panel; `path[l]` is the row of level `l` whose submenu is open as level `l + 1`.

use std::time::Duration;

use egui::{Context, Key, Modifiers};

use crate::{i18n::Direction, input::HoverIntent, kit::state::TempState, layout::Edge};

/// Item a level focuses when it opens.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FocusTarget {
    /// Selected item, else the first enabled one.
    SelectedOrFirst,
    /// Last enabled item.
    Last,
}

/// Outcome of [`MenuNav::walk`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Walk {
    /// Focused parent row opened its submenu, which takes focus.
    Opened,
    /// Level closed; focus belongs on `parent`, its row in the level below.
    Closed {
        /// Row of the level below that opened the closed level.
        parent: usize,
    },
    /// Arrow toward `Edge` that leaves the menu: either arrow on the root level, or the end-ward one on a row
    /// without a submenu; a menu bar moves to the adjacent menu.
    Bar(Edge),
}

/// Open path, hover intent and pending focus of one menu, in temp memory keyed by the kit.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct MenuNav {
    path: Vec<usize>,
    pending: HoverIntent<(usize, Option<usize>)>,
    focus: Option<(usize, FocusTarget)>,
}

impl TempState for MenuNav {}

impl MenuNav {
    /// Root level only, nothing pending; the root takes `focus`.
    pub fn reset(&mut self, focus: Option<FocusTarget>) {
        self.path.clear();
        self.pending.cancel();
        self.focus = focus.map(|f| (0, f));
    }

    /// Row of `level` whose submenu is open.
    pub fn open_child(&self, level: usize) -> Option<usize> {
        self.path.get(level).copied()
    }

    /// Open submenu rows, from the root.
    pub fn path(&self) -> &[usize] {
        &self.path
    }

    /// Opens the submenu of `row` on `level`, closing deeper ones; `focus` moves focus into it.
    pub fn open(&mut self, ctx: &Context, level: usize, row: usize, focus: bool) {
        self.path.truncate(level);
        self.path.push(row);
        self.pending.cancel();
        if focus {
            self.focus = Some((level + 1, FocusTarget::SelectedOrFirst));
        }
        crate::motion::request_frame(ctx);
    }

    /// Closes the submenu open from `level` and every deeper one.
    pub fn close_from(&mut self, level: usize) {
        self.path.truncate(level);
        self.pending.cancel();
    }

    /// Closes `level` (1 or deeper); the row of the level below that opened it.
    pub fn close_level(&mut self, level: usize) -> Option<usize> {
        let parent = level.checked_sub(1).and_then(|p| self.open_child(p));
        self.close_from(level.saturating_sub(1));
        parent
    }

    /// Pointer over `row` of `level` this pass: after the delay its submenu opens (`submenu`) or the open sibling
    /// closes. `toward_child`: the pointer heads into the open submenu, which keeps it. Acts only while the pointer moves.
    pub fn hover(&mut self, ctx: &Context, level: usize, row: usize, submenu: bool, toward_child: bool) {
        if ctx.input(|i| i.pointer.delta() == egui::Vec2::ZERO) {
            return;
        }
        let want = submenu.then_some(row);
        if want == self.open_child(level) {
            self.pending.cancel();
        } else if !toward_child {
            self.pending.request(ctx, (level, want));
        }
    }

    /// Applies the hover intent once it waited `delay`; true when the path changed.
    pub fn settle_hover(&mut self, ctx: &Context, delay: Duration) -> bool {
        let Some((level, want)) = self.pending.due(ctx, delay) else {
            return false;
        };
        self.path.truncate(level);
        self.path.extend(want);
        crate::motion::request_frame(ctx);
        true
    }

    /// Asks `level` to focus `target` when it next shows.
    pub fn request_focus(&mut self, level: usize, target: FocusTarget) {
        self.focus = Some((level, target));
    }

    /// Focus `level` should take this pass, once.
    pub fn take_focus(&mut self, level: usize) -> Option<FocusTarget> {
        self.focus
            .filter(|(l, _)| *l == level)
            .map(|(_, f)| f)
            .inspect(|_| self.focus = None)
    }

    /// Focus waiting for a level.
    pub fn focus_pending(&self) -> bool {
        self.focus.is_some()
    }

    /// <https://www.w3.org/WAI/ARIA/apg/patterns/menubar/> arrows on `row` of `level`, which holds focus. The
    /// end-ward arrow opens a parent row's submenu and reports [`Walk::Bar`] on any other row; the start-ward one
    /// closes a submenu level and reports [`Walk::Bar`] on the root level. Consumes the keys it acts on.
    pub fn walk(&mut self, ctx: &Context, dir: Direction, level: usize, row: usize, parent: bool) -> Option<Walk> {
        let fwd = dir.forward_key();
        let back = if fwd == Key::ArrowRight {
            Key::ArrowLeft
        } else {
            Key::ArrowRight
        };
        let walk = step(level, parent, |k| {
            let key = if k == Edge::End { fwd } else { back };
            ctx.input_mut(|i| i.consume_key(Modifiers::NONE, key))
        })?;
        match walk {
            Walk::Opened => self.open(ctx, level, row, true),
            Walk::Closed { .. } => {
                let parent = self.close_level(level)?;
                crate::motion::request_frame(ctx);
                return Some(Walk::Closed { parent });
            }
            Walk::Bar(_) => {}
        }
        Some(walk)
    }
}

// `pressed(edge)` consumes the arrow toward `edge` when it was pressed.
fn step(level: usize, parent: bool, mut pressed: impl FnMut(Edge) -> bool) -> Option<Walk> {
    match (level, parent) {
        (_, true) if pressed(Edge::End) => Some(Walk::Opened),
        _ if pressed(Edge::End) => Some(Walk::Bar(Edge::End)),
        (0, _) if pressed(Edge::Start) => Some(Walk::Bar(Edge::Start)),
        (0, _) => None,
        _ if pressed(Edge::Start) => Some(Walk::Closed { parent: 0 }),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{FocusTarget, MenuNav, Walk, step};
    use crate::layout::Edge;

    #[test]
    fn arrows_open_close_and_leave_the_root() {
        let only = |e: Edge| move |k: Edge| k == e;
        assert_eq!(step(1, true, only(Edge::End)), Some(Walk::Opened));
        assert_eq!(step(0, true, only(Edge::End)), Some(Walk::Opened));
        assert_eq!(step(0, false, only(Edge::End)), Some(Walk::Bar(Edge::End)));
        assert_eq!(step(0, true, only(Edge::Start)), Some(Walk::Bar(Edge::Start)));
        assert_eq!(step(2, false, only(Edge::Start)), Some(Walk::Closed { parent: 0 }));
        assert_eq!(
            step(2, false, only(Edge::End)),
            Some(Walk::Bar(Edge::End)),
            "a leaf row leaves for the next menu"
        );
    }

    #[test]
    fn path_opens_closes_and_takes_focus_once() {
        let ctx = egui::Context::default();
        let mut nav = MenuNav::default();
        nav.open(&ctx, 0, 3, true);
        nav.open(&ctx, 1, 2, false);
        assert_eq!(nav.path(), [3, 2]);
        assert_eq!(nav.take_focus(0), None);
        assert_eq!(nav.take_focus(1), Some(FocusTarget::SelectedOrFirst));
        assert_eq!(nav.take_focus(1), None);
        assert_eq!(nav.close_level(2), Some(2));
        assert_eq!(nav.path(), [3]);
        nav.open(&ctx, 0, 1, false);
        assert_eq!(nav.path(), [1]);
        nav.reset(Some(FocusTarget::Last));
        assert_eq!(nav.path(), [0_usize; 0]);
        assert_eq!(nav.take_focus(0), Some(FocusTarget::Last));
    }
}
