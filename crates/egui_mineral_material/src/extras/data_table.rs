use crate::Material;
use crate::MaterialExt;
use crate::MaterialUiExt as _;
use crate::Rt;
use egui_mineral::kit::state::TempState;
use egui_mineral::semantic::KeyedSalt;
use egui_mineral::widget::MineralUi;
use std::{borrow::Cow, cmp::Ordering, sync::Arc};

use egui::{Color32, Context, CursorIcon, Id, Modifiers, Pos2, Rangef, Rect, Response, Sense, Stroke, Ui, pos2, vec2};
use egui_mineral::widget::Availability;
use egui_mineral::{
    focus::{self, Cell, GridSize},
    i18n::{Direction, LocaleFormat},
    kit,
    layout::MinTarget,
    pagination::Paging,
    perf::VirtualList,
    range::{RangeModel, Track},
    select::{Check, Selection},
    semantic::{self, Action, Role, SemanticNode, StableId},
    table::{ColumnSize, ColumnWidths, SortOrder, SortState, sort_order},
};

use crate::actions::{ButtonIcon, ButtonVariant, IconButton, PushButton};
use crate::selection::{Menu, MenuItem};
use crate::widget::{
    common::{self, Interaction},
    indication, parts,
    row_parts::{self, RowColors, RowState},
};
use crate::{
    spec::{
        TypeScale, defaults,
        tokens::{button_small, divider, filled_text_field, list},
    },
    tokens::Tokens,
};

// MD3 has no data table; type follows the M2 roles
// M2 header Subtitle 2
const HEADER_FONT: TypeScale = TypeScale::TitleSmall;
// M2 cell Body 2
const CELL_FONT: TypeScale = TypeScale::BodyMedium;
// M2 pagination Caption
const FOOTER_FONT: TypeScale = TypeScale::BodySmall;
const SORT_ICON: f32 = defaults::button::ICON_SIZE;
const SORT_ICON_GAP: f32 = button_small::ICON_LABEL_SPACE;
const CELL_START: f32 = defaults::list::START_PADDING;
const CELL_END: f32 = defaults::list::END_PADDING;
const DEFAULT_COLUMN_WIDTH: f32 = 160.0;
const MIN_COLUMN_WIDTH: f32 = CELL_START + list::ITEM_LEADING_ICON_SIZE + CELL_END;
const MAX_COLUMN_WIDTH: f64 = 4096.0;

/// Cell value a [`DataTable`] renders; numbers are formatted per locale.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum TableCell<'a> {
    /// Start-aligned text.
    Text(Cow<'a, str>),
    /// End-aligned number, formatted per locale.
    Number(f64),
    /// Blank cell.
    Empty,
}

impl<'a> From<&'a str> for TableCell<'a> {
    fn from(s: &'a str) -> Self {
        Self::Text(Cow::Borrowed(s))
    }
}

impl From<String> for TableCell<'_> {
    fn from(s: String) -> Self {
        Self::Text(Cow::Owned(s))
    }
}

impl From<f64> for TableCell<'_> {
    fn from(v: f64) -> Self {
        Self::Number(v)
    }
}

impl From<i32> for TableCell<'_> {
    fn from(v: i32) -> Self {
        Self::Number(v.into())
    }
}

impl From<u32> for TableCell<'_> {
    fn from(v: u32) -> Self {
        Self::Number(v.into())
    }
}

/// Integers past 2^53 round to the nearest `f64`.
macro_rules! cell_from_wide {
    ($($ty:ty),+) => {$(
        impl From<$ty> for TableCell<'_> {
            #[expect(clippy::cast_precision_loss, reason = "cells hold f64")]
            fn from(v: $ty) -> Self {
                Self::Number(v as f64)
            }
        }
    )+};
}

cell_from_wide!(i64, u64, usize);

struct CellKey<'a> {
    cell: TableCell<'a>,
    locale: &'a dyn LocaleFormat,
}

impl Ord for CellKey<'_> {
    fn cmp(&self, other: &Self) -> Ordering {
        let rank = |c: &TableCell<'_>| match c {
            TableCell::Number(_) => 0,
            TableCell::Text(_) => 1,
            TableCell::Empty => 2,
        };
        match (&self.cell, &other.cell) {
            (TableCell::Number(x), TableCell::Number(y)) => x.total_cmp(y),
            (TableCell::Text(x), TableCell::Text(y)) => self.locale.compare(x, y),
            (a, b) => rank(a).cmp(&rank(b)),
        }
    }
}

impl PartialOrd for CellKey<'_> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for CellKey<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == Ordering::Equal
    }
}

impl Eq for CellKey<'_> {}

/// Column definition of a [`DataTable`].
#[derive(Clone, Debug)]
#[must_use]
pub struct TableColumn {
    title: String,
    width: f32,
    min_width: f32,
    numeric: bool,
    sortable: bool,
    resizable: bool,
    fraction_digits: Option<u8>,
}

