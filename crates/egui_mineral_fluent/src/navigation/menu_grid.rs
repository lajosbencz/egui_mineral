use egui::{Id, Margin, Rangef, Rect, Response, Sense, Stroke, StrokeKind, UiBuilder, pos2, text::LayoutJob, vec2};
use egui_mineral::{
    action::Clicked,
    anchor::{Align, Anchor, Side},
    focus::{self, Cell, Grid, GridSize, on_grid_key},
    i18n::I18n,
    kit::{self, state::TempState},
    layout::{Constraints, Measure, MinTarget},
    overlay::{self, Dismiss, Popup},
    semantic::{self, AgentKey, KeyedSalt, Role, SemanticNode},
    shortcut::ActionId,
    widget::{Availability, Interaction, MineralUi, MineralWidget, WidgetFrame, WidgetResponse},
};

use super::menu::text_width;
use crate::ColorToken::{
    ColorNeutralBackground1, ColorNeutralBackground1Hover, ColorNeutralBackground1Pressed, ColorNeutralForeground2,
    ColorNeutralForeground2BrandSelected, ColorNeutralForeground2Hover, ColorNeutralForeground2Pressed,
    ColorNeutralForeground3, ColorNeutralForeground3Hover, ColorNeutralForeground3Pressed,
    ColorNeutralForegroundDisabled, ColorStrokeFocus2, ColorSubtleBackground, ColorSubtleBackgroundHover,
    ColorSubtleBackgroundPressed,
};
use crate::{
    Fluent, FluentExt, Icon, Rt,
    spec::{
        defaults::{menu, menu_grid as d},
        tokens::geometry as g,
    },
    surfaces::overlay::{Look, framed},
    theme::{Elevation, Radius},
    typography::TypeRamp,
    widget::{
        focus as ring, paint,
        state::{Ramp, State},
    },
};

/// Fluent `MenuGridItem`: the main cell of a row, with an icon, a label and a sub text below it.
#[derive(Clone, Debug)]
#[must_use]
pub struct MenuGridItem {
    label: String,
    icon: Option<Icon>,
    sub_text: Option<String>,
    disabled: bool,
    action_key: Option<ActionId>,
}

impl MenuGridItem {
    /// Visible text and accessible name.
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            icon: None,
            sub_text: None,
            disabled: false,
            action_key: None,
        }
    }

    /// `icon` slot before the label.
    pub fn icon(mut self, icon: impl Into<Icon>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    /// `subText` slot below the label.
    pub fn secondary_text(mut self, text: impl Into<String>) -> Self {
        self.sub_text = Some(text.into());
        self
    }

    crate::widget::setters! {
        action_key
    }

    crate::widget::setters! {
        /// Not operable.
        disabled
    }
}

impl From<&str> for MenuGridItem {
    fn from(s: &str) -> Self {
        Self::new(s)
    }
}

/// Fluent `MenuGridCell` holding an icon button: a secondary action of a row.
#[derive(Clone, Debug)]
#[must_use]
pub struct MenuGridAction {
    label: String,
    icon: Icon,
    disabled: bool,
    action_key: Option<ActionId>,
}

impl MenuGridAction {
    /// `label` is the accessible name of the icon-only cell.
    pub fn new(label: impl Into<String>, icon: impl Into<Icon>) -> Self {
        Self {
            label: label.into(),
            icon: icon.into(),
            disabled: false,
            action_key: None,
        }
    }

    crate::widget::setters! {
        action_key
    }

    crate::widget::setters! {
        /// Not operable.
        disabled
    }
}

/// Cell of a [`MenuGridRow`].
#[derive(Clone, Debug)]
#[non_exhaustive]
pub enum MenuGridCell {
    /// [`MenuGridItem`].
    Item(MenuGridItem),
    /// [`MenuGridAction`].
    Action(MenuGridAction),
}

impl From<MenuGridItem> for MenuGridCell {
    fn from(item: MenuGridItem) -> Self {
        Self::Item(item)
    }
}

impl From<MenuGridAction> for MenuGridCell {
    fn from(action: MenuGridAction) -> Self {
        Self::Action(action)
    }
}

impl From<&str> for MenuGridCell {
    fn from(label: &str) -> Self {
        Self::Item(label.into())
    }
}

