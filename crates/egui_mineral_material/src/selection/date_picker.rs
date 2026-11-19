use crate::Material;
use crate::MaterialExt;
use crate::MaterialUiExt as _;
use crate::Rt;
use egui::{
    Align, Color32, Context, CornerRadius, Frame, Id, Key, Margin, Pos2, Rangef, Rect, ScrollArea, Sense, Stroke,
    StrokeKind, Ui, UiBuilder, Vec2, pos2, scroll_area::ScrollBarVisibility, vec2,
};
use egui_mineral::i18n::Sides;
use egui_mineral::kit::state::TempState;
use egui_mineral::semantic::KeyedSalt;
use egui_mineral::widget::Availability;
use egui_mineral::widget::MineralUi;
use egui_mineral::{
    anchor::{self, Anchor, Side},
    date::{self, Date, DateBounds, DateRange, MonthGrid, Weekday, add_months, weekdays_from},
    focus,
    i18n::{DateFields, DateLength, Direction, LocaleFormat, NameWidth},
    kit,
    overlay::Popup,
    semantic::{self, Role, SemanticNode, StableId},
    widget::SizeExt,
};

use crate::containment::dialog;
use crate::selection::{PickerModalResponse, PickerResponse};
use crate::text_inputs::text_field::FieldChrome;
use crate::text_inputs::{TextField, TextFieldVariant};
use crate::widget::{
    common::{self, Interaction},
    indication, parts,
};
use crate::{Size, actions::IconButton};
use crate::{
    adaptive::MIN_INTERACTIVE,
    icons::{
        ICON_ARROW_DROP_DOWN, ICON_CALENDAR_MONTH, ICON_DATE_RANGE, ICON_EDIT, ICON_KEYBOARD_ARROW_LEFT,
        ICON_KEYBOARD_ARROW_RIGHT,
    },
    motion,
    spec::{
        ShapeRole,
        defaults::{button as cbutton, date_picker as dp},
        tokens::{baseline_button, button_small, date_picker_modal as m, divider},
    },
    tokens::Tokens,
    tr,
};

// Compose DateEntryContainer, DatePickerDialog
const WIDTH: f32 = m::CONTAINER_WIDTH;
const BODY_H: f32 = dp::CELL_SIZE * (dp::MAX_CALENDAR_ROWS + 1) as f32;
// Compose Month requiredHeight
const MONTH_H: f32 = dp::CELL_SIZE * dp::MAX_CALENDAR_ROWS as f32;
// Compose VerticalMonthsList
const MONTH_ITEM_H: f32 = dp::MONTH_SUBHEAD_TOP_PADDING
    + m::RANGE_SELECTION_MONTH_SUBHEAD_FONT.line_height()
    + dp::MONTH_SUBHEAD_BOTTOM_PADDING
    + MONTH_H;
const INLINE_H: f32 = dp::MONTH_YEAR_HEIGHT + BODY_H;
// m3.material.io docked date picker; Compose has no docked variant
const DOCKED_SHAPE: ShapeRole = ShapeRole::CornerLarge;

enum Sel<'a> {
    Single(&'a mut Option<Date>),
    Range(&'a mut DateRange),
}

impl Sel<'_> {
    fn anchor(&self) -> Option<Date> {
        match self {
            Sel::Single(v) => **v,
            Sel::Range(r) => r.start(),
        }
    }

    fn is_endpoint(&self, d: Date) -> bool {
        match self {
            Sel::Single(v) => **v == Some(d),
            Sel::Range(r) => r.start() == Some(d) || r.end() == Some(d),
        }
    }

    fn animates(&self, d: Date) -> bool {
        match self {
            Sel::Single(_) => true,
            Sel::Range(r) => r.start() == Some(d),
        }
    }

    // Compose SelectedRangeInfo
    fn span(&self) -> Option<(Date, Date)> {
        match self {
            Sel::Single(_) => None,
            Sel::Range(r) => r.start().zip(r.end()),
        }
    }

    // Compose updateDateSelection
    fn pick(&mut self, d: Date) {
        match self {
            Sel::Single(v) => **v = Some(d),
            Sel::Range(r) => {
                **r = match (r.start(), r.end()) {
                    (Some(s), None) if d >= s => DateRange::new(s, d),
                    _ => DateRange::starting(d),
                };
            }
        }
    }
}

#[derive(Clone, Copy)]
struct Limits {
    min: Date,
    max: Date,
}

impl Limits {
    fn contains(self, d: Date) -> bool {
        self.min <= d && d <= self.max
    }

    fn clamp(self, d: Date) -> Date {
        d.max(self.min).min(self.max)
    }
}

#[derive(Clone, Default)]
struct View {
    init: bool,
    year: i16,
    month: i8,
    focus: Option<Date>,
    offset: f32,
    scroll: Option<f32>,
    years: bool,
    year_focus: Option<i16>,
    year_stop: Option<i16>,
    year_scroll: Option<f32>,
}

impl TempState for View {}

#[derive(Clone, Default)]
struct Work {
    single: Option<Date>,
    range: DateRange,
    input: bool,
    texts: [String; 2],
}

// Compose DateEntryContainer
struct Header<'a> {
    title: &'a str,
    headline: &'a [String],
    range: bool,
    input: bool,
}

struct YearCx<'a> {
    grid: Id,
    stop: i16,
    first: i16,
    count: usize,
    size: Vec2,
    width: f32,
    dir: Direction,
    pending: Option<i16>,
    current: i16,
    selected: i16,
    t: &'a Tokens,
    loc: &'a dyn LocaleFormat,
}

#[derive(Default)]
struct YearHits {
    focused: Option<i16>,
    picked: Option<i16>,
    shown: bool,
}

struct ModalCx<'a> {
    ctx: &'a Context,
    id: Id,
    grid_id: Id,
    range: bool,
    title: String,
    choice: dialog::ChoiceLabels,
}

/// Compose `DisplayMode` / `TimePickerDisplayMode`: calendar or dial, or text entry.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum DisplayMode {
    #[default]
    /// Calendar grid (`DisplayMode.Picker`).
    Picker,
    /// Text field entry (`DisplayMode.Input`).
    Input,
}

/// Builder methods shared by [`DatePicker`] and [`DateRangePicker`].
macro_rules! config_builders {
    () => {
        crate::widget::setters! {
            /// Days and fields ignore input; modals do not open.
            disabled |s| s.cfg.disabled
        }

        crate::widget::setters! {
            /// Replaces the `<id_salt>` key prefix.
            agent_key(salt) |s| s.cfg.salt
        }

        /// Should overlap the browsable years (Compose `DatePickerDefaults.YearRange`); days
        /// outside them are not selectable.
        #[track_caller]
        pub fn bounds(mut self, bounds: DateBounds) -> Self {
            let (lo, hi) = year_range_dates();
            if bounds.min().is_some_and(|m| m > hi) || bounds.max().is_some_and(|m| m < lo) {
                egui_mineral::kit::sanitize::report(format_args!("bounds {bounds:?} miss the years {lo}..={hi}"));
            }
            self.cfg.bounds = bounds;
            self
        }

        /// Default from the locale ([`LocaleFormat`](egui_mineral::i18n::LocaleFormat)).
        pub fn first_weekday(mut self, first_weekday: Weekday) -> Self {
            self.cfg.first_weekday = Some(first_weekday);
            self
        }

        /// Date highlighted as today; default the system clock ([`date::today`](egui_mineral::date::today)).
        pub fn today(mut self, today: Date) -> Self {
            self.cfg.today = Some(today);
            self
        }

        crate::text_inputs::text_field::field_chrome_setters!(text cfg.chrome);

        /// Docked field and input fields; default filled for the docked field, outlined for the
        /// inputs of a modal.
        pub fn variant(mut self, variant: TextFieldVariant) -> Self {
            self.cfg.variant = Some(variant);
            self
        }

        /// Helper text below the docked field and the modal input fields.
        pub fn supporting(mut self, supporting: impl Into<String>) -> Self {
            self.cfg.chrome.supporting = Some(supporting.into());
            self
        }

        dialog::picker_label_setters!(cfg.labels);

        /// [`show`](Self::show) and the initial mode of [`show_modal`](Self::show_modal).
        pub fn input_mode(mut self, mode: DisplayMode) -> Self {
            self.mode = mode;
            self
        }
    };
}

