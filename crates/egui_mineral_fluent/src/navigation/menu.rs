use egui::{Id, Key, Margin, Modifiers, Rangef, Rect, Response, Sense, Stroke, Ui, pos2, text::LayoutJob, vec2};
use egui_mineral::focus::Items;
use egui_mineral::select::RadioMark;
use egui_mineral::{
    action::{ActionValue, NoAction},
    anchor::{Align, Anchor, Side},
    focus::RovingFocus,
    i18n::I18n,
    kit::{
        self,
        menu::{MenuNav, Walk},
        state::TempState,
    },
    layout::{Axis, Constraints, Edge, Measure, MinTarget},
    overlay::{self, Dismiss, Popup},
    semantic::{self, AgentKey, KeyedSalt, Role, SemanticNode},
    widget::{Availability, Interaction, MineralUi, MineralWidget, WidgetFrame, WidgetResponse},
};

use crate::spec::ColorToken::{
    ColorNeutralBackground1, ColorNeutralBackground1Hover, ColorNeutralBackground1Pressed, ColorNeutralForeground2,
    ColorNeutralForeground2BrandSelected, ColorNeutralForeground2Hover, ColorNeutralForeground2Pressed,
    ColorNeutralForeground3, ColorNeutralForeground3Hover, ColorNeutralForeground3Pressed,
    ColorNeutralForegroundDisabled, ColorNeutralStroke1, ColorNeutralStroke2,
};
use crate::{
    Fluent, FluentExt, Icon, Rt,
    spec::{
        defaults::menu as d,
        tokens::{geometry as g, typography as ty},
    },
    surfaces::overlay::{Look, framed},
    theme::{Elevation, Radius, Tokens},
    typography::{self, TypeRamp},
    widget::{
        focus as ring, paint,
        state::{Ramp, State},
    },
};

/// Fluent `MenuItem`: the content of a [`MenuUi`] row.
#[derive(Clone, Debug)]
#[must_use]
pub struct MenuItem {
    label: String,
    icon: Option<Icon>,
    secondary: Option<String>,
    disabled: bool,
    salt: Option<KeyedSalt>,
}

impl MenuItem {
    /// Visible text and accessible name.
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            icon: None,
            secondary: None,
            disabled: false,
            salt: None,
        }
    }

    /// `icon` slot; every row of the menu reserves it once one has an icon.
    pub fn icon(mut self, icon: impl Into<Icon>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    /// `secondaryContent` slot: trailing caption text such as a shortcut.
    pub fn secondary_text(mut self, text: impl Into<String>) -> Self {
        self.secondary = Some(text.into());
        self
    }

    crate::widget::setters! {
        item_salt
    }

    crate::widget::setters! {
        /// Not operable.
        disabled
    }
}

impl From<&str> for MenuItem {
    fn from(s: &str) -> Self {
        Self::new(s)
    }
}

impl From<String> for MenuItem {
    fn from(s: String) -> Self {
        Self::new(s)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    Item,
    Checkbox(bool),
    Radio(bool),
    Submenu(bool),
    SplitMain,
}

impl Kind {
    const fn checkmark(self) -> bool {
        matches!(self, Self::Checkbox(_) | Self::Radio(_))
    }

    const fn checked(self) -> bool {
        matches!(self, Self::Checkbox(true) | Self::Radio(true))
    }
}

// What the previous pass saw: the rows reserve slots and stretch to the widest one.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Memo {
    width: f32,
    checkmarks: bool,
    icons: bool,
}

impl TempState for Memo {}

impl Memo {
    const MIN_WIDTH: f32 = d::POPOVER_MIN_WIDTH - 2.0 * d::POPOVER_PADDING;

    const MAX_WIDTH: f32 = d::POPOVER_MAX_WIDTH - 2.0 * d::POPOVER_PADDING;

    const FIRST: Self = Self {
        width: Self::MIN_WIDTH,
        checkmarks: false,
        icons: false,
    };
}

struct Row<'a> {
    salt: KeyedSalt,
    item: &'a MenuItem,
    kind: Kind,
    memo: Memo,
    width: f32,
    radio_mark: RadioMark,
}

pub(super) fn text_width(ui: &MineralUi<'_, Fluent>, text: &str, ramp: TypeRamp) -> f32 {
    let job = LayoutJob::single_section(text.to_owned(), ramp.format(ui.ctx(), egui::Color32::PLACEHOLDER));
    ui.painter().layout_job(job).size().x
}

impl Row<'_> {
    // MenuItem flex row: padding, `gap` between the slots present
    fn natural(ui: &MineralUi<'_, Fluent>, item: &MenuItem, kind: Kind, memo: Memo) -> f32 {
        let slot = d::ICON_SIZE + d::ITEM_GAP;
        let mut w = 2.0 * d::ITEM_PADDING;
        if memo.checkmarks || kind.checkmark() {
            w += slot;
        }
        if memo.icons || item.icon.is_some() {
            w += slot;
        }
        w += text_width(ui, &item.label, TypeRamp::Body1) + 2.0 * d::CONTENT_PADDING;
        if let Some(s) = &item.secondary {
            w += d::ITEM_GAP + text_width(ui, s, TypeRamp::Caption1) + 2.0 * d::CONTENT_PADDING;
        }
        if matches!(kind, Kind::Submenu(_)) {
            w += slot;
        }
        if kind == Kind::SplitMain {
            w += d::SPLIT_TRIGGER_WIDTH;
        }
        w
    }
}

