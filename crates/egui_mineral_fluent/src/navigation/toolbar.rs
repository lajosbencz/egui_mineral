use egui::{Frame, Margin, UiBuilder, vec2};
use egui_mineral::kit::choice::Choice;
use egui_mineral::select::ItemKey;
use egui_mineral::{
    action::{ActionValue, NoAction, Widen},
    focus::{self as roving, RovingFocus},
    layout::Axis,
    semantic::{self, AgentKey, KeyedSalt, Role, SemanticNode},
    widget::MineralUi,
};

use crate::FluentExt as _;
use crate::{
    Fluent, Icon, Rt, Size,
    actions::{
        ButtonAppearance, PushButton,
        button::{Metrics, Mode},
    },
    spec::{defaults::toolbar as d, tokens::geometry as g},
    surfaces::Divider,
};

use super::{
    Menu, MenuItem, MenuUi,
    overflow::{self, Overflow, OverflowItem, OverflowSpace},
};

#[derive(Clone, Debug)]
struct Base {
    label: String,
    icon: Option<Icon>,
    label_visible: bool,
    appearance: ButtonAppearance,
    disabled: bool,
    salt: Option<KeyedSalt>,
}

impl Base {
    fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            icon: None,
            label_visible: true,
            appearance: ButtonAppearance::Subtle,
            disabled: false,
            salt: None,
        }
    }
}

macro_rules! item_setters {
    ($ty:ty) => {
        impl $ty {
            /// `icon` slot before the label; filled while hovered or checked.
            pub fn icon(mut self, icon: impl Into<Icon>) -> Self {
                self.0.icon = Some(icon.into());
                self
            }

            /// `false` with an icon is the icon-only button; the label stays the accessible name.
            pub fn label_visible(mut self, visible: bool) -> Self {
                self.0.label_visible = visible;
                self
            }

            /// Default [`ButtonAppearance::Subtle`].
            pub fn appearance(mut self, appearance: ButtonAppearance) -> Self {
                self.0.appearance = appearance;
                self
            }

            /// Identity within the toolbar (egui id and agent key part), stable when items are inserted
            /// before it; default the button's value, else its index.
            pub fn salt(mut self, id_salt: impl Into<KeyedSalt>) -> Self {
                self.0.salt = Some(id_salt.into());
                self
            }

            /// Greys out the button and ignores input.
            pub fn disabled(mut self, disabled: bool) -> Self {
                self.0.disabled = disabled;
                self
            }
        }
    };
}

/// Fluent `ToolbarButton`: a subtle [`PushButton`] in a [`Toolbar`].
#[derive(Debug)]
#[must_use]
pub struct ToolbarButton(Base);

impl ToolbarButton {
    /// Visible text and accessible name.
    pub fn new(label: impl Into<String>) -> Self {
        Self(Base::new(label))
    }
}

item_setters!(ToolbarButton);

impl From<&str> for ToolbarButton {
    fn from(s: &str) -> Self {
        Self::new(s)
    }
}

/// Fluent `ToolbarToggleButton`: a click flips the bound value.
#[derive(Debug)]
#[must_use]
pub struct ToolbarToggleButton<'a> {
    base: Base,
    checked: &'a mut bool,
}

impl<'a> ToolbarToggleButton<'a> {
    /// Visible text and accessible name; `checked` flips on click.
    pub fn new(label: impl Into<String>, checked: &'a mut bool) -> Self {
        Self {
            base: Base::new(label),
            checked,
        }
    }

    /// `icon` slot before the label; filled while hovered or checked.
    pub fn icon(mut self, icon: impl Into<Icon>) -> Self {
        self.base.icon = Some(icon.into());
        self
    }

    /// `false` with an icon is the icon-only button; the label stays the accessible name.
    pub fn label_visible(mut self, visible: bool) -> Self {
        self.base.label_visible = visible;
        self
    }

    /// Default [`ButtonAppearance::Subtle`].
    pub fn appearance(mut self, appearance: ButtonAppearance) -> Self {
        self.base.appearance = appearance;
        self
    }

