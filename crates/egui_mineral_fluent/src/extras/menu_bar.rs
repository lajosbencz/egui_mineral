use egui::{Context, Id, Key, Modifiers, Response, Vec2};
use egui_mineral::{
    focus::{self, RovingFocus, TabStop},
    kit::{
        self,
        menubar::{self, BarState},
        state::TempState,
    },
    layout::Axis,
    overlay::Dismiss,
    semantic::{self, KeyedSalt, Role, SemanticNode, StableId},
    widget::MineralUi,
};

use crate::{
    Fluent, Size,
    actions::PushButton,
    navigation::{Menu, MenuUi},
    widget::vocab::ButtonAppearance,
};

// Not in Fluent UI React v9: a desktop menu bar of subtle buttons opening Fluent `Menu`s.
const BAR_H: f32 = 32.0;

type Rows<'a> = Box<dyn FnOnce(&mut MenuUi<'_, '_>) + 'a>;

/// Top-level entry of a [`MenuBar`]: the bar title and the rows of its [`Menu`].
#[must_use]
pub struct BarMenu<'a> {
    title: String,
    mnemonic: Option<char>,
    disabled: bool,
    rows: Rows<'a>,
}

kit::opaque_debug!(BarMenu<'_>);

impl<'a> BarMenu<'a> {
    /// `title` names the bar button and the menu; `rows` places the [`MenuUi`] rows while the menu is open.
    pub fn new(title: impl Into<String>, rows: impl FnOnce(&mut MenuUi<'_, '_>) + 'a) -> Self {
        Self {
            title: title.into(),
            mnemonic: None,
            disabled: false,
            rows: Box::new(rows),
        }
    }

    /// Alt+letter opens the menu; the letter is underlined while mnemonics are visible.
    pub fn mnemonic(mut self, mnemonic: char) -> Self {
        self.mnemonic = Some(mnemonic);
        self
    }

    crate::widget::setters! {
        /// Not operable.
        disabled
    }
}

/// Result of [`MenuBar::show`].
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct MenuBarResponse {
    /// Bar response.
    pub response: Response,
    /// Open menu surface; `None` while all menus are closed.
    pub popup: Option<Response>,
    /// Index of the open menu.
    pub open: Option<usize>,
}

/// Horizontal menu bar over [`BarMenu`]s with APG menubar keyboard behavior; not in Fluent UI React v9.
/// Row clicks are read from the [`MenuUi`] responses inside the rows.
#[must_use = "call .show(..)"]
pub struct MenuBar<'a> {
    salt: KeyedSalt,
    label: Option<String>,
    disabled: bool,
    menus: Vec<BarMenu<'a>>,
}

