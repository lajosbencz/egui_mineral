use crate::msg;
use egui_mineral::focus::Items;
use std::{borrow::Cow, ops::Range, sync::Arc};

use egui::{CursorIcon, Galley, Id, Rangef, Rect, Sense, Stroke, StrokeKind, UiBuilder, pos2, text::LayoutJob, vec2};
use egui_mineral::{
    focus::{self, Cell, RovingFocus, TabStop},
    i18n::{Direction, I18n},
    kit::{self, state::TempState},
    layout::{Constraints, Measure},
    select::{ItemKey, Selection, SelectionMode},
    semantic::{self, AgentKey, KeyedSalt, Role, SemanticNode, StableId},
    table::{ColumnWidths, Rows, SortOrder, SortState, sort_order},
    widget::{Availability, MineralUi, MineralWidget, WidgetFrame, WidgetResponse},
};

use crate::spec::ColorToken::{
    ColorBrandBackground2, ColorNeutralForeground1, ColorNeutralForeground1Hover, ColorNeutralForeground1Pressed,
    ColorNeutralForeground2, ColorNeutralStroke1, ColorNeutralStroke2, ColorStrokeFocus2, ColorSubtleBackgroundHover,
    ColorSubtleBackgroundPressed, ColorSubtleBackgroundSelected,
};
use crate::{
    Fluent, FluentExt,
    inputs::{Checkbox, CheckboxShape},
    spec::defaults::table as d,
    typography::TypeRamp,
    widget::paint,
};

use super::{Avatar, AvatarSize};

/// Table `size`: the row height.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum TableSize {
    /// `medium`: 44 high rows.
    #[default]
    Medium,
    /// `small`: 34 high rows.
    Small,
    /// `extra-small`: 24 high rows.
    ExtraSmall,
}

impl TableSize {
    /// Every variant, largest first.
    pub const ALL: &'static [Self] = &[Self::Medium, Self::Small, Self::ExtraSmall];

    fn index(self) -> usize {
        crate::widget::dense_tables!(TableSize: d::ROW_HEIGHTS, d::MEDIA);
        self as usize
    }

    fn row_height(self) -> f32 {
        d::ROW_HEIGHTS.get(self.index()).copied().unwrap_or_default()
    }

    fn media(self) -> f32 {
        d::MEDIA.get(self.index()).copied().unwrap_or_default()
    }
}

/// Selected row `appearance`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum TableRowAppearance {
    /// `neutral`: `colorSubtleBackgroundSelected`.
    #[default]
    Neutral,
    /// `brand`: `colorBrandBackground2`.
    Brand,
    /// `none`: no highlight.
    None,
}

impl TableRowAppearance {
    /// Every variant.
    pub const ALL: &'static [Self] = &[Self::Neutral, Self::Brand, Self::None];
}

/// Fluent `TableHeaderCell`: a column's header, with a sort button when sortable.
#[derive(Clone, Debug)]
#[must_use]
pub struct TableColumn {
    label: String,
    sortable: bool,
    width: Option<f32>,
}

impl TableColumn {
    /// Header text and accessible name; not sortable, an equal share of the width.
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            sortable: false,
            width: None,
        }
    }

    /// `sortable`: a click or Enter on the header cycles the bound [`SortState`].
    pub fn sortable(mut self, sortable: bool) -> Self {
        self.sortable = sortable;
        self
    }

    /// Fixed width in points instead of an equal share; ignored with [`Table::resizable`].
    #[track_caller]
    pub fn width(mut self, width: f32) -> Self {
        self.width = Some(egui_mineral::kit::sanitize::at_least(width, 0.0.., "TableColumn width"));
        self
    }
}

impl From<&str> for TableColumn {
    fn from(label: &str) -> Self {
        Self::new(label)
    }
}

/// Fluent `TableCellLayout`: media, a primary line and a description inside a cell.
#[derive(Clone, Debug)]
#[must_use = "show with ui.mineral(..)"]
pub struct TableCellLayout<'a> {
    salt: KeyedSalt,
    key: AgentKey,
    text: String,
    description: Option<String>,
    media: Option<Avatar<'a>>,
    size: TableSize,
    primary: bool,
}

impl<'a> TableCellLayout<'a> {
    /// Agent key `<id_salt>-table-cell-layout`.
    pub fn new(id_salt: impl Into<KeyedSalt>, text: impl Into<String>) -> Self {
        Self {
            salt: id_salt.into(),
            key: egui_mineral::agent_key!("table-cell-layout"),
            text: text.into(),
            description: None,
            media: None,
            size: TableSize::Medium,
            primary: false,
        }
    }