impl Row<'_> {
    fn trailing(&self, ui: &MineralUi<'_, Fluent>, t: &Tokens, s: State, rect: Rect, cy: f32, color: egui::Color32) {
        let dir = ui.i18n().direction();
        let at = |from: f32, w: f32| dir.start_x(rect, Rangef::new(from, from + w));
        let slot = d::ICON_SIZE + d::ITEM_GAP;
        let secondary = Ramp::new(
            ColorNeutralForeground3,
            ColorNeutralForeground3Hover,
            ColorNeutralForeground3Pressed,
            ColorNeutralForegroundDisabled,
        );
        let trailing = rect.width() - d::ITEM_PADDING;
        if matches!(self.kind, Kind::Submenu(_)) {
            let chevron = dir.pick(ui.icons().chevron_right, ui.icons().chevron_left);
            let left = trailing - d::ICON_SIZE;
            paint::icon(
                ui.painter(),
                pos2(at(left, d::ICON_SIZE) + d::ICON_SIZE / 2.0, cy),
                chevron,
                false,
                d::ICON_SIZE,
                color,
            );
        }
        if let Some(text) = &self.item.secondary {
            let end = if matches!(self.kind, Kind::Submenu(_)) {
                trailing - slot
            } else {
                trailing
            };
            let galley = ui.painter().layout_job(LayoutJob::single_section(
                text.clone(),
                typography::format(
                    ui.ctx(),
                    crate::typography::TypeSpec {
                        size: ty::FONT_SIZE_BASE_200,
                        line_height: ty::LINE_HEIGHT_BASE_300,
                        weight: ty::FONT_WEIGHT_REGULAR,
                    },
                    secondary.pick(s, t),
                ),
            ));
            let w = galley.size().x;
            let sx = at(end - d::CONTENT_PADDING - w, w);
            ui.painter()
                .galley(pos2(sx, cy - galley.size().y / 2.0), galley, secondary.pick(s, t));
        }
    }
}

impl MineralWidget for Row<'_> {
    type Kit = Fluent;
    type State = ();
    type Output = ();

    fn id_salt(&self) -> egui::IdSalt {
        self.salt.id_salt()
    }

    fn enabled(&self) -> bool {
        !self.item.disabled
    }

    fn sense(&self) -> Sense {
        Sense::click()
    }

    fn min_target(&self, _: &MineralUi<'_, Self::Kit>) -> MinTarget {
        MinTarget::None
    }

    fn layout(&self, _: &MineralUi<'_, Fluent>, c: Constraints, (): &()) -> Measure {
        c.constrain(vec2(self.width, d::ITEM_MIN_HEIGHT)).into()
    }

    fn update(&mut self, ui: &mut MineralUi<'_, Fluent>, frame: &WidgetFrame<'_>, (): &mut ()) -> WidgetResponse {
        let t = ui.tokens();
        let r = &frame.response;
        let availability = Availability::disabled_if(self.item.disabled);
        kit::interact::hover_cursor(ui.core(), r, availability);
        kit::interact::follow_press(r);
        let i = Interaction::of(r, availability);
        let open = self.kind == Kind::Submenu(true);
        let s = if open && !i.disabled {
            State::Hover
        } else {
            State::of(i)
        };
        let engaged = s.engaged();
        let rect = frame.zones.visual;
        let radius = Radius::Medium.on(rect);
        let fill = Ramp::new(
            ColorNeutralBackground1,
            ColorNeutralBackground1Hover,
            ColorNeutralBackground1Pressed,
            ColorNeutralBackground1,
        );
        let fg = Ramp::new(
            ColorNeutralForeground2,
            ColorNeutralForeground2Hover,
            ColorNeutralForeground2Pressed,
            ColorNeutralForegroundDisabled,
        );
        ui.painter().rect_filled(rect, radius, fill.pick(s, &t));
        let color = fg.pick(s, &t);
        let icon_color = if engaged && !i.disabled {
            t.color(ColorNeutralForeground2BrandSelected)
        } else {
            color
        };
        let dir = ui.i18n().direction();
        let cy = rect.top() + d::ITEM_PADDING + typography::TypeRamp::Body1.line_height() / 2.0;
        let at = |from: f32, w: f32| dir.start_x(rect, Rangef::new(from, from + w));
        let mut x = d::ITEM_PADDING;
        let slot = d::ICON_SIZE + d::ITEM_GAP;
        if self.memo.checkmarks || self.kind.checkmark() {
            let c = pos2(at(x, d::ICON_SIZE) + d::ICON_SIZE / 2.0, cy);
            match self.kind {
                Kind::Radio(true) if self.radio_mark == RadioMark::Dot => {
                    let r = crate::spec::defaults::radio::INDICATOR * crate::spec::defaults::radio::DOT_SCALE / 2.0;
                    ui.painter().circle_filled(c, r, color);
                }
                k if k.checked() => {
                    paint::icon(ui.painter(), c, ui.icons().checkmark, true, d::CHECKMARK_SIZE, color);
                }
                _ => {}
            }
            x += slot;
        }
        if self.memo.icons || self.item.icon.is_some() {
            if let Some(icon) = self.item.icon {
                paint::icon(
                    ui.painter(),
                    pos2(at(x, d::ICON_SIZE) + d::ICON_SIZE / 2.0, cy),
                    icon,
                    engaged && !i.disabled,
                    d::ICON_SIZE,
                    icon_color,
                );
            }
            x += slot;
        }
        let galley = ui.painter().layout_job(LayoutJob::single_section(
            self.item.label.clone(),
            TypeRamp::Body1.format(ui.ctx(), color),
        ));
        let tx = at(x + d::CONTENT_PADDING, galley.size().x);
        ui.painter().galley(pos2(tx, cy - galley.size().y / 2.0), galley, color);
        self.trailing(ui, &t, s, rect, cy, color);
        if i.focused {
            ring::outline(ui.painter(), rect, radius, &t);
        }
        WidgetResponse::new(r.clone(), ()).with_changed(r.clicked() && !self.item.disabled)
    }

    fn semantics(&self, _: &I18n, _: &WidgetResponse, (): &()) -> Option<SemanticNode> {
        let role = match self.kind {
            Kind::Checkbox(_) => Role::MenuItemCheckBox,
            Kind::Radio(_) => Role::MenuItemRadio,
            _ => Role::MenuItem,
        };
        let mut n = SemanticNode::new(
            egui_mineral::agent_key!("menuitem").id(&self.salt),
            role,
            &self.item.label,
        )
        .clickable(Availability::disabled_if(self.item.disabled));
        if self.kind.checkmark() {
            n.state.checked = Some(self.kind.checked().into());
        }
        if let Kind::Submenu(open) = self.kind {
            n.state.expanded = Some(open);
        }
        Some(n)
    }
}

