use crate::Material;
use crate::MaterialExt;
use crate::MaterialUiExt as _;
use crate::Rt;
use crate::adaptive::Density;
use egui_mineral::i18n::I18n;
use egui_mineral::kit::state::TempState;
use egui_mineral::semantic::KeyedSalt;
use egui_mineral::widget::MineralUi;
use std::time::Duration;

use egui::{
    Color32, Context, CornerRadius, Id, Key, KeyboardShortcut, LayerId, Modifiers, Rangef, Rect, Response, Stroke, Ui,
    UiBuilder, accesskit::HasPopup, pos2, vec2,
};
use egui_mineral::widget::Availability;
use egui_mineral::{
    action::Clicked,
    anchor::{Align, Anchor, SafeTriangle, Side, Surroundings},
    focus::{self, Modality, RovingFocus, Typeahead},
    i18n::Direction,
    input::{Gesture, HoverIntent},
    kit,
    layout::{Axis, Constraints, Measure},
    overlay::Dismiss,
    semantic::{self, Role, SemanticNode, StableId},
    shortcut::{self, ActionId, Mnemonic, Scope, Trigger},
    widget::{MineralWidget, WidgetFrame, WidgetResponse},
};

use crate::widget::leading::Leading;
use crate::widget::{indication, parts};
use crate::{
    Icon,
    spec::{
        ShapeRole, defaults,
        tokens::{divider, list, menu, segmented_menu, standard_menu, vibrant_menu},
    },
    tokens::Tokens,
};
use crate::{
    spec::ElevationLevel,
    widget::common::{self},
};

// Pointer travel grace; non-Compose
const SUBMENU_DELAY: Duration = Duration::from_millis(200);

/// Menu color set (Compose `MenuDefaults.*Colors`). `Baseline` is MD3 2023; `Standard` and `Vibrant`
/// are MD3 Expressive.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum MenuColors {
    #[default]
    /// MD3 2023 menu (`MenuDefaults.itemColors`).
    Baseline,
    /// Expressive menu, surface colors (`selectableItemColors`).
    Standard,
    /// Expressive menu, tertiary colors (`selectableItemVibrantColors`).
    Vibrant,
}

// Compose MenuDefaults.itemColors, selectableItemColors, selectableItemVibrantColors
#[derive(Clone, Copy)]
struct ItemColors {
    container: Color32,
    label: Color32,
    icon: Color32,
    selected_container: Color32,
    selected_label: Color32,
    selected_icon: Color32,
    disabled_label: Color32,
    disabled_icon: Color32,
}

// Compose DropdownMenuItem
struct ItemMetrics {
    height: f32,
    icon: f32,
    gap: f32,
    inset: f32,
}

// No MD3 value; one label line with padding
const GROUP_LABEL_MIN: f32 = 24.0;

impl MenuColors {
    fn expressive(self) -> bool {
        self != Self::Baseline
    }

    fn metrics(self, density: Density) -> ItemMetrics {
        use defaults::menu as d;
        if self.expressive() {
            ItemMetrics {
                height: segmented_menu::ITEM,
                icon: segmented_menu::ITEM_LEADING_ICON_SIZE,
                gap: d::ICON_TEXT_SPACING,
                inset: d::SELECTABLE_ITEM_INSET,
            }
        } else {
            ItemMetrics {
                height: density.dense_to(d::ITEM_HEIGHT, list::ITEM_LEADING_ICON_SIZE + 4.0),
                icon: list::ITEM_LEADING_ICON_SIZE,
                gap: d::ITEM_HORIZONTAL_PADDING,
                inset: 0.0,
            }
        }
    }

    fn colors(self, t: &Tokens) -> ItemColors {
        match self {
            Self::Baseline => ItemColors {
                container: t.role(menu::CONTAINER_COLOR),
                label: t.role(list::ITEM_LABEL_TEXT_COLOR),
                icon: t.role(list::ITEM_LEADING_ICON_COLOR),
                selected_container: t.role(menu::LIST_ITEM_SELECTED_CONTAINER_COLOR),
                selected_label: t.role(menu::LIST_ITEM_SELECTED_LABEL_TEXT_COLOR),
                selected_icon: t.role(menu::LIST_ITEM_SELECTED_LEADING_TRAILING_ICON_COLOR),
                disabled_label: t.role_alpha(
                    list::ITEM_DISABLED_LABEL_TEXT_COLOR,
                    list::ITEM_DISABLED_LABEL_TEXT_OPACITY,
                ),
                disabled_icon: t.role_alpha(
                    list::ITEM_DISABLED_LEADING_ICON_COLOR,
                    list::ITEM_DISABLED_LEADING_ICON_OPACITY,
                ),
            },
            Self::Standard => {
                use standard_menu as m;
                ItemColors {
                    container: t.role(m::CONTAINER_COLOR),
                    label: t.role(m::ITEM_LABEL_TEXT_COLOR),
                    icon: t.role(m::ITEM_LEADING_ICON_COLOR),
                    selected_container: t.role(m::ITEM_SELECTED_CONTAINER_COLOR),
                    selected_label: t.role(m::ITEM_SELECTED_LABEL_TEXT_COLOR),
                    selected_icon: t.role(m::ITEM_SELECTED_LEADING_ICON_COLOR),
                    disabled_label: t
                        .role_alpha(m::ITEM_DISABLED_LABEL_TEXT_COLOR, m::ITEM_DISABLED_LABEL_TEXT_OPACITY),
                    disabled_icon: t.role_alpha(
                        m::ITEM_DISABLED_LEADING_ICON_COLOR,
                        m::ITEM_DISABLED_LEADING_ICON_OPACITY,
                    ),
                }
            }
            Self::Vibrant => {
                use vibrant_menu as m;
                ItemColors {
                    container: t.role(m::CONTAINER_COLOR),
                    label: t.role(m::ITEM_LABEL_TEXT_COLOR),
                    icon: t.role(m::ITEM_LEADING_ICON_COLOR),
                    selected_container: t.role(m::ITEM_SELECTED_CONTAINER_COLOR),
                    selected_label: t.role(m::ITEM_SELECTED_LABEL_TEXT_COLOR),
                    selected_icon: t.role(m::ITEM_SELECTED_LEADING_ICON_COLOR),
                    disabled_label: t
                        .role_alpha(m::ITEM_DISABLED_LABEL_TEXT_COLOR, m::ITEM_DISABLED_LABEL_TEXT_OPACITY),
                    disabled_icon: t.role_alpha(
                        m::ITEM_DISABLED_LEADING_ICON_COLOR,
                        m::ITEM_DISABLED_LEADING_ICON_OPACITY,
                    ),
                }
            }
        }
    }