    /// `media` slot: an [`Avatar`] sized by the table size (24 when `primary`).
    pub fn media(mut self, avatar: Avatar<'a>) -> Self {
        self.media = Some(avatar);
        self
    }

    /// `description` slot below the text.
    pub fn secondary_text(mut self, text: impl Into<String>) -> Self {
        self.description = Some(text.into());
        self
    }

    /// Matches the table's size; default [`TableSize::Medium`].
    pub fn size(mut self, size: TableSize) -> Self {
        self.size = size;
        self
    }

    /// `appearance: primary`: semibold text and a 24 media.
    pub fn primary(mut self, primary: bool) -> Self {
        self.primary = primary;
        self
    }

    crate::widget::setters! {
        /// Replaces the `<id_salt>-table-cell-layout` agent key.
        agent_key(key)
    }

    fn media_side(&self) -> f32 {
        if self.primary {
            d::MEDIA_PRIMARY
        } else {
            self.size.media()
        }
    }

    // `TableCellLayout` truncate: one line per text, cut to `width` with an ellipsis.
    fn lines(
        &self,
        ui: &MineralUi<'_, Fluent>,
        color: egui::Color32,
        width: f32,
    ) -> (Arc<Galley>, Option<Arc<Galley>>) {
        let layout = |text: &str, ramp: TypeRamp, c| {
            let mut job = LayoutJob::single_section(text.to_owned(), ramp.format(ui.ctx(), c));
            job.wrap.max_width = width.max(0.0);
            job.wrap.max_rows = 1;
            job.wrap.break_anywhere = true;
            job.wrap.overflow_character = Some('\u{2026}');
            ui.painter().layout_job(job)
        };
        let main = if self.primary {
            TypeRamp::Body1Strong
        } else {
            TypeRamp::Body1
        };
        let muted = ui.tokens().color(ColorNeutralForeground2);
        (
            layout(&self.text, main, color),
            self.description
                .as_deref()
                .map(|s| layout(s, TypeRamp::Caption1, muted)),
        )
    }
}

impl MineralWidget for TableCellLayout<'_> {
    type Kit = Fluent;
    type State = ();
    type Output = ();

    kit::widget_meta!(salt, hover, no_target);

    fn layout(&self, ui: &MineralUi<'_, Fluent>, c: Constraints, (): &()) -> Measure {
        let media = if self.media.is_some() {
            self.media_side() + d::LAYOUT_GAP
        } else {
            0.0
        };
        let (main, desc) = self.lines(ui, egui::Color32::WHITE, c.max.x - media);
        let w = media + main.size().x.max(desc.as_ref().map_or(0.0, |g| g.size().x));
        let h = (main.size().y + desc.as_ref().map_or(0.0, |g| g.size().y)).max(if self.media.is_some() {
            self.media_side()
        } else {
            0.0
        });
        c.constrain(vec2(w, h)).into()
    }

    fn update(&mut self, ui: &mut MineralUi<'_, Fluent>, frame: &WidgetFrame<'_>, (): &mut ()) -> WidgetResponse {
        let t = ui.tokens();
        let rect = frame.zones.visual;
        let dir = ui.i18n().direction();
        let media = if self.media.is_some() {
            self.media_side() + d::LAYOUT_GAP
        } else {
            0.0
        };
        let (main, desc) = self.lines(ui, t.color(ColorNeutralForeground1), rect.width() - media);
        let mut x = 0.0;
        if let Some(avatar) = &self.media {
            let side = self.media_side();
            let px = dir.start_x(rect, Rangef::new(0.0, side));
            let at = Rect::from_min_size(pos2(px, rect.center().y - side / 2.0), vec2(side, side));
            let avatar_size = match (self.primary, self.size) {
                (true, _) => AvatarSize::S24,
                (false, TableSize::ExtraSmall) => AvatarSize::S16,
                (false, _) => AvatarSize::S20,
            };
            avatar.clone().size(avatar_size).paint(ui, at, &t);
            x = side + d::LAYOUT_GAP;
        }
        let total = main.size().y + desc.as_ref().map_or(0.0, |g| g.size().y);
        let mut y = rect.center().y - total / 2.0;
        for g in std::iter::once(main).chain(desc) {
            let gx = dir.start_x(rect, Rangef::new(x, x + g.size().x));
            let h = g.size().y;
            ui.painter().galley(pos2(gx, y), g, t.color(ColorNeutralForeground1));
            y += h;
        }
        WidgetResponse::new(frame.response.clone(), ())
    }

    fn semantics(&self, _: &I18n, _: &WidgetResponse, (): &()) -> Option<SemanticNode> {
        Some(SemanticNode::new(self.key.id(&self.salt), Role::Label, &self.text))
    }
}