/// MD3 single-date picker over a jiff civil [`Date`].
#[must_use = "call .show(..)"]
pub struct DatePicker<'a> {
    cfg: Config,
    value: &'a mut Option<Date>,
    mode: DisplayMode,
    label: Option<String>,
}

egui_mineral::kit::opaque_debug!(DatePicker<'_>);

impl<'a> DatePicker<'a> {
    /// `value` is the bound date; `None` selects nothing. Agent key prefix `<id_salt>`.
    pub fn new(id_salt: impl Into<KeyedSalt>, value: &'a mut Option<Date>) -> Self {
        Self {
            cfg: Config::new(id_salt.into()),
            value,
            mode: DisplayMode::Picker,
            label: None,
        }
    }

    config_builders!();

    /// Docked field label; defaults to the headline.
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// [`DisplayMode::Picker`]: Compose `DatePicker` without its header (month grid).
    /// [`DisplayMode::Input`]: MD3 docked picker, a date field with a calendar popup.
    pub fn show(self, ui: &mut MineralUi<'_, Material>) -> PickerResponse {
        match self.mode {
            DisplayMode::Picker => self.cfg.inline(ui, self.value),
            DisplayMode::Input => {
                let label = self.label.unwrap_or_else(|| self.cfg.title_text(ui.rt(), false));
                self.cfg.docked(ui, &label, self.value)
            }
        }
    }

    /// Compose `DatePickerDialog`; the value is written on confirm.
    pub fn show_modal(self, rt: &Rt, open: &mut bool) -> PickerModalResponse {
        self.cfg
            .modal_single(rt, open, self.value, self.mode == DisplayMode::Input)
    }
}

/// MD3 date range picker over a jiff civil [`DateRange`].
#[must_use = "call .show(..)"]
pub struct DateRangePicker<'a> {
    cfg: Config,
    value: &'a mut DateRange,
    mode: DisplayMode,
}