kit::opaque_debug!(MenuBar<'_>);

impl<'a> MenuBar<'a> {
    /// Horizontal row of menu titles; agent key prefix `<id_salt>`.
    pub fn new(id_salt: impl Into<KeyedSalt>) -> Self {
        Self {
            salt: id_salt.into(),
            label: None,
            disabled: false,
            menus: Vec::new(),
        }
    }

    /// Replaces the `<id_salt>` key prefix.
    pub fn agent_key(mut self, salt: impl Into<KeyedSalt>) -> Self {
        self.salt = salt.into();
        self
    }

    /// Accessible name of the bar; default `mineral-menu-strip`.
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    crate::widget::setters! {
        /// Disables every menu.
        disabled
    }

    /// Appends a menu.
    pub fn menu(mut self, menu: BarMenu<'a>) -> Self {
        self.menus.push(menu);
        self
    }

    /// Paints the bar and the open menu.
    pub fn show(self, ui: &mut MineralUi<'_, Fluent>) -> MenuBarResponse {
        let rt = ui.rt();
        let ctx = ui.ctx().clone();
        let dir = rt.i18n().direction();
        let id = ui.make_persistent_id(self.salt.id_salt());
        let mut bar = BarState::load(&ctx, id);
        let disabled: Vec<bool> = self.menus.iter().map(|m| m.disabled || self.disabled).collect();
        let enabled = |i: usize| disabled.get(i).is_some_and(|d| !d);
        let stop = focus::tab_stop(&ctx, id, bar.open, self.menus.len(), enabled);
        if bar.open.is_some_and(|o| !enabled(o)) {
            bar.open = None;
        }
        let builder = crate::widget::row(ui, dir, BAR_H);
        let mut triggers: Vec<Response> = Vec::with_capacity(self.menus.len());
        let row = ui.scope_builder(builder, |ui| {
            ui.spacing_mut().item_spacing = Vec2::ZERO;
            for (i, menu) in self.menus.iter().enumerate() {
                let open = bar.open == Some(i);
                let mut button = PushButton::new(self.salt.child(("menu", i)), &menu.title)
                    .appearance(if open {
                        ButtonAppearance::Secondary
                    } else {
                        ButtonAppearance::Subtle
                    })
                    .size(Size::Medium)
                    .disabled(disabled[i])
                    .agent_key(StableId::of(&self.salt, (i, "menubar-item")));
                if let Some(m) = menu.mnemonic {
                    button = button.mnemonic(m);
                }
                button.tab_stop = Some(TabStop::at(i, stop));
                triggers.push(rt.ui(ui).mineral(button).response().clone());
            }
        });
        let ids: Vec<Id> = triggers.iter().map(|t| t.id).collect();
        let pointer_moved = ctx.input(|i| i.pointer.delta() != Vec2::ZERO);
        for (i, t) in triggers.iter().enumerate() {
            if !enabled(i) {
                continue;
            }
            let switched = bar.open.is_some_and(|o| o != i) && t.hovered() && pointer_moved;
            if (t.clicked() && bar.open != Some(i)) || switched {
                bar.open = Some(i);
            } else if t.has_focus() && bar.open.is_none() {
                for key in kit::input::plain_key_presses(&ctx) {
                    if matches!(key, Key::ArrowDown | Key::ArrowUp) {
                        bar.open = Some(i);
                    }
                }
            }
        }
        bar.toggle_activation(rt.core(), self.disabled, &ids, stop);
        if bar.open.is_none() {
            RovingFocus::new(Axis::Horizontal).update_group(rt.core(), id, &ids, enabled);
        }
        let mut popup = None;
        let count = self.menus.len();
        if let Some(o) = bar.open {
            let trigger = triggers[o].clone();
            let mut open = !triggers[o].clicked();
            let menu = self.menus.into_iter().nth(o);
            if let Some(menu) = menu {
                let out =
                    Menu::new(self.salt.child(("popup", o)), &menu.title).show(rt, &trigger, &mut open, menu.rows);
                popup = out.response;
                if out.dismissed == Some(Dismiss::Escape) {
                    menubar::refocus_title(&ctx, id, o, trigger.id);
                }
                if !open {
                    bar.open = None;
                } else if let Some(nav) = arrow(&ctx, dir) {
                    let next = menubar::adjacent(o, nav, count);
                    bar.open = Some(next);
                }
            }
        }
        bar.settle(&ctx, &ids);
        semantic::emit(&ctx, Some(row.response.id), row.response.rect, || {
            let name = crate::tr_or(rt.i18n(), self.label.as_deref(), "mineral-menu-strip");
            let mut n = SemanticNode::new(StableId::of(&self.salt, "menubar"), Role::MenuBar, name);
            n.state.disabled = self.disabled;
            n
        });
        let open = bar.open;
        bar.store(&ctx, id);
        MenuBarResponse {
            response: row.response,
            popup,
            open,
        }
    }
}

// An end-to-start or start-to-end arrow no row consumed: -1 or 1.
fn arrow(ctx: &Context, dir: egui_mineral::i18n::Direction) -> Option<i8> {
    let (forward, back) = dir.pick((Key::ArrowRight, Key::ArrowLeft), (Key::ArrowLeft, Key::ArrowRight));
    ctx.input_mut(|i| {
        if i.consume_key(Modifiers::NONE, forward) {
            Some(1)
        } else if i.consume_key(Modifiers::NONE, back) {
            Some(-1)
        } else {
            None
        }
    })
}