impl MenuGridCell {
    fn disabled(&self) -> bool {
        match self {
            Self::Item(i) => i.disabled,
            Self::Action(a) => a.disabled,
        }
    }

    fn label(&self) -> &str {
        match self {
            Self::Item(i) => &i.label,
            Self::Action(a) => &a.label,
        }
    }

    fn action_key(&self) -> Option<&ActionId> {
        match self {
            Self::Item(i) => i.action_key.as_ref(),
            Self::Action(a) => a.action_key.as_ref(),
        }
    }

    fn height(&self) -> f32 {
        match self {
            Self::Item(i) => {
                let text = TypeRamp::Body1.line_height()
                    + i.sub_text.as_ref().map_or(0.0, |_| TypeRamp::Caption2.line_height());
                text.max(d::ICON_SIZE).max(d::CELL_MIN_HEIGHT)
            }
            Self::Action(_) => d::CELL_MIN_HEIGHT,
        }
    }
}

/// Fluent `MenuGridRow`: cells in one line; the items share the width the actions leave.
#[derive(Clone, Debug, Default)]
#[must_use]
pub struct MenuGridRow {
    cells: Vec<MenuGridCell>,
    salt: Option<KeyedSalt>,
}

impl MenuGridRow {
    /// No cells.
    pub fn new() -> Self {
        Self::default()
    }

    /// Appends a cell.
    pub fn cell(mut self, cell: impl Into<MenuGridCell>) -> Self {
        self.cells.push(cell.into());
        self
    }

    crate::widget::setters! {
        item_salt
    }
}

struct GridCell<'a> {
    salt: KeyedSalt,
    cell: &'a MenuGridCell,
    row_state: State,
    tab_stop: focus::TabStop,
    width: f32,
    height: f32,
}