struct SplitTrigger<'a> {
    salt: KeyedSalt,
    label: &'a str,
    open: bool,
    disabled: bool,
}

impl MineralWidget for SplitTrigger<'_> {
    type Kit = Fluent;
    type State = ();
    type Output = ();

    kit::widget_meta!(salt, enabled, no_target);

    fn sense(&self) -> Sense {
        Sense::click()
    }

    fn layout(&self, _: &MineralUi<'_, Fluent>, c: Constraints, (): &()) -> Measure {
        c.constrain(vec2(d::SPLIT_TRIGGER_WIDTH, d::ITEM_MIN_HEIGHT)).into()
    }

    fn update(&mut self, ui: &mut MineralUi<'_, Fluent>, frame: &WidgetFrame<'_>, (): &mut ()) -> WidgetResponse {
        let t = ui.tokens();
        let r = &frame.response;
        let availability = Availability::disabled_if(self.disabled);
        kit::interact::hover_cursor(ui.core(), r, availability);
        kit::interact::follow_press(r);
        let i = Interaction::of(r, availability);
        let s = if self.open && !i.disabled {
            State::Hover
        } else {
            State::of(i)
        };
        let rect = frame.zones.visual;
        let dir = ui.i18n().direction();
        let radius = Radius::Medium.on(rect);
        // splitItemTrigger: the start corners are square
        let flat = |c: egui::CornerRadius| match dir {
            egui_mineral::i18n::Direction::Ltr => egui::CornerRadius { nw: 0, sw: 0, ..c },
            egui_mineral::i18n::Direction::Rtl => egui::CornerRadius { ne: 0, se: 0, ..c },
        };
        let fill = Ramp::new(
            ColorNeutralBackground1,
            ColorNeutralBackground1Hover,
            ColorNeutralBackground1Pressed,
            ColorNeutralBackground1,
        );
        ui.painter().rect_filled(rect, flat(radius), fill.pick(s, &t));
        let color = Ramp::new(
            ColorNeutralForeground2,
            ColorNeutralForeground2Hover,
            ColorNeutralForeground2Pressed,
            ColorNeutralForegroundDisabled,
        )
        .pick(s, &t);
        let at = |from: f32, w: f32| dir.start_x(rect, Rangef::new(from, from + w));
        let cy = rect.center().y;
        ui.painter().rect_filled(
            Rect::from_center_size(
                pos2(at(0.0, g::STROKE_WIDTH_THIN) + g::STROKE_WIDTH_THIN / 2.0, cy),
                vec2(g::STROKE_WIDTH_THIN, d::SPLIT_SEPARATOR_HEIGHT),
            ),
            0,
            t.color(ColorNeutralStroke1),
        );
        let x = g::STROKE_WIDTH_THIN + d::ITEM_GAP;
        let chevron = dir.pick(ui.icons().chevron_right, ui.icons().chevron_left);
        paint::icon(
            ui.painter(),
            pos2(at(x, d::ICON_SIZE) + d::ICON_SIZE / 2.0, cy),
            chevron,
            false,
            d::ICON_SIZE,
            color,
        );
        if i.focused {
            ring::outline(ui.painter(), rect, radius, &t);
        }
        WidgetResponse::new(r.clone(), ()).with_changed(r.clicked() && !self.disabled)
    }

    fn semantics(&self, _: &I18n, response: &WidgetResponse, (): &()) -> Option<SemanticNode> {
        let mut n = SemanticNode::new(
            egui_mineral::agent_key!("menusplit").id(&self.salt),
            Role::MenuItem,
            self.label,
        )
        .clickable(Availability::disabled_if(self.disabled));
        n.state.expanded = Some(self.open);
        n.state.focused = response.has_focus();
        Some(n)
    }
}