impl TableColumn {
    /// `title` is the header text and accessible name; sortable and resizable by default.
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            width: DEFAULT_COLUMN_WIDTH,
            min_width: MIN_COLUMN_WIDTH,
            numeric: false,
            sortable: true,
            resizable: true,
            fraction_digits: None,
        }
    }
    /// Initial width in dp; non-positive values snap to the minimum.
    #[track_caller]
    pub fn width(mut self, width: f32) -> Self {
        self.width = egui_mineral::kit::sanitize::at_least(width, f32::MIN_POSITIVE.., "width");
        self
    }
    /// Resize floor in dp; non-positive values snap to the minimum.
    #[track_caller]
    pub fn min_width(mut self, min_width: f32) -> Self {
        self.min_width = egui_mineral::kit::sanitize::at_least(min_width, f32::MIN_POSITIVE.., "min_width");
        self
    }
    /// End-aligns header and cells.
    pub fn numeric(mut self, numeric: bool) -> Self {
        self.numeric = numeric;
        self
    }

    /// Fixed fraction digits of number cells.
    pub fn fraction_digits(mut self, fraction_digits: u8) -> Self {
        self.fraction_digits = Some(fraction_digits);
        self
    }
    /// Header click cycles the sort; default `true`.
    pub fn sortable(mut self, sortable: bool) -> Self {
        self.sortable = sortable;
        self
    }
    /// Header edge drag resizes; default `true`.
    pub fn resizable(mut self, resizable: bool) -> Self {
        self.resizable = resizable;
        self
    }
}

impl TempState for TableState {}

#[derive(Clone)]
struct TableState {
    sort: SortState,
    widths: ColumnWidths,
    paging: Option<Paging>,
    order: Arc<Vec<usize>>,
    order_key: Option<OrderKey>,
    pending: Option<Cell>,
    stop: Option<Cell>,
}

/// Result of [`DataTable::show`].
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct DataTableResponse {
    /// Table response; marked changed on selection changes.
    pub response: egui::Response,
    /// Data row index activated this frame.
    pub activated: Option<usize>,
    /// Sort after this frame.
    pub sort: SortState,
    /// Sort changed this frame.
    pub sort_changed: bool,
    /// Selection only; sort and page changes have their own flags.
    pub changed: bool,
    /// Page or page size changed this frame.
    pub page_changed: bool,
}

#[derive(Default)]
struct Events {
    activated: Option<usize>,
    sort: SortState,
    sort_changed: bool,
    changed: bool,
    page_changed: bool,
}

enum RowEvent {
    Check(usize, Modifiers),
    Click(usize, Modifiers),
}

type SortFn<'a> = Box<dyn Fn(usize, SortState) -> Vec<usize> + 'a>;

/// Virtualized table with sorting, selection, resizable columns and paging; non-MD3 (MUI X `DataGrid` pattern).
#[must_use = "call .show(..)"]
pub struct DataTable<'a> {
    salt: KeyedSalt,
    label: String,
    columns: Vec<TableColumn>,
    rows: usize,
    selection: Option<&'a mut Selection<usize>>,
    page_sizes: Vec<usize>,
    max_height: Option<f32>,
    data_version: u64,
    sort_key: Option<SortFn<'a>>,
    disabled: bool,
}