/// Result of showing a [`Table`] or [`DataGrid`].
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct TableResponse<K> {
    /// Spans the table; `mark_changed` when the sort or selection changed.
    pub response: egui::Response,
    /// Agrees with `response.changed()`: the sort or the selection changed.
    pub changed: bool,
    /// Key of the row clicked or activated with Enter or Space this frame.
    pub activated: Option<K>,
}

struct SortKey<'a>(Box<dyn Fn(usize, SortState) -> Vec<usize> + 'a>);

impl std::fmt::Debug for SortKey<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SortKey")
    }
}

/// Fluent `Table` of `TableHeader`, `TableRow`, `TableCell` and `TableSelectionCell`: a header row and body rows
/// of cells whose content the app draws, with sortable columns, selectable rows and resizable widths.
#[derive(Debug)]
#[must_use = "call .show(..)"]
pub struct Table<'a, K> {
    salt: KeyedSalt,
    key: AgentKey,
    label: String,
    size: TableSize,
    appearance: TableRowAppearance,
    columns: Vec<TableColumn>,
    sort: Option<&'a mut SortState>,
    selection: Option<&'a mut Selection<K>>,
    widths: Option<&'a mut ColumnWidths>,
    sort_key: Option<SortKey<'a>>,
    grid: bool,
}

impl<'a, K: ItemKey> Table<'a, K> {
    /// `label` names the table for assistive technology; agent key `<id_salt>-table`.
    pub fn new(
        id_salt: impl Into<KeyedSalt>,
        label: impl Into<String>,
        columns: impl IntoIterator<Item = impl Into<TableColumn>>,
    ) -> Self {
        Self {
            salt: id_salt.into(),
            key: egui_mineral::agent_key!("table"),
            label: label.into(),
            size: TableSize::Medium,
            appearance: TableRowAppearance::Neutral,
            columns: columns.into_iter().map(Into::into).collect(),
            sort: None,
            selection: None,
            widths: None,
            sort_key: None,
            grid: false,
        }
    }

    /// Default [`TableSize::Medium`].
    pub fn size(mut self, size: TableSize) -> Self {
        self.size = size;
        self
    }

    /// Highlight of selected rows; default [`TableRowAppearance::Neutral`].
    pub fn row_appearance(mut self, appearance: TableRowAppearance) -> Self {
        self.appearance = appearance;
        self
    }

    /// `sortState`: headers of sortable columns cycle it; [`Self::sort_by_key`] orders the rows.
    pub fn sort(mut self, sort: &'a mut SortState) -> Self {
        self.sort = Some(sort);
        self
    }

    /// Selectable rows bound by row key: a checkbox column, or a circular one for a single selection.
    pub fn selection(mut self, selection: &'a mut Selection<K>) -> Self {
        self.selection = Some(selection);
        self
    }

    /// Resizable columns: drag the handle at each header's end.
    pub fn resizable(mut self, widths: &'a mut ColumnWidths) -> Self {
        self.widths = Some(widths);
        self
    }

    /// Sort key of a data cell; rows follow [`Self::sort`]'s column and direction, ties keep data order. Without
    /// it, sortable headers only cycle the state.
    pub fn sort_by_key<S: Ord>(mut self, key: impl Fn(Cell) -> S + 'a) -> Self {
        self.sort_key = Some(SortKey(Box::new(move |rows, sort| sort_order(rows, sort, &key))));
        self
    }

    crate::widget::setters! {
        /// Replaces the `<id_salt>-table` agent key.
        agent_key(key)
    }

    fn column_widths(&self, avail: f32, selection: f32) -> Vec<f32> {
        let n = self.columns.len();
        if let Some(w) = self.widths.as_deref().filter(|w| w.len() == n) {
            return (0..n).map(|i| w.width(i)).collect();
        }
        let fixed: f32 = self.columns.iter().filter_map(|c| c.width).sum();
        let flex = self.columns.iter().filter(|c| c.width.is_none()).count().max(1);
        let share = ((avail - selection - fixed) / flex as f32).max(0.0);
        self.columns.iter().map(|c| c.width.unwrap_or(share)).collect()
    }