    fn elevation(self) -> ElevationLevel {
        if self.expressive() {
            segmented_menu::CONTAINER_ELEVATION
        } else {
            menu::CONTAINER_ELEVATION
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum InGroup {
    Alone,
    First,
    Middle,
    Last,
}

fn vertical_radius(top: u8, bottom: u8) -> CornerRadius {
    CornerRadius {
        nw: top,
        ne: top,
        sw: bottom,
        se: bottom,
    }
}

/// Trailing slot of a [`MenuItem`]: an icon or a shortcut label.
#[derive(Clone, Copy, Debug)]
#[non_exhaustive]
pub enum MenuTrailing {
    /// Trailing icon.
    Icon(Icon),
    /// Shortcut label; bound to the item's [`MenuItem::command`] while the menu is shown.
    Shortcut(KeyboardShortcut),
}

impl From<Icon> for MenuTrailing {
    fn from(icon: Icon) -> Self {
        Self::Icon(icon)
    }
}

impl From<crate::MaterialIcon> for MenuTrailing {
    fn from(icon: crate::MaterialIcon) -> Self {
        Self::from(Icon::from(icon))
    }
}

impl From<KeyboardShortcut> for MenuTrailing {
    fn from(shortcut: KeyboardShortcut) -> Self {
        Self::Shortcut(shortcut)
    }
}

/// Entry of a [`Menu`].
#[must_use]
pub struct MenuItem {
    label: String,
    key: Option<StableId>,
    leading: Option<Leading>,
    trailing: Option<MenuTrailing>,
    command: Option<ActionId>,
    action_key: Option<egui_mineral::action::ActionKey>,
    mnemonic: Option<Mnemonic>,
    tooltip: Option<String>,
    selected: bool,
    disabled: bool,
    submenu: Vec<Entry>,
    salt: Option<egui_mineral::semantic::KeyedSalt>,
}

egui_mineral::kit::opaque_debug!(MenuItem);

impl MenuItem {
    crate::widget::setters! {
        /// Default the [`Self::command`] action, else the index; [`Self::agent_key`] overrides only the key.
        item_salt
    }

    /// `label` is the visible text and accessible name.
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            key: None,
            leading: None,
            trailing: None,
            command: None,
            action_key: None,
            mnemonic: None,
            tooltip: None,
            selected: false,
            disabled: false,
            salt: None,
            submenu: Vec::new(),
        }
    }

    crate::widget::setters! {
        action_key
    }

    /// Overrides the `<menu-key>-<item>-menu-item` default (`<item>` from [`Self::salt`]).
    pub fn agent_key(mut self, key: impl Into<StableId>) -> Self {
        self.key = Some(key.into());
        self
    }

    /// Leading icon or app paint.
    pub fn leading(mut self, leading: impl Into<Leading>) -> Self {
        self.leading = Some(leading.into());
        self
    }

    /// Trailing icon or shortcut label.
    pub fn trailing(mut self, trailing: impl Into<MenuTrailing>) -> Self {
        self.trailing = Some(trailing.into());
        self
    }

    /// Makes the item a view of the app command `id`: activating it fires the command
    /// ([`shortcut::fire`]) instead of reporting the item as clicked. A [`MenuTrailing::Shortcut`]
    /// is bound to `id` while the menu is shown (global; focus-scoped in context menus); other
    /// views of `id` may bind the same shortcut.
    pub fn command(mut self, id: impl Into<ActionId>) -> Self {
        self.command = Some(id.into());
        self
    }

    /// Letter that activates the item while its menu is open (bare or Alt).
    pub fn mnemonic(mut self, mnemonic: char) -> Self {
        self.mnemonic = Some(Mnemonic::new(mnemonic));
        self
    }
    /// [`PlainTooltip`](crate::communication::PlainTooltip) on hover; also the item's accessible description.
    pub fn tooltip(mut self, text: impl Into<String>) -> Self {
        self.tooltip = Some(text.into());
        self
    }

    /// Checked state of a selectable item.
    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }
    crate::widget::setters! {
        /// Greys out the item; it cannot be activated.
        disabled
    }

    /// Nested menu opened from this item; `build` gets an empty [`Menu`].
    pub fn submenu(mut self, build: impl FnOnce(Menu) -> Menu) -> Self {
        self.submenu = build(Menu::new("submenu", "")).resolved().entries;
        self
    }

    fn has_submenu(&self) -> bool {
        !self.submenu.is_empty()
    }
}

#[expect(
    clippy::large_enum_variant,
    reason = "entries are mostly items; boxing allocates per item per pass"
)]
enum Entry {
    Item(MenuItem),
    Divider,
    Group(Option<String>),
}

fn disable_all(entries: &mut [Entry]) {
    for e in entries {
        if let Entry::Item(i) = e {
            i.disabled = true;
            disable_all(&mut i.submenu);
        }
    }
}

fn item_at<'e>(entries: &'e [Entry], target: usize, next: &mut usize) -> Option<&'e MenuItem> {
    for e in entries {
        let Entry::Item(i) = e else { continue };
        if *next == target {
            return Some(i);
        }
        *next += 1;
        if let Some(found) = item_at(&i.submenu, target, next) {
            return Some(found);
        }
    }
    None
}

fn count(entries: &[Entry]) -> usize {
    entries
        .iter()
        .map(|e| match e {
            Entry::Item(i) => 1 + count(&i.submenu),
            _ => 0,
        })
        .sum()
}

#[derive(Clone, Copy)]
enum SlotKind {
    Row(usize, InGroup),
    Divider,
    Label(usize),
}

struct Geometry {
    size: egui::Vec2,
    slots: Vec<(SlotKind, Rect)>,
    groups: Vec<Rect>,
}

/// Result of [`Menu::show`], [`Menu::show_context`] and [`Menu::show_at`].
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct MenuResponse {
    /// Menu surface; `None` while closed.
    pub response: Option<Response>,
    /// Clicked item without a [`MenuItem::command`], with its [`MenuItem::key`]; command items fire their
    /// command instead.
    pub clicked: Option<Clicked>,
    /// Why it closed without an activation.
    pub dismissed: Option<Dismiss>,
}

#[derive(Default)]
pub(crate) struct Outcome {
    pub(crate) panel: Option<Response>,
    pub(crate) activated: Option<usize>,
    pub(crate) fired: bool,
    pub(crate) closed: bool,
    pub(crate) dismissed: Option<Dismiss>,
    pub(crate) nav: i8,
    pub(crate) first_rect: Option<Rect>,
}