/// Result of one [`MenuUi`] row.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct MenuItemResponse<R = ()> {
    /// Focusable row.
    pub response: Response,
    /// Click, Enter or Space on the enabled row this pass.
    pub clicked: bool,
    /// Submenu content while it is open; `()` for other rows.
    pub inner: R,
}

struct Shared<A> {
    close: bool,
    clicked: Option<A>,
    inside: Vec<Rect>,
    radio_mark: RadioMark,
    nav: MenuNav,
    bar: Option<Edge>,
}

impl<A> Default for Shared<A> {
    fn default() -> Self {
        Self {
            close: false,
            clicked: None,
            inside: Vec::new(),
            radio_mark: RadioMark::Check,
            nav: MenuNav::default(),
            bar: None,
        }
    }
}

/// Places the rows of a [`Menu`] or submenu; `A` is the value its [`Self::item`] rows report.
pub struct MenuUi<'u, 's, A = NoAction> {
    ui: &'u mut Ui,
    rt: &'u Rt,
    salt: KeyedSalt,
    memo: Memo,
    seen: Memo,
    depth: usize,
    index: usize,
    rows: Vec<SeenRow>,
    shared: &'s mut Shared<A>,
}

// A row shown this pass, in order.
#[derive(Clone, Copy)]
struct SeenRow {
    id: Id,
    enabled: bool,
    initial: Option<char>,
    submenu: bool,
    // Submenu trigger of a split row.
    split: Option<Id>,
}

kit::opaque_debug!(MenuUi<'_, '_, A>);

impl<A: ActionValue> MenuUi<'_, '_, A> {
    fn row(&mut self, item: &MenuItem, kind: Kind) -> (Response, bool) {
        let i = self.index;
        self.index += 1;
        let salt = item.salt.clone().unwrap_or_else(|| self.salt.child(i));
        let mut mui = self.rt.ui(self.ui);
        let natural = Row::natural(&mui, item, kind, self.memo);
        self.seen.width = self.seen.width.max(natural);
        self.seen.checkmarks |= kind.checkmark();
        self.seen.icons |= item.icon.is_some();
        let mut width = self.memo.width.max(natural).min(d::ITEM_MAX_WIDTH);
        if kind == Kind::SplitMain {
            width -= d::SPLIT_TRIGGER_WIDTH;
        }
        let out = mui.mineral(Row {
            salt,
            item,
            kind,
            memo: self.memo,
            width,
            radio_mark: self.shared.radio_mark,
        });
        let clicked = out.changed();
        let response = out.into_response();
        self.rows.push(SeenRow {
            id: response.id,
            enabled: !item.disabled,
            initial: initial(&item.label),
            submenu: matches!(kind, Kind::Submenu(_)),
            split: None,
        });
        if response.hovered() {
            let submenu = matches!(kind, Kind::Submenu(_)) && !item.disabled;
            self.shared.nav.hover(self.ui.ctx(), self.depth, i, submenu, false);
        }
        (response, clicked)
    }

    fn activate(&mut self, item: &MenuItem, value: A, clicked: bool) {
        if clicked {
            self.shared.close = true;
            if !item.disabled {
                self.shared.clicked = Some(value);
            }
        }
    }

    /// `MenuItem`: activating it closes the whole menu and reports `value` as [`MenuResponse::clicked`].
    pub fn item(&mut self, value: A, item: impl Into<MenuItem>) -> MenuItemResponse {
        let item = item.into();
        let (response, clicked) = self.row(&item, Kind::Item);
        self.activate(&item, value, clicked);
        MenuItemResponse {
            response,
            clicked: clicked && !item.disabled,
            inner: (),
        }
    }

    /// `MenuItemCheckbox`: a click flips `checked`; the menu stays open.
    pub fn checkbox_item(&mut self, item: impl Into<MenuItem>, checked: &mut bool) -> MenuItemResponse {
        let item = item.into();
        let (response, clicked) = self.row(&item, Kind::Checkbox(*checked));
        let clicked = clicked && !item.disabled;
        if clicked {
            *checked = !*checked;
        }
        MenuItemResponse {
            response,
            clicked,
            inner: (),
        }
    }