    /// Shows the table; `rows` counts and keys the data rows, `cell` draws data row `row` in column `column`.
    pub fn show(
        self,
        ui: &mut MineralUi<'_, Fluent>,
        rows: impl Rows<K> + Copy,
        cell: impl FnMut(&mut MineralUi<'_, Fluent>, usize, usize),
    ) -> TableResponse<K> {
        run(self, ui, &rows, cell)
    }
}

/// Fluent `DataGrid`: a [`Table`] that is one composite widget: a single Tab stop, arrow keys, Home and End
/// move between cells, Enter or Space on a header sorts and on a cell selects the row.
#[derive(Debug)]
#[must_use = "call .show(..)"]
pub struct DataGrid<'a, K>(Table<'a, K>);

impl<'a, K: ItemKey> DataGrid<'a, K> {
    /// `label` names the grid for assistive technology; agent key `<id_salt>-table`.
    pub fn new(
        id_salt: impl Into<KeyedSalt>,
        label: impl Into<String>,
        columns: impl IntoIterator<Item = impl Into<TableColumn>>,
    ) -> Self {
        let mut table = Table::new(id_salt, label, columns);
        table.grid = true;
        Self(table)
    }

    /// See [`Table::size`].
    pub fn size(mut self, size: TableSize) -> Self {
        self.0 = self.0.size(size);
        self
    }

    /// See [`Table::row_appearance`].
    pub fn row_appearance(mut self, appearance: TableRowAppearance) -> Self {
        self.0 = self.0.row_appearance(appearance);
        self
    }

    /// See [`Table::sort`].
    pub fn sort(mut self, sort: &'a mut SortState) -> Self {
        self.0 = self.0.sort(sort);
        self
    }

    /// See [`Table::selection`].
    pub fn selection(mut self, selection: &'a mut Selection<K>) -> Self {
        self.0 = self.0.selection(selection);
        self
    }

    /// See [`Table::resizable`].
    pub fn resizable(mut self, widths: &'a mut ColumnWidths) -> Self {
        self.0 = self.0.resizable(widths);
        self
    }

    /// See [`Table::sort_by_key`].
    pub fn sort_by_key<S: Ord>(mut self, key: impl Fn(Cell) -> S + 'a) -> Self {
        self.0 = self.0.sort_by_key(key);
        self
    }

    /// See [`Table::show`].
    pub fn show(
        self,
        ui: &mut MineralUi<'_, Fluent>,
        rows: impl Rows<K> + Copy,
        cell: impl FnMut(&mut MineralUi<'_, Fluent>, usize, usize),
    ) -> TableResponse<K> {
        self.0.show(ui, rows, cell)
    }
}

// Row-major cell indices of the grid: the Tab stop, and the cell that takes focus once its row is shown.
#[derive(Clone, Copy, Debug, Default)]
struct GridFocus {
    stop: usize,
    pending: Option<usize>,
}

impl TempState for GridFocus {}

struct Grid {
    on: bool,
    columns: usize,
    focus: GridFocus,
    focused: Option<(usize, Id)>,
}

impl Grid {
    fn load(ctx: &egui::Context, id: Id, on: bool, columns: usize, cells: usize) -> Self {
        let mut focus = if on {
            GridFocus::load(ctx, id)
        } else {
            GridFocus::default()
        };
        focus.stop = focus.stop.min(cells.saturating_sub(1));
        focus.pending = focus.pending.filter(|p| *p < cells);
        Self {
            on,
            columns: columns.max(1),
            focus,
            focused: None,
        }
    }

    fn tab_stop(&self, index: usize) -> TabStop {
        if index == self.focus.stop || self.focus.pending == Some(index) {
            TabStop::Stop
        } else {
            TabStop::Skip
        }
    }

    fn sense(&self, index: usize, clickable: bool) -> Sense {
        if self.on {
            focus::roving_sense(Sense::click(), self.tab_stop(index))
        } else if clickable {
            Sense::click()
        } else {
            Sense::hover()
        }
    }

    fn row(&self, index: usize) -> usize {
        index / self.columns
    }

    fn track(&mut self, index: usize, r: &egui::Response) {
        if !self.on {
            return;
        }
        let pressed = r.is_pointer_button_down_on();
        let pending = self.focus.pending == Some(index);
        if pressed || pending {
            r.request_focus();
            self.focus.pending = None;
        }
        if pressed || pending || r.has_focus() {
            self.focus.stop = index;
            self.focused = Some((index, r.id));
        }
    }

    // Arrow, Home and End keys: the target takes focus when its row is shown, after scrolling to it.
    fn finish(mut self, ui: &mut MineralUi<'_, Fluent>, id: Id, cells: usize, geo: &Geometry) {
        if !self.on {
            return;
        }
        if let Some((current, focused)) = self.focused
            && let Some(next) = RovingFocus::grid(self.columns).on_key(ui.core(), focused, current, &Items::all(cells))
        {
            self.focus.pending = Some(next);
            ui.scroll_to_rect(geo.row(self.row(next)), None);
            #[expect(
                clippy::disallowed_methods,
                reason = "a key moved the grid focus: the target cell takes it in the next pass"
            )]
            ui.ctx().request_repaint();
        }
        self.focus.store(ui.ctx(), id);
    }
}

