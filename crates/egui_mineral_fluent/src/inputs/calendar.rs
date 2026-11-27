use crate::msg;
use egui::{Color32, CornerRadius, Id, Pos2, Rangef, Rect, Response, Sense, Stroke, StrokeKind, vec2};
use egui_mineral::{
    date::{self, Date, DateBounds, MonthGrid, Weekday, add_months, weekdays_from},
    focus,
    i18n::{DateFields, DateLength, Direction, I18n, LocaleFormat, NameWidth},
    kit,
    kit::paint::FocusStroke,
    layout::{Constraints, Measure, MinTarget},
    semantic::{self, AgentKey, KeyedSalt, Role, SemanticNode, StableId},
    widget::{Availability, Interaction, MineralUi, MineralWidget, WidgetFrame, WidgetResponse},
};

use crate::spec::ColorToken::{
    ColorBrandBackground, ColorBrandBackgroundInvertedHover, ColorBrandBackgroundInvertedPressed,
    ColorBrandBackgroundInvertedSelected, ColorBrandForeground1, ColorBrandForeground2,
    ColorBrandForegroundOnLightHover, ColorBrandForegroundOnLightPressed, ColorBrandStroke1, ColorNeutralForeground1,
    ColorNeutralForeground1Static, ColorNeutralForeground4, ColorNeutralForegroundDisabled,
    ColorNeutralForegroundOnBrand, ColorNeutralStroke2, ColorStrokeFocus2,
};
use crate::{
    Fluent, FluentExt,
    spec::{
        defaults::calendar as cal,
        tokens::{geometry as g, typography as ty},
    },
    theme::Tokens,
    widget::paint,
};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum Mode {
    #[default]
    Days,
    Months,
    Years,
}

#[derive(Clone, Debug, Default)]
pub struct View {
    mode: Mode,
    year: i16,
    month: i8,
    years_from: i16,
    focus: Option<Date>,
    shown: Option<Date>,
    frame: u64,
    started: bool,
}

/// Fluent `Calendar` (`react-calendar-compat`): a month of days with month and year pickers.
#[derive(Debug)]
#[must_use = "show with ui.mineral(..)"]
pub struct Calendar<'a> {
    salt: KeyedSalt,
    key: AgentKey,
    label: String,
    value: &'a mut Option<Date>,
    bounds: DateBounds,
    first_weekday: Option<Weekday>,
    today: Option<Date>,
    week_numbers: bool,
    go_to_today: bool,
    autofocus: bool,
    disabled: bool,
}

impl<'a> Calendar<'a> {
    /// `label` is the accessible name; `value` the selected day. Agent key `<id_salt>-calendar`.
    pub fn new(id_salt: impl Into<KeyedSalt>, label: impl Into<String>, value: &'a mut Option<Date>) -> Self {
        Self {
            salt: id_salt.into(),
            key: egui_mineral::agent_key!("calendar"),
            label: label.into(),
            value,
            bounds: DateBounds::default(),
            first_weekday: None,
            today: None,
            week_numbers: false,
            go_to_today: true,
            autofocus: false,
            disabled: false,
        }
    }

    /// `minDate` and `maxDate`: days outside are disabled.
    pub fn bounds(mut self, bounds: DateBounds) -> Self {
        self.bounds = bounds;
        self
    }

    /// `firstDayOfWeek`, default the locale's.
    pub fn first_weekday(mut self, first_weekday: Weekday) -> Self {
        self.first_weekday = Some(first_weekday);
        self
    }

    /// Overrides the system date as the highlighted day.
    pub fn today(mut self, today: Date) -> Self {
        self.today = Some(today);
        self
    }

    /// `showWeekNumbers`: ISO week numbers before each week.
    pub fn week_numbers(mut self, show: bool) -> Self {
        self.week_numbers = show;
        self
    }

    /// `showGoToToday`, default on.
    pub fn go_to_today(mut self, show: bool) -> Self {
        self.go_to_today = show;
        self
    }

    /// Moves focus into the day grid the first time the calendar is shown, as `DatePicker` does on opening.
    pub fn autofocus(mut self, autofocus: bool) -> Self {
        self.autofocus = autofocus;
        self
    }

    crate::widget::setters! {
        /// Greys out the calendar; nothing can be picked.
        disabled
    }