egui_mineral::kit::opaque_debug!(DateRangePicker<'_>);

impl<'a> DateRangePicker<'a> {
    /// `value` is the bound range. Agent key prefix `<id_salt>`.
    pub fn new(id_salt: impl Into<KeyedSalt>, value: &'a mut DateRange) -> Self {
        Self {
            cfg: Config::new(id_salt.into()),
            value,
            mode: DisplayMode::Picker,
        }
    }

    config_builders!();

    /// Compose `DateRangePicker` without its header: the weekdays over the vertically scrolling
    /// months, as tall as the [`DatePicker`] month grid (Compose takes the list height from its
    /// parent). [`DisplayMode::Input`] has no inline form: it shows the same calendar.
    pub fn show(self, ui: &mut MineralUi<'_, Material>) -> PickerResponse {
        self.cfg.inline_range(ui, self.value)
    }

    /// Compose `DateRangePicker` header and days in the `DatePickerDialog` surface; the value is
    /// written on confirm.
    pub fn show_modal(self, rt: &Rt, open: &mut bool) -> PickerModalResponse {
        self.cfg
            .modal_range(rt, open, self.value, self.mode == DisplayMode::Input)
    }
}

#[derive(Clone)]
struct Config {
    salt: KeyedSalt,
    bounds: DateBounds,
    first_weekday: Option<Weekday>,
    today: Option<Date>,
    labels: dialog::PickerLabels,
    disabled: bool,
    chrome: FieldChrome,
    variant: Option<TextFieldVariant>,
}

impl Config {
    fn new(salt: KeyedSalt) -> Self {
        Self {
            salt,
            bounds: DateBounds::default(),
            first_weekday: None,
            today: None,
            labels: dialog::PickerLabels::default(),
            disabled: false,
            chrome: FieldChrome::default(),
            variant: None,
        }
    }

    fn field_chrome(&self, default: TextFieldVariant) -> FieldChrome {
        FieldChrome {
            variant: self.variant.unwrap_or(default),
            ..self.chrome.clone()
        }
    }

    fn title_text(&self, rt: &Rt, range: bool) -> String {
        let id = if range {
            "mineral-date-range-headline"
        } else {
            "mineral-date-headline"
        };
        self.labels.headline(rt.i18n(), id)
    }

    fn week_start(&self, loc: &dyn LocaleFormat) -> Weekday {
        self.first_weekday.unwrap_or_else(|| loc.first_weekday())
    }

    fn today_or_clock(&self) -> Date {
        self.today.unwrap_or_else(egui_mineral::date::today)
    }

    // Compose DatePickerDefaults.YearRange
    fn limits(&self) -> Limits {
        let (lo, hi) = year_range_dates();
        Limits {
            min: self.bounds.min().map_or(lo, |m| m.clamp(lo, hi)),
            max: self.bounds.max().map_or(hi, |m| m.clamp(lo, hi)),
        }
    }

    fn inline(&self, ui: &mut MineralUi<'_, Material>, value: &mut Option<Date>) -> PickerResponse {
        let rt = ui.rt();
        let id = ui.make_persistent_id(self.salt.id_salt());
        let out = ui.scope(|ui| Self::surface(&mut rt.ui(ui), |ui| self.calendar(ui, id, &mut Sel::Single(value))));
        PickerResponse::new(out.response, out.inner)
    }

    fn inline_range(&self, ui: &mut MineralUi<'_, Material>, value: &mut DateRange) -> PickerResponse {
        let rt = ui.rt();
        let id = ui.make_persistent_id(self.salt.id_salt());
        let out = ui.scope(|ui| Self::surface(&mut rt.ui(ui), |ui| self.range_calendar(ui, id, INLINE_H, value)));
        PickerResponse::new(out.response, out.inner)
    }

    // Compose DateEntryContainer
    fn surface<R>(ui: &mut MineralUi<'_, Material>, add: impl FnOnce(&mut MineralUi<'_, Material>) -> R) -> R {
        let rt = ui.rt();
        let t = rt.tokens();
        Frame::new()
            .fill(t.role(m::CONTAINER_COLOR))
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing = Vec2::ZERO;
                add(&mut rt.ui(ui))
            })
            .inner
    }

    fn docked(&self, ui: &mut MineralUi<'_, Material>, label: &str, value: &mut Option<Date>) -> PickerResponse {
        let rt = ui.rt();
        let ctx = rt.ctx();
        let t = rt.tokens();
        let base = ui.make_persistent_id(self.salt.id_salt());
        let (open_id, text_id, grid_id) = (base.with("open"), base.with("text"), base.with("grid"));
        let field_salt = self.salt.child("field");
        let field_id = crate::text_inputs::text_field::field_id(ui, &field_salt);
        let edit_id = field_id.with("edit");
        let mut open = bool::load(ctx, open_id) && !self.disabled;
        let loc = rt.i18n().format();
        let loc = loc.as_ref();
        let mut text = String::load_or(ctx, text_id, || format_date(loc, *value));
        if !ctx.memory(|m| m.has_focus(edit_id)) {
            text = format_date(loc, *value);
        }
        let error = self.text_error(rt, loc, &text);
        let hint = input_hint(rt, loc);
        let field = TextField::new(field_salt, label, &mut text)
            .with_chrome(self.field_chrome(TextFieldVariant::Filled))
            .trailing_icon(ICON_CALENDAR_MONTH, crate::tr(rt.i18n(), "mineral-date-picker-open"))
            .placeholder_or(hint)
            .maybe_error(error)
            .disabled(self.disabled);
        let out = ui.mineral(field);
        let toggled = out.output().trailing_clicked && !self.disabled;
        let mut changed = false;
        if out.changed() {
            if text.trim().is_empty() {
                changed = value.take().is_some();
            } else if let Some(d) = self.parse(loc, &text) {
                changed = *value != Some(d);
                *value = Some(d);
                View::remove(ctx, grid_id);
            }
        }
        if toggled {
            open = !open;
        }
        if open {
            let shown = Popup::new(
                self.salt.child("popup"),
                Anchor::new(Side::Bottom, anchor::Align::Start),
            )
            .role(Role::Dialog)
            .name(self.title_text(rt, false))
            .anchor_rect(out.zones().visual)
            .return_focus(edit_id)
            .show(rt, out.response(), &mut open, |ui, _| {
                Frame::new()
                    .fill(t.role(m::CONTAINER_COLOR))
                    .corner_radius(DOCKED_SHAPE.corner_radius(&t.shape, ui.i18n().direction()))
                    .shadow(t.elevation_shadow(m::CONTAINER_ELEVATION))
                    .show(ui, |ui| {
                        ui.spacing_mut().item_spacing = Vec2::ZERO;
                        self.calendar(&mut rt.ui(ui), grid_id, &mut Sel::Single(value))
                    })
                    .inner
            });
            if shown.inner == Some(true) {
                changed = true;
                open = false;
            }
            if !open {
                View::remove(ctx, grid_id);
                text = format_date(loc, *value);
            }
        }
        open.store(ctx, open_id);
        text.store(ctx, text_id);
        // Fresh response: the field's own `changed` fires on unparsed text.
        PickerResponse::new(out.response().interact(Sense::hover()), changed)
    }

    fn modal_single(&self, rt: &Rt, open: &mut bool, value: &mut Option<Date>, input: bool) -> PickerModalResponse {
        let loc = rt.i18n().format();
        let work = Work {
            single: *value,
            input,
            texts: [format_date(loc.as_ref(), *value), String::new()],
            ..Work::default()
        };
        self.modal_impl(rt, open, work, false, value, |w| w.single)
    }

    fn modal_range(&self, rt: &Rt, open: &mut bool, value: &mut DateRange, input: bool) -> PickerModalResponse {
        let loc = rt.i18n().format();
        let work = Work {
            range: *value,
            input,
            texts: [
                format_date(loc.as_ref(), value.start()),
                format_date(loc.as_ref(), value.end()),
            ],
            ..Work::default()
        };
        self.modal_impl(rt, open, work, true, value, |w| w.range)
    }

    // Compose DatePickerDialog
    fn modal_impl<V: PartialEq>(
        &self,
        rt: &Rt,
        open: &mut bool,
        initial: Work,
        range: bool,
        value: &mut V,
        pick: impl FnOnce(&Work) -> V,
    ) -> PickerModalResponse {
        let ctx = rt.ctx();
        let id = egui_mineral::kit::scoped_id(rt.core(), Id::unique(("mineral::date_modal", self.salt.id_salt())));
        let grid_id = id.with("grid");
        let mc = ModalCx {
            ctx,
            id,
            grid_id,
            range,
            title: self.title_text(rt, range),
            choice: self.labels.choice(rt.i18n()),
        };
        dialog::PickerModal {
            id,
            salt: &self.salt,
            disabled: self.disabled,
            initial,
        }
        .show(
            rt,
            open,
            value,
            pick,
            |ui, work| (self.modal_body(ui, &mc, work), mc.title.as_str()),
            |d| d.remove::<View>(grid_id),
        )
    }

    fn modal_body(
        &self,
        ui: &mut MineralUi<'_, Material>,
        mc: &ModalCx<'_>,
        work: &mut Work,
    ) -> Option<dialog::Choice> {
        let rt = ui.rt();
        let (ctx, id, range) = (mc.ctx, mc.id, mc.range);
        let loc = rt.i18n().format();
        let loc = loc.as_ref();
        let dir = rt.i18n().direction();
        ui.set_width(WIDTH);
        ui.spacing_mut().item_spacing = Vec2::ZERO;
        let headline = if range {
            headline_range(rt, loc, &work.range)
        } else {
            vec![headline_single(rt, loc, work.single, work.input)]
        };
        let header = Header {
            title: &mc.title,
            headline: &headline,
            range,
            input: work.input,
        };
        if self.header(ui, id, &header) {
            work.input = !work.input;
            if work.input {
                work.texts = if range {
                    [format_date(loc, work.range.start()), format_date(loc, work.range.end())]
                } else {
                    [format_date(loc, work.single), String::new()]
                };
            } else {
                View::remove(ctx, mc.grid_id);
            }
        }
        let mut valid = true;
        if work.input {
            valid = self.input_fields(ui, id, work, range);
        } else if range {
            // Compose `DatePickerDialog` `heightIn(max)`: the months list takes what the header
            // and the buttons leave.
            let height = m::CONTAINER_HEIGHT - ui.min_rect().height() - dialog_buttons_height(rt);
            self.range_calendar(ui, mc.grid_id, height, &mut work.range);
        } else {
            self.calendar(ui, mc.grid_id, &mut Sel::Single(&mut work.single));
        }
        let complete = valid
            && if range {
                work.range.is_complete()
            } else {
                work.single.is_some()
            };
        let (end, bottom) = (
            margin(dp::DIALOG_BUTTONS_END_PADDING),
            margin(dp::DIALOG_BUTTONS_BOTTOM_PADDING),
        );
        let Sides { left, right } = dir.sides(0, end);
        let mut action = None;
        Frame::new()
            .inner_margin(Margin {
                left,
                right,
                top: 0,
                bottom,
            })
            .show(ui, |ui| {
                let size = vec2(WIDTH - dp::DIALOG_BUTTONS_END_PADDING, 0.0);
                ui.allocate_ui(size, |ui| {
                    action = dialog::confirm_cancel(&mut rt.ui(ui), &self.salt, &mc.choice, complete);
                });
            });
        action
    }

    // Compose DatePickerHeader
    fn header(&self, ui: &mut MineralUi<'_, Material>, _id: Id, h: &Header<'_>) -> bool {
        let rt = ui.rt();
        let t = rt.tokens();
        let dir = rt.i18n().direction();
        let (start, min_h, title_top, font) = if h.range {
            (
                dp::RANGE_HEADER_START_PADDING,
                m::RANGE_SELECTION_HEADER_CONTAINER_HEIGHT - dp::RANGE_HEADER_HEIGHT_OFFSET,
                0.0,
                m::RANGE_SELECTION_HEADER_HEADLINE_FONT,
            )
        } else {
            (
                dp::HEADER_START_PADDING,
                m::HEADER_CONTAINER_HEIGHT,
                dp::TITLE_TOP_PADDING,
                m::HEADER_HEADLINE_FONT,
            )
        };
        let (icon, msg) = if h.input {
            (ICON_DATE_RANGE, "mineral-date-to-calendar")
        } else {
            (ICON_EDIT, "mineral-date-to-input")
        };
        let toggle = IconButton::new(self.salt.child("mode"), icon, tr(rt.i18n(), msg));
        let toggle_size = toggle.footprint(rt, ui.density());
        let headline_box = font.line_height() + dp::HEADLINE_BOTTOM_PADDING;
        let row_h = headline_box.max(toggle_size.y + dp::MODE_TOGGLE_PADDING);
        let title_h = title_top + m::HEADER_SUPPORTING_TEXT_FONT.line_height();
        let height = min_h.max(title_h + row_h + divider::THICKNESS);
        let (rect, _) = ui.allocate_exact_size(vec2(WIDTH, height), Sense::hover());
        let x = |from_start: f32, w: f32| dir.start_x(rect, Rangef::new(from_start, from_start + w));
        let p = ui.painter();
        let title = parts::type_wrapped(
            p,
            h.title,
            m::HEADER_SUPPORTING_TEXT_FONT,
            t.role(m::HEADER_SUPPORTING_TEXT_COLOR),
            WIDTH - start - dp::HEADER_END_PADDING,
            1,
        );
        p.galley(
            pos2(x(start, title.size().x), rect.top() + title_top),
            title,
            Color32::PLACEHOLDER,
        );
        let divider_top = rect.bottom() - divider::THICKNESS;
        let row_top = divider_top - row_h;
        let toggle_slot = toggle_size.x + dp::MODE_TOGGLE_PADDING;
        let headline_w = WIDTH - toggle_slot - start - dp::HEADER_END_PADDING;
        let mut offset = 0.0;
        for text in h.headline {
            let g = parts::type_wrapped(
                p,
                text,
                font,
                t.role(m::HEADER_HEADLINE_COLOR),
                (headline_w - offset).max(0.0),
                1,
            );
            let w = g.size().x;
            p.galley(
                pos2(x(start + offset, w), row_top + (row_h - headline_box) / 2.0),
                g,
                Color32::PLACEHOLDER,
            );
            offset += w + dp::RANGE_HEADLINE_SPACING;
        }
        p.hline(
            rect.x_range(),
            divider_top + divider::THICKNESS / 2.0,
            Stroke::new(divider::THICKNESS, t.role(divider::COLOR)),
        );
        let slot = Rect::from_min_size(
            pos2(
                x(WIDTH - toggle_slot, toggle_size.x),
                row_top + (row_h - toggle_size.y - dp::MODE_TOGGLE_PADDING) / 2.0,
            ),
            toggle_size,
        );
        rt.ui(&mut ui.new_child(UiBuilder::new().id_salt("mode").max_rect(slot)))
            .mineral(toggle)
            .clicked()
    }

    // Compose DateInputContent, DateRangeInputContent
    fn input_fields(&self, ui: &mut MineralUi<'_, Material>, _id: Id, work: &mut Work, range: bool) -> bool {
        let rt = ui.rt();
        let loc = rt.i18n().format();
        let loc = loc.as_ref();
        let hint = input_hint(rt, loc);
        let labels: &[&str] = if range {
            &["mineral-date-start", "mineral-date-end"]
        } else {
            &["mineral-date-label"]
        };
        let n = labels.len() as f32;
        let inner = WIDTH - 2.0 * dp::INPUT_HORIZONTAL_PADDING;
        let field_w = (inner - (n - 1.0) * dp::RANGE_INPUT_SPACING) / n;
        let errors: Vec<Option<String>> = work.texts[..labels.len()]
            .iter()
            .map(|text| self.text_error(rt, loc, text))
            .collect();
        let mut valid = errors.iter().all(Option::is_none);
        let side = margin(dp::INPUT_HORIZONTAL_PADDING);
        // Compose keeps the error-free bottom padding by taking the supporting text's top padding off.
        let bottom = if valid {
            dp::INPUT_BOTTOM_PADDING
        } else {
            dp::INPUT_BOTTOM_PADDING - SUPPORTING_TOP_PADDING
        };
        Frame::new()
            .inner_margin(Margin {
                left: side,
                right: side,
                top: margin(dp::INPUT_TOP_PADDING),
                bottom: margin(bottom),
            })
            .show(ui, |ui| {
                ui.set_width(inner);
                ui.spacing_mut().item_spacing = vec2(dp::RANGE_INPUT_SPACING, 0.0);
                ui.with_layout(rt.i18n().direction().row().with_cross_align(Align::Min), |ui| {
                    for (i, (msg, error)) in labels.iter().zip(errors).enumerate() {
                        rt.ui(ui).mineral(
                            TextField::new(self.salt.child((i, "date")), tr(rt.i18n(), msg), &mut work.texts[i])
                                .with_chrome(self.field_chrome(TextFieldVariant::Outlined))
                                .placeholder_or(hint.clone())
                                .maybe_error(error)
                                .preferred_width(field_w),
                        );
                    }
                });
            });
        let ends = [0, 1].map(|i| self.parse(loc, &work.texts[i]));
        if range {
            work.range = match ends {
                [Some(s), Some(e)] if s <= e => DateRange::new(s, e),
                [Some(s), end] => {
                    valid &= end.is_none();
                    DateRange::starting(s)
                }
                [None, _] => DateRange::default(),
            };
        } else {
            work.single = ends[0];
        }
        valid
    }

    fn text_error(&self, rt: &Rt, loc: &dyn LocaleFormat, text: &str) -> Option<String> {
        if text.trim().is_empty() {
            return None;
        }
        match loc.parse_date(text) {
            None => Some(tr(rt.i18n(), "mineral-date-invalid")),
            Some(d) if !self.limits().contains(d) => Some(tr(rt.i18n(), "mineral-date-out-of-range")),
            Some(_) => None,
        }
    }

    fn parse(&self, loc: &dyn LocaleFormat, text: &str) -> Option<Date> {
        loc.parse_date(text).filter(|d| self.limits().contains(*d))
    }

    fn initial_view(&self, sel: &Sel<'_>) -> Date {
        self.limits()
            .clamp(sel.anchor().unwrap_or_else(|| self.today_or_clock()))
    }

    // Compose DatePickerContent
    fn calendar(&self, ui: &mut MineralUi<'_, Material>, id: Id, sel: &mut Sel<'_>) -> bool {
        let rt = ui.rt();
        let ctx = rt.ctx();
        let t = rt.tokens();
        let mut v = View::load(ctx, id);
        // Bounds and persisted state change between frames.
        let shown = v.init.then(|| Date::new(v.year, v.month, 1).ok()).flatten();
        let d = self.limits().clamp(shown.unwrap_or_else(|| self.initial_view(sel)));
        (v.year, v.month, v.init) = (d.year(), d.month(), true);
        let (nav, _) = ui.allocate_exact_size(vec2(WIDTH, dp::MONTH_YEAR_HEIGHT), Sense::hover());
        let nav = nav.shrink2(vec2(dp::HORIZONTAL_PADDING, 0.0));
        let (body, _) = ui.allocate_exact_size(vec2(WIDTH, BODY_H), Sense::hover());
        let body = body.shrink2(vec2(dp::HORIZONTAL_PADDING, 0.0));
        self.months_navigation(ui, id, nav, &mut v, &t);
        let picked = if v.years {
            self.year_grid(ui, id, body, &mut v, &t);
            false
        } else {
            self.month(ui, id, body, sel, &mut v, &t)
        };
        v.store(ctx, id);
        picked
    }

    // Compose MonthsNavigation
    fn months_navigation(&self, ui: &mut MineralUi<'_, Material>, id: Id, nav: Rect, v: &mut View, t: &Tokens) {
        let rt = ui.rt();
        let ctx = rt.ctx();
        let loc = rt.i18n().format();
        let dir = rt.i18n().direction();
        let title = loc.month_year(v.year, v.month);
        if self.year_menu(ui, id, nav, &title, v.years, t) {
            v.years = !v.years;
            if v.years {
                let first = self.limits().min.year();
                let row = usize::try_from(v.year - first - YEARS_IN_ROW_I16).unwrap_or_default() / dp::YEARS_IN_ROW;
                v.year_scroll = Some(row as f32 * YEAR_PITCH);
                // Compose focuses the displayed year when the grid opens; pointer opens keep focus put.
                if !ctx.input(|i| i.pointer.any_released()) {
                    v.year_focus = Some(v.year);
                }
            }
        }
        if v.years {
            return;
        }
        let limits = self.limits();
        let first = Date::new(v.year, v.month, 1).unwrap_or(limits.min);
        let prev = add_months(first, -1);
        let next = add_months(first, 1);
        let prev_disabled = self.disabled || first <= limits.min;
        let next_disabled = self.disabled || next > limits.max;
        let prev_icon = dir.pick(ICON_KEYBOARD_ARROW_LEFT, ICON_KEYBOARD_ARROW_RIGHT);
        let next_icon = dir.pick(ICON_KEYBOARD_ARROW_RIGHT, ICON_KEYBOARD_ARROW_LEFT);
        let density = ui.density();
        let arrow = |ui: &mut Ui, from_end: usize, which: &'static str, icon, msg: &str, disabled: bool| {
            let b = IconButton::new(self.salt.child((which, "month")), icon, tr(rt.i18n(), msg)).disabled(disabled);
            let size = b.footprint(rt, density);
            let offset = size.x * (from_end as f32 + 1.0);
            let x = dir.reversed().start_x(nav, Rangef::new(offset - size.x, offset));
            let slot = Rect::from_min_size(pos2(x, nav.center().y - size.y / 2.0), size);
            rt.ui(&mut ui.new_child(UiBuilder::new().id_salt(which).max_rect(slot)))
                .mineral(b)
                .clicked()
        };
        if arrow(ui, 1, "prev", prev_icon, "mineral-date-prev-month", prev_disabled) {
            (v.year, v.month) = (prev.year(), prev.month());
        }
        if arrow(ui, 0, "next", next_icon, "mineral-date-next-month", next_disabled) {
            (v.year, v.month) = (next.year(), next.month());
        }
    }

    // Compose YearPickerMenuButton, TextButton
    fn year_menu(
        &self,
        ui: &mut MineralUi<'_, Material>,
        id: Id,
        nav: Rect,
        title: &str,
        expanded: bool,
        t: &Tokens,
    ) -> bool {
        let rt = ui.rt();
        let ctx = rt.ctx();
        let dir = rt.i18n().direction();
        let color = t.role(dp::NAVIGATION_CONTENT_COLOR);
        // `ButtonDefaults.textStyleFor` the 40dp baseline height.
        let font = Size::Small.metrics().font;
        let label = parts::type_galley(rt.core(), ui.painter(), title, font, color, None);
        let w = (2.0 * cbutton::TEXT_SPACE + label.size().x + button_small::ICON_LABEL_SPACE + dp::YEAR_MENU_ICON_SIZE)
            .max(cbutton::MIN_WIDTH);
        let h = baseline_button::CONTAINER_HEIGHT;
        let x = dir.start_x(nav, Rangef::new(0.0, 0.0 + w));
        let visual = Rect::from_min_size(pos2(x, nav.center().y - h / 2.0), vec2(w, h));
        let hit = visual.expand2(vec2(0.0, (MIN_INTERACTIVE.points(rt.core()) - h).max(0.0) / 2.0));
        let resp = kit::input::interact(
            ui,
            hit,
            id.with("year-menu"),
            Sense::click(),
            Availability::disabled_if(self.disabled),
        );
        let content = if self.disabled {
            common::disabled_content(t)
        } else {
            color
        };
        let r = CornerRadius::from(h / 2.0);
        indication::paint(
            ui,
            &resp,
            visual,
            r,
            content,
            Interaction::of(&resp, Availability::disabled_if(self.disabled)),
            t,
        );
        kit::input::hover_cursor(rt.core(), &resp, Availability::disabled_if(self.disabled));
        let p = ui.painter();
        let text_x = dir.start_x(
            visual,
            Rangef::new(cbutton::TEXT_SPACE, cbutton::TEXT_SPACE + label.size().x),
        );
        let icon_x = dir.reversed().start_x(
            visual,
            Rangef::point(cbutton::TEXT_SPACE + dp::YEAR_MENU_ICON_SIZE / 2.0),
        );
        let text_y = visual.center().y - label.size().y / 2.0;
        p.galley(pos2(text_x, text_y), label, content);
        let angle = if expanded { std::f32::consts::PI } else { 0.0 };
        parts::icon_rotated(
            p,
            pos2(icon_x, visual.center().y),
            ICON_ARROW_DROP_DOWN,
            dp::YEAR_MENU_ICON_SIZE,
            content,
            angle,
        );
        semantic::emit(ctx, Some(resp.id), visual, || {
            let mut n = SemanticNode::new(StableId::of(&self.salt, "year-button"), Role::Button, title)
                .clickable(Availability::disabled_if(self.disabled));
            n.state.expanded = Some(expanded);
            n
        });
        resp.clicked()
    }

    // Compose YearPicker
    fn year_grid(&self, ui: &mut MineralUi<'_, Material>, id: Id, body: Rect, v: &mut View, t: &Tokens) {
        let rt = ui.rt();
        let ctx = rt.ctx();
        let loc = rt.i18n().format();
        let dir = rt.i18n().direction();
        let limits = self.limits();
        let (first, last) = (limits.min.year(), limits.max.year());
        let count = usize::try_from(last - first + 1).unwrap_or_default();
        let rows = count.div_ceil(dp::YEARS_IN_ROW);
        let size = vec2(m::SELECTION_YEAR_CONTAINER_WIDTH, m::SELECTION_YEAR_CONTAINER_HEIGHT);
        let grid = id.with("years");
        let height = body.height() - divider::THICKNESS;
        let area = Rect::from_min_size(body.min, vec2(body.width(), height));
        let mut scroll = ScrollArea::vertical()
            .id_salt(grid)
            .auto_shrink([false; 2])
            .max_height(height);
        if let Some(off) = v.year_scroll.take() {
            scroll = scroll.vertical_scroll_offset(off);
        }
        let yc = YearCx {
            grid,
            stop: v.year_focus.or(v.year_stop).unwrap_or(v.year),
            first,
            count,
            size,
            width: body.width(),
            dir,
            pending: v.year_focus.take().map(|y| y.clamp(first, last)),
            current: self.today_or_clock().year(),
            selected: v.year,
            t,
            loc: loc.as_ref(),
        };
        let mut hits = YearHits::default();
        {
            let ui = &mut ui.new_child(UiBuilder::new().id_salt("years").max_rect(area));
            ui.spacing_mut().item_spacing = vec2(0.0, dp::YEARS_VERTICAL_SPACING);
            scroll.show_rows(ui, size.y, rows, |ui, range| {
                for row in range {
                    self.year_row(&mut rt.ui(ui), &yc, row, &mut hits);
                }
            });
        }
        ui.painter().hline(
            body.x_range().expand(dp::HORIZONTAL_PADDING),
            area.bottom() + divider::THICKNESS / 2.0,
            Stroke::new(divider::THICKNESS, t.role(divider::COLOR)),
        );
        if let Some(target) = yc.pending.filter(|_| !hits.shown) {
            let row = usize::try_from(target - first).unwrap_or_default() / dp::YEARS_IN_ROW;
            v.year_scroll = Some(row as f32 * YEAR_PITCH);
            v.year_focus = Some(target);
        }
        if let Some(cur) = hits.focused {
            let target = Self::year_target(ctx, dir, grid, cur, first..=last);
            if target != cur {
                v.year_focus = Some(target);
            }
        }
        if hits.focused.is_some() {
            v.year_stop = hits.focused;
        }
        if let Some(d) = hits.picked.and_then(|year| Date::new(year, v.month, 1).ok()) {
            let d = limits.clamp(d);
            (v.year, v.month, v.years, v.year_stop) = (d.year(), d.month(), false, None);
        }
    }

    fn year_row(&self, ui: &mut MineralUi<'_, Material>, yc: &YearCx<'_>, row: usize, hits: &mut YearHits) {
        let rt = ui.rt();
        let (line, _) = ui.allocate_exact_size(vec2(yc.width, yc.size.y), Sense::hover());
        let col_w = yc.width / dp::YEARS_IN_ROW as f32;
        for col in 0..dp::YEARS_IN_ROW {
            let i = row * dp::YEARS_IN_ROW + col;
            if i >= yc.count {
                break;
            }
            let year = yc.first.saturating_add(i16::try_from(i).unwrap_or(i16::MAX));
            let slot = yc.dir.slot(col, dp::YEARS_IN_ROW);
            let cell = Rect::from_center_size(
                pos2(line.left() + col_w * (slot as f32 + 0.5), line.center().y),
                yc.size,
            );
            let sense = focus::roving_sense(Sense::click(), focus::TabStop::at(year, yc.stop));
            let resp = kit::input::interact(
                &rt.ui(ui),
                cell,
                yc.grid.with(year),
                sense,
                Availability::disabled_if(self.disabled),
            );
            kit::focus::follow_press(&resp);
            if yc.pending == Some(year) {
                resp.request_focus();
                resp.scroll_to_me(None);
                hits.shown = true;
            }
            if resp.has_focus() {
                hits.focused = Some(year);
            }
            if resp.clicked() {
                hits.picked = Some(year);
            }
            self.paint_year(ui, &resp, cell, year, yc);
        }
    }

    fn year_target(ctx: &Context, dir: Direction, grid: Id, cur: i16, years: std::ops::RangeInclusive<i16>) -> i16 {
        kit::focus::lock_arrows(ctx, grid.with(cur), kit::focus::Arrows::Both);
        let mut target = cur;
        for key in kit::input::plain_key_presses(ctx) {
            let delta = match key {
                Key::ArrowLeft | Key::ArrowRight => {
                    if key == dir.forward_key() {
                        1
                    } else {
                        -1
                    }
                }
                Key::ArrowUp => -YEARS_IN_ROW_I16,
                Key::ArrowDown => YEARS_IN_ROW_I16,
                _ => continue,
            };
            target = (target + delta).clamp(*years.start(), *years.end());
        }
        target
    }

    // Compose Year
    fn paint_year(&self, ui: &MineralUi<'_, Material>, resp: &egui::Response, cell: Rect, year: i16, yc: &YearCx<'_>) {
        let (t, selected, current) = (yc.t, year == yc.selected, year == yc.current);
        let text = &yc.loc.native_digits(&year.to_string());
        let rt = ui.rt();
        let enabled = !self.disabled;
        let dim = |col: Color32| {
            if enabled { col } else { common::dim(col, t) }
        };
        let content = dim(if selected {
            t.role(m::SELECTION_YEAR_SELECTED_LABEL_TEXT_COLOR)
        } else if current {
            t.role(m::DATE_TODAY_LABEL_TEXT_COLOR)
        } else {
            t.role(m::SELECTION_YEAR_UNSELECTED_LABEL_TEXT_COLOR)
        });
        let r = m::SELECTION_YEAR_STATE_LAYER_SHAPE.corner_radius(&t.shape, Direction::Ltr);
        let r = common::radius(cell, r.nw);
        let k = egui_mineral::motion::spring(
            ui.core(),
            resp.id.with("selected"),
            f32::from(selected),
            motion::DEFAULT_EFFECTS,
        )
        .clamp(0.0, 1.0);
        let p = ui.painter();
        if k > 0.0 {
            p.rect_filled(
                cell,
                r,
                common::with_alpha(dim(t.role(m::SELECTION_YEAR_SELECTED_CONTAINER_COLOR)), k),
            );
        }
        indication::paint(
            ui,
            resp,
            cell,
            r,
            content,
            Interaction::of(resp, Availability::enabled_if(enabled)),
            t,
        );
        if current && !selected {
            p.rect_stroke(
                cell,
                r,
                Stroke::new(
                    m::DATE_TODAY_CONTAINER_OUTLINE_WIDTH,
                    t.role(m::DATE_TODAY_CONTAINER_OUTLINE_COLOR),
                ),
                StrokeKind::Inside,
            );
        }
        kit::input::hover_cursor(rt.core(), resp, Availability::enabled_if(enabled));
        let g = parts::type_galley(rt.core(), p, text, m::SELECTION_YEAR_LABEL_TEXT_FONT, content, None);
        p.galley(cell.center() - g.size() / 2.0, g, content);
        semantic::emit(ui.ctx(), Some(resp.id), cell, || {
            let mut n = SemanticNode::new(StableId::of(&self.salt, (year, "year")), Role::Button, text)
                .clickable(Availability::enabled_if(enabled));
            n.state.selected = Some(selected);
            n
        });
    }

    // Compose WeekDays
    fn weekdays(&self, ui: &MineralUi<'_, Material>, top_left: Pos2, loc: &dyn LocaleFormat, t: &Tokens) {
        let rt = ui.rt();
        let dir = rt.i18n().direction();
        let color = t.role(m::WEEKDAYS_LABEL_TEXT_COLOR);
        let p = ui.painter();
        for (col, w) in weekdays_from(self.week_start(loc)).iter().enumerate() {
            let cell = Rect::from_min_size(
                pos2(col_x(top_left.x, col, dir), top_left.y),
                Vec2::splat(dp::CELL_SIZE),
            );
            let g = parts::type_galley(
                rt.core(),
                p,
                &loc.weekday_name(*w, NameWidth::Narrow),
                m::WEEKDAYS_LABEL_TEXT_FONT,
                color,
                None,
            );
            p.galley(cell.center() - g.size() / 2.0, g, color);
        }
    }

    // Compose Day
    fn day(
        &self,
        ui: &MineralUi<'_, Material>,
        env: &DayEnv<'_>,
        cell: Rect,
        d: Date,
        sel: &Sel<'_>,
        tab_stop: focus::TabStop,
    ) -> egui::Response {
        let enabled = !self.disabled && env.limits.contains(d);
        let sense = focus::roving_sense(Sense::click(), tab_stop);
        let resp = kit::input::interact(ui, cell, env.id.with(d), sense, Availability::enabled_if(enabled));
        kit::focus::follow_press(&resp);
        let span = sel.span();
        let day = DayPaint {
            cell,
            text: env.loc.native_digits(&d.day().to_string()),
            enabled,
            endpoint: sel.is_endpoint(d),
            animate: sel.animates(d),
            in_span: span.is_some_and(|(s, e)| d >= s && d <= e),
            span_start: span.is_some_and(|(s, _)| s == d),
            span_end: span.is_some_and(|(_, e)| e == d),
            today: env.today == d,
            dir: env.dir,
        };
        paint_day(ui, env.t, &day, &resp);
        semantic::emit(ui.ctx(), Some(resp.id), cell, || {
            let mut node = SemanticNode::new(
                StableId::of(&self.salt, (d.to_string(), "cell")),
                Role::GridCell,
                env.loc.date(d, DateFields::YearMonthDay, DateLength::Long),
            )
            .clickable(Availability::enabled_if(enabled));
            node.state.selected = Some(day.endpoint);
            if day.today {
                node.value = Some("today".to_owned());
            }
            node
        });
        resp
    }

    fn day_env<'a>(&self, rt: &Rt, id: Id, loc: &'a dyn LocaleFormat, t: &'a Tokens) -> DayEnv<'a> {
        DayEnv {
            id,
            loc,
            limits: self.limits(),
            today: self.today_or_clock(),
            dir: rt.i18n().direction(),
            t,
        }
    }

    // Compose WeekDays, Month
    fn month(
        &self,
        ui: &mut MineralUi<'_, Material>,
        id: Id,
        body: Rect,
        sel: &mut Sel<'_>,
        v: &mut View,
        t: &Tokens,
    ) -> bool {
        let rt = ui.rt();
        let ctx = rt.ctx();
        let dir = rt.i18n().direction();
        let loc = rt.i18n().format();
        let loc = loc.as_ref();
        let env = self.day_env(rt, id, loc, t);
        let week_start = self.week_start(loc);
        let Ok(first) = Date::new(v.year, v.month, 1) else {
            return false;
        };
        let grid = MonthGrid::new(first, week_start);
        self.weekdays(ui, body.min, loc, t);
        let grid_top = body.top() + dp::CELL_SIZE;
        let mut picked = false;
        let mut focused_date = None;
        // One Tab stop per grid: focused day, else the selection, else today, else the first day.
        let month_days: Vec<Date> = grid.days.iter().copied().filter(|d| grid.in_month(*d)).collect();
        let index_of = |d: Option<Date>| {
            d.filter(|d| grid.in_month(*d) && env.limits.contains(*d))
                .map(day_index)
        };
        let stop = focus::tab_stop(
            ctx,
            id,
            index_of(sel.anchor()).or(index_of(Some(env.today))),
            month_days.len(),
            |i| env.limits.contains(month_days[i]),
        );
        let mut day_ids = Vec::with_capacity(month_days.len());
        for (row, week) in grid.weeks().enumerate() {
            let top = grid_top + dp::CELL_SIZE * row as f32;
            for (col, d) in week.iter().copied().enumerate() {
                if !grid.in_month(d) {
                    continue;
                }
                let cell = Rect::from_min_size(pos2(col_x(body.left(), col, env.dir), top), Vec2::splat(dp::CELL_SIZE));
                let resp = self.day(ui, &env, cell, d, sel, focus::TabStop::at(day_index(d), stop));
                day_ids.push((day_index(d), resp.id));
                if resp.has_focus() {
                    focused_date = Some(d);
                }
                if resp.clicked() {
                    sel.pick(d);
                    picked = true;
                }
            }
        }
        kit::focus::remember(ctx, id, day_ids);
        let grid_rect = Rect::from_min_max(pos2(body.left(), grid_top), body.max);
        let title = loc.month_year(v.year, v.month);
        semantic::emit(ctx, None, grid_rect, || {
            let mut n = SemanticNode::new(StableId::of(&self.salt, "grid"), Role::Grid, &title);
            n.state.disabled = self.disabled;
            n
        });

        // <https://www.w3.org/WAI/ARIA/apg/patterns/dialog-modal/examples/datepicker-dialog/>
        if let Some(cur) = focused_date {
            kit::focus::lock_arrows(ctx, id.with(cur), kit::focus::Arrows::Both);
            let target = key_target(ctx, cur, env.limits, dir, week_start);
            if target != cur {
                v.focus = Some(target);
                (v.year, v.month) = (target.year(), target.month());
            }
        }
        if let Some(f) = v.focus
            && grid.in_month(f)
        {
            ctx.memory_mut(|m| m.request_focus(id.with(f)));
            v.focus = None;
        }
        picked
    }

    // Compose DateRangePickerContent
    fn range_calendar(&self, ui: &mut MineralUi<'_, Material>, id: Id, height: f32, range: &mut DateRange) -> bool {
        let rt = ui.rt();
        let ctx = rt.ctx();
        let t = rt.tokens();
        let loc = rt.i18n().format();
        let mut v = View::load(ctx, id);
        let (rect, _) = ui.allocate_exact_size(vec2(WIDTH, height), Sense::hover());
        let body = rect.shrink2(vec2(dp::HORIZONTAL_PADDING, 0.0));
        self.weekdays(ui, body.min, loc.as_ref(), &t);
        let list = Rect::from_min_max(pos2(body.left(), body.top() + dp::CELL_SIZE), body.max);
        let env = self.day_env(rt, id, loc.as_ref(), &t);
        let picked = self.months_list(ui, &env, list, &mut Sel::Range(range), &mut v);
        v.store(ctx, id);
        picked
    }

    // Compose VerticalMonthsList
    fn months_list(
        &self,
        ui: &mut MineralUi<'_, Material>,
        env: &DayEnv<'_>,
        list: Rect,
        sel: &mut Sel<'_>,
        v: &mut View,
    ) -> bool {
        let rt = ui.rt();
        let dir = rt.i18n().direction();
        let week_start = self.week_start(env.loc);
        let first = env.limits.min.first_of_month();
        let count = month_offset(first, env.limits.max).unwrap_or_default() + 1;
        let max_offset = (count as f32 * MONTH_ITEM_H - list.height()).max(0.0);
        if !v.init {
            v.init = true;
            v.scroll = Some(month_top(first, self.initial_view(sel)));
        }
        let has_focus = |d: &Date| rt.ctx().memory(|m| m.has_focus(env.id.with(*d)));
        if let Some(cur) = v.focus.filter(has_focus) {
            let target = key_target(rt.ctx(), cur, env.limits, dir, week_start);
            if target != cur {
                rt.ctx().memory_mut(|m| m.request_focus(env.id.with(target)));
                v.focus = Some(target);
                let top = month_top(first, target) + day_top(target, week_start);
                let off = v.scroll.unwrap_or(v.offset);
                if top < off {
                    v.scroll = Some(top);
                } else if top + dp::CELL_SIZE > off + list.height() {
                    v.scroll = Some(top + dp::CELL_SIZE - list.height());
                }
            }
        }
        let mut scroll = ScrollArea::vertical()
            .id_salt(env.id.with("months"))
            .auto_shrink([false; 2])
            .scroll_bar_visibility(ScrollBarVisibility::AlwaysHidden);
        if let Some(off) = v.scroll.take() {
            scroll = scroll.vertical_scroll_offset(off.clamp(0.0, max_offset));
        }
        let mut picked = false;
        let mut focused = None;
        let mut shown_first = 0;
        let out = {
            let ui = &mut ui.new_child(UiBuilder::new().id_salt("months").max_rect(list));
            scroll.show_viewport(ui, |ui, viewport| {
                ui.set_height(count as f32 * MONTH_ITEM_H);
                let origin = ui.max_rect().min;
                let at = |y: f32| {
                    #[expect(clippy::cast_sign_loss, clippy::cast_possible_truncation, reason = "y >= 0")]
                    let i = (y.max(0.0) / MONTH_ITEM_H) as usize;
                    i.min(count - 1)
                };
                let visible = at(viewport.min.y)..at(viewport.max.y) + 1;
                shown_first = visible.start;
                let shown =
                    |d: Date| env.limits.contains(d) && month_offset(first, d).is_some_and(|i| visible.contains(&i));
                let held = v.focus.filter(has_focus);
                let pinned = held
                    .and_then(|d| month_offset(first, d))
                    .filter(|i| *i < count && !visible.contains(i));
                let stop = held
                    .or(sel.anchor().filter(|d| shown(*d)))
                    .or(Some(env.today).filter(|d| shown(*d)))
                    .or(v.focus.filter(|d| shown(*d)))
                    .unwrap_or_else(|| env.limits.clamp(add_months(first, month_count(visible.start))));
                for i in visible.chain(pinned) {
                    let month = add_months(first, month_count(i));
                    let top = origin.y + i as f32 * MONTH_ITEM_H;
                    let item = Rect::from_min_size(pos2(origin.x, top), vec2(list.width(), MONTH_ITEM_H));
                    month_subhead(ui, item, month, env);
                    let grid_top = item.bottom() - MONTH_H;
                    let grid = MonthGrid::new(month, week_start);
                    for (row, week) in grid.weeks().enumerate() {
                        for (col, d) in week.iter().copied().enumerate() {
                            if !grid.in_month(d) {
                                continue;
                            }
                            let cell = Rect::from_min_size(
                                pos2(col_x(item.left(), col, env.dir), grid_top + dp::CELL_SIZE * row as f32),
                                Vec2::splat(dp::CELL_SIZE),
                            );
                            let resp = self.day(&rt.ui(ui), env, cell, d, sel, focus::TabStop::at(d, stop));
                            if resp.has_focus() {
                                focused = Some(d);
                                kit::focus::lock_arrows(ui.ctx(), resp.id, kit::focus::Arrows::Both);
                            }
                            if resp.clicked() {
                                sel.pick(d);
                                picked = true;
                            }
                        }
                    }
                }
            })
        };
        v.offset = out.state.clamped_offset().y;
        if focused.is_some() {
            v.focus = focused;
        }
        let shown = add_months(first, month_count(shown_first));
        let title = env.loc.month_year(shown.year(), shown.month());
        semantic::emit(rt.ctx(), None, list, || {
            let mut n = SemanticNode::new(StableId::of(&self.salt, "grid"), Role::Grid, &title);
            n.state.disabled = self.disabled;
            n
        });
        picked
    }
}