// Keyboard opens move focus into the menu; pointer opens leave it, arrows move it in later.
pub(crate) fn open_focus(ctx: &Context) -> Option<FocusTarget> {
    (focus::modality(ctx) == Modality::Keyboard).then_some(FocusTarget::SelectedOrFirst)
}

// Item a menu level focuses when it opens.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum FocusTarget {
    SelectedOrFirst,
    Last,
}

#[derive(Clone)]
pub(crate) struct MenuState {
    pub(crate) open: bool,
    pub(crate) anchor: Rect,
    pub(crate) path: Vec<usize>,
    pub(crate) pending: HoverIntent<(usize, Option<usize>)>,
    pub(crate) focus: Option<(usize, FocusTarget)>,
    pub(crate) focused: bool,
    // Layer under the anchor when `show_at` opened it.
    pub(crate) parent: LayerId,
}

impl TempState for MenuState {}

impl Default for MenuState {
    fn default() -> Self {
        Self {
            open: false,
            anchor: Rect::NOTHING,
            path: Vec::new(),
            pending: HoverIntent::default(),
            focus: None,
            focused: false,
            parent: LayerId::background(),
        }
    }
}

/// MD3 menu. Item indices returned by the `show*` methods count every item in declaration
/// order, nested submenu items included (submenu parents do not activate).
#[must_use = "call .show(..)"]
pub struct Menu {
    salt: KeyedSalt,
    pub(crate) label: String,
    pub(crate) mnemonic: Option<Mnemonic>,
    entries: Vec<Entry>,
    style: MenuColors,
    emit_anchor: bool,
    pub(crate) disabled: bool,
}

egui_mineral::kit::opaque_debug!(Menu);

impl Menu {
    /// `label` is the accessible name of the menu; agent key prefix `<id_salt>`.
    pub fn new(id_salt: impl Into<KeyedSalt>, label: impl Into<String>) -> Self {
        Self {
            salt: id_salt.into(),
            label: label.into(),
            mnemonic: None,
            entries: Vec::new(),
            style: MenuColors::default(),
            emit_anchor: true,
            disabled: false,
        }
    }

    crate::widget::setters! {
        /// Keeps the menu closed and disables every item; its trigger reports disabled.
        disabled
    }

    pub(crate) fn resolved(mut self) -> Self {
        if self.disabled {
            disable_all(&mut self.entries);
        }
        self
    }

    crate::widget::setters! {
        /// Replaces the `<id_salt>` key prefix.
        agent_key(salt)
    }

    /// Alt+letter opens the menu from a `MenuStrip`.
    pub fn mnemonic(mut self, mnemonic: char) -> Self {
        self.mnemonic = Some(Mnemonic::new(mnemonic));
        self
    }

    /// Appends one item.
    pub fn item(mut self, item: impl Into<MenuItem>) -> Self {
        self.entries.push(Entry::Item(item.into()));
        self
    }

    /// Appends items.
    pub fn items(mut self, items: impl IntoIterator<Item = impl Into<MenuItem>>) -> Self {
        self.entries.extend(items.into_iter().map(|i| Entry::Item(i.into())));
        self
    }

    /// Divider between items.
    pub fn divider(mut self) -> Self {
        self.entries.push(Entry::Divider);
        self
    }

    /// Starts an unlabeled group: a divider in baseline menus, a separate segment in expressive ones.
    pub fn group(mut self) -> Self {
        self.entries.push(Entry::Group(None));
        self
    }

    /// Starts a group with a heading; see [`Self::group`].
    pub fn group_label(mut self, group_label: impl Into<String>) -> Self {
        self.entries.push(Entry::Group(Some(group_label.into())));
        self
    }

    pub(crate) fn emit_anchor(mut self, emit: bool) -> Self {
        self.emit_anchor = emit;
        self
    }

    /// Default [`MenuColors::Baseline`].
    pub fn colors(mut self, colors: MenuColors) -> Self {
        self.style = colors;
        self
    }

    fn width(&self, rt: &Rt, density: Density, entries: &[Entry]) -> f32 {
        use defaults::menu as d;
        let ctx = rt.ctx();
        let m = self.style.metrics(density);
        let format = defaults::menu::ITEM_FONT.format(ctx, Color32::PLACEHOLDER);
        let job = |s: &str, mnemonic| parts::format_job(rt.core(), s, &format, mnemonic);
        let icon = m.icon + m.gap;
        // Jobs read `ctx`, so they are built before the fonts lock.
        let rows: Vec<_> = entries
            .iter()
            .filter_map(|e| match e {
                Entry::Item(i) => Some(i),
                _ => None,
            })
            .map(|i| {
                let lead = if i.leading.is_some() || (i.selected && self.style.expressive()) {
                    icon
                } else {
                    0.0
                };
                let (trail, shortcut) = match (&i.trailing, i.has_submenu()) {
                    (Some(MenuTrailing::Shortcut(s)), _) => (m.gap, Some(job(&shortcut::label(ctx, s), None))),
                    (Some(MenuTrailing::Icon(_)), _) | (None, true) => (icon, None),
                    (None, false) => (0.0, None),
                };
                (
                    2.0 * d::ITEM_HORIZONTAL_PADDING + lead + trail,
                    job(&i.label, i.mnemonic),
                    shortcut,
                )
            })
            .collect();
        let w = ctx.fonts_mut(|f| {
            rows.into_iter()
                .map(|(fixed, label, shortcut)| {
                    fixed + f.layout_job(label).size().x + shortcut.map_or(0.0, |j| f.layout_job(j).size().x)
                })
                .fold(0.0, f32::max)
        });
        w.clamp(d::ITEM_MIN_WIDTH, d::ITEM_MAX_WIDTH) + 2.0 * m.inset
    }

    pub(crate) fn bind_shortcuts(&self, ctx: &Context, scope: Scope) -> bool {
        Self::bind_level(ctx, scope, &self.entries)
    }

    fn bind_level(ctx: &Context, scope: Scope, entries: &[Entry]) -> bool {
        let mut fired = false;
        for entry in entries {
            let Entry::Item(item) = entry else { continue };
            if let Some(command) = &item.command
                && !item.disabled
                && !item.has_submenu()
            {
                if let Some(MenuTrailing::Shortcut(shortcut)) = item.trailing {
                    shortcut::bind(ctx, scope, shortcut, command);
                }
                fired |= shortcut::fired(ctx, command);
            }
            fired |= Self::bind_level(ctx, scope, &item.submenu);
        }
        fired
    }

    fn edge_padding(&self) -> f32 {
        if self.style.expressive() {
            defaults::menu::GROUP_VERTICAL_PADDING
        } else {
            defaults::menu::VERTICAL_PADDING
        }
    }