struct Geometry {
    area: Rect,
    dir: Direction,
    height: f32,
    selection: f32,
    widths: Vec<f32>,
}

impl Geometry {
    // Row 0 is the header.
    fn row(&self, r: usize) -> Rect {
        let top = self.area.min.y + self.height * r as f32;
        Rect::from_min_size(pos2(self.area.min.x, top), vec2(self.area.width(), self.height))
    }

    fn cell(&self, row: Rect, from: f32, width: f32) -> Rect {
        let x = self.dir.start_x(self.area, Rangef::new(from, from + width));
        Rect::from_min_size(pos2(x, row.min.y), vec2(width, self.height))
    }

    fn column(&self, c: usize) -> f32 {
        self.widths.get(c).copied().unwrap_or_default()
    }

    // Body rows that cross `clip`.
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "clamped to 0..=rows"
    )]
    fn visible(&self, clip: Rect, rows: usize) -> Range<usize> {
        let top = self.area.min.y + self.height;
        let at = |y: f32| ((y - top) / self.height).clamp(0.0, rows as f32);
        at(clip.min.y).floor() as usize..at(clip.max.y).ceil() as usize
    }
}

// Display order of the data rows.
struct Order {
    explicit: Option<Vec<usize>>,
    rows: usize,
}

impl Order {
    fn len(&self) -> usize {
        self.explicit.as_ref().map_or(self.rows, |o| o.len().min(self.rows))
    }

    fn data(&self, r: usize) -> Option<usize> {
        match &self.explicit {
            Some(o) => o.get(r).copied(),
            None => (r < self.rows).then_some(r),
        }
    }

    fn all(&self) -> Cow<'_, [usize]> {
        match &self.explicit {
            Some(o) => Cow::Borrowed(o.get(..self.len()).unwrap_or(o)),
            None => Cow::Owned((0..self.rows).collect()),
        }
    }

    // Stale keys in the selection do not count.
    fn selected<K: ItemKey>(&self, selection: &Selection<K>, rows: &dyn Rows<K>) -> usize {
        if selection.selected().next().is_none() {
            return 0;
        }
        self.keys(rows).filter(|k| selection.is_selected(k)).count()
    }

    fn keys<'r, K>(&self, rows: &'r dyn Rows<K>) -> impl Iterator<Item = K> + 'r {
        self.all().into_owned().into_iter().filter_map(|d| rows.key(d))
    }
}

struct Events<K> {
    sort: bool,
    selection: bool,
    activated: Option<K>,
}

fn single<K: ItemKey>(selection: Option<&Selection<K>>) -> bool {
    selection.is_some_and(|s| s.mode() == SelectionMode::Single)
}