    crate::widget::setters! {
        /// Replaces the `<id_salt>-calendar` agent key.
        agent_key(key)
    }

    fn width(&self) -> f32 {
        if self.week_numbers {
            cal::WIDTH_WEEK_NUMBERS
        } else {
            cal::WIDTH
        }
    }

    fn height(&self) -> f32 {
        let go_today = if self.go_to_today {
            cal::GO_TODAY_MARGIN_TOP + cal::GO_TODAY_HEIGHT
        } else {
            0.0
        };
        cal::PADDING
            + cal::HEADER_HEIGHT
            + cal::GRID_MARGIN_TOP
            + (cal::WEEKS + 1) as f32 * cal::DAY_CELL
            + cal::GRID_PAD_BOTTOM
            + go_today
            + cal::PADDING
    }

    fn week_start(&self, fmt: &dyn LocaleFormat) -> Weekday {
        self.first_weekday.unwrap_or_else(|| fmt.first_weekday())
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Look {
    Rest,
    Selected,
    Current,
    Muted,
}

struct Cx<'a> {
    id: Id,
    dir: Direction,
    t: &'a Tokens,
    disabled: bool,
}

// Fixed-size cell at `(x, y)` from the start edge of `root`.
fn cell(cx: &Cx<'_>, root: Rect, x: f32, y: f32, size: Pos2) -> Rect {
    let left = cx.dir.start_x(root, Rangef::new(x, x + size.x));
    Rect::from_min_size(Pos2::new(left, root.top() + y), vec2(size.x, size.y))
}

fn text_color(look: Look, hot: Interaction, disabled: bool) -> crate::spec::ColorToken {
    if disabled {
        ColorNeutralForegroundDisabled
    } else if hot.pressed && look == Look::Rest {
        ColorBrandForegroundOnLightPressed
    } else if hot.hovered && look == Look::Rest {
        ColorBrandForegroundOnLightHover
    } else {
        match look {
            Look::Rest => ColorNeutralForeground1,
            Look::Selected => ColorNeutralForeground1Static,
            Look::Current => ColorNeutralForegroundOnBrand,
            Look::Muted => ColorNeutralForeground4,
        }
    }
}

// Hover and press fill of `CalendarDayGrid`, `CalendarPicker` items and the header buttons.
fn fill(hot: Interaction, look: Look, disabled: bool) -> Option<crate::spec::ColorToken> {
    if disabled {
        return None;
    }
    match look {
        Look::Current => Some(ColorBrandBackground),
        Look::Selected => Some(if hot.pressed {
            ColorBrandBackgroundInvertedPressed
        } else {
            ColorBrandBackgroundInvertedSelected
        }),
        _ if hot.pressed => Some(ColorBrandBackgroundInvertedPressed),
        _ if hot.hovered => Some(ColorBrandBackgroundInvertedHover),
        _ => None,
    }
}

struct Item<'a> {
    rect: Rect,
    id: Id,
    text: &'a str,
    size: f32,
    weight: u16,
    look: Look,
    sense: Sense,
    disabled: bool,
}

// Button with centered text; the shared look of day, month and year cells.
fn item(ui: &mut MineralUi<'_, Fluent>, cx: &Cx<'_>, i: &Item<'_>) -> Response {
    let availability = Availability::enabled_if(!i.disabled && !cx.disabled);
    let r = kit::interact::interact(ui, i.rect, i.id, i.sense, availability);
    kit::interact::follow_press(&r);
    let hot = Interaction::of(&r, availability);
    let off = i.disabled || cx.disabled;
    let radius = CornerRadius::from(g::BORDER_RADIUS_MEDIUM);
    let p = ui.painter().clone();
    if let Some(f) = fill(hot, i.look, off) {
        p.rect_filled(i.rect, radius, cx.t.color(f));
    }
    if i.look == Look::Selected && !off {
        p.rect_stroke(
            i.rect,
            radius,
            Stroke::new(g::STROKE_WIDTH_THIN, cx.t.color(ColorBrandStroke1)),
            StrokeKind::Inside,
        );
    }
    let color = cx.t.color(text_color(i.look, hot, off));
    let format = crate::typography::format(
        ui.ctx(),
        crate::typography::TypeSpec {
            size: i.size,
            line_height: i.size + 4.0,
            weight: i.weight,
        },
        color,
    );
    let galley = p.layout_no_wrap(i.text.to_owned(), format.font_id, color);
    p.galley(i.rect.center() - galley.size() / 2.0, galley, color);
    if hot.focused && !off {
        p.add(
            FocusStroke::inside(Stroke::new(cal::FOCUS_RING, cx.t.color(ColorStrokeFocus2)))
                .shape(i.rect.expand(cal::FOCUS_RING / 2.0), radius),
        );
    }
    r
}