    fn geometry(&self, rt: &Rt, density: Density, entries: &[Entry]) -> Geometry {
        use defaults::menu as d;
        let width = self.width(rt, density, entries);
        let exp = self.style.expressive();
        let item_h = self.style.metrics(density).height;
        let label_h = density.dense_to(d::GROUP_LABEL_HEIGHT, GROUP_LABEL_MIN);
        let divider_h = divider::THICKNESS + 2.0 * d::DIVIDER_VERTICAL_PADDING;
        let edge = self.edge_padding();
        let mut slots = Vec::new();
        let mut groups = Vec::new();
        let mut rows = Vec::new();
        let mut y = edge;
        let mut group_top = 0.0;
        let push = |slots: &mut Vec<(SlotKind, Rect)>, kind, h: f32, y: &mut f32| {
            slots.push((kind, Rect::from_min_size(pos2(0.0, *y), vec2(width, h))));
            *y += h;
        };
        for (e, entry) in entries.iter().enumerate() {
            match entry {
                Entry::Item(_) => {
                    rows.push(slots.len());
                    push(&mut slots, SlotKind::Row(e, InGroup::Alone), item_h, &mut y);
                }
                Entry::Divider => push(&mut slots, SlotKind::Divider, divider_h, &mut y),
                Entry::Group(label) => {
                    if !rows.is_empty() {
                        place_rows(&mut slots, &mut rows);
                        if exp {
                            y += edge;
                            groups.push(Rect::from_min_max(pos2(0.0, group_top), pos2(width, y)));
                            y += segmented_menu::SEGMENTED_GAP;
                            group_top = y;
                            y += edge;
                        } else {
                            push(&mut slots, SlotKind::Divider, divider_h, &mut y);
                        }
                    }
                    if label.is_some() {
                        push(&mut slots, SlotKind::Label(e), label_h, &mut y);
                    }
                }
            }
        }
        place_rows(&mut slots, &mut rows);
        y += edge;
        groups.push(Rect::from_min_max(pos2(0.0, group_top), pos2(width, y)));
        Geometry {
            size: vec2(width, y),
            slots,
            groups,
        }
    }

    /// Dropdown opened by [`Self::show`] on `anchor` is open.
    pub fn is_open(ctx: &egui::Context, anchor: &Response) -> bool {
        MenuState::try_load(ctx, dropdown_id(anchor)).is_some_and(|st| st.open)
    }

    /// Context menu opened by [`Self::show_context`] on `target` is open.
    pub fn is_context_open(ctx: &egui::Context, target: &Response) -> bool {
        MenuState::try_load(ctx, context_id(target)).is_some_and(|st| st.open)
    }

    /// [`Self::show_context`] on `target` is needed this pass: open, or opening (secondary click,
    /// long press, Shift+F10, Menu key). Lets per-row context menus be built only when used:
    /// `if Menu::context_requested(rt, &r) { menu.show_context(rt, &r); }`. Item shortcuts of a menu
    /// built this way are bound only while it is shown.
    pub fn context_requested(rt: &Rt, target: &Response) -> bool {
        Self::is_context_open(&target.ctx, target) || Self::context_trigger(rt.core(), target, false).is_some()
    }

    fn context_trigger(core: &egui_mineral::runtime::Core, target: &Response, consume: bool) -> Option<Rect> {
        let ctx = &target.ctx;
        let layer = target.layer_id;
        let at = if target.secondary_clicked() {
            target.interact_pointer_pos()
        } else {
            // Core only, no MineralUi.
            core.input().gestures().find_map(|g| match g {
                Gesture::LongPress { pos }
                    if target.rect.contains(*pos) && ctx.layer_id_at(*pos).is_none_or(|l| l == layer) =>
                {
                    Some(*pos)
                }
                _ => None,
            })
        };
        if let Some(p) = at {
            return Some(Rect::from_min_size(p, egui::Vec2::ZERO));
        }
        // Shift+F10 or the Menu (Apps) key.
        let keys = [(Modifiers::SHIFT, Key::F10), (Modifiers::NONE, Key::Menu)];
        let pressed = |i: &mut egui::InputState| {
            keys.iter().any(|&(m, k)| {
                if consume {
                    i.consume_key(m, k)
                } else {
                    i.events.iter().any(|e| {
                        matches!(e, egui::Event::Key { key, pressed: true, modifiers, .. }
                            if *key == k && modifiers.matches_logically(m))
                    })
                }
            })
        };
        (target.has_focus() && ctx.input_mut(pressed))
            .then(|| Rect::from_min_size(target.rect.left_bottom(), egui::Vec2::ZERO))
    }

    /// Dropdown below `anchor`, toggled by its clicks.
    pub fn show(self, rt: &Rt, anchor: &Response) -> MenuResponse {
        let this = self.resolved();
        let ctx = anchor.ctx.clone();
        let id = dropdown_id(anchor);
        let mut st = MenuState::load(&ctx, id);
        if this.bind_shortcuts(&ctx, shortcut::scope_of(&ctx, anchor.layer_id)) || this.disabled {
            st.open = false;
        }
        let mut toggled_closed = false;
        if anchor.clicked() && !this.disabled {
            if st.open {
                st.open = false;
                toggled_closed = true;
            } else {
                Self::open(&mut st, anchor.rect, open_focus(&ctx));
            }
        } else if anchor.has_focus() && !st.open && !this.disabled {
            for key in kit::interact::plain_key_presses(&ctx) {
                match key {
                    Key::ArrowDown => Self::open(&mut st, anchor.rect, Some(FocusTarget::SelectedOrFirst)),
                    Key::ArrowUp => Self::open(&mut st, anchor.rect, Some(FocusTarget::Last)),
                    _ => {}
                }
            }
        }
        if st.open {
            st.anchor = anchor.rect;
        }
        let place = common::popup_anchor(Side::Bottom, Align::Start);
        let out = this.run(rt, id, &mut st, place, Some(anchor.rect), anchor.layer_id);
        if out.closed {
            anchor.request_focus();
        }
        let open = st.open;
        st.store(&ctx, id);
        this.emit_trigger(&ctx, anchor, open, Role::Button, "menu-button", None);
        MenuResponse {
            response: out.panel,
            clicked: this.clicked(out.activated),
            dismissed: out.dismissed.or(toggled_closed.then_some(Dismiss::Close)),
        }
    }

