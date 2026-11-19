//! Menu bar model: which menu is open and the F10 / lone Alt activation of the bar.

use egui::{Id, Key, Memory, Modifiers};

use crate::{kit::state::TempState, runtime::Core, shortcut};

/// Menu bar state in temp memory, keyed by the bar id.
#[derive(Clone, Copy, Default)]
pub struct BarState {
    /// Open menu.
    pub open: Option<usize>,
    /// Mnemonic underlines are shown.
    pub revealed: bool,
    /// Set by F10 or a lone Alt until focus leaves the bar.
    pub active: bool,
    /// Focus before the bar was activated.
    pub return_to: Option<Id>,
}

impl TempState for BarState {}

impl BarState {
    /// Closes the open menu and hides the mnemonics.
    pub fn close(&mut self, ctx: &egui::Context) {
        self.open = None;
        self.active = false;
        self.revealed = false;
        shortcut::set_mnemonics_revealed(ctx, false);
    }

    /// Windows and Linux menu bar toggle: F10 or a lone Alt focuses the trigger `stop` of `ids`, again returns focus.
    pub fn toggle_activation(&mut self, core: &Core, disabled: bool, ids: &[Id], stop: usize) {
        let ctx = core.ctx();
        if !core.policies().mnemonics || disabled {
            return;
        }
        let f10 = ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::F10));
        if !f10 && !shortcut::lone_alt(core) {
            return;
        }
        let focused = ctx.memory(Memory::focused);
        if self.open.is_some() || focused.is_some_and(|f| ids.contains(&f)) {
            self.close(ctx);
            match self.return_to.take() {
                Some(id) => ctx.memory_mut(|m| m.request_focus(id)),
                None => ctx.memory_mut(|m| focused.into_iter().for_each(|f| m.surrender_focus(f))),
            }
        } else if let Some(&first) = ids.get(stop) {
            self.return_to = focused;
            self.active = true;
            self.revealed = true;
            shortcut::set_mnemonics_revealed(ctx, true);
            ctx.memory_mut(|m| m.request_focus(first));
        }
    }

    /// Clears the activation once nothing is open and focus left the bar `ids`, hiding the mnemonics.
    pub fn settle(&mut self, ctx: &egui::Context, ids: &[Id]) {
        if self.open.is_none() && !ctx.memory(|m| m.focused().is_some_and(|f| ids.contains(&f))) {
            self.active = false;
            self.return_to = None;
        }
        if self.open.is_none() && self.revealed && !self.active {
            self.revealed = false;
            shortcut::set_mnemonics_revealed(ctx, false);
        }
    }
}

/// The menu after `open` for an arrow `nav` of -1 or 1, wrapping at both ends of `count` menus.
pub fn adjacent(open: usize, nav: i8, count: usize) -> usize {
    open.checked_add_signed(isize::from(nav))
        .map_or(count - 1, |j| j % count)
}

#[cfg(test)]
mod tests {
    use super::adjacent;

    #[test]
    fn arrows_wrap_at_both_ends() {
        assert_eq!(adjacent(0, -1, 3), 2);
        assert_eq!(adjacent(2, 1, 3), 0);
        assert_eq!(adjacent(1, 1, 3), 2);
    }
}