// Compose VerticalMonthsList
fn month_subhead(ui: &Ui, item: Rect, month: Date, env: &DayEnv<'_>) {
    let color = env.t.role(m::RANGE_SELECTION_MONTH_SUBHEAD_COLOR);
    let g = parts::type_wrapped(
        ui.painter(),
        &env.loc.month_year(month.year(), month.month()),
        m::RANGE_SELECTION_MONTH_SUBHEAD_FONT,
        color,
        item.width() - dp::MONTH_SUBHEAD_START_PADDING,
        1,
    );
    let x = env.dir.start_x(
        item,
        Rangef::new(
            dp::MONTH_SUBHEAD_START_PADDING,
            dp::MONTH_SUBHEAD_START_PADDING + g.size().x,
        ),
    );
    ui.painter()
        .galley(pos2(x, item.top() + dp::MONTH_SUBHEAD_TOP_PADDING), g, color);
}

struct DayEnv<'a> {
    id: Id,
    loc: &'a dyn LocaleFormat,
    limits: Limits,
    today: Date,
    dir: Direction,
    t: &'a Tokens,
}

fn col_x(left: f32, col: usize, dir: Direction) -> f32 {
    left + dp::CELL_SIZE * dir.slot(col, dp::DAYS_IN_WEEK) as f32
}