fn run<K: ItemKey>(
    mut tb: Table<'_, K>,
    ui: &mut MineralUi<'_, Fluent>,
    keys: &dyn Rows<K>,
    mut cell: impl FnMut(&mut MineralUi<'_, Fluent>, usize, usize),
) -> TableResponse<K> {
    let id = ui.make_persistent_id(tb.salt.id_salt());
    let selectable = tb.selection.is_some();
    let selection = if selectable { d::SELECTION_WIDTH } else { 0.0 };
    let widths = tb.column_widths(ui.available_width(), selection);
    let order = Order {
        explicit: tb
            .sort
            .as_deref()
            .copied()
            .zip(tb.sort_key.as_ref())
            .map(|(sort, key)| (key.0)(keys.len(), sort)),
        rows: keys.len(),
    };
    let shown = order.len();
    let height = tb.size.row_height();
    let columns = tb.columns.len() + usize::from(selectable);
    let cells = (shown + 1) * columns;
    let size = vec2(selection + widths.iter().sum::<f32>(), height * (shown + 1) as f32);
    let (area, response) = ui.allocate_exact_size(size, Sense::hover());
    let geo = Geometry {
        area,
        dir: ui.i18n().direction(),
        height,
        selection,
        widths,
    };
    let grid_id = id.with("mineral::grid");
    let mut grid = Grid::load(ui.ctx(), grid_id, tb.grid, columns, cells);
    let mut ev = Events {
        sort: false,
        selection: false,
        activated: None,
    };
    let pass = Pass {
        id,
        geo: &geo,
        order: &order,
        keys,
    };
    header(ui, &mut tb, &pass, &mut grid, &mut ev);
    let visible = geo.visible(ui.clip_rect(), shown);
    // The Tab stop's row is shown wherever it is, so Tab still reaches the grid.
    let stop = grid
        .on
        .then(|| grid.row(grid.focus.stop).checked_sub(1))
        .flatten()
        .filter(|r| !visible.contains(r));
    let drawn = (stop.filter(|s| *s < visible.start).into_iter())
        .chain(visible.clone())
        .chain(stop.filter(|s| *s >= visible.end));
    for r in drawn {
        let Some((data, key)) = order.data(r).and_then(|d| Some((d, keys.key(d)?))) else {
            continue;
        };
        body_row(ui, &mut tb, &pass, &mut grid, &mut ev, (r, data, key), &mut cell);
    }
    if kit::nodes::wanted(ui.ctx(), None) {
        for r in (0..shown).filter(|r| !visible.contains(r) && stop != Some(*r)) {
            if let Some(key) = order.data(r).and_then(|d| keys.key(d)) {
                culled_row(ui, &tb, &pass, r, &key);
            }
        }
    }
    grid.finish(ui, grid_id, cells, &geo);
    semantic::emit(ui.ctx(), Some(response.id), area, || {
        SemanticNode::new(tb.key.id(&tb.salt), Role::Grid, &tb.label)
    });
    let changed = ev.sort || ev.selection;
    let mut response = response;
    if changed {
        response.mark_changed();
    }
    TableResponse {
        response,
        changed,
        activated: ev.activated,
    }
}

fn header<K: ItemKey>(
    ui: &mut MineralUi<'_, Fluent>,
    tb: &mut Table<'_, K>,
    pass: &Pass<'_, K>,
    grid: &mut Grid,
    ev: &mut Events<K>,
) {
    let Pass {
        id, geo, order, keys, ..
    } = *pass;
    let rt = ui.rt();
    let row = geo.row(0);
    let selectable = tb.selection.is_some();
    if selectable {
        let rect = geo.cell(row, 0.0, geo.selection);
        if single(tb.selection.as_deref()) {
            if grid.on {
                let r = ui.interact(rect, id.with("select-all"), grid.sense(0, false));
                grid.track(0, &r);
            }
        } else {
            let selected = tb.selection.as_deref().map_or(0, |s| order.selected(s, keys));
            let mut all = selected == order.len() && selected > 0;
            let label = crate::tr(rt.i18n(), msg::TABLE_SELECT_ALL);
            let mut check = Checkbox::new(tb.salt.child("select-all"), label, &mut all)
                .mixed(selected > 0 && selected < order.len())
                .label_visible(false);
            check.tab_stop = grid.on.then(|| grid.tab_stop(0));
            let r = place(ui, rect, |ui| rt.ui(ui).mineral(check));
            grid.track(0, r.response());
            if r.changed()
                && let Some(s) = tb.selection.as_mut()
            {
                if all {
                    for k in order.keys(keys) {
                        s.select(k);
                    }
                } else {
                    s.clear();
                }
                ev.selection = true;
            }
        }
    }
    let mut x = geo.selection;
    for (c, col) in tb.columns.iter().enumerate() {
        let w = geo.column(c);
        let rect = geo.cell(row, x, w);
        let index = c + usize::from(selectable);
        let hid = id.with(("header", c));
        let r = ui.interact(rect, hid, grid.sense(index, col.sortable));
        grid.track(index, &r);
        let sorted = tb.sort.as_ref().and_then(|s| s.order(c));
        header_cell(ui, rect, col, sorted, &r);
        if r.clicked()
            && col.sortable
            && let Some(s) = tb.sort.as_mut()
        {
            s.cycle(c);
            ev.sort = true;
        }
        semantic::emit(ui.ctx(), Some(hid), rect, || {
            let label = || (&col.label).into();
            let name = match sorted {
                Some(SortOrder::Ascending) => rt.i18n().tr(msg::TableSortedAscending { label: label() }),
                Some(SortOrder::Descending) => rt.i18n().tr(msg::TableSortedDescending { label: label() }),
                None => col.label.clone(),
            };
            let mut n = SemanticNode::new(StableId::of(&tb.salt, ("header", c)), Role::ColumnHeader, name);
            if col.sortable {
                n = n.clickable(Availability::Enabled);
            }
            n
        });
        if let Some(widths) = tb.widths.as_mut() {
            let edge = geo.cell(row, x + w, 0.0).left_center();
            resize_handle(ui, id.with(("resize", c)), edge, geo, c, widths);
        }
        x += w;
    }
}

fn resize_handle(
    ui: &MineralUi<'_, Fluent>,
    id: Id,
    edge: egui::Pos2,
    geo: &Geometry,
    c: usize,
    widths: &mut ColumnWidths,
) {
    let handle = Rect::from_center_size(edge, vec2(d::RESIZE_HANDLE, geo.height));
    let r = ui.interact(handle, id, Sense::drag());
    if r.hovered() || r.dragged() {
        ui.ctx().set_cursor_icon(CursorIcon::ResizeHorizontal);
        let line = Rect::from_center_size(edge, vec2(d::RESIZE_LINE, geo.height));
        ui.painter()
            .rect_filled(line, 0.0, ui.tokens().color(ColorNeutralStroke1));
    }
    if r.dragged() {
        widths.set(c, widths.width(c) + r.drag_delta().x * geo.dir.sign());
    }
}

struct Pass<'p, K> {
    id: Id,
    geo: &'p Geometry,
    order: &'p Order,
    keys: &'p dyn Rows<K>,
}