impl MineralWidget for GridCell<'_> {
    type Kit = Fluent;
    type State = ();
    type Output = ();

    fn id_salt(&self) -> egui::IdSalt {
        self.salt.id_salt()
    }

    fn enabled(&self) -> bool {
        !self.cell.disabled()
    }

    fn sense(&self) -> Sense {
        focus::roving_sense(Sense::click(), self.tab_stop)
    }

    fn min_target(&self, _: &MineralUi<'_, Self::Kit>) -> MinTarget {
        MinTarget::None
    }

    fn layout(&self, _: &MineralUi<'_, Fluent>, c: Constraints, (): &()) -> Measure {
        c.constrain(vec2(self.width, self.height)).into()
    }

    fn update(&mut self, ui: &mut MineralUi<'_, Fluent>, frame: &WidgetFrame<'_>, (): &mut ()) -> WidgetResponse {
        let t = ui.tokens();
        let r = &frame.response;
        let availability = Availability::disabled_if(self.cell.disabled());
        kit::input::hover_cursor(ui.core(), r, availability);
        kit::focus::follow_press(r);
        let i = Interaction::of(r, availability);
        let rect = frame.zones.visual;
        let dir = ui.i18n().direction();
        let fg = |s| {
            Ramp::new(
                ColorNeutralForeground2,
                ColorNeutralForeground2Hover,
                ColorNeutralForeground2Pressed,
                ColorNeutralForegroundDisabled,
            )
            .pick(s, &t)
        };
        match self.cell {
            MenuGridCell::Item(item) => {
                let s = if i.disabled { State::Disabled } else { self.row_state };
                let color = fg(s);
                let engaged = s.engaged() && !i.disabled;
                let cy = rect.center().y;
                let mut x = 0.0;
                if let Some(icon) = item.icon {
                    let c = pos2(
                        dir.start_x(rect, Rangef::new(x + d::ICON_SIZE / 2.0, x + d::ICON_SIZE / 2.0)),
                        cy,
                    );
                    let tint = if engaged {
                        t.color(ColorNeutralForeground2BrandSelected)
                    } else {
                        color
                    };
                    paint::icon(ui.painter(), c, icon, engaged, d::ICON_SIZE, tint);
                    x += d::ICON_SIZE + d::ROW_GAP;
                }
                x += d::CONTENT_PADDING;
                let sub = item.sub_text.as_ref();
                let body = TypeRamp::Body1.line_height();
                let block = body + sub.map_or(0.0, |_| TypeRamp::Caption2.line_height());
                let top = cy - block / 2.0;
                let line = |text: &str, ramp: TypeRamp, color, y: f32| {
                    let job = LayoutJob::single_section(text.to_owned(), ramp.format(ui.ctx(), color));
                    let galley = ui.painter().layout_job(job);
                    let px = dir.start_x(rect, Rangef::new(x, x + galley.size().x));
                    ui.painter().galley(pos2(px, y), galley, color);
                };
                line(&item.label, TypeRamp::Body1, color, top);
                if let Some(sub) = sub {
                    let muted = Ramp::new(
                        ColorNeutralForeground3,
                        ColorNeutralForeground3Hover,
                        ColorNeutralForeground3Pressed,
                        ColorNeutralForegroundDisabled,
                    )
                    .pick(s, &t);
                    line(sub, TypeRamp::Caption2, muted, top + body);
                }
                if i.focused {
                    ring::outline(ui.painter(), rect.expand(1.0), Radius::Medium.on(rect), &t);
                }
            }
            MenuGridCell::Action(action) => {
                let s = State::of(i);
                let side = d::CELL_MIN_HEIGHT.min(rect.width());
                let button = Rect::from_center_size(rect.center(), vec2(side, side));
                let radius = Radius::Medium.on(button);
                let fill = Ramp::new(
                    ColorSubtleBackground,
                    ColorSubtleBackgroundHover,
                    ColorSubtleBackgroundPressed,
                    ColorSubtleBackground,
                );
                ui.painter().rect_filled(button, radius, fill.pick(s, &t));
                paint::icon(
                    ui.painter(),
                    button.center(),
                    action.icon,
                    s.engaged(),
                    d::ICON_SIZE,
                    fg(s),
                );
                if i.focused {
                    ui.painter().rect_stroke(
                        button,
                        radius,
                        Stroke::new(g::STROKE_WIDTH_THICK, t.color(ColorStrokeFocus2)),
                        StrokeKind::Inside,
                    );
                }
            }
        }
        WidgetResponse::new(r.clone(), ()).with_changed(r.clicked() && !self.cell.disabled())
    }

    fn semantics(&self, _: &I18n, response: &WidgetResponse, (): &()) -> Option<SemanticNode> {
        let mut n = SemanticNode::new(
            AgentKey::Kind("gridcell").id(&self.salt),
            Role::GridCell,
            self.cell.label(),
        )
        .clickable(Availability::disabled_if(self.cell.disabled()));
        n.state.focused = response.has_focus();
        Some(n)
    }
}

/// Result of one [`MenuGridUi`] row.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct MenuGridRowResponse {
    /// Spans the row.
    pub response: Response,
    /// Responses of the cells, in order.
    pub cells: Vec<Response>,
    /// Cell activated this pass; its index is the column.
    pub clicked: Option<Clicked>,
}

// Widest row of the previous pass: rows stretch to it, as the menu popover sizes to its content.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Memo {
    width: f32,
}

impl TempState for Memo {}

impl Memo {
    const MIN_WIDTH: f32 = menu::POPOVER_MIN_WIDTH - 2.0 * menu::POPOVER_PADDING;

    fn sized(ctx: &egui::Context, id: Id) -> Self {
        Self::try_load(ctx, id).unwrap_or(Self { width: Self::MIN_WIDTH })
    }

    // Stores the widest row seen this pass and asks for a pass when it changed.
    fn update(self, ctx: &egui::Context, id: Id, seen: f32) {
        let next = Self {
            width: seen.clamp(Self::MIN_WIDTH, d::ROW_MAX_WIDTH),
        };
        if next != self {
            next.store(ctx, id);
            egui_mineral::motion::request_frame(ctx);
        }
    }
}

impl MenuGridCell {
    fn natural_width(&self, ui: &MineralUi<'_, Fluent>) -> f32 {
        match self {
            Self::Item(i) => {
                let icon = i.icon.map_or(0.0, |_| d::ICON_SIZE + d::ROW_GAP);
                let sub = i
                    .sub_text
                    .as_ref()
                    .map_or(0.0, |s| text_width(ui, s, TypeRamp::Caption2));
                icon + text_width(ui, &i.label, TypeRamp::Body1).max(sub) + 2.0 * d::CONTENT_PADDING
            }
            Self::Action(_) => d::ACTION_WIDTH,
        }
    }
}

