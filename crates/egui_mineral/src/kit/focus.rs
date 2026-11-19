//! Focus plumbing: press-to-focus, arrow locks, roving group memory.

use egui::{Context, EventFilter, Id};

use crate::{focus::tab_stop_key, layout::Axis};

/// Focuses `response` on a pointer press or click this pass, as browsers do for focusable elements (egui
/// itself only focuses text edits on click); `true` when focus moved. Focusable senses get it from
/// [`super::input::interact`] and [`crate::widget::MineralUi::mineral`]; roving items built with
/// [`crate::focus::roving_sense`] call it themselves, since [`crate::focus::TabStop::Skip`] is not
/// focusable. Keeps the group's Tab stop on the clicked item: "When a component is clicked/tapped, authors
/// should take the same steps to set the correct tabindex"
/// (<https://www.w3.org/WAI/ARIA/apg/practices/keyboard-interface/>).
pub fn follow_press(response: &egui::Response) -> bool {
    let ctx = &response.ctx;
    // A press released in the same pass clears `is_pointer_button_down_on` but still clicks.
    let press = (response.is_pointer_button_down_on() && ctx.input(|i| i.pointer.any_pressed())) || response.clicked();
    let moved = press && !response.has_focus();
    if moved {
        ctx.memory_mut(|m| m.request_focus(response.id));
    }
    moved
}

/// Arrow keys a focused widget keeps from egui's focus navigation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Arrows {
    /// Arrows along one axis only.
    Axis(Axis),
    /// All four arrows (grids).
    Both,
}

/// Keeps egui from moving focus on the given arrow keys while `id` has focus.
pub fn lock_arrows(ctx: &Context, id: Id, arrows: Arrows) {
    let (horizontal_arrows, vertical_arrows) = match arrows {
        Arrows::Axis(Axis::Horizontal) => (true, false),
        Arrows::Axis(Axis::Vertical) => (false, true),
        Arrows::Both => (true, true),
    };
    ctx.memory_mut(|m| {
        m.set_focus_lock_filter(
            id,
            EventFilter {
                horizontal_arrows,
                vertical_arrows,
                ..EventFilter::default()
            },
        );
    });
}

/// Records the focused one of `items` (item index, widget id) as the group's last focused item for
/// [`crate::focus::tab_stop`].
pub fn remember(ctx: &Context, group: Id, items: impl IntoIterator<Item = (usize, Id)>) {
    if let Some(item) = items.into_iter().find(|(_, id)| ctx.memory(|m| m.has_focus(*id))) {
        ctx.data_mut(|d| d.insert_temp(tab_stop_key(group), item));
    }
}