    /// `MenuItemRadio`: a click writes `value` into `group`; the menu stays open.
    pub fn radio_item<T: PartialEq>(&mut self, item: impl Into<MenuItem>, group: &mut T, value: T) -> MenuItemResponse {
        let item = item.into();
        let (response, clicked) = self.row(&item, Kind::Radio(*group == value));
        let clicked = clicked && !item.disabled;
        if clicked {
            *group = value;
        }
        MenuItemResponse {
            response,
            clicked,
            inner: (),
        }
    }

    /// `MenuSplitGroup`: a row split into a main action and a submenu trigger. Activating the main part closes the
    /// whole menu and reports `value`; the trigger opens `add` beside the row on click, Enter, `hoverDelay` of hovering or the
    /// start-to-end arrow on it. That arrow on the main part moves focus to the trigger, its opposite back.
    pub fn split_item<R>(
        &mut self,
        value: A,
        item: impl Into<MenuItem>,
        add: impl FnOnce(&mut MenuUi<'_, '_, A>) -> R,
    ) -> MenuItemResponse<Option<R>> {
        let item = item.into();
        let i = self.index;
        let depth = self.depth;
        let open = self.shared.nav.open_child(depth) == Some(i);
        let (response, clicked) = self.row(&item, Kind::SplitMain);
        let ctx = self.ui.ctx().clone();
        let dir = self.rt.i18n().direction();
        let trigger_rect = {
            let w = d::SPLIT_TRIGGER_WIDTH;
            let x = dir.pick(response.rect.right(), response.rect.left() - w);
            Rect::from_min_size(pos2(x, response.rect.top()), vec2(w, response.rect.height()))
        };
        let rt = self.rt;
        let trigger = {
            let mut child = self.ui.new_child(egui::UiBuilder::new().max_rect(trigger_rect));
            rt.ui(&mut child).mineral(SplitTrigger {
                salt: self.salt.child(("split", i)),
                label: &item.label,
                open,
                disabled: item.disabled,
            })
        };
        let trigger_id = trigger.id;
        let triggered = trigger.changed();
        if let Some(row) = self.rows.get_mut(i) {
            row.split = Some(trigger_id);
        }
        let forward = dir.forward_key();
        let backward = if forward == Key::ArrowRight {
            Key::ArrowLeft
        } else {
            Key::ArrowRight
        };
        let on_main = !item.disabled && response.has_focus();
        if on_main && ctx.input_mut(|input| input.consume_key(Modifiers::NONE, forward)) {
            ctx.memory_mut(|m| m.request_focus(trigger_id));
        }
        let on_trigger = trigger.has_focus();
        let by_key = !item.disabled && on_trigger && ctx.input_mut(|input| input.consume_key(Modifiers::NONE, forward));
        if on_trigger && ctx.input_mut(|input| input.consume_key(Modifiers::NONE, backward)) {
            ctx.memory_mut(|m| m.request_focus(response.id));
        }
        if trigger.hovered() {
            self.shared.nav.hover(&ctx, depth, i, !item.disabled, false);
        }
        if !item.disabled && (triggered || by_key) {
            self.shared.nav.open(&ctx, depth, i, true);
        }
        self.activate(&item, value, clicked);
        if self.shared.nav.open_child(depth) != Some(i) {
            return MenuItemResponse {
                response,
                clicked: clicked && !item.disabled,
                inner: None,
            };
        }
        let sub_salt = self.salt.child(("sub", i));
        let shared = &mut *self.shared;
        let group = response.rect.union(trigger_rect);
        let area = Popup::new(sub_salt.clone(), Anchor::new(Side::End, Align::Start))
            .role(Role::Menu)
            .name(&item.label)
            .show_at(rt, group, |ui, _| {
                level(rt, ui, &sub_salt, false, depth + 1, shared, add)
            });
        shared.inside.push(area.response.rect);
        let (inner, back) = area.inner;
        if back {
            ctx.memory_mut(|m| m.request_focus(trigger_id));
        }
        MenuItemResponse {
            response,
            clicked: false,
            inner: Some(inner),
        }
    }

    /// `MenuDivider`.
    pub fn divider(&mut self) {
        let w = self.ui.available_width();
        let (rect, _) = self
            .ui
            .allocate_exact_size(vec2(w, 2.0 * d::DIVIDER_MARGIN + g::STROKE_WIDTH_THIN), Sense::hover());
        let t = self.rt.tokens();
        self.ui.painter().hline(
            rect.left() - d::DIVIDER_BLEED..=rect.right() + d::DIVIDER_BLEED,
            rect.center().y,
            Stroke::new(g::STROKE_WIDTH_THIN, t.color(ColorNeutralStroke2)),
        );
    }