struct Slot {
    id: Id,
    enabled: bool,
    row: usize,
    column: usize,
}

/// Places the rows of a [`MenuGrid`].
pub struct MenuGridUi<'u, 's> {
    ui: &'u mut MineralUi<'s, Fluent>,
    salt: &'u KeyedSalt,
    stop: usize,
    row: usize,
    slots: Vec<Slot>,
    activated: Option<Cell>,
    clicked: Option<Clicked>,
    memo: Memo,
    seen: f32,
}

kit::opaque_debug!(MenuGridUi<'_, '_>);

impl MenuGridUi<'_, '_> {
    /// `MenuGridRow`: the cells in one line on a row surface.
    pub fn row(&mut self, row: MenuGridRow) -> MenuGridRowResponse {
        let MenuGridRow {
            cells: cells_in,
            salt: row_salt,
        } = row;
        let index = self.row;
        self.row += 1;
        let t = self.ui.tokens();
        let dir = self.ui.i18n().direction();
        let (block, inline) = d::ROW_PADDING;
        let content = cells_in.iter().map(MenuGridCell::height).fold(0.0_f32, f32::max);
        let height = (content + 2.0 * block).max(d::ROW_MIN_HEIGHT);
        let natural = 2.0 * inline
            + d::ROW_GAP * cells_in.len().saturating_sub(1) as f32
            + cells_in.iter().map(|c| c.natural_width(self.ui)).sum::<f32>();
        self.seen = self.seen.max(natural);
        let width = self.memo.width.max(natural).min(d::ROW_MAX_WIDTH);
        let (rect, response) = self.ui.allocate_exact_size(vec2(width, height), Sense::hover());
        let over = self.ui.rect_contains_pointer(rect);
        let pressed = over && self.ui.input(|i| i.pointer.primary_down());
        let row_state = match (over, pressed) {
            (_, true) => State::Pressed,
            (true, false) => State::Hover,
            _ => State::Rest,
        };
        let fill = Ramp::new(
            ColorNeutralBackground1,
            ColorNeutralBackground1Hover,
            ColorNeutralBackground1Pressed,
            ColorNeutralBackground1,
        );
        self.ui
            .painter()
            .rect_filled(rect, Radius::Medium.on(rect), fill.pick(row_state, &t));

        let inner = rect.shrink2(vec2(inline, block));
        let gaps = d::ROW_GAP * cells_in.len().saturating_sub(1) as f32;
        let (items, actions) = cells_in.iter().fold((0_usize, 0.0_f32), |(n, w), c| match c {
            MenuGridCell::Item(_) => (n + 1, w),
            MenuGridCell::Action(_) => (n, w + d::ACTION_WIDTH),
        });
        let item_width = ((inner.width() - actions - gaps) / items.max(1) as f32).max(0.0);
        let rt = self.ui.rt();
        let mut cells = Vec::with_capacity(cells_in.len());
        let mut clicked = None;
        let salt = row_salt.clone().unwrap_or_else(|| self.salt.child(index));
        let stop = self.stop;
        let first = self.slots.len();
        {
            let ui = &mut self.ui.new_child(UiBuilder::new().max_rect(inner).layout(dir.row()));
            ui.spacing_mut().item_spacing.x = d::ROW_GAP;
            for (column, cell) in cells_in.iter().enumerate() {
                let flat = first + column;
                let widget = GridCell {
                    salt: salt.child(column),
                    cell,
                    row_state,
                    tab_stop: focus::TabStop::at(flat, stop),
                    width: match cell {
                        MenuGridCell::Item(_) => item_width,
                        MenuGridCell::Action(_) => d::ACTION_WIDTH,
                    },
                    height: inner.height(),
                };
                let out = rt.ui(ui).mineral(widget);
                if out.changed() {
                    clicked = Some(Clicked::new(column, cell.action_key().cloned()));
                }
                let r = out.into_response();
                self.slots.push(Slot {
                    id: r.id,
                    enabled: !cell.disabled(),
                    row: index,
                    column,
                });
                cells.push(r);
            }
        }
        if let Some(c) = &clicked {
            self.activated = Some(Cell {
                row: index,
                column: c.index(),
            });
            self.clicked.clone_from(&clicked);
        }
        let name = cells_in.first().map_or("", MenuGridCell::label).to_owned();
        let key = AgentKey::Kind("gridrow").id(&salt);
        semantic::emit(self.ui.ctx(), None, rect, || SemanticNode::new(key, Role::Row, name));
        MenuGridRowResponse {
            response,
            cells,
            clicked,
        }
    }
}