fn emit_button(ui: &MineralUi<'_, Fluent>, r: &Response, key: StableId, name: &str, selected: Option<bool>, off: bool) {
    semantic::emit(ui.ctx(), Some(r.id), r.rect, || {
        let mut n = SemanticNode::new(key, Role::Button, name).clickable(Availability::disabled_if(off));
        n.state.selected = selected;
        n
    });
}

struct Run<'a> {
    key: StableId,
    value: Option<Date>,
    bounds: DateBounds,
    week_numbers: bool,
    go_to_today: bool,
    autofocus: bool,
    cx: Cx<'a>,
    fmt: &'a dyn LocaleFormat,
    i18n: &'a I18n,
    root: Rect,
    today: Date,
    week_start: Weekday,
}

impl Run<'_> {
    fn key(&self, part: impl semantic::KeySalt) -> StableId {
        self.key.sub(part)
    }

    // Month-year or year button and the two navigation buttons; returns the navigation direction.
    fn header(
        &self,
        ui: &mut MineralUi<'_, Fluent>,
        title: &str,
        clickable: bool,
        nav: [(bool, &str); 2],
    ) -> (bool, i32) {
        let cx = &self.cx;
        let nav_w = cal::NAV_BUTTON;
        let title_w = self.root.width() - 2.0 * cal::PADDING - 2.0 * nav_w;
        let tr = cell(
            cx,
            self.root,
            cal::PADDING,
            cal::PADDING,
            Pos2::new(title_w, cal::HEADER_HEIGHT),
        );
        let tid = cx.id.with("title");
        let availability = Availability::enabled_if(clickable && !cx.disabled);
        let r = kit::interact::interact(ui, tr, tid, Sense::click(), availability);
        kit::interact::follow_press(&r);
        let hot = Interaction::of(&r, availability);
        let radius = CornerRadius::from(g::BORDER_RADIUS_MEDIUM);
        let p = ui.painter().clone();
        let look = Look::Rest;
        if clickable && let Some(f) = fill(hot, look, cx.disabled) {
            p.rect_filled(tr, radius, cx.t.color(f));
        }
        let color = cx.t.color(if clickable {
            text_color(look, hot, cx.disabled)
        } else {
            ColorNeutralForeground1
        });
        let font = crate::typography::font(ui.ctx(), ty::FONT_SIZE_BASE_300, ty::FONT_WEIGHT_SEMIBOLD);
        let galley = p.layout_no_wrap(title.to_owned(), font, color);
        let x = cx.dir.start_x(
            tr,
            Rangef::new(cal::HEADER_TEXT_PAD_START, cal::HEADER_TEXT_PAD_START + galley.size().x),
        );
        p.galley(Pos2::new(x, tr.center().y - galley.size().y / 2.0), galley, color);
        if hot.focused {
            p.add(FocusStroke::inside(Stroke::new(cal::FOCUS_RING, cx.t.color(ColorStrokeFocus2))).shape(tr, radius));
        }
        semantic::emit(ui.ctx(), Some(r.id), tr, || {
            SemanticNode::new(self.key("title"), Role::Button, title).clickable(availability)
        });
        let mut step = 0;
        for (k, ((off, name), icon)) in nav
            .into_iter()
            .zip([ui.icons().chevron_up, ui.icons().chevron_down])
            .enumerate()
        {
            let x = cal::PADDING + title_w + k as f32 * nav_w;
            let rect = cell(cx, self.root, x, cal::PADDING, Pos2::new(nav_w, cal::HEADER_HEIGHT));
            let id = cx.id.with(("nav", k));
            let av = Availability::enabled_if(!off && !cx.disabled);
            let r = kit::interact::interact(ui, rect, id, Sense::click(), av);
            kit::interact::follow_press(&r);
            let hot = Interaction::of(&r, av);
            let off = off || cx.disabled;
            if let Some(f) = fill(hot, Look::Rest, off) {
                p.rect_filled(rect, radius, cx.t.color(f));
            }
            let color = if off {
                ColorNeutralForegroundDisabled
            } else if hot.pressed {
                ColorBrandForegroundOnLightPressed
            } else if hot.hovered {
                ColorBrandForegroundOnLightHover
            } else {
                ColorNeutralForeground1
            };
            paint::icon(
                &p,
                rect.center(),
                icon,
                false,
                ty::FONT_SIZE_BASE_300,
                cx.t.color(color),
            );
            if hot.focused {
                p.add(
                    FocusStroke::inside(Stroke::new(cal::FOCUS_RING, cx.t.color(ColorStrokeFocus2)))
                        .shape(rect, radius),
                );
            }
            emit_button(ui, &r, self.key(("nav", k)), name, None, off);
            if r.clicked() {
                step = if k == 0 { -1 } else { 1 };
            }
        }
        (r.clicked() && clickable, step)
    }
}