    /// Identity within the toolbar (egui id and agent key part), stable when items are inserted
    /// before it; default its index.
    pub fn salt(mut self, id_salt: impl Into<KeyedSalt>) -> Self {
        self.base.salt = Some(id_salt.into());
        self
    }

    /// Greys out the button and ignores input.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.base.disabled = disabled;
        self
    }
}

/// Fluent `ToolbarRadioButton`: one option of a [`ToolbarRadioGroup`].
#[derive(Debug)]
#[must_use]
pub struct ToolbarRadioButton(Base);

impl ToolbarRadioButton {
    /// Visible text and accessible name.
    pub fn new(label: impl Into<String>) -> Self {
        Self(Base::new(label))
    }
}

item_setters!(ToolbarRadioButton);

impl From<&str> for ToolbarRadioButton {
    fn from(s: &str) -> Self {
        Self::new(s)
    }
}

/// Radio buttons sharing one bound value (Fluent `checkedValues` name).
#[derive(Debug)]
#[must_use]
pub struct ToolbarRadioGroup<'a, K> {
    selected: Choice<'a, K>,
    buttons: Vec<ToolbarRadioButton>,
}

impl<'a, K: ItemKey> ToolbarRadioGroup<'a, K> {
    /// Checks the button whose value equals `selected`, none when no button matches.
    pub fn new(selected: &'a mut K) -> Self {
        Self {
            selected: Choice::new(selected),
            buttons: Vec::new(),
        }
    }

    /// Appends one radio button carrying `value`.
    pub fn item(mut self, value: K, button: impl Into<ToolbarRadioButton>) -> Self {
        self.selected.push(value);
        self.buttons.push(button.into());
        self
    }

    /// Appends `(value, button)` pairs.
    pub fn items(mut self, options: impl IntoIterator<Item = (K, impl Into<ToolbarRadioButton>)>) -> Self {
        for (value, button) in options {
            self = self.item(value, button);
        }
        self
    }
}

// A radio group with its value type erased, so one toolbar holds groups of different types.
trait Radios {
    fn selected(&self) -> Option<usize>;
    fn select(&mut self, index: usize) -> bool;
}

impl<K: ItemKey> Radios for Choice<'_, K> {
    fn selected(&self) -> Option<usize> {
        self.index()
    }

    fn select(&mut self, index: usize) -> bool {
        Choice::select(self, index)
    }
}

struct RadioEntry<'a> {
    selected: Box<dyn Radios + 'a>,
    buttons: Vec<ToolbarRadioButton>,
}