    /// Context menu at the pointer on secondary click or long press, or below `target` on Shift+F10 or the Menu key.
    pub fn show_context(self, rt: &Rt, target: &Response) -> MenuResponse {
        let this = self.resolved();
        let ctx = target.ctx.clone();
        let id = context_id(target);
        let mut st = MenuState::load(&ctx, id);
        if this.bind_shortcuts(&ctx, Scope::Focus(target.id)) || this.disabled {
            st.open = false;
        }
        if let Some(at) = Self::context_trigger(rt.core(), target, true).filter(|_| !this.disabled) {
            Self::open(&mut st, at, open_focus(&ctx));
        }
        let place = common::popup_anchor(Side::Bottom, Align::Start);
        let out = this.run(rt, id, &mut st, place, None, target.layer_id);
        if out.closed {
            target.request_focus();
        }
        st.store(&ctx, id);
        ctx.accesskit_node_builder(target.id, |b| b.set_has_popup(HasPopup::Menu));
        MenuResponse {
            response: out.panel,
            clicked: this.clicked(out.activated),
            dismissed: out.dismissed,
        }
    }

    /// Menu at `anchor` while `open`, which the app owns: e.g. a rect from
    /// [`ToolbarResponse::actions`](crate::navigation::ToolbarResponse::actions) or
    /// `TreeAction::rect`. Closing (an item, Escape, an outside press other than
    /// on `anchor`) clears `open`. State is keyed by the menu's `id_salt` alone, unique per window.
    pub fn show_at(self, rt: &Rt, anchor: Rect, open: &mut bool) -> MenuResponse {
        let this = self.resolved();
        let ctx = rt.ctx().clone();
        let id = kit::scoped_id(rt.core(), Id::unique(("mineral::menu_at", this.salt.id_salt())));
        let mut st = MenuState::load(&ctx, id);
        if *open && !st.open {
            st.parent = ctx.layer_id_at(anchor.center()).unwrap_or_else(LayerId::background);
            Self::open(&mut st, anchor, open_focus(&ctx));
        }
        if this.bind_shortcuts(&ctx, shortcut::scope_of(&ctx, st.parent)) || this.disabled {
            *open = false;
        }
        st.open = *open;
        if st.open {
            st.anchor = anchor;
        }
        let place = common::popup_anchor(Side::Bottom, Align::Start);
        let parent = st.parent;
        let out = this.run(rt, id, &mut st, place, Some(anchor), parent);
        *open = st.open;
        st.store(&ctx, id);
        MenuResponse {
            response: out.panel,
            clicked: this.clicked(out.activated),
            dismissed: out.dismissed,
        }
    }

    pub(crate) fn clicked(&self, index: Option<usize>) -> Option<Clicked> {
        index.map(|i| {
            Clicked::new(
                i,
                item_at(&self.entries, i, &mut 0).and_then(|item| item.action_key.clone()),
            )
        })
    }

    pub(crate) fn open(st: &mut MenuState, anchor: Rect, focus: Option<FocusTarget>) {
        st.open = true;
        st.anchor = anchor;
        st.path.clear();
        st.pending.cancel();
        st.focus = focus.map(|f| (0, f));
    }