fn key_target(ctx: &Context, cur: Date, limits: Limits, dir: Direction, week_start: Weekday) -> Date {
    kit::input::key_presses(ctx)
        .into_iter()
        .fold(cur, |target, (key, mods)| {
            date::on_key(target, key, mods, dir, week_start).map_or(target, |next| limits.clamp(next))
        })
}

fn month_offset(first: Date, d: Date) -> Option<usize> {
    let months = (i32::from(d.year()) - i32::from(first.year())) * 12 + i32::from(d.month()) - i32::from(first.month());
    usize::try_from(months).ok()
}

fn month_top(first: Date, d: Date) -> f32 {
    month_offset(first, d).unwrap_or_default() as f32 * MONTH_ITEM_H
}

fn month_count(i: usize) -> i32 {
    i32::try_from(i).unwrap_or(i32::MAX)
}

fn day_top(d: Date, week_start: Weekday) -> f32 {
    let grid = MonthGrid::new(d, week_start);
    let index = grid.days.iter().position(|x| *x == d).unwrap_or_default();
    MONTH_ITEM_H - MONTH_H + dp::CELL_SIZE * (index / dp::DAYS_IN_WEEK) as f32
}

#[expect(clippy::cast_possible_wrap, reason = "YearsInRow is 3")]
const YEARS_IN_ROW_I16: i16 = dp::YEARS_IN_ROW as i16;
const YEAR_PITCH: f32 = m::SELECTION_YEAR_CONTAINER_HEIGHT + dp::YEARS_VERTICAL_SPACING;
// Compose TextFieldDefaults.supportingTextPadding
const SUPPORTING_TOP_PADDING: f32 = 4.0;