struct Hit {
    id: Id,
    index: Option<usize>,
    focused: bool,
    clicked: bool,
}

const GRID_TOP: f32 = cal::PADDING + cal::HEADER_HEIGHT + cal::GRID_MARGIN_TOP;

fn bounds_allow_month(b: DateBounds, year: i16, month: i8) -> bool {
    let Ok(first) = Date::new(year, month, 1) else {
        return false;
    };
    b.min().is_none_or(|m| first.last_of_month() >= m) && b.max().is_none_or(|m| first <= m)
}

fn year_range(from: i16) -> std::ops::RangeInclusive<i16> {
    from..=from.saturating_add(cal::YEARS - 1)
}

impl Run<'_> {
    fn label(&self, ui: &MineralUi<'_, Fluent>, rect: Rect, text: String, color: crate::spec::ColorToken) {
        let color = self.cx.t.color(color);
        let format = crate::typography::format(
            ui.ctx(),
            crate::typography::TypeSpec {
                size: ty::FONT_SIZE_BASE_200,
                line_height: ty::LINE_HEIGHT_BASE_200,
                weight: ty::FONT_WEIGHT_REGULAR,
            },
            color,
        );
        let galley = ui.painter().layout_no_wrap(text, format.font_id, color);
        ui.painter().galley(rect.center() - galley.size() / 2.0, galley, color);
    }

    fn day_header(&self, ui: &mut MineralUi<'_, Fluent>, st: &mut View, first: Date) {
        let (prev, next) = (add_months(first, -1), add_months(first, 1));
        let names = [
            self.i18n.tr(msg::CALENDAR_PREVIOUS_MONTH),
            self.i18n.tr(msg::CALENDAR_NEXT_MONTH),
        ];
        let nav = [
            (
                !bounds_allow_month(self.bounds, prev.year(), prev.month()),
                names[0].as_str(),
            ),
            (
                !bounds_allow_month(self.bounds, next.year(), next.month()),
                names[1].as_str(),
            ),
        ];
        let title = self.fmt.month_year(st.year, st.month);
        let (open_months, step) = self.header(ui, &title, true, nav);
        if open_months {
            st.mode = Mode::Months;
        }
        if step != 0 {
            let to = add_months(first, step);
            (st.year, st.month) = (to.year(), to.month());
        }
    }

    fn left(&self) -> f32 {
        cal::PADDING + if self.week_numbers { cal::WEEK_NUMBER_WIDTH } else { 0.0 }
    }

    fn weekday_labels(&self, ui: &MineralUi<'_, Fluent>) {
        let size = Pos2::new(cal::DAY_CELL, cal::DAY_CELL);
        for (col, wd) in weekdays_from(self.week_start).into_iter().enumerate() {
            let rect = cell(
                &self.cx,
                self.root,
                self.left() + col as f32 * cal::DAY_CELL,
                GRID_TOP,
                size,
            );
            self.label(
                ui,
                rect,
                self.fmt.weekday_name(wd, NameWidth::Narrow),
                ColorNeutralForeground1,
            );
        }
    }

    fn week_number(&self, ui: &MineralUi<'_, Fluent>, y: f32, week: &[Date]) {
        let Some(mid) = week.get(3) else { return };
        let cx = &self.cx;
        let size = Pos2::new(cal::WEEK_NUMBER_WIDTH, cal::DAY_CELL);
        let rect = cell(cx, self.root, cal::PADDING, y, size);
        let number = self.fmt.native_digits(&mid.iso_week_date().week().to_string());
        self.label(ui, rect, number, ColorNeutralForeground4);
        let edge = cx.dir.start_x(rect, Rangef::new(rect.width() - 1.0, rect.width()));
        ui.painter().vline(
            edge,
            rect.y_range(),
            Stroke::new(g::STROKE_WIDTH_THIN, cx.t.color(ColorNeutralStroke2)),
        );
    }

    // One Tab stop per grid: the focused day, else the selection, else today, else the first day.
    fn tab_stop(&self, ctx: &egui::Context, st: &mut View, grid: &MonthGrid) -> usize {
        let b = self.bounds;
        let in_month: Vec<Date> = grid.days.iter().copied().filter(|d| grid.in_month(*d)).collect();
        let index_of = |d: Option<Date>| {
            d.filter(|d| grid.in_month(*d) && b.contains(*d))
                .and_then(|d| usize::try_from(d.day() - 1).ok())
        };
        let found = focus::tab_stop(
            ctx,
            self.cx.id,
            index_of(self.value).or(index_of(Some(self.today))),
            &focus::Items::new(in_month.len(), |i| in_month.get(i).is_some_and(|d| b.contains(*d))),
        );
        // The day about to take focus must be focusable this pass.
        let stop = st
            .focus
            .filter(|f| grid.in_month(*f))
            .map_or(found, |f| usize::try_from(f.day() - 1).unwrap_or(found));
        if !st.started {
            st.started = true;
            if self.autofocus
                && let Some(d) = in_month.get(stop)
            {
                st.focus = Some(*d);
            }
        }
        stop
    }

    fn day(&self, ui: &mut MineralUi<'_, Fluent>, grid: &MonthGrid, stop: usize, at: (usize, usize), d: Date) -> Hit {
        let cx = &self.cx;
        let (row, col) = at;
        let size = Pos2::new(cal::DAY_CELL, cal::DAY_CELL);
        let y = GRID_TOP + (row + 1) as f32 * cal::DAY_CELL;
        let outer = cell(cx, self.root, self.left() + col as f32 * cal::DAY_CELL, y, size);
        let inside = grid.in_month(d);
        let allowed = self.bounds.contains(d);
        let selected = self.value == Some(d);
        let today = d == self.today;
        let look = if selected {
            Look::Selected
        } else if today {
            Look::Current
        } else if !inside {
            Look::Muted
        } else {
            Look::Rest
        };
        let idx = usize::try_from(d.day() - 1).unwrap_or(0);
        let sense = if inside {
            focus::roving_sense(Sense::click(), focus::TabStop::at(idx, stop))
        } else {
            Sense::click()
        };
        let diameter = if today && !selected {
            cal::TODAY_MARKER
        } else {
            cal::DAY_BUTTON
        };
        let text = self.fmt.native_digits(&d.day().to_string());
        let r = item(
            ui,
            cx,
            &Item {
                rect: Rect::from_center_size(outer.center(), vec2(diameter, diameter)),
                id: cx.id.with(d),
                text: &text,
                size: ty::FONT_SIZE_BASE_200,
                weight: if today {
                    ty::FONT_WEIGHT_SEMIBOLD
                } else {
                    ty::FONT_WEIGHT_REGULAR
                },
                look,
                sense,
                disabled: !allowed,
            },
        );
        let off = !allowed || cx.disabled;
        semantic::emit(ui.ctx(), Some(r.id), outer, || {
            let mut n = SemanticNode::new(
                self.key((d.to_string(), "day")),
                Role::GridCell,
                self.fmt.date(d, DateFields::YearMonthDay, DateLength::Long),
            )
            .clickable(Availability::disabled_if(off));
            n.state.selected = Some(selected);
            if today {
                n.value = Some("today".to_owned());
            }
            n
        });
        Hit {
            id: r.id,
            index: inside.then_some(idx),
            focused: inside && r.has_focus(),
            clicked: r.clicked() && !off,
        }
    }

    fn days(&self, ui: &mut MineralUi<'_, Fluent>, st: &mut View) -> Option<Date> {
        let ctx = ui.ctx().clone();
        let first = Date::new(st.year, st.month, 1).ok()?;
        self.day_header(ui, st, first);
        let grid = MonthGrid::new(first, self.week_start);
        self.weekday_labels(ui);
        let stop = self.tab_stop(&ctx, st, &grid);
        let mut ids = Vec::new();
        let (mut focused, mut picked) = (None, None);
        for (row, week) in grid.weeks().enumerate() {
            if self.week_numbers {
                self.week_number(ui, GRID_TOP + (row + 1) as f32 * cal::DAY_CELL, week);
            }
            for (col, d) in week.iter().copied().enumerate() {
                let hit = self.day(ui, &grid, stop, (row, col), d);
                ids.extend(hit.index.map(|i| (i, hit.id)));
                focused = if hit.focused { Some(d) } else { focused };
                picked = if hit.clicked { Some(d) } else { picked };
            }
        }
        kit::interact::remember(&ctx, self.cx.id, ids);
        if let Some(d) = picked {
            st.shown = Some(d);
            if !grid.in_month(d) {
                (st.year, st.month) = (d.year(), d.month());
            }
        }
        self.arrow_keys(&ctx, st, &grid, focused);
        self.go_today(ui, st);
        picked
    }

    // <https://www.w3.org/WAI/ARIA/apg/patterns/dialog-modal/examples/datepicker-dialog/>
    fn arrow_keys(&self, ctx: &egui::Context, st: &mut View, grid: &MonthGrid, focused: Option<Date>) {
        let (cx, b) = (&self.cx, self.bounds);
        if let Some(cur) = focused {
            kit::interact::lock_arrows(ctx, cx.id.with(cur), kit::interact::Arrows::Both);
            let target = kit::interact::key_presses(ctx).into_iter().fold(cur, |t, (key, mods)| {
                date::on_key(t, key, mods, cx.dir, self.week_start).map_or(t, |n| b.clamp(n))
            });
            if target != cur {
                st.focus = Some(target);
                (st.year, st.month) = (target.year(), target.month());
            }
        }
        if let Some(f) = st.focus
            && grid.in_month(f)
        {
            if focused == Some(f) {
                st.focus = None;
            } else {
                ctx.memory_mut(|m| m.request_focus(cx.id.with(f)));
            }
        }
    }

    fn go_today(&self, ui: &mut MineralUi<'_, Fluent>, st: &mut View) {
        if !self.go_to_today {
            return;
        }
        let cx = &self.cx;
        let text = self.i18n.tr(msg::CALENDAR_GO_TO_TODAY);
        let p = ui.painter().clone();
        let format = crate::typography::format(
            ui.ctx(),
            crate::typography::TypeSpec {
                size: ty::FONT_SIZE_BASE_200,
                line_height: cal::GO_TODAY_HEIGHT,
                weight: ty::FONT_WEIGHT_REGULAR,
            },
            Color32::WHITE,
        );
        let w = p
            .layout_no_wrap(text.clone(), format.font_id.clone(), Color32::WHITE)
            .size()
            .x
            + 2.0 * cal::GO_TODAY_PAD;
        let y = cal::PADDING
            + cal::HEADER_HEIGHT
            + cal::GRID_MARGIN_TOP
            + (cal::WEEKS + 1) as f32 * cal::DAY_CELL
            + cal::GRID_PAD_BOTTOM
            + cal::GO_TODAY_MARGIN_TOP;
        let x = self.root.width() - cal::GO_TODAY_MARGIN_END - w;
        let rect = cell(cx, self.root, x, y, Pos2::new(w, cal::GO_TODAY_HEIGHT));
        let av = Availability::enabled_if(!cx.disabled);
        let r = kit::interact::interact(ui, rect, cx.id.with("today"), Sense::click(), av);
        kit::interact::follow_press(&r);
        let hot = Interaction::of(&r, av);
        let color = cx.t.color(if cx.disabled {
            ColorNeutralForegroundDisabled
        } else if hot.pressed {
            ColorBrandForeground2
        } else if hot.hovered {
            ColorBrandForeground1
        } else {
            ColorNeutralForeground1
        });
        let galley = p.layout_no_wrap(text.clone(), format.font_id, color);
        p.galley(rect.center() - galley.size() / 2.0, galley, color);
        if hot.focused {
            p.add(
                FocusStroke::inside(Stroke::new(g::STROKE_WIDTH_THIN, cx.t.color(ColorStrokeFocus2)))
                    .shape(rect, CornerRadius::from(g::BORDER_RADIUS_SMALL)),
            );
        }
        emit_button(ui, &r, self.key("go-to-today"), &text, None, cx.disabled);
        if r.clicked() {
            (st.year, st.month) = (self.today.year(), self.today.month());
            st.mode = Mode::Days;
            st.focus = Some(self.today);
        }
    }

    fn picker_header(&self, ui: &mut MineralUi<'_, Fluent>, st: &mut View) {
        let fmt = self.fmt;
        let months = st.mode == Mode::Months;
        let (title, names) = if months {
            (
                fmt.native_digits(&st.year.to_string()),
                [msg::CALENDAR_PREVIOUS_YEAR, msg::CALENDAR_NEXT_YEAR],
            )
        } else {
            let r = year_range(st.years_from);
            (
                format!(
                    "{} - {}",
                    fmt.native_digits(&r.start().to_string()),
                    fmt.native_digits(&r.end().to_string())
                ),
                [msg::CALENDAR_PREVIOUS_YEARS, msg::CALENDAR_NEXT_YEARS],
            )
        };
        let labels = names.map(|n| self.i18n.tr(n));
        let nav = [(false, labels[0].as_str()), (false, labels[1].as_str())];
        let (open_years, step) = self.header(ui, &title, months, nav);
        if open_years {
            st.mode = Mode::Years;
            st.years_from = st.year.saturating_sub(cal::YEARS / 2).max(1);
        }
        let by = i16::try_from(step).unwrap_or(0);
        if months {
            st.year = st.year.saturating_add(by).clamp(1, 9999);
        } else {
            st.years_from = st
                .years_from
                .saturating_add(by.saturating_mul(cal::YEARS))
                .clamp(1, 9999 - cal::YEARS + 1);
        }
    }

    // Slot `i` of the month or year grid: text, current, selected, allowed.
    fn picker_slot(&self, st: &View, i: usize, selected: Option<usize>) -> (String, Look, bool) {
        let k = i16::try_from(i).unwrap_or(0);
        let (text, current, allowed) = if st.mode == Mode::Months {
            let m = i8::try_from(k + 1).unwrap_or(1);
            (
                self.fmt.month_name(m, NameWidth::Abbreviated),
                self.today.year() == st.year && self.today.month() == m,
                bounds_allow_month(self.bounds, st.year, m),
            )
        } else {
            let y = st.years_from.saturating_add(k);
            (
                self.fmt.native_digits(&y.to_string()),
                self.today.year() == y,
                y <= 9999,
            )
        };
        let look = if selected == Some(i) {
            Look::Selected
        } else if current {
            Look::Current
        } else {
            Look::Rest
        };
        (text, look, allowed)
    }

    // Months of `st.year` or twelve years, as one grid of `CalendarPicker` items.
    fn picker(&self, ui: &mut MineralUi<'_, Fluent>, st: &mut View) {
        let cx = &self.cx;
        let months = st.mode == Mode::Months;
        self.picker_header(ui, st);
        let n = cal::ITEM_COLUMNS * cal::MONTH_ROWS;
        let top = cal::PADDING + cal::HEADER_HEIGHT + cal::ITEM_GRID_MARGIN_TOP;
        let (year, bounds) = (st.year, self.bounds);
        let enabled = |i: usize| !months || bounds_allow_month(bounds, year, i8::try_from(i + 1).unwrap_or(1));
        let group = cx.id.with("picker");
        let selected = if months {
            self.value
                .filter(|d| d.year() == st.year)
                .and_then(|d| usize::try_from(d.month() - 1).ok())
        } else {
            self.value
                .and_then(|d| usize::try_from(i32::from(d.year()) - i32::from(st.years_from)).ok())
        };
        let stop = focus::tab_stop(
            ui.ctx(),
            group,
            selected.filter(|i| *i < n),
            &focus::Items::new(n, enabled),
        );
        let mut ids = Vec::with_capacity(n);
        let mut chosen = None;
        for i in 0..n {
            let (row, col) = (i / cal::ITEM_COLUMNS, i % cal::ITEM_COLUMNS);
            let x = cal::PADDING + col as f32 * (cal::ITEM_BUTTON + cal::ITEM_GAP);
            let y = top + row as f32 * (cal::ITEM_BUTTON + cal::ITEM_ROW_GAP);
            let rect = cell(cx, self.root, x, y, Pos2::new(cal::ITEM_BUTTON, cal::ITEM_BUTTON));
            let (text, look, allowed) = self.picker_slot(st, i, selected);
            let r = item(
                ui,
                cx,
                &Item {
                    rect,
                    id: group.with(i),
                    text: &text,
                    size: ty::FONT_SIZE_BASE_200,
                    weight: if look == Look::Rest {
                        ty::FONT_WEIGHT_REGULAR
                    } else {
                        ty::FONT_WEIGHT_SEMIBOLD
                    },
                    look,
                    sense: focus::roving_sense(Sense::click(), focus::TabStop::at(i, stop)),
                    disabled: !allowed,
                },
            );
            ids.push(r.id);
            let part = if months { "month" } else { "year" };
            let off = !allowed || cx.disabled;
            emit_button(ui, &r, self.key((i, part)), &text, Some(look == Look::Selected), off);
            if r.clicked() && allowed {
                chosen = Some(i16::try_from(i).unwrap_or(0));
            }
        }
        focus::RovingFocus::grid(cal::ITEM_COLUMNS).update_group(ui.core(), group, &ids, enabled);
        let Some(k) = chosen else { return };
        if months {
            st.month = i8::try_from(k + 1).unwrap_or(1);
            st.mode = Mode::Days;
            st.focus = Date::new(st.year, st.month, 1).ok().map(|d| self.bounds.clamp(d));
        } else {
            st.year = st.years_from.saturating_add(k);
            st.mode = Mode::Months;
        }
    }
}