    fn level_entries<'e>(&'e self, path: &[usize], level: usize) -> Option<&'e [Entry]> {
        let mut entries: &[Entry] = &self.entries;
        for &e in path.get(..level)? {
            match entries.get(e) {
                Some(Entry::Item(i)) if i.has_submenu() => entries = &i.submenu,
                _ => return None,
            }
        }
        Some(entries)
    }

    fn child_base(entries: &[Entry], base: usize, entry: usize) -> usize {
        base + entries[..entry]
            .iter()
            .map(|e| match e {
                Entry::Item(i) => 1 + count(&i.submenu),
                _ => 0,
            })
            .sum::<usize>()
            + 1
    }

    // `parent`: layer the menu opened from; each level stays above it and the level before.
    pub(crate) fn run(
        &self,
        rt: &Rt,
        id: Id,
        st: &mut MenuState,
        place: Anchor,
        trigger: Option<Rect>,
        parent: LayerId,
    ) -> Outcome {
        let ctx = rt.ctx();
        let mut out = Outcome::default();
        if !st.open {
            return out;
        }
        let t = rt.tokens();
        let dir = rt.i18n().direction();
        let surroundings = Surroundings::new(ctx.content_rect(), dir);
        let fr = Frame::new(ctx, id, &t, dir);
        let mut panels: Vec<Rect> = Vec::new();
        let pad_top = self.edge_padding();
        let mut anchor_rect = st.anchor;
        let mut base = 0;
        let mut level = 0;
        let mut key_level: Option<KeyLevel> = None;
        let mut level_ids: Vec<Vec<RowRect>> = Vec::new();
        let mut parent = parent;
        while let Some(entries) = self.level_entries(&st.path, level) {
            let geo = self.geometry(rt, rt.kit().density(), entries);
            let placement = if level == 0 {
                place.place(anchor_rect, geo.size, surroundings)
            } else {
                common::popup_anchor(Side::End, Align::Start).place(
                    anchor_rect.translate(vec2(0.0, -pad_top)),
                    geo.size,
                    surroundings,
                )
            };
            let rect = placement.rect;
            panels.push(rect);
            let open_child = st.path.get(level).copied();
            let mut lr = LevelRun {
                fr: &fr,
                entries,
                level,
                base,
                open_child,
                st: &mut *st,
                out: &mut out,
                rows: Vec::new(),
                enabled: Vec::new(),
            };
            let area = egui_mineral::kit::layer::floating(rt.core(), id.with(("level", level)), rect.min)
                .show(ctx, |ui| self.paint_level(&mut lr, &mut rt.ui(ui), &geo, rect));
            let LevelRun { rows, enabled, .. } = lr;
            egui_mineral::kit::layer::keep_above(ctx, area.response.layer_id, parent);
            parent = area.response.layer_id;
            let panel_id = area.response.id;
            semantic::emit(ctx, Some(panel_id), rect, || {
                SemanticNode::new(StableId::of(&self.salt, (level, "menu")), Role::Menu, &self.label)
            });
            if level == 0 {
                out.first_rect = Some(rect);
                out.panel = Some(area.response.clone());
            }
            Self::settle_focus(
                rt,
                &fr,
                st,
                &LevelFocus {
                    level,
                    entries,
                    enabled: &enabled,
                },
                &mut key_level,
            );
            let child_rect = open_child.and_then(|c| rows.iter().find(|r| r.0 == c).map(|r| r.2));
            level_ids.push(rows);
            let Some(child) = open_child else { break };
            let Some(row_rect) = child_rect else {
                st.path.truncate(level);
                break;
            };
            base = Self::child_base(entries, base, child);
            anchor_rect = row_rect;
            level += 1;
        }

        // Hover intent resolves after the delay unless the pointer heads into the open submenu.
        if let Some((l, want)) = st.pending.due(ctx, SUBMENU_DELAY) {
            st.path.truncate(l);
            if let Some(e) = want {
                st.path.push(e);
            }
        }

        // egui moves focus on (Shift+)Tab before the rows run, so judge by last pass's focus too.
        let held_focus = st.focused || key_level.is_some();
        st.focused = key_level.is_some();
        self.handle_keys(rt, &fr, st, &mut out, key_level, &level_ids);

        panels.extend(trigger);
        Self::dismissals(rt, id, st, &mut out, held_focus, &panels);
        out
    }

    fn dismissals(rt: &Rt, id: Id, st: &mut MenuState, out: &mut Outcome, held_focus: bool, panels: &[Rect]) {
        let ctx = rt.ctx();
        if st.open && held_focus && ctx.input(|i| i.key_pressed(Key::Tab)) {
            st.open = false;
            out.dismissed = Some(Dismiss::FocusOut);
        }

        // Outside press closes everything.
        if st.open && egui_mineral::kit::layer::outside_press_after_open(rt.core(), id, panels) {
            st.open = false;
            out.dismissed = Some(Dismiss::Outside);
        }
        if out.activated.is_some() || out.fired {
            st.open = false;
            out.closed = true;
        }
        if !st.open {
            st.focused = false;
            st.path.clear();
            st.pending.cancel();
        }
    }

    fn paint_level(&self, lr: &mut LevelRun<'_, '_>, ui: &mut MineralUi<'_, Material>, geo: &Geometry, rect: Rect) {
        let rt = ui.rt();
        let t = lr.fr.t;
        ui.spacing_mut().item_spacing = egui::Vec2::ZERO;
        let colors = self.style.colors(t);
        self.paint_groups(ui, geo, rect, &colors, lr.fr);
        for (kind, slot) in &geo.slots {
            let slot = slot.translate(rect.min.to_vec2());
            match *kind {
                SlotKind::Divider => {
                    let inset = defaults::menu::DIVIDER_HORIZONTAL_PADDING;
                    ui.painter().hline(
                        slot.left() + inset..=slot.right() - inset,
                        slot.center().y,
                        Stroke::new(divider::THICKNESS, t.role(divider::COLOR)),
                    );
                }
                SlotKind::Label(e) => {
                    if let Entry::Group(Some(text)) = &lr.entries[e] {
                        let g = parts::type_galley(
                            rt.core(),
                            ui.painter(),
                            text,
                            defaults::menu::ITEM_FONT,
                            colors.label,
                            None,
                        );
                        let pad = defaults::menu::GROUP_LABEL_START_PADDING;
                        let x = lr.fr.dir.start_x(slot, Rangef::new(pad, pad + g.size().x));
                        ui.painter()
                            .galley(pos2(x, slot.center().y - g.size().y / 2.0), g, colors.label);
                    }
                }
                SlotKind::Row(e, in_group) => self.show_row(lr, ui, e, in_group, slot),
            }
        }
    }

    fn paint_groups(&self, ui: &Ui, geo: &Geometry, rect: Rect, colors: &ItemColors, fr: &Frame<'_>) {
        let (t, dir) = (fr.t, fr.dir);
        let n = geo.groups.len();
        let rad = |r: ShapeRole| r.scale().radius(&t.shape);
        // Compose `MenuDefaults.groupShape(index, count)`.
        let r = |i: usize| {
            if !self.style.expressive() {
                return menu::CONTAINER_SHAPE.corner_radius(&t.shape, dir);
            }
            let (outer, inner) = (rad(segmented_menu::CONTAINER_SHAPE), rad(segmented_menu::GROUP_SHAPE));
            vertical_radius(
                if i == 0 { outer } else { inner },
                if i + 1 == n { outer } else { inner },
            )
        };
        for (i, g) in geo.groups.iter().enumerate() {
            let g = g.translate(rect.min.to_vec2());
            common::elevation(ui.painter(), g, r(i), self.style.elevation(), t);
        }
        for (i, g) in geo.groups.iter().enumerate() {
            ui.painter()
                .rect_filled(g.translate(rect.min.to_vec2()), r(i), colors.container);
        }
    }

    fn show_row(
        &self,
        lr: &mut LevelRun<'_, '_>,
        ui: &mut MineralUi<'_, Material>,
        e: usize,
        in_group: InGroup,
        slot: Rect,
    ) {
        let rt = ui.rt();
        let LevelRun {
            fr,
            entries,
            level,
            base,
            open_child,
            ..
        } = *lr;
        let ctx = fr.ctx;
        let Entry::Item(item) = &entries[e] else { return };
        let index = Self::child_base(entries, base, e) - 1;
        let resp = ui
            .scope_builder(UiBuilder::new().max_rect(slot), |ui| {
                let own = crate::widget::item_salt(&self.salt, item.salt.as_ref(), item.command.as_ref(), index);
                rt.ui(ui).mineral(MenuRow {
                    salt: own.child("row"),
                    key: item.key.clone().unwrap_or_else(|| StableId::of(&own, "menu-item")),
                    item,
                    width: slot.width(),
                    style: self.style,
                    in_group,
                    expanded: item.has_submenu().then_some(open_child == Some(e)),
                })
            })
            .inner;
        lr.rows.push((e, resp.id, slot));
        if let Some(tip) = &item.tooltip {
            crate::communication::PlainTooltip::new(tip).show(rt, resp.response());
        }
        if item.disabled {
            return;
        }
        lr.enabled.push((e, resp.id, item.label.as_str()));
        let key_hit = item.mnemonic.is_some_and(|m| {
            shortcut::bind_mnemonic(ctx, Scope::Focus(ui.unique_id()), m, Trigger::Either, resp.id);
            shortcut::take_item(ctx, resp.id)
        });
        if resp.clicked() || key_hit {
            if item.has_submenu() {
                lr.st.path.truncate(level);
                lr.st.path.push(e);
                lr.st.pending.cancel();
                lr.st.focus = Some((level + 1, FocusTarget::SelectedOrFirst));
            } else if let Some(command) = &item.command {
                shortcut::fire(ctx, command);
                lr.out.fired = true;
            } else {
                lr.out.activated = Some(index);
            }
        }
        if fr.moved && resp.hovered() {
            let toward_child = open_child.is_some_and(|_| {
                let sub = ctx.memory(|m| m.area_rect(fr.id.with(("level", level + 1))));
                sub.zip(fr.apex)
                    .zip(fr.pointer)
                    .is_some_and(|((s, a), p)| SafeTriangle::new(a, s).contains(p))
            });
            let want = item.has_submenu().then_some(e);
            if want == open_child {
                lr.st.pending.cancel();
            } else if !toward_child {
                lr.st.pending.request(ctx, (level, want));
            }
        }
    }

    fn settle_focus(
        rt: &Rt,
        fr: &Frame<'_>,
        st: &mut MenuState,
        lf: &LevelFocus<'_>,
        key_level: &mut Option<KeyLevel>,
    ) {
        let ctx = fr.ctx;
        let LevelFocus {
            level,
            entries,
            enabled,
        } = *lf;
        if let Some((l, pos)) = st.focus
            && l == level
        {
            let selected = enabled
                .iter()
                .position(|x| matches!(&entries[x.0], Entry::Item(i) if i.selected));
            let start = match pos {
                FocusTarget::Last => enabled.len().checked_sub(1),
                FocusTarget::SelectedOrFirst => selected.or((!enabled.is_empty()).then_some(0)),
            };
            if let Some(s) = start {
                ctx.memory_mut(|m| m.request_focus(enabled[s].1));
            }
            st.focus = None;
        }
        let ids: Vec<Id> = enabled.iter().map(|e| e.1).collect();
        let focused = ctx.memory(egui::Memory::focused);
        if let Some(f) = focused.and_then(|focused| ids.iter().position(|&i| i == focused)) {
            let entry_of: Vec<usize> = enabled.iter().map(|e| e.0).collect();
            let ta_key = fr.id.with(("typeahead", level));
            let mut ta = Typeahead::load(ctx, ta_key);
            if let Some(i) = ta.update(rt.core(), Some(f), enabled.iter().map(|e| e.2)) {
                ctx.memory_mut(|m| m.request_focus(ids[i]));
            }
            ta.store(ctx, ta_key);
            *key_level = Some((level, f, ids, entry_of));
        }
    }

    fn handle_keys(
        &self,
        rt: &Rt,
        fr: &Frame<'_>,
        st: &mut MenuState,
        out: &mut Outcome,
        key_level: Option<KeyLevel>,
        level_ids: &[Vec<RowRect>],
    ) {
        let ctx = fr.ctx;
        // Escape closes the level whose row held focus when the pass began, else the whole menu.
        let escape_level = level_ids.iter().enumerate().rev().find_map(|(l, rows)| {
            rows.iter()
                .any(|r| egui_mineral::kit::layer::consume_escape_from(ctx, r.1))
                .then_some(l)
        });
        match escape_level {
            Some(l) if l > 0 => Self::close_level(ctx, st, level_ids, l),
            Some(_) => Self::dismiss(st, out),
            None if st.open && egui_mineral::kit::layer::consume_escape(ctx) => Self::dismiss(st, out),
            None => {}
        }
        // <https://www.w3.org/WAI/ARIA/apg/patterns/menubar/>, on the level holding focus.
        if let Some((l, f, ids, entry_of)) = key_level {
            RovingFocus::new(Axis::Vertical).update(rt.core(), &ids, |_| true);
            kit::interact::lock_arrows(ctx, ids[f], kit::interact::Arrows::Both);
            let fwd = fr.dir.forward_key();
            let entries = self.level_entries(&st.path, l).unwrap_or(&[]);
            for key in kit::interact::plain_key_presses(ctx) {
                let is_parent = matches!(entries.get(entry_of[f]), Some(Entry::Item(i)) if i.has_submenu());
                match key {
                    k if k == fwd && is_parent => {
                        st.path.truncate(l);
                        st.path.push(entry_of[f]);
                        st.focus = Some((l + 1, FocusTarget::SelectedOrFirst));
                    }
                    Key::ArrowLeft | Key::ArrowRight if l > 0 => {
                        if key != fwd {
                            Self::close_level(ctx, st, level_ids, l);
                        }
                    }
                    Key::ArrowLeft | Key::ArrowRight => out.nav = if key == fwd { 1 } else { -1 },
                    _ => {}
                }
            }
        } else if st.open && st.focus.is_none() {
            // Opened by pointer, nothing focused yet.
            for key in kit::interact::plain_key_presses(ctx) {
                st.focus = match key {
                    Key::ArrowDown => Some((0, FocusTarget::SelectedOrFirst)),
                    Key::ArrowUp => Some((0, FocusTarget::Last)),
                    _ => continue,
                };
                #[expect(clippy::disallowed_methods, reason = "one pass to focus the first or last item")]
                ctx.request_repaint();
            }
        }
    }

    fn dismiss(st: &mut MenuState, out: &mut Outcome) {
        st.open = false;
        out.closed = true;
        out.dismissed = Some(Dismiss::Escape);
    }

    fn close_level(ctx: &Context, st: &mut MenuState, rows: &[Vec<(usize, Id, Rect)>], level: usize) {
        if let Some(parent) = st.path.get(level - 1).copied()
            && let Some(&(_, id, _)) = rows.get(level - 1).and_then(|r| r.iter().find(|r| r.0 == parent))
        {
            ctx.memory_mut(|m| m.request_focus(id));
        }
        st.path.truncate(level - 1);
        st.pending.cancel();
    }

    pub(crate) fn emit_trigger(
        &self,
        ctx: &Context,
        anchor: &Response,
        open: bool,
        role: Role,
        kind: &'static str,
        access_key: Option<Mnemonic>,
    ) {
        ctx.accesskit_node_builder(anchor.id, |b| {
            b.set_expanded(open);
            b.set_has_popup(HasPopup::Menu);
        });
        if !self.emit_anchor || !kit::nodes::wanted(ctx, None) {
            return;
        }
        let mut node = SemanticNode::new(StableId::of(&self.salt, kind), role, &self.label)
            .clickable(Availability::disabled_if(self.disabled));
        node.state.expanded = Some(open);
        node.access_key = access_key;
        node.rect = anchor.rect;
        node.widget_id = Some(anchor.id);
        kit::nodes::push(ctx, node);
    }
}