    /// `MenuGroup` with its `MenuGroupHeader`; `add` places the group's rows.
    pub fn group<R>(&mut self, header: impl Into<String>, add: impl FnOnce(&mut Self) -> R) -> R {
        let header = header.into();
        let t = self.rt.tokens();
        let w = self.memo.width.max(self.seen.width).min(d::ITEM_MAX_WIDTH);
        let (rect, _) = self
            .ui
            .allocate_exact_size(vec2(w, d::GROUP_HEADER_HEIGHT), Sense::hover());
        let color = t.color(ColorNeutralForeground3);
        let galley = self.ui.painter().layout_job(LayoutJob::single_section(
            header.clone(),
            typography::format(
                self.ui.ctx(),
                crate::typography::TypeSpec {
                    size: ty::FONT_SIZE_BASE_200,
                    line_height: ty::LINE_HEIGHT_BASE_200,
                    weight: ty::FONT_WEIGHT_SEMIBOLD,
                },
                color,
            ),
        ));
        let dir = self.rt.i18n().direction();
        let x = dir.start_x(
            rect,
            Rangef::new(d::GROUP_HEADER_PADDING, d::GROUP_HEADER_PADDING + galley.size().x),
        );
        self.ui
            .painter()
            .galley(pos2(x, rect.center().y - galley.size().y / 2.0), galley, color);
        let key = egui_mineral::agent_key!("menugroup").id(&self.salt.child(("group", self.index)));
        semantic::emit(self.ui.ctx(), None, rect, || {
            SemanticNode::new(key, Role::Group, &header)
        });
        add(self)
    }

    /// `Menu` as a submenu: a row with a chevron that opens `add` beside it on click, the start-to-end arrow,
    /// or `hoverDelay` of hovering.
    pub fn submenu<R>(
        &mut self,
        item: impl Into<MenuItem>,
        add: impl FnOnce(&mut MenuUi<'_, '_, A>) -> R,
    ) -> MenuItemResponse<Option<R>> {
        let item = item.into();
        let i = self.index;
        let depth = self.depth;
        let open = self.shared.nav.open_child(depth) == Some(i);
        let (response, clicked) = self.row(&item, Kind::Submenu(open));
        let ctx = self.ui.ctx().clone();
        if !item.disabled && clicked {
            self.shared.nav.open(&ctx, depth, i, true);
        }
        if self.shared.nav.open_child(depth) != Some(i) {
            return MenuItemResponse {
                response,
                clicked: false,
                inner: None,
            };
        }
        let sub_salt = self.salt.child(("sub", i));
        let rt = self.rt;
        let shared = &mut *self.shared;
        let area = Popup::new(sub_salt.clone(), Anchor::new(Side::End, Align::Start))
            .role(Role::Menu)
            .name(&item.label)
            .show_at(rt, response.rect, |ui, _| {
                level(rt, ui, &sub_salt, false, depth + 1, shared, add)
            });
        shared.inside.push(area.response.rect);
        let (inner, back) = area.inner;
        if back {
            ctx.memory_mut(|m| m.request_focus(response.id));
        }
        MenuItemResponse {
            response,
            clicked: false,
            inner: Some(inner),
        }
    }
}