egui_mineral::kit::opaque_debug!(DataTable<'_>);

struct Geometry {
    dir: Direction,
    sel_w: f32,
    slot: f32,
    table_w: f32,
    cols: Vec<(f32, f32)>,
}

impl Geometry {
    fn x_range(&self, area: Rect, x: f32, w: f32) -> egui::Rangef {
        let x0 = self.dir.start_x(area, Rangef::new(x, x + w));
        egui::Rangef::new(x0, x0 + w)
    }

    fn cell(&self, row: Rect, c: usize) -> Rect {
        let (x, w) = self.cols[c];
        Rect::from_x_y_ranges(self.x_range(row, x, w), row.y_range())
    }

    fn check(&self, row: Rect) -> Rect {
        Rect::from_x_y_ranges(self.x_range(row, 0.0, self.sel_w), row.y_range())
    }

    fn check_center(&self, row: Rect) -> Pos2 {
        pos2(self.x_range(row, CELL_START, self.slot).center(), row.center().y)
    }
}

impl<'a> DataTable<'a> {
    /// `label` names the table for assistive technology; `rows` is the data row count. Agent key prefix `<id_salt>`.
    pub fn new(
        id_salt: impl Into<KeyedSalt>,
        label: impl Into<String>,
        columns: impl IntoIterator<Item = impl Into<TableColumn>>,
        rows: usize,
    ) -> Self {
        let id_salt: KeyedSalt = id_salt.into();
        Self {
            salt: id_salt,
            label: label.into(),
            columns: columns.into_iter().map(Into::into).collect(),
            rows,
            selection: None,
            page_sizes: Vec::new(),
            max_height: None,
            data_version: 0,
            sort_key: None,
            disabled: false,
        }
    }

    crate::widget::setters! {
        /// Replaces the `<id_salt>` key prefix.
        agent_key(salt)
    }

    /// Checkbox column bound to selected data row indices.
    pub fn selection(mut self, selection: &'a mut Selection<usize>) -> Self {
        self.selection = Some(selection);
        self
    }

    /// Pages rows with a footer; the first size is the initial one; sizes are at least 1.
    #[track_caller]
    pub fn paginate(mut self, sizes: impl IntoIterator<Item = usize>) -> Self {
        self.page_sizes = sizes
            .into_iter()
            .map(|s| egui_mineral::kit::sanitize::at_least(s, 1.., "DataTable page size"))
            .collect();
        self
    }

    /// Body height cap in dp; rows scroll past it; non-positive values snap to the minimum.
    #[track_caller]
    pub fn max_height(mut self, max_height: f32) -> Self {
        self.max_height = Some(egui_mineral::kit::sanitize::at_least(
            max_height,
            f32::MIN_POSITIVE..,
            "max_height",
        ));
        self
    }

    /// Version of the cell data; default 0. The sorted row order is cached on sort, rows, `data_version` and locale:
    /// change the version whenever cell values change, or a sorted table keeps the stale order.
    pub fn data_version(mut self, data_version: u64) -> Self {
        self.data_version = data_version;
        self
    }

    /// Custom sort key of a data cell; default compares cell values.
    pub fn sort_by_key<K: Ord>(mut self, key: impl Fn(Cell) -> K + 'a) -> Self {
        self.sort_key = Some(Box::new(move |rows, sort| sort_order(rows, sort, &key)));
        self
    }

    crate::widget::setters! {
        /// Greys out the table; rows cannot be selected or activated.
        disabled
    }

    fn widths(&self) -> ColumnWidths {
        ColumnWidths::new(self.columns.iter().map(|c| ColumnSize {
            width: c.width,
            min: c.min_width,
        }))
    }

    fn cell_text<'v>(&self, fmt: &dyn LocaleFormat, c: usize, v: &'v TableCell<'_>) -> Cow<'v, str> {
        match v {
            TableCell::Text(s) => Cow::Borrowed(s),
            TableCell::Number(n) => fmt
                .number_lossy(*n, self.columns[c].fraction_digits.map(i16::from))
                .into(),
            TableCell::Empty => Cow::Borrowed(""),
        }
    }

    /// `cell(row, column)` yields each visible cell; rows are data indices.
    pub fn show<'d>(
        self,
        ui: &mut MineralUi<'_, Material>,
        cell: impl Fn(usize, usize) -> TableCell<'d>,
    ) -> DataTableResponse {
        let rt = ui.rt();
        let out = ui.scope(|ui| self.run(&mut rt.ui(ui), &cell));
        let e = out.inner;
        let mut response = out.response;
        if e.changed {
            response.mark_changed();
        }
        DataTableResponse {
            response,
            activated: e.activated,
            sort: e.sort,
            sort_changed: e.sort_changed,
            changed: e.changed,
            page_changed: e.page_changed,
        }
    }

    fn load_state<'d>(&self, rt: &Rt, id: Id, cell: &dyn Fn(usize, usize) -> TableCell<'d>) -> TableState {
        let ctx = rt.ctx();
        let mut st = TableState::load_or(ctx, id, || TableState {
            sort: SortState::default(),
            widths: self.widths(),
            paging: self.page_sizes.first().map(|s| Paging::new(*s)),
            order: Arc::new(Vec::new()),
            order_key: None,
            pending: None,
            stop: None,
        });
        // Column set changed: sort and widths may index removed columns.
        if st.widths.len() != self.columns.len() {
            st.widths = self.widths();
            st.sort = SortState::default();
        }
        match (self.page_sizes.first(), &mut st.paging) {
            (None, paging) => *paging = None,
            (Some(&first), None) => st.paging = Some(Paging::new(first)),
            (Some(&first), Some(p)) if !self.page_sizes.contains(&p.size()) => p.set_size(first),
            (Some(_), Some(_)) => {}
        }
        let order_key = OrderKey {
            sort: st.sort,
            rows: self.rows,
            data_version: self.data_version,
            i18n: rt.i18n().revision(),
        };
        if st.order_key != Some(order_key) {
            let order = if let Some(f) = &self.sort_key {
                f(self.rows, st.sort)
            } else {
                let locale = rt.i18n().format();
                sort_order(self.rows, st.sort, |c| CellKey {
                    cell: cell(c.row, c.column),
                    locale: &*locale,
                })
            };
            st.order = Arc::new(order);
            st.order_key = Some(order_key);
        }
        st
    }

    fn emit_grid(&self, ctx: &Context, id: Id) {
        semantic::emit_parent(ctx, id, || {
            let mut n = SemanticNode::new(StableId::of(&self.salt, "grid"), Role::Grid, &self.label);
            n.state.selected = None;
            n.state.disabled = self.disabled;
            n
        });
    }

    fn footer(&self, ui: &mut Ui, rt: &Rt, row_h: f32, t: &Tokens, paging: &mut Paging) -> bool {
        let (salt, sizes, rows, disabled) = (&self.salt, &self.page_sizes, self.rows, self.disabled);
        let dir = rt.i18n().direction();
        let fmt = rt.i18n().format();
        let before = *paging;
        let layout = dir.reversed().row();
        let text_color = t.role(list::ITEM_SUPPORTING_TEXT_COLOR);
        let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), row_h), Sense::hover());
        egui_mineral::layout::child(&mut rt.ui(ui), "footer", rect, layout, |ui| {
            ui.spacing_mut().item_spacing.x = crate::adaptive::grid(2);
            let back = dir.pick(ui.icons().chevron_left, ui.icons().chevron_right);
            let fwd = dir.pick(ui.icons().chevron_right, ui.icons().chevron_left);
            let last = paging.page_count(rows) - 1;
            if ui
                .mineral(
                    IconButton::new(salt.child("next"), fwd, crate::tr(rt.i18n(), "mineral-pagination-next"))
                        .disabled(disabled || paging.index() >= last),
                )
                .clicked()
            {
                paging.set_index(paging.index() + 1, rows);
            }
            if ui
                .mineral(
                    IconButton::new(
                        salt.child("prev"),
                        back,
                        crate::tr(rt.i18n(), "mineral-pagination-prev"),
                    )
                    .disabled(disabled || paging.index() == 0),
                )
                .clicked()
            {
                paging.set_index(paging.index() - 1, rows);
            }
            let range = paging.range(rows);
            let (s, e) = if range.is_empty() {
                (0, 0)
            } else {
                (range.start + 1, range.end)
            };
            let text = rt.i18n().tr(
                "mineral-table-range",
                &[
                    ("start", fmt.count(s).into()),
                    ("end", fmt.count(e).into()),
                    ("total", fmt.count(rows).into()),
                ],
            );
            ui.add_space(crate::adaptive::grid(4));
            parts::type_label(ui, &text, FOOTER_FONT, text_color, 1);
            ui.add_space(crate::adaptive::grid(4));
            // MDC data table pagination: "Rows per page" label, then the value with a trailing drop-down arrow.
            if sizes.len() > 1 {
                let anchor = ui.mineral(
                    PushButton::new(salt.child("page-size"), fmt.count(paging.size()))
                        .variant(ButtonVariant::Text)
                        .icon(ButtonIcon::Trailing(ui.icons().arrow_drop_down))
                        .disabled(disabled),
                );
                let mut menu = Menu::new(
                    salt.child("page-size-menu"),
                    crate::tr(rt.i18n(), "mineral-table-rows-per-page"),
                )
                .emit_anchor(false);
                for s in sizes {
                    menu = menu.item(MenuItem::new(fmt.count(*s)).selected(*s == paging.size()));
                }
                if let Some(k) = menu.show(rt, anchor.response()).clicked {
                    paging.set_size(sizes[k]);
                }
                parts::type_label(
                    ui,
                    &crate::tr(rt.i18n(), "mineral-table-rows-per-page"),
                    FOOTER_FONT,
                    text_color,
                    1,
                );
            }
        });
        let changed = *paging != before;
        if changed && ui.ctx().current_pass_index() == 0 {
            // The body above already showed the old page.
            ui.ctx().request_discard("table page changed after the body laid out");
        }
        changed
    }

    fn geometry(&self, widths: &ColumnWidths, dir: Direction, sel_w: f32, slot: f32, avail_w: f32) -> Geometry {
        let extra = (avail_w - sel_w - widths.total()).max(0.0);
        let n_cols = self.columns.len();
        Geometry {
            dir,
            sel_w,
            slot,
            table_w: sel_w + widths.total() + extra,
            cols: (0..n_cols)
                .map(|c| {
                    let w = widths.width(c) + if c + 1 == n_cols { extra } else { 0.0 };
                    (sel_w + widths.offset(c), w)
                })
                .collect(),
        }
    }

    fn run<'d>(mut self, ui: &mut MineralUi<'_, Material>, cell: &dyn Fn(usize, usize) -> TableCell<'d>) -> Events {
        let rt = ui.rt();
        let ctx = rt.ctx();
        let t = rt.tokens();
        let row_h = row_parts::row_height(ui.density());
        let id = ui.make_persistent_id(self.salt.id_salt());
        let dir = rt.i18n().direction();
        let mut st = self.load_state(rt, id, cell);
        let order = st.order.clone();
        let page = match st.paging.as_mut() {
            Some(p) => {
                p.clamp_index(self.rows);
                p.range(self.rows)
            }
            None => 0..self.rows,
        };
        let page_rows = page.len();
        let mut out = Events::default();

        let has_sel = self.selection.is_some();
        let slot = row_parts::control_slot(rt);
        let sel_w = if has_sel { CELL_START + slot } else { 0.0 };
        let avail_w = ui.available_width();
        let geo = self.geometry(&st.widths, dir, sel_w, slot, avail_w);
        let grid_cols = self.columns.len() + usize::from(has_sel);
        let footer_h = if st.paging.is_some() { row_h } else { 0.0 };
        let body_h = self
            .max_height
            .unwrap_or_else(|| ui.available_height() - row_h - footer_h)
            .min(row_h * page_rows as f32)
            .max(row_h);
        #[expect(clippy::cast_sign_loss, reason = ">= 1")]
        let view_rows = (body_h / row_h).floor().max(1.0) as usize;

        self.emit_grid(ctx, id);

        let first_body = usize::from(page_rows > 0);
        let stop = st
            .pending
            .or(st.stop)
            .filter(|p| p.row <= page_rows && p.column < grid_cols)
            .unwrap_or(Cell {
                row: first_body,
                column: 0,
            });
        let mut pass = Pass::default();
        let view = TableView {
            table: &self,
            rt,
            fmt: rt.i18n().format(),
            ctx,
            t: &t,
            geo,
            id,
            stop,
            row_h,
            body_h,
            order: &order,
            page: page.clone(),
            cell_shape: list::ITEM_CONTAINER_SHAPE.corner_radius(&t.shape, dir),
        };
        let start = ui.cursor().min;
        if view.geo.table_w > avail_w + 0.5 {
            egui::ScrollArea::horizontal()
                .id_salt((self.salt.id_salt(), "mineral::table_h"))
                .show(ui, |ui| view.grid(&mut rt.ui(ui), &mut st, &mut pass, cell));
        } else {
            view.grid(ui, &mut st, &mut pass, cell);
        }

        if let Some((cell_pos, fid)) = pass.focused {
            kit::focus::lock_arrows(ctx, fid, kit::focus::Arrows::Both);
            let grid = focus::Grid::new(GridSize {
                rows: page_rows + 1,
                columns: grid_cols,
            })
            .jump_rows(view_rows);
            if let Some(target) = grid_focus_target(ctx, cell_pos, grid, dir) {
                st.pending = Some(target);
            }
        }
        if let Some(c) = pass.sort_click {
            st.sort.cycle(c);
            out.sort_changed = true;
        }
        let page_ids = order.get(page).unwrap_or_default();
        self.apply_selection(&pass, page_ids, &order, &mut out);
        out.activated = pass.events.iter().rev().find_map(|e| match e {
            RowEvent::Click(d, _) => Some(*d),
            RowEvent::Check(..) => None,
        });

        if let Some(paging) = st.paging.as_mut() {
            out.page_changed = self.footer(ui, rt, row_h, &t, paging);
            if out.page_changed {
                st.pending = None;
            }
        }
        out.sort = st.sort;
        kit::semantic::remember_rect(
            ctx,
            id,
            Rect::from_min_max(start, pos2(start.x + avail_w, ui.cursor().min.y)),
        );
        st.store(ctx, id);
        out
    }

    fn apply_selection(&mut self, pass: &Pass, page_ids: &[usize], order: &[usize], out: &mut Events) {
        let Some(sel) = self.selection.as_deref_mut() else {
            return;
        };
        if let Some(on) = pass.header_check {
            for d in page_ids.iter().copied() {
                out.changed |= if on { sel.select(d) } else { sel.deselect(&d) };
            }
        }
        for e in &pass.events {
            let (RowEvent::Check(d, m) | RowEvent::Click(d, m)) = *e;
            out.changed |= toggle_row(sel, d, m, order);
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
struct OrderKey {
    sort: SortState,
    rows: usize,
    data_version: u64,
    // I18n::revision: collation follows the locale
    i18n: u64,
}

#[derive(Clone, Copy)]
struct RowAt {
    grid: usize,
    data: usize,
}

#[derive(Default)]
struct Pass {
    focused: Option<(Cell, Id)>,
    events: Vec<RowEvent>,
    header_check: Option<bool>,
    sort_click: Option<usize>,
    // Cell responses of the row being painted, reused across rows.
    cells: Vec<Response>,
}

struct TableView<'v, 'a> {
    table: &'v DataTable<'a>,
    rt: &'v crate::Rt,
    fmt: Arc<dyn LocaleFormat>,
    ctx: &'v Context,
    t: &'v Tokens,
    geo: Geometry,
    id: Id,
    stop: Cell,
    row_h: f32,
    body_h: f32,
    order: &'v [usize],
    page: std::ops::Range<usize>,
    cell_shape: egui::CornerRadius,
}

impl TableView<'_, '_> {
    fn has_sel(&self) -> bool {
        self.table.selection.is_some()
    }

    fn is_selected(&self, d: usize) -> bool {
        self.table.selection.as_deref().is_some_and(|s| s.is_selected(&d))
    }

    fn sense_at(&self, pos: Cell) -> Sense {
        if self.table.disabled {
            Sense::hover()
        } else {
            focus::roving_sense(Sense::click(), focus::TabStop::at(pos, self.stop))
        }
    }

    fn grid<'d>(
        &self,
        ui: &mut MineralUi<'_, Material>,
        st: &mut TableState,
        pass: &mut Pass,
        cell: &dyn Fn(usize, usize) -> TableCell<'d>,
    ) {
        ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
        let (hrect, _) = ui.allocate_exact_size(vec2(self.geo.table_w, self.row_h), Sense::hover());
        self.header(ui, st, pass, hrect);
        let (brect, _) = ui.allocate_exact_size(vec2(self.geo.table_w, self.body_h), Sense::hover());
        self.body(ui, st, pass, brect, cell);
    }

    fn header(&self, ui: &mut MineralUi<'_, Material>, st: &mut TableState, pass: &mut Pass, hrect: Rect) {
        let head = RowColors::of(
            RowState {
                disabled: self.table.disabled,
                ..RowState::default()
            },
            self.t,
        );
        ui.painter().rect_filled(hrect, self.cell_shape, head.container);
        row_parts::bottom_divider(ui.painter(), hrect, self.t);
        let multiple = self
            .table
            .selection
            .as_deref()
            .is_some_and(|s| s.mode() == egui_mineral::select::SelectionMode::Multiple);
        if multiple {
            self.select_all(ui, st, pass, hrect);
        } else if self.has_sel() {
            self.selection_header(ui, st, pass, hrect);
        }
        for c in 0..self.table.columns.len() {
            self.header_cell(ui, st, pass, hrect, c);
        }
    }

    fn select_all(&self, ui: &mut MineralUi<'_, Material>, st: &mut TableState, pass: &mut Pass, hrect: Rect) {
        let rt = ui.rt();
        let disabled = self.table.disabled;
        let all_on_page = !self.page.is_empty() && self.page_ids().all(|d| self.is_selected(d));
        let any_on_page = self.page_ids().any(|d| self.is_selected(d));
        let r = kit::input::interact(
            &rt.ui(ui),
            self.geo.check(hrect),
            self.id.with("all"),
            self.sense_at(Cell { row: 0, column: 0 }),
            Availability::Enabled,
        );
        kit::input::hover_cursor(self.rt.core(), &r, Availability::disabled_if(disabled));
        if r.clicked() && !disabled {
            pass.header_check = Some(!all_on_page);
        }
        take_cell(self.ctx, st, Cell { row: 0, column: 0 }, &r, &mut pass.focused);
        let state = match (all_on_page, any_on_page) {
            (true, _) => Check::Checked,
            (false, true) => Check::Mixed,
            (false, false) => Check::Unchecked,
        };
        row_parts::paint_checkbox(ui, &r, self.geo.check_center(hrect), state, disabled, self.t);
        check_semantics(
            self.rt,
            &r,
            StableId::of(&self.table.salt, "all-checkbox"),
            "mineral-table-select-all",
            state,
            disabled,
        );
    }

    // So the grid has no hole at (0, 0)
    fn selection_header(&self, ui: &MineralUi<'_, Material>, st: &mut TableState, pass: &mut Pass, hrect: Rect) {
        let pos = Cell { row: 0, column: 0 };
        let r = kit::input::interact(
            ui,
            self.geo.check(hrect),
            self.id.with("selection"),
            self.sense_at(pos),
            Availability::Enabled,
        );
        take_cell(self.ctx, st, pos, &r, &mut pass.focused);
        semantic::emit(self.ctx, Some(r.id), r.rect, || {
            let mut n = SemanticNode::new(
                StableId::of(&self.table.salt, "selection-header"),
                Role::ColumnHeader,
                crate::tr(self.rt.i18n(), "mineral-table-selection"),
            );
            n.depth = 1;
            n.actions.push(Action::Focus);
            n
        });
    }

    fn page_ids(&self) -> impl Iterator<Item = usize> {
        self.order.get(self.page.clone()).into_iter().flatten().copied()
    }

    fn header_cell(
        &self,
        ui: &mut MineralUi<'_, Material>,
        st: &mut TableState,
        pass: &mut Pass,
        hrect: Rect,
        c: usize,
    ) {
        let rt = ui.rt();
        let ctx = self.ctx;
        let disabled = self.table.disabled;
        let col = &self.table.columns[c];
        let rect = self.geo.cell(hrect, c);
        let gc = c + usize::from(self.has_sel());
        let r = kit::input::interact(
            &rt.ui(ui),
            rect,
            self.id.with(("h", c)),
            self.sense_at(Cell { row: 0, column: gc }),
            Availability::Enabled,
        );
        take_cell(ctx, st, Cell { row: 0, column: gc }, &r, &mut pass.focused);
        let sort = st.sort.order(c);
        if col.sortable {
            kit::input::hover_cursor(rt.core(), &r, Availability::disabled_if(disabled));
            if r.clicked() && !disabled {
                pass.sort_click = Some(c);
            }
        }
        paint_header(ui, self.t, rect, col, sort, &r, disabled);
        semantic::emit(ctx, Some(r.id), rect, || {
            let mut n = SemanticNode::new(
                StableId::of(&self.table.salt, (c, "header")),
                Role::ColumnHeader,
                &col.title,
            );
            n.value = sort.map(|d| {
                crate::tr(
                    rt.i18n(),
                    match d {
                        SortOrder::Ascending => "mineral-table-sort-ascending",
                        SortOrder::Descending => "mineral-table-sort-descending",
                    },
                )
            });
            n.depth = 1;
            n.state.disabled = disabled;
            if !disabled {
                n.actions.push(Action::Focus);
                if col.sortable {
                    n.actions.push(Action::Click);
                }
            }
            n
        });
        if col.resizable && !disabled {
            resize_handle(
                &rt.ui(ui),
                self.t,
                self.id,
                &self.table.salt,
                c,
                col,
                &self.geo,
                hrect,
                &mut st.widths,
            );
        }
    }

    fn body<'d>(
        &self,
        ui: &mut MineralUi<'_, Material>,
        st: &mut TableState,
        pass: &mut Pass,
        brect: Rect,
        cell: &dyn Fn(usize, usize) -> TableCell<'d>,
    ) {
        let rt = ui.rt();
        let page_rows = self.page.len();
        if page_rows == 0 {
            let g = parts::type_wrapped(
                ui.painter(),
                &crate::tr(self.rt.i18n(), "mineral-table-empty"),
                list::ITEM_SUPPORTING_TEXT_FONT,
                self.t.role(list::ITEM_SUPPORTING_TEXT_COLOR),
                brect.width(),
                1,
            );
            ui.painter()
                .galley(brect.center() - g.size() / 2.0, g, Color32::PLACEHOLDER);
            return;
        }
        let row_h = self.row_h;
        let pending_row = st.pending.map(|p| p.row).filter(|r| *r > 0);
        let mut first = true;
        let scope = self.table.salt.child("body");
        egui_mineral::layout::child(ui, scope, brect, rt.i18n().direction().column(), |ui| {
            ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
            VirtualList::uniform(self.table.salt.child("rows"), row_h).show(ui, page_rows, |ui, i| {
                if std::mem::take(&mut first)
                    && let Some(pr) = pending_row
                {
                    let top = ui.cursor().min.y + ((pr - 1) as f32 - i as f32) * row_h;
                    ui.scroll_to_rect(
                        Rect::from_min_size(pos2(ui.cursor().min.x, top), vec2(1.0, row_h)),
                        None,
                    );
                }
                self.row(&mut rt.ui(ui), st, pass, i, cell);
            });
        });
    }

    fn row<'d>(
        &self,
        ui: &mut MineralUi<'_, Material>,
        st: &mut TableState,
        pass: &mut Pass,
        i: usize,
        cell: &dyn Fn(usize, usize) -> TableCell<'d>,
    ) {
        let Some(&d) = self.order.get(self.page.start + i) else {
            return;
        };
        let disabled = self.table.disabled;
        let (rect, _) = ui.allocate_exact_size(vec2(self.geo.table_w, self.row_h), Sense::hover());
        let selected = self.is_selected(d);
        let mut cells = std::mem::take(&mut pass.cells);
        let check = self.row_inputs(ui, st, pass, &mut cells, rect, RowAt { grid: i + 1, data: d });
        let col = RowColors::of(
            RowState {
                selected,
                disabled,
                dragged: false,
            },
            self.t,
        );
        self.paint_row(ui, rect, &col, &cells, d, selected);
        if let Some(r) = &check {
            let state = if selected { Check::Checked } else { Check::Unchecked };
            row_parts::paint_checkbox(ui, r, self.geo.check_center(rect), state, disabled, self.t);
            check_semantics(
                self.rt,
                r,
                StableId::of(&self.table.salt, (d, "checkbox")),
                "mineral-table-select-row",
                state,
                disabled,
            );
        }
        for (c, r) in cells.iter().enumerate() {
            self.paint_cell(ui, &col, r, Cell { row: d, column: c }, cell);
        }
        pass.cells = cells;
    }

    fn row_inputs(
        &self,
        ui: &mut MineralUi<'_, Material>,
        st: &mut TableState,
        pass: &mut Pass,
        cells: &mut Vec<Response>,
        rect: Rect,
        at: RowAt,
    ) -> Option<Response> {
        let RowAt {
            grid: grid_row,
            data: d,
        } = at;
        let rt = ui.rt();
        let ctx = self.ctx;
        let disabled = self.table.disabled;
        let has_sel = self.has_sel();
        let check = has_sel.then(|| {
            let r = kit::input::interact(
                &rt.ui(ui),
                self.geo.check(rect),
                self.id.with(("check", d)),
                self.sense_at(Cell {
                    row: grid_row,
                    column: 0,
                }),
                Availability::Enabled,
            );
            kit::input::hover_cursor(rt.core(), &r, Availability::disabled_if(disabled));
            if r.clicked() && !disabled {
                pass.events.push(RowEvent::Check(d, ui.input(|i| i.modifiers)));
            }
            take_cell(
                ctx,
                st,
                Cell {
                    row: grid_row,
                    column: 0,
                },
                &r,
                &mut pass.focused,
            );
            r
        });
        cells.clear();
        for c in 0..self.table.columns.len() {
            {
                let gc = c + usize::from(has_sel);
                let r = kit::input::interact(
                    &rt.ui(ui),
                    self.geo.cell(rect, c),
                    self.id.with(("c", d, c)),
                    self.sense_at(Cell {
                        row: grid_row,
                        column: gc,
                    }),
                    Availability::Enabled,
                );
                take_cell(
                    ctx,
                    st,
                    Cell {
                        row: grid_row,
                        column: gc,
                    },
                    &r,
                    &mut pass.focused,
                );
                if r.clicked() && !disabled {
                    pass.events.push(RowEvent::Click(d, ui.input(|i| i.modifiers)));
                }
                cells.push(r);
            }
        }
        check
    }

    fn paint_row(
        &self,
        ui: &mut egui_mineral::widget::MineralUi<'_, crate::Material>,
        rect: Rect,
        col: &RowColors,
        cells: &[Response],
        d: usize,
        selected: bool,
    ) {
        let rt = ui.rt();
        ui.painter().rect_filled(rect, self.cell_shape, col.container);
        if !self.table.disabled {
            // One state layer for the row; each cell keeps its own ripple across the row.
            let hovered = ui.rect_contains_pointer(rect);
            for (k, r) in cells.iter().enumerate() {
                let i = Interaction {
                    hovered: k == 0 && hovered,
                    ..Interaction::default()
                };
                indication::paint(&rt.ui(ui), r, rect, self.cell_shape, col.label, i, self.t);
            }
        }
        row_parts::bottom_divider(ui.painter(), rect, self.t);
        semantic::emit(self.ctx, None, rect, || {
            let mut n = SemanticNode::new(StableId::of(&self.table.salt, (d, "row")), Role::Row, "");
            n.state.selected = self.has_sel().then_some(selected);
            n.state.disabled = self.table.disabled;
            n.depth = 1;
            n
        });
    }

    fn paint_cell<'d>(
        &self,
        ui: &mut Ui,
        col: &RowColors,
        r: &Response,
        at: Cell,
        cell: &dyn Fn(usize, usize) -> TableCell<'d>,
    ) {
        let Cell { row: d, column: c } = at;
        let disabled = self.table.disabled;
        let cr = r.rect;
        let v = cell(d, c);
        let text = self.table.cell_text(self.fmt.as_ref(), c, &v);
        let numeric = self.table.columns[c].numeric || matches!(v, TableCell::Number(_));
        let p = ui.painter();
        let g = parts::type_wrapped(p, &text, CELL_FONT, col.label, cr.width() - CELL_START - CELL_END, 1);
        // Numbers align to the reading end.
        let (side, pad) = if numeric {
            (self.geo.dir.reversed(), CELL_END)
        } else {
            (self.geo.dir, CELL_START)
        };
        let x = side.start_x(cr, Rangef::new(pad, pad + g.size().x));
        p.galley(pos2(x, cr.center().y - g.size().y / 2.0), g, Color32::PLACEHOLDER);
        let focus = Interaction {
            focused: focus::visible(&r.ctx, r.id),
            ..Interaction::default()
        };
        common::state_layer(p, cr, self.cell_shape, col.label, focus, self.t);
        semantic::emit(self.ctx, Some(r.id), cr, || {
            let mut n = SemanticNode::new(
                StableId::of(&self.table.salt, (d, c, "cell")),
                Role::Cell,
                text.as_ref(),
            )
            .clickable(Availability::disabled_if(disabled));
            n.depth = 2;
            n
        });
    }
}