/// Result of showing a [`MenuGrid`].
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct MenuGridResponse<R = ()> {
    /// Surface, on passes it showed.
    pub response: Option<Response>,
    /// Why it closed this pass.
    pub dismissed: Option<Dismiss>,
    /// Position of the cell activated this pass.
    pub activated: Option<Cell>,
    /// Cell activated this pass; its index is the column, [`Self::activated`] has the row.
    pub clicked: Option<Clicked>,
    /// What `add` returned, on passes it showed.
    pub inner: Option<R>,
}

impl<R> MenuGridResponse<R> {
    fn closed(dismissed: Option<Dismiss>) -> Self {
        Self {
            response: None,
            dismissed,
            activated: None,
            clicked: None,
            inner: None,
        }
    }
}

/// Fluent `MenuGrid` in its `MenuPopover`: a popup of rows of cells with grid keyboard navigation, anchored below
/// its trigger. One cell is the Tab stop; arrows move by cell
/// and row (the column clamps to a shorter row), Home and End go to the row's first and last cell, Ctrl+Home and
/// Ctrl+End to the grid's, and Enter or Space activates.
#[derive(Debug)]
#[must_use = "call .show(..)"]
pub struct MenuGrid {
    salt: KeyedSalt,
    key: AgentKey,
    label: String,
}

impl MenuGrid {
    /// `label` names the grid for assistive technology. Agent key `<id_salt>-menugrid`.
    pub fn new(id_salt: impl Into<KeyedSalt>, label: impl Into<String>) -> Self {
        Self {
            salt: id_salt.into(),
            key: AgentKey::Kind("menugrid"),
            label: label.into(),
        }
    }

    crate::widget::setters! {
        /// Replaces the `<id_salt>-menugrid` agent key.
        agent_key(key)
    }

