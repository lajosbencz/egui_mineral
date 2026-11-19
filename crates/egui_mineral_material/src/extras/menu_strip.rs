use crate::Material;
use crate::MaterialExt;
use egui::{Context, CornerRadius, Id, Key, Rangef, Rect, Response, Sense, pos2, vec2};
use egui_mineral::kit::{
    menubar::{self, BarState},
    state::TempState,
};
use egui_mineral::semantic::KeyedSalt;
use egui_mineral::widget::Availability;
use egui_mineral::widget::MineralUi;
use egui_mineral::{
    anchor::{Align, Side},
    focus::{self, RovingFocus},
    kit,
    layout::Axis,
    semantic::{self, Role, SemanticNode, StableId},
    shortcut::{self, Scope, Trigger},
};

use crate::spec::TypeScale;
use crate::widget::common::{self, Interaction};
use crate::widget::{indication, parts};

use crate::selection::{
    Menu,
    menu::{FocusTarget, MenuState, open_focus},
};

// Desktop menu bar, non-Compose
const BAR_H: f32 = 40.0;
const BAR_INSET: f32 = 4.0;

/// Item of a [`MenuStrip`]: menu index on the bar, item index within that [`Menu`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct MenuPath {
    /// Menu index on the bar.
    pub menu: usize,
    /// Item index in the menu's [`MenuResponse::clicked`](crate::selection::MenuResponse::clicked) numbering.
    pub item: usize,
}

/// Result of [`MenuStrip::show`].
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct MenuStripResponse {
    /// Bar response.
    pub response: Response,
    /// Open menu surface; `None` while all menus are closed.
    pub popup: Option<Response>,
    /// Clicked item without a [`MenuItem::command`](crate::selection::MenuItem::command).
    pub clicked: Option<MenuPath>,
}

/// Horizontal menu bar over [`Menu`]s with APG menubar keyboard behavior; non-MD3.
#[must_use = "call .show(..)"]
pub struct MenuStrip {
    salt: KeyedSalt,
    label: Option<String>,
    disabled: bool,
    menus: Vec<Menu>,
}

egui_mineral::kit::opaque_debug!(MenuStrip);

impl MenuStrip {
    /// Horizontal row of menu titles; agent key prefix `<id_salt>`.
    pub fn new(id_salt: impl Into<KeyedSalt>) -> Self {
        let id_salt: KeyedSalt = id_salt.into();
        Self {
            salt: id_salt,
            label: None,
            disabled: false,
            menus: Vec::new(),
        }
    }

    crate::widget::setters! {
        /// Replaces the `<id_salt>` key prefix.
        agent_key(salt)
    }

    /// Accessible name of the bar; default `mineral-menu-strip`.
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    crate::widget::setters! {
        /// Disables every menu; see [`Menu::disabled`].
        disabled
    }

    /// Appends a menu; its label is the bar title, its mnemonic opens it.
    pub fn menu(mut self, menu: impl Into<Menu>) -> Self {
        self.menus.push(menu.into());
        self
    }

    /// Appends menus; see [`Self::menu`].
    pub fn menus(mut self, menus: impl IntoIterator<Item = impl Into<Menu>>) -> Self {
        self.menus.extend(menus.into_iter().map(Into::into));
        self
    }

    /// Paints the bar and the open menu.
    pub fn show(mut self, ui: &mut MineralUi<'_, Material>) -> MenuStripResponse {
        let rt = ui.rt();
        let disabled = self.disabled;
        self.menus = self
            .menus
            .into_iter()
            .map(|m| {
                let off = m.disabled || disabled;
                m.disabled(off).resolved()
            })
            .collect();
        let ctx = ui.ctx().clone();
        let t = rt.tokens();
        let id = ui.make_persistent_id(self.salt.id_salt());
        let mut bar = BarState::load(&ctx, id);
        let width = ui.available_width();
        let (rect, bar_resp) = ui.allocate_exact_size(vec2(width, BAR_H), Sense::hover());
        ui.painter().rect_filled(rect, 0, t.color.surface_container);
        let enabled = |i: usize| self.menus.get(i).is_some_and(|m| !m.disabled);
        let tab_stop = focus::tab_stop(&ctx, id, bar.open, self.menus.len(), enabled);
        if bar.open.is_some_and(|o| !enabled(o)) {
            bar.open = None;
        }
        let scope = shortcut::scope_of(&ctx, ui.layer_id());
        let buttons = self.paint_titles(ui, id, rect, bar.open, tab_stop, scope);
        let ids: Vec<Id> = buttons.iter().map(|b| b.id).collect();
        let fired = self.react(&ctx, id, &mut bar, &buttons, scope);
        bar.toggle_activation(rt.core(), self.disabled, &ids, tab_stop);
        if bar.open.is_none() {
            RovingFocus::new(Axis::Horizontal).update_group(rt.core(), id, &ids, enabled);
        }

        let mut result = None;
        let mut menu = None;
        if fired {
            bar.open = None;
        }
        if let Some(i) = bar.open {
            let menu_id = id.with(("menu", i));
            let mut st = MenuState::load(&ctx, menu_id);
            let place = common::popup_anchor(Side::Bottom, Align::Start);
            let out = self.menus[i].run(rt, menu_id, &mut st, place, Some(rect), bar_resp.layer_id);
            if let Some(item) = out.activated {
                result = Some(MenuPath { menu: i, item });
            }
            menu = out.panel;
            let still_open = st.open;
            st.store(&ctx, menu_id);
            if out.nav != 0 {
                let next = menubar::adjacent(i, out.nav, self.menus.len());
                bar.open = Some(next);
                Self::reopen(&ctx, id, next, buttons[next].rect, Some(FocusTarget::SelectedOrFirst));
            } else if !still_open {
                bar.open = None;
                if out.closed {
                    menubar::refocus_title(&ctx, id, i, ids[i]);
                }
            }
        }
        for (i, b) in buttons.iter().enumerate() {
            self.menus[i].emit_trigger(
                &ctx,
                b,
                bar.open == Some(i),
                Role::MenuItem,
                "menubar-item",
                self.menus[i].mnemonic,
            );
        }
        bar.settle(&ctx, &ids);
        semantic::emit(&ctx, Some(bar_resp.id), rect, || {
            let name = crate::tr_or(rt.i18n(), self.label.as_deref(), "mineral-menu-strip");
            let mut n = SemanticNode::new(StableId::of(&self.salt, "menubar"), Role::MenuBar, name);
            n.state.disabled = disabled;
            n
        });
        bar.store(&ctx, id);
        MenuStripResponse {
            response: bar_resp,
            popup: menu,
            clicked: result,
        }
    }