// Surface of menu level `depth`; returns the content result and whether the level closed itself.
fn level<R, A: ActionValue>(
    rt: &Rt,
    ui: &mut Ui,
    salt: &KeyedSalt,
    opened: bool,
    depth: usize,
    shared: &mut Shared<A>,
    add: impl FnOnce(&mut MenuUi<'_, '_, A>) -> R,
) -> (R, bool) {
    let ctx = ui.ctx().clone();
    let t = rt.tokens();
    let id = ui.make_persistent_id(salt.id_salt());
    let memo_id = id.with("mineral::menu-memo");
    let loaded = Memo::try_load(&ctx, memo_id);
    let memo = loaded.unwrap_or_else(|| {
        (ctx.current_pass_index() == 0).then(|| ctx.request_discard("mineral: unsized container"));
        Memo::FIRST
    });
    let look = Look {
        fill: t.color(ColorNeutralBackground1),
        radius: g::BORDER_RADIUS_MEDIUM,
        elevation: Elevation::E16,
    };
    let focus_first = opened || shared.nav.take_focus(depth).is_some();
    let inner = framed(ui, &t, &look, Margin::same(d::POPOVER_PADDING as i8), |ui| {
        ui.set_width(memo.width);
        ui.spacing_mut().item_spacing.y = d::LIST_GAP;
        let mut mui = MenuUi {
            ui,
            rt,
            salt: salt.clone(),
            memo,
            seen: Memo {
                width: 0.0,
                checkmarks: false,
                icons: false,
            },
            depth,
            index: 0,
            rows: Vec::new(),
            shared,
        };
        let out = add(&mut mui);
        let MenuUi { rows, seen, .. } = mui;
        (out, rows, seen)
    });
    let (out, rows, seen) = inner.inner;
    let ids: Vec<Id> = rows.iter().map(|r| r.id).collect();
    let enabled = |i: usize| rows.get(i).is_some_and(|r| r.enabled);
    let next = Memo {
        width: seen.width.clamp(Memo::MIN_WIDTH, Memo::MAX_WIDTH),
        ..seen
    };
    // A level whose size equals FIRST must still store, or it reads as unsized and discards every pass.
    if loaded.is_none() || next != memo {
        next.store(&ctx, memo_id);
    }
    if next != memo {
        egui_mineral::motion::request_frame(&ctx);
    }
    let within = ctx
        .memory(egui::Memory::focused)
        .is_some_and(|f| rows.iter().any(|r| r.id == f || r.split == Some(f)));
    // The area's sizing pass drops focus requests, so the request repeats until focus lands.
    let pending_id = id.with("mineral::menu-focus");
    let pending = !within && (focus_first || ctx.data(|d| d.get_temp::<bool>(pending_id)).unwrap_or(false));
    if pending {
        if let Some(first) = rows.iter().find_map(|r| r.enabled.then_some(r.id)) {
            ctx.memory_mut(|m| m.request_focus(first));
        }
        ctx.data_mut(|d| d.insert_temp(pending_id, true));
        egui_mineral::motion::request_frame(&ctx);
    } else {
        ctx.data_mut(|d| d.remove::<bool>(pending_id));
    }
    RovingFocus::new(Axis::Vertical)
        .wrap(d::WRAP)
        .update(rt.core(), &ids, enabled);
    step_from_trigger(rt, &rows);
    let mut back = false;
    if within {
        if ctx.input(|i| i.key_pressed(Key::Tab)) {
            shared.close = true;
        }
        if let Some(i) = focused_index(&ctx, &ids) {
            if let Some(to) = typeahead(&ctx, i, &rows).and_then(|to| ids.get(to)) {
                ctx.memory_mut(|m| m.request_focus(*to));
            }
            let parent = rows.get(i).is_some_and(|r| r.submenu && r.enabled);
            match shared.nav.walk(&ctx, rt.i18n().direction(), depth, i, parent) {
                Some(Walk::Closed { .. }) => back = true,
                Some(Walk::Bar(edge)) => shared.bar = Some(edge),
                Some(Walk::Opened) | None => {}
            }
        }
        if depth > 0 && !back && overlay_escape(&ctx) {
            shared.nav.close_level(depth);
            back = true;
        }
    }
    (out, back)
}

// Up and Down on a split trigger step to the neighbouring rows.
fn step_from_trigger(rt: &Rt, rows: &[SeenRow]) {
    let ctx = rt.ctx();
    let Some((row, trigger)) = ctx
        .memory(egui::Memory::focused)
        .and_then(|f| rows.iter().position(|r| r.split == Some(f)).map(|i| (i, f)))
    else {
        return;
    };
    kit::interact::lock_arrows(ctx, trigger, kit::interact::Arrows::Axis(Axis::Vertical));
    let dir = rt.i18n().direction();
    let roving = RovingFocus::new(Axis::Vertical).wrap(d::WRAP);
    let enabled_at = |i: usize| rows.get(i).is_some_and(|r| r.enabled);
    for key in kit::interact::plain_key_presses(ctx) {
        if let Some(to) = roving
            .target(key, dir, row, &Items::new(rows.len(), enabled_at))
            .and_then(|to| rows.get(to))
        {
            ctx.memory_mut(|m| m.request_focus(to.id));
        }
    }
}

fn initial(label: &str) -> Option<char> {
    label.chars().next().and_then(|c| c.to_lowercase().next())
}

fn focused_index(ctx: &egui::Context, ids: &[Id]) -> Option<usize> {
    let focused = ctx.memory(egui::Memory::focused)?;
    ids.iter().position(|&id| id == focused)
}

// useMenuList focusItemMatchingFirstCharacter: next enabled row after `from` whose label starts with the typed
// character, wrapping; one-character text events only.
fn typeahead(ctx: &egui::Context, from: usize, rows: &[SeenRow]) -> Option<usize> {
    let typed = ctx.input(|i| {
        i.events.iter().rev().find_map(|e| match e {
            egui::Event::Text(t) => {
                let mut chars = t.chars();
                chars.next().filter(|c| !c.is_whitespace() && chars.next().is_none())
            }
            _ => None,
        })
    })?;
    let key = typed.to_lowercase().next()?;
    let n = rows.len();
    (1..=n)
        .map(|k| (from + k) % n)
        .find(|&i| rows.get(i).is_some_and(|r| r.enabled && r.initial == Some(key)))
}

fn overlay_escape(ctx: &egui::Context) -> bool {
    kit::layer::consume_escape(ctx)
}

/// Result of [`Menu::show`].
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct MenuResponse<R = (), A = NoAction> {
    /// Surface, on passes it showed.
    pub response: Option<Response>,
    /// What `add` returned, on passes it showed.
    pub inner: Option<R>,
    /// Why it closed this pass.
    pub dismissed: Option<Dismiss>,
    /// Value of the item activated this pass ([`MenuUi::item`] or the main part of [`MenuUi::split_item`], also
    /// in a submenu).
    pub clicked: Option<A>,
    /// Reading-order arrow on the root level toward this edge; a menu bar opens the adjacent menu.
    pub nav: Option<Edge>,
}