    /// Shown while `open`, anchored below `trigger`, whose clicks toggle it; `add` places the rows. The first
    /// enabled cell takes focus on opening; Escape, Tab and outside presses close it, activating a cell does not.
    pub fn show<R>(
        self,
        rt: &Rt,
        trigger: &Response,
        open: &mut bool,
        add: impl FnOnce(&mut MenuGridUi<'_, '_>) -> R,
    ) -> MenuGridResponse<R> {
        let ctx = rt.ctx();
        let Self { salt, key, label } = self;
        let mut dismissed = None;
        if trigger.clicked() {
            *open = !*open;
            if !*open {
                dismissed = Some(Dismiss::Close);
            }
        }
        if !*open {
            return MenuGridResponse::closed(dismissed);
        }
        let id = trigger.id.with(("mineral::fluent-menu-grid", salt.id_salt()));
        let seen_id = id.with("mineral::menu-grid-cells");
        let seen: usize = ctx.data(|d| d.get_temp(seen_id)).unwrap_or(0);
        let stop = focus::tab_stop(ctx, id, None, seen, |_| true);
        let dir = rt.i18n().direction();
        let t = rt.tokens();
        // `useMenuPopoverStyles`
        let look = Look {
            fill: t.color(ColorNeutralBackground1),
            radius: g::BORDER_RADIUS_MEDIUM,
            elevation: Elevation::E16,
        };
        let memo_id = id.with("mineral::menu-grid-memo");
        let memo = Memo::sized(ctx, memo_id);
        let mut slots = Vec::new();
        let mut activated = None;
        let mut clicked = None;
        let mut seen = 0.0;
        let area = Popup::new(salt.child("area"), Anchor::new(Side::Bottom, Align::Start))
            .role(Role::Grid)
            .name(&label)
            .show_at(rt, trigger.rect, |ui, _| {
                framed(ui, &t, &look, Margin::same(menu::POPOVER_PADDING as i8), |ui| {
                    ui.scope_builder(UiBuilder::new().layout(dir.column()), |ui| {
                        ui.set_width(memo.width);
                        ui.spacing_mut().item_spacing.y = d::GRID_GAP;
                        let mut grid = MenuGridUi {
                            ui: &mut rt.ui(ui),
                            salt: &salt,
                            stop,
                            row: 0,
                            slots: Vec::new(),
                            activated: None,
                            clicked: None,
                            memo,
                            seen: 0.0,
                        };
                        let inner = add(&mut grid);
                        slots = grid.slots;
                        activated = grid.activated;
                        clicked = grid.clicked;
                        seen = grid.seen;
                        inner
                    })
                    .inner
                })
                .inner
            });
        ctx.data_mut(|d| d.insert_temp(seen_id, slots.len()));
        memo.update(ctx, memo_id, seen);
        let within = ctx
            .memory(egui::Memory::focused)
            .is_some_and(|f| slots.iter().any(|s| s.id == f));
        // The area's sizing pass drops focus requests, so the request repeats until focus lands.
        let opened_id = id.with("mineral::menu-grid-opened");
        if !within && !ctx.data(|d| d.get_temp::<bool>(opened_id)).unwrap_or(false) {
            if let Some(first) = slots.iter().find(|s| s.enabled) {
                ctx.memory_mut(|m| m.request_focus(first.id));
            }
            egui_mineral::motion::request_frame(ctx);
        } else {
            ctx.data_mut(|d| d.insert_temp(opened_id, true));
        }
        let tab = within && ctx.input(|i| i.key_pressed(egui::Key::Tab));
        navigate(rt, &slots);
        kit::focus::remember(ctx, id, slots.iter().enumerate().map(|(i, s)| (i, s.id)));
        let inside = [area.response.rect, trigger.rect];
        let mut dismissal = overlay::dismissal(rt.core(), area.response.layer_id, &inside, Some(trigger.id));
        if tab {
            ctx.memory_mut(|m| m.request_focus(trigger.id));
            dismissal = dismissal.or(Some(Dismiss::Close));
        }
        *open = dismissal.is_none();
        if !*open {
            ctx.data_mut(|d| d.remove::<bool>(opened_id));
        }
        let node_key = key.id(&salt);
        semantic::emit(ctx, Some(area.response.id), area.response.rect, || {
            SemanticNode::new(node_key, Role::Grid, label)
        });
        MenuGridResponse {
            response: Some(area.response),
            dismissed: dismissal,
            activated,
            clicked,
            inner: Some(area.inner),
        }
    }
}

fn navigate(rt: &crate::Rt, slots: &[Slot]) {
    let ctx = rt.ctx();
    let Some((cur, focused)) = ctx
        .memory(egui::Memory::focused)
        .and_then(|f| slots.iter().position(|s| s.id == f).map(|i| (i, f)))
    else {
        return;
    };
    kit::focus::lock_arrows(ctx, focused, kit::focus::Arrows::Both);
    let dir = rt.i18n().direction();
    let rows = slots.last().map_or(0, |s| s.row + 1);
    let columns = |row: usize| slots.iter().filter(|s| s.row == row).count();
    let widest = (0..rows).map(columns).max().unwrap_or(0);
    let grid = Grid::new(GridSize { rows, columns: widest });
    let flat = |c: Cell| {
        slots
            .iter()
            .position(|s| s.row == c.row && s.column == c.column.min(columns(c.row).saturating_sub(1)))
    };
    let mut at = cur;
    for (key, modifiers) in kit::input::key_presses(ctx) {
        let Some(from) = slots.get(at) else { break };
        let mut cell = Cell {
            row: from.row,
            column: from.column,
        };
        // Disabled cells are skipped by repeating the step; the guard bounds a row of nothing but disabled cells.
        for _ in 0..slots.len() {
            let Some(next) = on_grid_key(grid, cell, key, modifiers, dir) else {
                break;
            };
            if next == cell {
                break;
            }
            cell = Cell {
                column: next.column.min(columns(next.row).saturating_sub(1)),
                ..next
            };
            if let Some(i) = flat(cell).filter(|i| slots.get(*i).is_some_and(|s| s.enabled)) {
                at = i;
                break;
            }
        }
    }
    if at != cur
        && let Some(target) = slots.get(at)
    {
        ctx.memory_mut(|m| m.request_focus(target.id));
    }
}