egui_mineral::kit::opaque_debug!(RadioEntry<'_>);

#[derive(Debug)]
enum Entry<'a, A> {
    Button(A, ToolbarButton),
    Toggle(ToolbarToggleButton<'a>),
    Radio(RadioEntry<'a>),
    Divider,
    Group(Vec<Entry<'a, A>>),
}

impl<'a, A> Entry<'a, A> {
    fn widen<B>(self) -> Entry<'a, B>
    where
        A: Widen<B>,
    {
        match self {
            Self::Button(v, b) => Entry::Button(v.widen(), b),
            Self::Toggle(t) => Entry::Toggle(t),
            Self::Radio(r) => Entry::Radio(r),
            Self::Divider => Entry::Divider,
            Self::Group(g) => Entry::Group(g.into_iter().map(Entry::widen).collect()),
        }
    }
}

fn widen_all<A: Widen<B>, B>(entries: Vec<Entry<'_, A>>) -> Vec<Entry<'_, B>> {
    entries.into_iter().map(Entry::widen).collect()
}

/// Fluent `ToolbarGroup`: items grouped in one toolbar; it adds no visuals. Built by [`Toolbar::group`].
#[derive(Debug)]
#[must_use]
pub struct ToolbarGroup<'a, A = NoAction>(Vec<Entry<'a, A>>);

macro_rules! entries {
    ($ty:ident, $field:tt) => {
        /// Appends the button reporting `value`.
        pub fn action<B: ItemKey>(self, value: B, button: impl Into<ToolbarButton>) -> $ty<'a, B>
        where
            A: Widen<B>,
        {
            let mut next = self.retype();
            next.$field.push(Entry::Button(value, button.into()));
            next
        }

        /// Appends `(value, button)` pairs.
        pub fn actions<B: ItemKey>(self, buttons: impl IntoIterator<Item = (B, impl Into<ToolbarButton>)>) -> $ty<'a, B>
        where
            A: Widen<B>,
        {
            let mut next = self.retype();
            next.$field
                .extend(buttons.into_iter().map(|(v, b)| Entry::Button(v, b.into())));
            next
        }

        /// Appends a toggle button.
        pub fn toggle(mut self, button: ToolbarToggleButton<'a>) -> Self {
            self.$field.push(Entry::Toggle(button));
            self
        }

        /// Appends radio buttons.
        pub fn radio_group<K: ItemKey + 'a>(mut self, group: ToolbarRadioGroup<'a, K>) -> Self {
            self.$field.push(Entry::Radio(RadioEntry {
                selected: Box::new(group.selected),
                buttons: group.buttons,
            }));
            self
        }

        /// Appends a [`Divider`] across the toolbar.
        pub fn divider(mut self) -> Self {
            self.$field.push(Entry::Divider);
            self
        }

        /// Appends a group; `build` fills it like the toolbar, starting from the values reported so far.
        pub fn group<B: ActionValue>(self, build: impl FnOnce(ToolbarGroup<'a, A>) -> ToolbarGroup<'a, B>) -> $ty<'a, B>
        where
            A: Widen<B>,
        {
            let group = build(ToolbarGroup(Vec::new()));
            let mut next: $ty<'a, B> = self.retype();
            next.$field.push(Entry::Group(group.0));
            next
        }
    };
}

impl<'a, A: ActionValue> ToolbarGroup<'a, A> {
    fn retype<B>(self) -> ToolbarGroup<'a, B>
    where
        A: Widen<B>,
    {
        ToolbarGroup(widen_all(self.0))
    }

    entries!(ToolbarGroup, 0);
}

enum Kind<'a, A> {
    Plain(A),
    Toggle(&'a mut bool),
    Radio { group: usize, value: usize },
}

enum Slot<'a, A> {
    Divider,
    Item(Box<Base>, Kind<'a, A>, usize),
}

fn flatten<'a, A>(entries: Vec<Entry<'a, A>>, slots: &mut Vec<Slot<'a, A>>, groups: &mut Vec<Box<dyn Radios + 'a>>) {
    for entry in entries {
        match entry {
            Entry::Button(v, b) => slots.push(Slot::Item(Box::new(b.0), Kind::Plain(v), 0)),
            Entry::Toggle(t) => slots.push(Slot::Item(Box::new(t.base), Kind::Toggle(t.checked), 0)),
            Entry::Radio(g) => {
                let group = groups.len();
                groups.push(g.selected);
                slots.extend(
                    g.buttons
                        .into_iter()
                        .enumerate()
                        .map(|(value, b)| Slot::Item(Box::new(b.0), Kind::Radio { group, value }, 0)),
                );
            }
            Entry::Divider => slots.push(Slot::Divider),
            Entry::Group(g) => flatten(g, slots, groups),
        }
    }
}

// useToolbarStyles.styles.ts `useStyles`
const fn padding(size: Size, vertical: bool) -> (f32, f32) {
    match (size, vertical) {
        (_, true) | (Size::Medium, _) => d::PADDING_MEDIUM,
        (Size::Small, false) => d::PADDING_SMALL,
        (Size::Large, false) => d::PADDING_LARGE,
    }
}

fn button<'a>(base: Base, salt: KeyedSalt) -> PushButton<'a> {
    let mut b = PushButton::new(salt, base.label)
        .appearance(base.appearance)
        .label_visible(base.label_visible)
        .disabled(base.disabled);
    if let Some(icon) = base.icon {
        b = b.icon(icon);
    }
    b
}

// useToolbarDividerStyles.styles.ts `root`: padding on both sides of the line
const DIVIDER_WIDTH: f32 = 2.0 * d::DIVIDER_PADDING + g::STROKE_WIDTH_THIN;

struct Run<A> {
    ids: Vec<egui::Id>,
    clicked: Option<A>,
    changed: bool,
    stop: usize,
}

impl<A: ActionValue> Run<A> {
    fn item<'a>(
        &mut self,
        rt: &Rt,
        ui: &mut egui::Ui,
        salt: &KeyedSalt,
        (base, kind, index): (Base, Kind<'a, A>, usize),
        groups: &mut [Box<dyn Radios + 'a>],
    ) {
        let pos = self.ids.len();
        let off = base.disabled;
        let item_salt = base.salt.clone().unwrap_or_else(|| match &kind {
            Kind::Plain(v) => salt.join(&v.salt()),
            _ => salt.child(index),
        });
        let mut ran = None;
        let mut b = button(base, item_salt);
        b.tab_stop = Some(roving::TabStop::at(pos, self.stop));
        let radio = match kind {
            Kind::Plain(v) => {
                ran = Some(v);
                None
            }
            Kind::Toggle(checked) => {
                b = b.bind(checked);
                None
            }
            Kind::Radio { group, value } => {
                b.radio = true;
                if groups.get(group).is_some_and(|g| g.selected() == Some(value)) {
                    b.mode = Mode::Checked;
                }
                Some((group, value))
            }
        };
        let r = rt.ui(ui).mineral(b);
        let clicked = r.response().clicked() && !off;
        if clicked && ran.is_some() {
            self.clicked = ran;
        }
        match radio {
            Some((group, value)) if clicked => self.pick(groups, group, value),
            None if r.changed() => self.changed = true,
            _ => {}
        }
        self.ids.push(r.id);
    }

    fn overflow_menu<'a>(
        &mut self,
        rt: &Rt,
        ui: &MineralUi<'_, Fluent>,
        (menu, owner): (Menu, egui::Id),
        trigger: &egui::Response,
        hidden: Vec<Slot<'a, A>>,
        groups: &mut [Box<dyn Radios + 'a>],
    ) {
        let mut open = overflow::is_open(ui.ctx(), owner);
        let r = menu.show(rt, trigger, &mut open, |m: &mut MenuUi<'_, '_, A>| {
            for slot in hidden {
                match slot {
                    Slot::Divider => m.divider(),
                    Slot::Item(base, kind, n) => self.row(m, (*base, kind, n), groups),
                }
            }
        });
        if r.clicked.is_some() {
            self.clicked = r.clicked;
        }
        overflow::set_open(ui.ctx(), owner, open);
    }

    fn pick(&mut self, groups: &mut [Box<dyn Radios + '_>], group: usize, value: usize) {
        if let Some(g) = groups.get_mut(group) {
            self.changed |= g.select(value);
        }
    }

    fn row<'a>(
        &mut self,
        m: &mut MenuUi<'_, '_, A>,
        (base, kind, _): (Base, Kind<'a, A>, usize),
        groups: &mut [Box<dyn Radios + 'a>],
    ) {
        let mut item = MenuItem::new(base.label).disabled(base.disabled);
        if let Some(icon) = base.icon {
            item = item.icon(icon);
        }
        match kind {
            Kind::Plain(v) => {
                m.item(v, item);
            }
            Kind::Toggle(checked) => {
                if m.checkbox_item(item, checked).clicked {
                    self.changed = true;
                }
            }
            Kind::Radio { group, value } => {
                let Some(selected) = groups.get_mut(group) else { return };
                let mut current = selected.selected();
                if m.radio_item(item, &mut current, Some(value)).clicked {
                    self.changed |= selected.select(value);
                }
            }
        }
    }
}

/// Result of showing a [`Toolbar`].
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct ToolbarResponse<A = NoAction> {
    /// Spans every item; `mark_changed` when `changed`.
    pub response: egui::Response,
    /// Agrees with `response.changed()`; a toggle or radio value changed.
    pub changed: bool,
    /// Value of the button clicked this frame, also when it sits in the overflow menu; toggles and radio buttons
    /// report through `changed`.
    pub clicked: Option<A>,
}

/// Fluent `Toolbar`: a row or column of buttons, toggles, radio buttons and dividers. One Tab stop;
/// arrow keys along the axis move focus (wrapping), Home and End jump.
#[derive(Debug)]
#[must_use = "call .show(..)"]
pub struct Toolbar<'a, A = NoAction> {
    salt: KeyedSalt,
    key: AgentKey,
    label: String,
    size: Size,
    vertical: bool,
    overflow: Option<Overflow>,
    items: Vec<Entry<'a, A>>,
}

impl Toolbar<'_> {
    /// `label` names the toolbar for assistive technology; agent key `<id_salt>-toolbar`.
    pub fn new(id_salt: impl Into<KeyedSalt>, label: impl Into<String>) -> Self {
        Self {
            salt: id_salt.into(),
            key: egui_mineral::agent_key!("toolbar"),
            label: label.into(),
            size: Size::Medium,
            vertical: false,
            overflow: None,
            items: Vec::new(),
        }
    }
}

impl<'a, A: ActionValue> Toolbar<'a, A> {
    fn retype<B>(self) -> Toolbar<'a, B>
    where
        A: Widen<B>,
    {
        Toolbar {
            salt: self.salt,
            key: self.key,
            label: self.label,
            size: self.size,
            vertical: self.vertical,
            overflow: self.overflow,
            items: widen_all(self.items),
        }
    }

    /// Padding family; default [`Size::Medium`]. Buttons stay medium.
    pub fn size(mut self, size: Size) -> Self {
        self.size = size;
        self
    }

    /// Default [`Axis::Horizontal`]; [`Axis::Vertical`] stacks the items in a column and does not overflow.
    pub fn axis(mut self, axis: Axis) -> Self {
        self.vertical = axis == Axis::Vertical;
        self
    }

    /// Moves the items that do not fit the available width into a menu behind a trailing button.
    pub fn overflow(mut self, overflow: Overflow) -> Self {
        self.overflow = Some(overflow);
        self
    }

    entries!(Toolbar, items);

    crate::widget::setters! {
        /// Replaces the `<id_salt>-toolbar` agent key.
        agent_key(key)
    }

    /// Shows the toolbar.
    pub fn show(self, ui: &mut MineralUi<'_, Fluent>) -> ToolbarResponse<A> {
        let rt = ui.rt();
        let Self {
            salt,
            key,
            label,
            size,
            vertical,
            overflow,
            items,
        } = self;
        let dir = rt.i18n().direction();
        let (block, inline) = padding(size, vertical);
        let mut slots = Vec::new();
        let mut groups = Vec::new();
        flatten(items, &mut slots, &mut groups);
        number(&mut slots);
        let (shown, hidden) = plan(ui, &salt, overflow.as_ref().filter(|_| !vertical), slots, inline);
        let more = !hidden.is_empty();
        let mut disabled = disabled_flags(&shown);
        disabled.extend(more.then_some(false));
        let enabled = |i: usize| disabled.get(i).is_some_and(|d| !d);
        let id = ui.make_persistent_id(salt.id_salt());
        let stop = roving::tab_stop(ui.ctx(), id, None, &roving::Items::new(disabled.len(), enabled));
        let m = Metrics::button(Size::Medium, false);
        let row_height = m.pad_top + m.pad_bottom + m.line_height + 2.0 * g::STROKE_WIDTH_THIN;
        let mut run = Run {
            ids: Vec::with_capacity(disabled.len()),
            clicked: None,
            changed: false,
            stop,
        };
        let mut trigger = None;
        let body = |ui: &mut egui::Ui| {
            ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
            for (at, slot) in shown.into_iter().enumerate() {
                match slot {
                    Slot::Divider => show_divider(rt, ui, salt.child(("divider", at)), vertical),
                    Slot::Item(base, kind, n) => run.item(rt, ui, &salt, (*base, kind, n), &mut groups),
                }
            }
            if let Some(o) = overflow.as_ref().filter(|_| more) {
                let mut b = overflow::trigger(salt.child("overflow"), o.label(), &rt.icons());
                b.tab_stop = Some(roving::TabStop::at(run.ids.len(), stop));
                let r = rt.ui(ui).mineral(b);
                run.ids.push(r.id);
                trigger = Some(r.response().clone());
            }
        };
        let group = ui
            .push_id(salt.id_salt(), |ui| {
                Frame::new()
                    .inner_margin(Margin::symmetric(inline as i8, block as i8))
                    .show(ui, |ui| {
                        let builder = if vertical {
                            UiBuilder::new().layout(dir.column())
                        } else {
                            crate::widget::row(ui, dir, row_height)
                        };
                        ui.scope_builder(builder, body)
                    })
                    .response
            })
            .inner;
        if let (Some(trigger), Some(o)) = (trigger, overflow.as_ref()) {
            let menu = Menu::new(salt.child("overflow-menu"), o.label());
            run.overflow_menu(rt, ui, (menu, id), &trigger, hidden, &mut groups);
        }
        let axis = if vertical { Axis::Vertical } else { Axis::Horizontal };
        RovingFocus::new(axis)
            .wrap(d::WRAP)
            .update_group(ui.core(), id, &run.ids, enabled);
        semantic::emit(ui.ctx(), Some(group.id), group.rect, || {
            SemanticNode::new(key.id(&salt), Role::Toolbar, &label)
        });
        let mut response = group;
        if run.changed {
            response.mark_changed();
        }
        ToolbarResponse {
            response,
            changed: run.changed,
            clicked: run.clicked,
        }
    }
}

// Slots split into shown and overflowed ones; the trailing menu button's width is reserved.
fn plan<'s, A>(
    ui: &MineralUi<'_, Fluent>,
    salt: &KeyedSalt,
    overflow: Option<&Overflow>,
    slots: Vec<Slot<'s, A>>,
    inline: f32,
) -> (Vec<Slot<'s, A>>, Vec<Slot<'s, A>>) {
    let Some(o) = overflow else {
        return (slots, Vec::new());
    };
    let items: Vec<OverflowItem> = slots
        .iter()
        .map(|s| match s {
            Slot::Divider => OverflowItem::new(DIVIDER_WIDTH),
            Slot::Item(b, ..) => OverflowItem::new(button(b.as_ref().clone(), salt.clone()).natural(ui).x),
        })
        .collect();
    let menu = overflow::trigger(salt.clone(), o.label(), &ui.icons()).natural(ui).x;
    let fit = o.fit(
        &items,
        OverflowSpace {
            available: ui.available_width() - 2.0 * inline,
            menu,
        },
    );
    let (mut shown, mut hidden) = (Vec::new(), Vec::new());
    for (slot, visible) in slots.into_iter().zip(fit) {
        if visible { &mut shown } else { &mut hidden }.push(slot);
    }
    (shown, hidden)
}

fn number<A>(slots: &mut [Slot<'_, A>]) {
    let mut index = 0;
    for slot in slots {
        if let Slot::Item(.., n) = slot {
            *n = index;
            index += 1;
        }
    }
}

fn disabled_flags<A>(slots: &[Slot<'_, A>]) -> Vec<bool> {
    slots
        .iter()
        .filter_map(|s| match s {
            Slot::Item(b, ..) => Some(b.disabled),
            Slot::Divider => None,
        })
        .collect()
}

fn show_divider(rt: &Rt, ui: &mut egui::Ui, salt: KeyedSalt, vertical: bool) {
    let axis = if vertical { Axis::Horizontal } else { Axis::Vertical };
    Frame::new()
        .inner_margin(Margin::symmetric(d::DIVIDER_PADDING as i8, 0))
        .show(ui, |ui| {
            rt.ui(ui).mineral(Divider::new(salt).axis(axis));
        });
}