impl MineralWidget for Calendar<'_> {
    type Kit = Fluent;
    type State = View;
    type Output = ();

    kit::widget_meta!(salt, enabled);

    fn sense(&self) -> Sense {
        Sense::hover()
    }

    fn min_target(&self, _: &MineralUi<'_, Self::Kit>) -> MinTarget {
        MinTarget::None
    }

    fn layout(&self, _: &MineralUi<'_, Fluent>, c: Constraints, _: &View) -> Measure {
        c.constrain(vec2(self.width(), self.height())).into()
    }

    fn update(&mut self, ui: &mut MineralUi<'_, Fluent>, frame: &WidgetFrame<'_>, st: &mut View) -> WidgetResponse<()> {
        let rt = ui.rt();
        let ctx = rt.ctx();
        let i18n = rt.i18n();
        let fmt = i18n.format();
        let today = self.today.unwrap_or_else(date::today);
        let now = ctx.cumulative_frame_nr();
        let id = ui.make_persistent_id(self.salt.id_salt());
        if !st.started || st.frame.saturating_add(1) < now || st.shown != *self.value {
            let at = self.value.unwrap_or(today);
            let at = self.bounds.clamp(at);
            *st = View {
                year: at.year(),
                month: at.month(),
                years_from: at.year().saturating_sub(cal::YEARS / 2).max(1),
                shown: *self.value,
                started: st.started && st.frame.saturating_add(1) >= now,
                ..View::default()
            };
        }
        st.frame = now;
        let run = Run {
            key: self.key.id(&self.salt),
            value: *self.value,
            bounds: self.bounds,
            week_numbers: self.week_numbers,
            go_to_today: self.go_to_today,
            autofocus: self.autofocus,
            cx: Cx {
                id,
                dir: i18n.direction(),
                t: &ui.tokens(),
                disabled: self.disabled,
            },
            fmt: fmt.as_ref(),
            i18n,
            root: frame.zones.visual,
            today,
            week_start: self.week_start(fmt.as_ref()),
        };
        let picked = match st.mode {
            Mode::Days => run.days(ui, st),
            Mode::Months | Mode::Years => {
                run.picker(ui, st);
                None
            }
        };
        let changed = picked.is_some_and(|d| *self.value != Some(d));
        if let Some(d) = picked {
            *self.value = Some(d);
        }
        WidgetResponse::new(frame.response.clone(), ()).with_changed(changed)
    }

    fn semantics(&self, _: &I18n, _: &WidgetResponse<()>, _: &View) -> Option<SemanticNode> {
        let mut n = SemanticNode::new(self.key.id(&self.salt), Role::Grid, &self.label);
        n.state.disabled = self.disabled;
        Some(n)
    }
}