// Compose DatePickerDialog
fn dialog_buttons_height(rt: &Rt) -> f32 {
    MIN_INTERACTIVE.points(rt.core()).max(baseline_button::CONTAINER_HEIGHT) + dp::DIALOG_BUTTONS_BOTTOM_PADDING
}

fn year_range_dates() -> (Date, Date) {
    (
        Date::constant(dp::YEAR_RANGE.0, 1, 1),
        Date::constant(dp::YEAR_RANGE.1, 12, 31),
    )
}

#[expect(clippy::cast_possible_truncation, reason = "dp paddings fit i8")]
fn margin(v: f32) -> i8 {
    v as i8
}

// Compose Day
fn paint_day(ui: &MineralUi<'_, Material>, t: &Tokens, p: &DayPaint, resp: &egui::Response) {
    let rt = ui.rt();
    let dim = |col: Color32| common::dim(col, t);
    let container = Rect::from_center_size(p.cell.center(), vec2(m::DATE_CONTAINER_WIDTH, m::DATE_CONTAINER_HEIGHT));
    let round = common::radius(
        container,
        m::DATE_CONTAINER_SHAPE.corner_radius(&t.shape, Direction::Ltr).nw,
    );
    let painter = ui.painter();
    if p.in_span && !(p.span_start && p.span_end) {
        // Compose `drawRangeBackground`: cell-wide rows, halved at the selection ends, state layer
        // high.
        let (mut x0, mut x1) = (p.cell.left(), p.cell.right());
        let ends = p.dir.sides(p.span_start, p.span_end);
        if ends.left {
            x0 = p.cell.center().x;
        }
        if ends.right {
            x1 = p.cell.center().x;
        }
        let strip = Rect::from_center_size(p.cell.center(), vec2(p.cell.width(), m::DATE_STATE_LAYER_HEIGHT));
        painter.rect_filled(
            Rect::from_x_y_ranges(x0..=x1, strip.y_range()),
            0,
            t.role(m::RANGE_SELECTION_ACTIVE_INDICATOR_CONTAINER_COLOR),
        );
    }
    let unselected = if p.in_span {
        if p.enabled {
            t.role(m::SELECTION_DATE_IN_RANGE_LABEL_TEXT_COLOR)
        } else {
            dim(t.role(m::DATE_UNSELECTED_LABEL_TEXT_COLOR))
        }
    } else if p.today && p.enabled {
        t.role(m::DATE_TODAY_LABEL_TEXT_COLOR)
    } else if p.enabled {
        t.role(m::DATE_UNSELECTED_LABEL_TEXT_COLOR)
    } else {
        dim(t.role(m::DATE_UNSELECTED_LABEL_TEXT_COLOR))
    };
    let (selected_container, selected_content) = if p.enabled {
        (
            t.role(m::DATE_SELECTED_CONTAINER_COLOR),
            t.role(m::DATE_SELECTED_LABEL_TEXT_COLOR),
        )
    } else {
        (
            dim(t.role(m::DATE_SELECTED_CONTAINER_COLOR)),
            dim(t.role(m::DATE_SELECTED_LABEL_TEXT_COLOR)),
        )
    };
    let k = if p.animate {
        egui_mineral::motion::spring(
            rt.core(),
            resp.id.with("selected"),
            f32::from(p.endpoint),
            motion::DEFAULT_EFFECTS,
        )
        .clamp(0.0, 1.0)
    } else if p.endpoint {
        1.0
    } else {
        0.0
    };
    if k > 0.0 {
        painter.rect_filled(container, round, common::with_alpha(selected_container, k));
    }
    let content = unselected.lerp_to_gamma(selected_content, k);
    indication::paint(
        ui,
        resp,
        container,
        round,
        content,
        Interaction::of(resp, Availability::enabled_if(p.enabled)),
        t,
    );
    if p.today && !p.endpoint {
        painter.rect_stroke(
            container,
            round,
            Stroke::new(
                m::DATE_TODAY_CONTAINER_OUTLINE_WIDTH,
                t.role(m::DATE_TODAY_CONTAINER_OUTLINE_COLOR),
            ),
            StrokeKind::Inside,
        );
    }
    kit::input::hover_cursor(rt.core(), resp, Availability::enabled_if(p.enabled));
    let g = parts::type_galley(rt.core(), painter, &p.text, m::DATE_LABEL_TEXT_FONT, content, None);
    painter.galley(container.center() - g.size() / 2.0, g, content);
}