/// Fluent `Menu`: a popup list of rows anchored below its trigger. The trigger's clicks toggle it, Escape,
/// Tab and outside presses close it, and so does a row that is not a checkbox, radio or submenu.
/// Arrow keys move between rows (wrapping), the start-to-end arrow opens a submenu and its opposite closes one.
#[derive(Clone, Debug)]
#[must_use = "call .show(..)"]
pub struct Menu {
    salt: KeyedSalt,
    key: AgentKey,
    label: String,
    side: Side,
    align: Align,
    radio_mark: RadioMark,
}

impl Menu {
    /// `label` names the popup for assistive technology; below the trigger, start aligned.
    /// Agent key `<id_salt>-menu`.
    pub fn new(id_salt: impl Into<KeyedSalt>, label: impl Into<String>) -> Self {
        Self {
            salt: id_salt.into(),
            key: egui_mineral::agent_key!("menu"),
            label: label.into(),
            side: Side::Bottom,
            align: Align::Start,
            radio_mark: RadioMark::Check,
        }
    }

    /// Mark of checked radio items; default [`RadioMark::Check`], v9's checkmark. [`RadioMark::Dot`] is not in
    /// Fluent (docs/divergences.md).
    pub fn radio_mark(mut self, radio_mark: RadioMark) -> Self {
        self.radio_mark = radio_mark;
        self
    }

    crate::widget::setters! {
        /// Replaces the `<id_salt>-menu` agent key.
        agent_key(key)
    }

    /// Side of the trigger; default [`Side::Bottom`].
    pub fn side(mut self, side: Side) -> Self {
        self.side = side;
        self
    }

    /// Alignment along the trigger; default [`Align::Start`].
    pub fn align(mut self, align: Align) -> Self {
        self.align = align;
        self
    }

    /// Shown while `open`, anchored to `trigger`; `add` places the rows. The first enabled row takes focus on opening.
    /// `A` is inferred from the [`MenuUi::item`] values; a menu without such rows names it (`MenuUi<'_, '_, NoAction>`).
    pub fn show<R, A: ActionValue>(
        self,
        rt: &Rt,
        trigger: &Response,
        open: &mut bool,
        add: impl FnOnce(&mut MenuUi<'_, '_, A>) -> R,
    ) -> MenuResponse<R, A> {
        let ctx = rt.ctx();
        let mut dismissed = None;
        if trigger.clicked() {
            *open = !*open;
            if !*open {
                dismissed = Some(Dismiss::Close);
            }
        }
        if !*open {
            return MenuResponse {
                response: None,
                inner: None,
                dismissed,
                clicked: None,
                nav: None,
            };
        }
        let opened_id = trigger.id.with(("mineral::fluent-menu", self.salt.id_salt()));
        let pass = ctx.cumulative_pass_nr();
        let last: Option<u64> = ctx.data(|d| d.get_temp(opened_id));
        let opened = last.is_none_or(|p| p + 1 < pass);
        let nav_id = opened_id.with("nav");
        let mut nav = MenuNav::load(ctx, nav_id);
        if opened {
            nav.reset(None);
        }
        let mut shared = Shared {
            radio_mark: self.radio_mark,
            nav,
            ..Shared::default()
        };
        let area = Popup::new(self.salt.child("area"), Anchor::new(self.side, self.align))
            .role(Role::Menu)
            .name(&self.label)
            .show_at(rt, trigger.rect, |ui, _| {
                level(rt, ui, &self.salt, opened, 0, &mut shared, add)
            });
        ctx.data_mut(|d| d.insert_temp(opened_id, pass));
        let (inner, _) = area.inner;
        let mut inside = vec![area.response.rect, trigger.rect];
        inside.append(&mut shared.inside);
        let dismissal = overlay::dismissal(rt.core(), area.response.layer_id, &inside, Some(trigger.id));
        let tab_closed = shared.close;
        if tab_closed {
            ctx.memory_mut(|m| m.request_focus(trigger.id));
        }
        let dismissal = dismissal.or(tab_closed.then_some(Dismiss::Close));
        *open = dismissal.is_none();
        shared.nav.settle_hover(ctx, d::HOVER_DELAY);
        if *open {
            shared.nav.store(ctx, nav_id);
        } else {
            ctx.data_mut(|d| d.remove::<u64>(opened_id));
            ctx.data_mut(|d| d.remove::<MenuNav>(nav_id));
        }
        let key = self.key.id(&self.salt);
        let label = self.label;
        semantic::emit(ctx, Some(area.response.id), area.response.rect, || {
            SemanticNode::new(key, Role::Menu, label)
        });
        MenuResponse {
            response: Some(area.response),
            inner: Some(inner),
            dismissed: dismissal,
            clicked: shared.clicked,
            nav: shared.bar,
        }
    }
}