fn grid_focus_target(ctx: &Context, from: Cell, grid: focus::Grid, dir: Direction) -> Option<Cell> {
    let mut target = from;
    for (k, m) in kit::input::key_presses(ctx) {
        if let Some(next) = focus::on_grid_key(grid, target, k, m, dir) {
            target = next;
        }
    }
    (target != from).then_some(target)
}

fn toggle_row(sel: &mut Selection<usize>, d: usize, m: Modifiers, order: &[usize]) -> bool {
    if m.shift {
        let extend = Modifiers {
            shift: true,
            command: true,
            ..Modifiers::NONE
        };
        sel.click(d, extend, order)
    } else {
        sel.toggle(d)
    }
}

fn take_cell(ctx: &Context, st: &mut TableState, pos: Cell, r: &egui::Response, focused: &mut Option<(Cell, Id)>) {
    kit::focus::follow_press(r);
    if st.pending == Some(pos) {
        ctx.memory_mut(|m| m.request_focus(r.id));
        st.pending = None;
    }
    if r.has_focus() {
        *focused = Some((pos, r.id));
        st.stop = Some(pos);
    }
}

fn paint_header(
    ui: &MineralUi<'_, Material>,
    t: &Tokens,
    rect: Rect,
    col: &TableColumn,
    sort: Option<SortOrder>,
    r: &Response,
    disabled: bool,
) {
    let rt = ui.rt();
    let dir = rt.i18n().direction();
    let colors = RowColors::of(
        RowState {
            disabled,
            ..RowState::default()
        },
        t,
    );
    let shape = list::ITEM_CONTAINER_SHAPE.corner_radius(&t.shape, rt.i18n().direction());
    if col.sortable && !disabled {
        indication::paint(
            ui,
            r,
            rect,
            shape,
            colors.label,
            Interaction::of(r, Availability::Enabled),
            t,
        );
    } else {
        let focus = Interaction {
            focused: focus::visible(&r.ctx, r.id),
            ..Interaction::default()
        };
        common::state_layer(ui.painter(), rect, shape, colors.label, focus, t);
    }
    let p = ui.painter();
    let arrow = match sort {
        Some(SortOrder::Ascending) => Some((ui.icons().arrow_upward, colors.label)),
        Some(SortOrder::Descending) => Some((ui.icons().arrow_downward, colors.label)),
        None if col.sortable && r.hovered() => Some((ui.icons().arrow_upward, colors.trailing)),
        None => None,
    };
    let arrow_w = if col.sortable { SORT_ICON + SORT_ICON_GAP } else { 0.0 };
    let g = parts::type_wrapped(
        p,
        &col.title,
        HEADER_FONT,
        colors.label,
        rect.width() - CELL_START - CELL_END - arrow_w,
        1,
    );
    let y = rect.center().y;
    // Numeric titles sit at the reading end with the arrow before them (MD2); others at the start, arrow after.
    let (side, pad) = if col.numeric {
        (dir.reversed(), CELL_END)
    } else {
        (dir, CELL_START)
    };
    let text_x = side.start_x(rect, Rangef::new(pad, pad + g.size().x));
    let arrow_x = side.start_x(rect, Rangef::point(pad + g.size().x + SORT_ICON_GAP + SORT_ICON / 2.0));
    p.galley(pos2(text_x, y - g.size().y / 2.0), g, Color32::PLACEHOLDER);
    if let Some((icon, color)) = arrow {
        common::icon(p, pos2(arrow_x, y), icon, SORT_ICON, color);
    }
}