type RowRect = (usize, Id, Rect);
type KeyLevel = (usize, usize, Vec<Id>, Vec<usize>);

struct Frame<'a> {
    ctx: &'a Context,
    id: Id,
    t: &'a Tokens,
    dir: Direction,
    pointer: Option<egui::Pos2>,
    apex: Option<egui::Pos2>,
    moved: bool,
}

impl<'a> Frame<'a> {
    fn new(ctx: &'a Context, id: Id, t: &'a Tokens, dir: Direction) -> Self {
        let pointer = ctx.input(|i| i.pointer.hover_pos());
        let delta = ctx.input(|i| i.pointer.delta());
        Self {
            ctx,
            id,
            t,
            dir,
            pointer,
            apex: pointer.map(|p| p - delta),
            moved: delta != egui::Vec2::ZERO,
        }
    }
}

struct LevelRun<'a, 'e> {
    fr: &'a Frame<'a>,
    entries: &'e [Entry],
    level: usize,
    base: usize,
    open_child: Option<usize>,
    st: &'a mut MenuState,
    out: &'a mut Outcome,
    rows: Vec<RowRect>,
    enabled: Vec<(usize, Id, &'e str)>,
}

#[derive(Clone, Copy)]
struct LevelFocus<'a> {
    level: usize,
    entries: &'a [Entry],
    enabled: &'a [(usize, Id, &'a str)],
}