fn body_row<K: ItemKey>(
    ui: &mut MineralUi<'_, Fluent>,
    tb: &mut Table<'_, K>,
    pass: &Pass<'_, K>,
    grid: &mut Grid,
    ev: &mut Events<K>,
    (r, data, key): (usize, usize, K),
    cell: &mut impl FnMut(&mut MineralUi<'_, Fluent>, usize, usize),
) {
    let rt = ui.rt();
    let tokens = ui.tokens();
    let Pass { id, geo, order, keys } = *pass;
    let rect = geo.row(r + 1);
    let rid = id.with(("row", &key));
    let selectable = tb.selection.is_some();
    let sense = if selectable && !grid.on {
        Sense::click()
    } else {
        Sense::hover()
    };
    let row = ui.interact(rect, rid, sense);
    let selected = tb.selection.as_ref().is_some_and(|s| s.is_selected(&key));
    row_background(ui, rect, tb.appearance, selected, &row, &tokens);
    let mut clicked = row.clicked();
    let first = (r + 1) * grid.columns;
    if selectable {
        let single = single(tb.selection.as_deref());
        let mut on = selected;
        let label = crate::tr(rt.i18n(), msg::TABLE_SELECT_ROW);
        let mut check = Checkbox::new(tb.salt.child(("select", key.clone())), label, &mut on).label_visible(false);
        if single {
            check = check.shape(CheckboxShape::Circular);
        }
        check.tab_stop = grid.on.then(|| grid.tab_stop(first));
        let c = place(ui, geo.cell(rect, 0.0, geo.selection), |ui| rt.ui(ui).mineral(check));
        grid.track(first, c.response());
        if c.changed()
            && let Some(s) = tb.selection.as_mut()
        {
            ev.selection |= if single {
                s.select(key.clone())
            } else {
                s.toggle(key.clone())
            };
            ev.activated = Some(key.clone());
        }
    }
    let mut x = geo.selection;
    for c in 0..tb.columns.len() {
        let w = geo.column(c);
        let crect = geo.cell(rect, x, w);
        if grid.on {
            let index = first + c + usize::from(selectable);
            let resp = ui.interact(crect, id.with(("cell", &key, c)), grid.sense(index, true));
            grid.track(index, &resp);
            if focus::visible(ui.ctx(), resp.id) {
                focus_outline(ui, crect, &tokens);
            }
            clicked |= resp.clicked();
        }
        // Cell content clipped to its cell.
        place(ui, crect.shrink2(vec2(d::CELL_PADDING, 0.0)), |ui| {
            ui.set_clip_rect(ui.max_rect().intersect(ui.clip_rect()));
            cell(&mut rt.ui(ui), data, c);
        });
        x += w;
    }
    if clicked {
        if let Some(s) = tb.selection.as_mut() {
            let modifiers = ui.input(|i| i.modifiers);
            let all: Vec<K> = order.keys(keys).collect();
            ev.selection |= s.click(key.clone(), modifiers, &all);
        }
        ev.activated = Some(key.clone());
    }
    // `useTableRowStyles`: `extra-small` rows have no `borderBottom`.
    if tb.size != TableSize::ExtraSmall {
        ui.painter().hline(
            rect.x_range(),
            rect.bottom() - d::ROW_BORDER / 2.0,
            Stroke::new(d::ROW_BORDER, tokens.color(ColorNeutralStroke2)),
        );
    }
    emit_row(ui, tb, rid, rect, &key);
}

fn culled_row<K: ItemKey>(ui: &MineralUi<'_, Fluent>, tb: &Table<'_, K>, pass: &Pass<'_, K>, r: usize, key: &K) {
    let rid = pass.id.with(("row", key));
    kit::nodes::emit_culled(ui, rid, pass.geo.row(r + 1), || row_node(tb, key));
}

fn emit_row<K: ItemKey>(ui: &MineralUi<'_, Fluent>, tb: &Table<'_, K>, rid: Id, rect: Rect, key: &K) {
    semantic::emit(ui.ctx(), Some(rid), rect, || {
        let mut n = row_node(tb, key);
        n.actions.push(semantic::Action::ScrollIntoView);
        n
    });
}

fn row_node<K: ItemKey>(tb: &Table<'_, K>, key: &K) -> SemanticNode {
    let mut n = SemanticNode::new(StableId::of(&tb.salt, ("row", key.clone())), Role::Row, "");
    n.state.selected = tb.selection.as_ref().map(|s| s.is_selected(key));
    n
}

fn place<R>(ui: &mut egui::Ui, rect: Rect, add: impl FnOnce(&mut egui::Ui) -> R) -> R {
    add(&mut ui.new_child(UiBuilder::new().max_rect(rect)))
}

fn focus_outline(ui: &egui::Ui, rect: Rect, t: &crate::theme::Tokens) {
    ui.painter().rect_stroke(
        rect.shrink(d::FOCUS_OUTLINE / 2.0),
        crate::theme::Radius::Medium.on(rect),
        Stroke::new(d::FOCUS_OUTLINE, t.color(ColorStrokeFocus2)),
        StrokeKind::Inside,
    );
}

fn header_cell(
    ui: &MineralUi<'_, Fluent>,
    rect: Rect,
    col: &TableColumn,
    order: Option<SortOrder>,
    r: &egui::Response,
) {
    let t = ui.tokens();
    let p = ui.painter();
    // `useTableHeaderCellStyles.rootInteractive`
    let color = if col.sortable && r.is_pointer_button_down_on() {
        p.rect_filled(rect, 0.0, t.color(ColorSubtleBackgroundPressed));
        t.color(ColorNeutralForeground1Pressed)
    } else if col.sortable && r.hovered() {
        p.rect_filled(rect, 0.0, t.color(ColorSubtleBackgroundHover));
        t.color(ColorNeutralForeground1Hover)
    } else {
        t.color(ColorNeutralForeground1)
    };
    let galley = p.layout_job(LayoutJob::single_section(
        col.label.clone(),
        TypeRamp::Body1.format(ui.ctx(), color),
    ));
    let dir = ui.i18n().direction();
    let text = galley.size();
    let x = dir.start_x(rect, Rangef::new(d::CELL_PADDING, d::CELL_PADDING + text.x));
    p.galley(pos2(x, rect.center().y - text.y / 2.0), galley, color);
    if let Some(order) = order {
        let from = d::CELL_PADDING + text.x + d::HEADER_GAP;
        let icon_x = dir.start_x(rect, Rangef::new(from, from + d::SORT_ICON)) + d::SORT_ICON / 2.0;
        let icon = match order {
            SortOrder::Ascending => ui.icons().arrow_up,
            SortOrder::Descending => ui.icons().arrow_down,
        };
        // The padded icon box is centered, so the glyph sits half the top padding lower.
        let icon_y = rect.center().y + d::SORT_ICON_PADDING_TOP / 2.0;
        paint::icon(p, pos2(icon_x, icon_y), icon, false, d::SORT_ICON, color);
    }
    if focus::visible(ui.ctx(), r.id) {
        focus_outline(ui, rect, &t);
    }
}

// `useTableRowStyles`: `rootInteractive` hover and pressed over the `appearance` of a selected row, whose
// `:active` keeps the selected color.
fn row_background(
    ui: &MineralUi<'_, Fluent>,
    rect: Rect,
    appearance: TableRowAppearance,
    selected: bool,
    r: &egui::Response,
    t: &crate::theme::Tokens,
) {
    let idle = match appearance {
        TableRowAppearance::Neutral if selected => Some(ColorSubtleBackgroundSelected),
        TableRowAppearance::Brand if selected => Some(ColorBrandBackground2),
        _ => None,
    };
    // Cells and the selection checkbox cover the row: the pointer over any of them hovers it.
    let hovered = r.contains_pointer();
    let token = if hovered && ui.input(|i| i.pointer.primary_down()) {
        idle.or(Some(ColorSubtleBackgroundPressed))
    } else if hovered {
        Some(ColorSubtleBackgroundHover)
    } else {
        idle
    };
    if let Some(token) = token {
        ui.painter().rect_filled(rect, 0.0, t.color(token));
    }
}