// Text field active indicator styles
fn resize_stroke(r: &Response, t: &Tokens) -> Stroke {
    use filled_text_field as m;
    let (width, color) = if r.dragged() || focus::visible(&r.ctx, r.id) {
        (m::FOCUS_ACTIVE_INDICATOR_HEIGHT, m::FOCUS_ACTIVE_INDICATOR_COLOR)
    } else if r.hovered() {
        (m::HOVER_ACTIVE_INDICATOR_HEIGHT, m::HOVER_ACTIVE_INDICATOR_COLOR)
    } else {
        (divider::THICKNESS, divider::COLOR)
    };
    Stroke::new(width, t.role(color))
}

#[expect(clippy::too_many_arguments, reason = "geometry and state of one header cell")]
fn resize_handle(
    ui: &MineralUi<'_, Material>,
    t: &Tokens,
    id: Id,
    salt: &KeyedSalt,
    c: usize,
    col: &TableColumn,
    geo: &Geometry,
    header: Rect,
    widths: &mut ColumnWidths,
) {
    let rt = ui.rt();
    let ctx = ui.ctx();
    let cell = geo.cell(header, c);
    let edge = geo.dir.end_edge(cell);
    // The core splitter's hit width for the same gesture.
    let hit = MinTarget::Auto.points(rt.core());
    let rect = Rect::from_center_size(pos2(edge, header.center().y), vec2(hit, header.height()));
    let r = ui.interact(rect, id.with(("resize", c)), Sense::drag());
    let model = RangeModel::new(f64::from(col.min_width), MAX_COLUMN_WIDTH).step(f64::from(crate::adaptive::grid(2)));
    let w = widths.width(c);
    let track = Track::horizontal(rt.i18n().direction());
    let mut next = w;
    if r.dragged() {
        next = w + track.sign() * r.drag_delta().x;
    }
    next = model.drive(rt.core(), r.id, f64::from(next), track) as f32;
    if (next - w).abs() > f32::EPSILON {
        widths.set(c, next);
    }
    if r.hovered() || r.dragged() {
        ctx.set_cursor_icon(CursorIcon::ResizeColumn);
    }
    // Spans the header's content box.
    let line = rect.shrink2(vec2(0.0, defaults::list::VERTICAL_PADDING));
    ui.painter().vline(edge, line.y_range(), resize_stroke(&r, t));
    semantic::emit(ctx, Some(r.id), rect, || {
        let name = rt.i18n().tr("mineral-table-resize", &[("column", (&col.title).into())]);
        let mut n = SemanticNode::new(StableId::of(salt, (c, "resize")), Role::Splitter, name)
            .value(rt.i18n().format().integer(widths.width(c).round() as i64))
            .ranged(model.numeric(f64::from(widths.width(c))), Availability::Enabled);
        n.depth = 1;
        n
    });
}

fn check_semantics(rt: &Rt, r: &Response, id: StableId, name_id: &str, state: Check, disabled: bool) {
    let ctx = rt.ctx();
    semantic::emit(ctx, Some(r.id), r.rect, || {
        let name = crate::tr(rt.i18n(), name_id);
        let mut n = SemanticNode::new(id, Role::CheckBox, name).clickable(Availability::disabled_if(disabled));
        n.state.checked = Some(state);
        n.depth = 2;
        n
    });
}