struct MenuRow<'a> {
    salt: KeyedSalt,
    key: StableId,
    item: &'a MenuItem,
    width: f32,
    style: MenuColors,
    in_group: InGroup,
    expanded: Option<bool>,
}

impl MenuRow<'_> {
    // Compose MenuDefaults.itemShape(index, count), selectedItemShape
    fn shape(&self, t: &Tokens) -> CornerRadius {
        if !self.style.expressive() {
            return CornerRadius::ZERO;
        }
        let rad = |r: ShapeRole| r.scale().radius(&t.shape);
        if self.item.selected {
            return CornerRadius::same(rad(segmented_menu::ITEM_SELECTED_SHAPE));
        }
        let (outer, inner) = (
            rad(segmented_menu::ITEM_FIRST_CHILD_SHAPE),
            rad(segmented_menu::ITEM_SHAPE),
        );
        match self.in_group {
            InGroup::Alone | InGroup::Middle => CornerRadius::same(inner),
            InGroup::First => vertical_radius(outer, inner),
            InGroup::Last => vertical_radius(inner, outer),
        }
    }
}

fn place_rows(slots: &mut [(SlotKind, Rect)], rows: &mut Vec<usize>) {
    let n = rows.len();
    for (k, &si) in rows.iter().enumerate() {
        let pos = match (n, k) {
            (1, _) => InGroup::Alone,
            (_, 0) => InGroup::First,
            (n, k) if k + 1 == n => InGroup::Last,
            _ => InGroup::Middle,
        };
        if let SlotKind::Row(_, p) = &mut slots[si].0 {
            *p = pos;
        }
    }
    rows.clear();
}

impl MineralWidget for MenuRow<'_> {
    type Kit = Material;
    type State = ();
    type Output = ();

    egui_mineral::kit::widget_meta!(salt);

    fn enabled(&self) -> bool {
        !self.item.disabled
    }

    fn layout(&self, ui: &MineralUi<'_, Material>, constraints: Constraints, (): &Self::State) -> Measure {
        constraints
            .constrain(vec2(self.width, self.style.metrics(ui.density()).height))
            .into()
    }

    fn update(&mut self, ui: &mut MineralUi<'_, Material>, frame: &WidgetFrame<'_>, (): &mut ()) -> WidgetResponse {
        let rt = ui.rt();
        let t = ui.tokens();
        let item = self.item;
        let m = self.style.metrics(ui.density());
        let response = &frame.response;
        let rect = frame.zones.visual.shrink2(vec2(m.inset, 0.0));
        let i = parts::press(ui, frame, Availability::disabled_if(item.disabled)).interaction;
        let col = self.style.colors(&t);
        let (text, icon) = match (item.disabled, item.selected) {
            (true, _) => (col.disabled_label, col.disabled_icon),
            (false, true) => (col.selected_label, col.selected_icon),
            (false, false) => (col.label, col.icon),
        };
        let r = self.shape(&t);
        let p = ui.painter();
        if item.selected {
            p.rect_filled(rect, r, col.selected_container);
        } else if self.expanded == Some(true) {
            p.rect_filled(rect, r, common::with_alpha(text, t.state.hover));
        }
        indication::paint(ui, response, rect, r, text, i, &t);
        let dir = ui.i18n().direction();
        let pad = defaults::menu::ITEM_HORIZONTAL_PADDING;
        // Logical x of a `w` wide part starting `from_start` into the row.
        let x = |from_start: f32, w: f32| dir.start_x(rect, Rangef::new(from_start, from_start + w));
        let cy = rect.center().y;
        let mut off = pad;
        let check = Leading::Icon(ui.icons().check);
        let leading = item
            .leading
            .as_ref()
            .or((self.style.expressive() && item.selected).then_some(&check));
        if let Some(lead) = leading {
            lead.paint(p, pos2(x(off, m.icon) + m.icon / 2.0, cy), m.icon, icon);
            off += m.icon + m.gap;
        }
        let label = parts::type_galley(
            rt.core(),
            p,
            &item.label,
            defaults::menu::ITEM_FONT,
            text,
            item.mnemonic,
        );
        p.galley(pos2(x(off, label.size().x), cy - label.size().y / 2.0), label, text);
        let end = rect.width() - pad;
        let trailing_icon = match item.trailing {
            Some(MenuTrailing::Shortcut(s)) => {
                let g = parts::type_galley(
                    rt.core(),
                    p,
                    &shortcut::label(ui.ctx(), &s),
                    defaults::menu::ITEM_FONT,
                    icon,
                    None,
                );
                p.galley(pos2(x(end - g.size().x, g.size().x), cy - g.size().y / 2.0), g, icon);
                None
            }
            Some(MenuTrailing::Icon(glyph)) => Some(glyph),
            None => item.has_submenu().then_some(match dir {
                Direction::Ltr => ui.icons().arrow_right,
                Direction::Rtl => ui.icons().arrow_left,
            }),
        };
        if let Some(icon_id) = trailing_icon {
            common::icon(
                p,
                pos2(x(end - m.icon, m.icon) + m.icon / 2.0, cy),
                icon_id,
                m.icon,
                icon,
            );
        }
        WidgetResponse::new(response.clone(), ())
    }

    fn semantics(&self, _: &I18n, _: &WidgetResponse, (): &()) -> Option<SemanticNode> {
        let item = self.item;
        let mut n = SemanticNode::new(self.key.clone(), Role::MenuItem, &item.label)
            .clickable(Availability::disabled_if(item.disabled));
        n.shortcut = match item.trailing {
            Some(MenuTrailing::Shortcut(s)) => Some(s),
            _ => None,
        };
        n.access_key = item.mnemonic;
        n.state.selected = item.selected.then_some(true);
        n.state.expanded = self.expanded;
        Some(n)
    }
}

fn dropdown_id(anchor: &Response) -> egui::Id {
    anchor.id.with("mineral::menu")
}

fn context_id(target: &Response) -> egui::Id {
    target.id.with("mineral::context_menu")
}