struct DayPaint {
    cell: Rect,
    text: String,
    enabled: bool,
    endpoint: bool,
    animate: bool,
    in_span: bool,
    span_start: bool,
    span_end: bool,
    today: bool,
    dir: Direction,
}

fn format_date(loc: &dyn LocaleFormat, d: Option<Date>) -> String {
    d.map_or_else(String::new, |d| {
        loc.date(d, DateFields::YearMonthDay, DateLength::Numeric)
    })
}

fn input_hint(rt: &Rt, loc: &dyn LocaleFormat) -> String {
    let probe = Date::constant(2033, 11, 22);
    loc.ascii_digits(&loc.date(probe, DateFields::YearMonthDay, DateLength::Numeric))
        .replace("2033", &tr(rt.i18n(), "mineral-date-hint-year"))
        .replace("11", &tr(rt.i18n(), "mineral-date-hint-month"))
        .replace("22", &tr(rt.i18n(), "mineral-date-hint-day"))
}

// Compose DatePickerHeadline, yMMMd
fn headline_single(rt: &Rt, loc: &dyn LocaleFormat, d: Option<Date>, input: bool) -> String {
    match d {
        Some(d) => loc.date(d, DateFields::YearMonthDay, DateLength::Medium),
        None if input => tr(rt.i18n(), "mineral-date-input-headline"),
        None => tr(rt.i18n(), "mineral-date-picker-headline"),
    }
}

// Compose DateRangePickerHeadline
fn headline_range(rt: &Rt, loc: &dyn LocaleFormat, r: &DateRange) -> Vec<String> {
    let end = |d: Option<Date>, id: &str| {
        d.map_or_else(
            || tr(rt.i18n(), id),
            |d| loc.date(d, DateFields::YearMonthDay, DateLength::Medium),
        )
    };
    vec![
        end(r.start(), "mineral-date-start"),
        dp::RANGE_HEADLINE_DELIMITER.to_owned(),
        end(r.end(), "mineral-date-end"),
    ]
}

fn day_index(d: Date) -> usize {
    usize::try_from(d.day() - 1).unwrap_or_default()
}