    fn paint_titles(
        &self,
        ui: &mut MineralUi<'_, Material>,
        id: Id,
        rect: Rect,
        open_menu: Option<usize>,
        tab_stop: usize,
        scope: Scope,
    ) -> Vec<Response> {
        let rt = ui.rt();
        let ctx = rt.ctx();
        let ctx = &ctx;
        let t = rt.tokens();
        let dir = rt.i18n().direction();
        let font = TypeScale::LabelLarge.font(ctx);
        let mut buttons: Vec<Response> = Vec::with_capacity(self.menus.len());
        let mut x = BAR_INSET;
        for (i, menu) in self.menus.iter().enumerate() {
            let availability = Availability::disabled_if(menu.disabled);
            let open = open_menu == Some(i);
            let fg = if open {
                t.color.on_secondary_container
            } else {
                t.color.on_surface
            };
            let galley = parts::label_galley(rt.core(), ui.painter(), &menu.label, font.clone(), fg, menu.mnemonic);
            let w = galley.size().x + 24.0;
            let left = dir.start_x(rect, Rangef::new(x, x + w));
            let r = Rect::from_min_size(pos2(left, rect.top() + BAR_INSET), vec2(w, BAR_H - 2.0 * BAR_INSET));
            x += w + BAR_INSET;
            let resp = kit::input::interact(
                ui,
                r,
                id.with(("item", i)),
                focus::roving_sense(Sense::click(), focus::TabStop::at(i, tab_stop)),
                availability,
            );
            kit::focus::follow_press(&resp);
            if let Some(m) = menu.mnemonic.filter(|_| !menu.disabled) {
                shortcut::bind_mnemonic(ctx, scope, m, Trigger::Alt, resp.id);
            }
            let round = CornerRadius::same(t.shape.small);
            if open {
                ui.painter().rect_filled(r, round, t.color.secondary_container);
            }
            indication::paint(ui, &resp, r, round, fg, Interaction::of(&resp, availability), &t);
            ui.painter().galley(r.center() - galley.size() / 2.0, galley, fg);
            kit::input::hover_cursor(rt.core(), &resp, availability);
            buttons.push(resp);
        }
        buttons
    }

    fn react(&self, ctx: &Context, id: Id, bar: &mut BarState, buttons: &[Response], scope: Scope) -> bool {
        let mut fired = false;
        for menu in &self.menus {
            fired |= menu.bind_shortcuts(ctx, scope);
        }
        for (i, b) in buttons.iter().enumerate() {
            if self.menus[i].disabled {
                continue;
            }
            let key_hit = self.menus[i].mnemonic.is_some() && shortcut::take_item(ctx, b.id);
            if key_hit {
                bar.open = Some(i);
                bar.revealed = true;
                shortcut::set_mnemonics_revealed(ctx, true);
                Self::reopen(ctx, id, i, b.rect, Some(FocusTarget::SelectedOrFirst));
            } else if b.clicked() {
                bar.open = if bar.open == Some(i) { None } else { Some(i) };
                if bar.open.is_some() {
                    Self::reopen(ctx, id, i, b.rect, open_focus(ctx));
                }
            } else if bar.open.is_some_and(|o| o != i)
                && b.hovered()
                && ctx.input(|inp| inp.pointer.delta() != egui::Vec2::ZERO)
            {
                // APG menubar: focus follows into the new menu when the old one held it.
                let focus = bar
                    .open
                    .and_then(|o| MenuState::try_load(ctx, id.with(("menu", o))))
                    .is_some_and(|st| st.focused)
                    .then_some(FocusTarget::SelectedOrFirst);
                bar.open = Some(i);
                Self::reopen(ctx, id, i, b.rect, focus);
            } else if b.has_focus() && bar.open.is_none() {
                for key in kit::input::plain_key_presses(ctx) {
                    match key {
                        Key::ArrowDown => {
                            bar.open = Some(i);
                            Self::reopen(ctx, id, i, b.rect, Some(FocusTarget::SelectedOrFirst));
                        }
                        Key::ArrowUp => {
                            bar.open = Some(i);
                            Self::reopen(ctx, id, i, b.rect, Some(FocusTarget::Last));
                        }
                        _ => {}
                    }
                }
            }
        }
        fired
    }

    fn reopen(ctx: &Context, id: Id, i: usize, anchor: Rect, focus: Option<FocusTarget>) {
        let menu_id = id.with(("menu", i));
        let mut st = MenuState::default();
        Menu::open(&mut st, anchor, focus);
        st.store(ctx, menu_id);
    }
}
