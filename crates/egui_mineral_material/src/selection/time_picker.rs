use crate::Material;
use crate::MaterialExt;
use crate::MaterialUiExt as _;
use crate::Rt;
use crate::msg;
use egui::{
    Align2, Color32, Context, CornerRadius, Frame, Id, Margin, Pos2, Rangef, Rect, Sense, Stroke, StrokeKind, TextEdit,
    Ui, UiBuilder, Vec2, pos2, vec2,
};
use egui_mineral::kit::state::TempState;
use egui_mineral::semantic::KeyedSalt;
use egui_mineral::widget::Availability;
use egui_mineral::widget::MineralUi;
use std::{f32::consts::TAU, sync::Arc};

use egui_mineral::{
    date::{Time, from_12h, to_12h},
    i18n::{DayPeriod, Direction, HourCycle, LocaleFormat, PeriodPosition},
    kit,
    layout::Axis,
    range::{RangeModel, Track},
    semantic::{self, NumericRange, Role, SemanticNode, StableId},
};

use crate::containment::dialog;
use crate::selection::{PickerModalResponse, PickerResponse};
use crate::widget::{
    common::{self, Interaction},
    indication, parts,
};
use crate::{
    actions::{IconButton, button},
    selection::date_picker::DisplayMode,
};
use crate::{
    motion,
    spec::{
        ColorRole, TypeScale,
        defaults::time_picker as ct,
        tokens::{time_input as ti, time_picker as tp},
    },
    tokens::Tokens,
    tr,
};

const DIAL: f32 = tp::CLOCK_DIAL_CONTAINER_SIZE;
const NUMBERS_W: f32 = 2.0 * tp::TIME_SELECTOR_CONTAINER_WIDTH + ct::SEPARATOR_WIDTH;
const SUPPORT_H: f32 =
    ct::SUPPORT_TOP_PADDING + ct::SUPPORT_MIN_LINES as f32 * ti::TIME_FIELD_SUPPORTING_TEXT_FONT.line_height();

#[derive(Clone, Copy, Default, PartialEq, Eq)]
enum Field {
    #[default]
    Hour,
    Minute,
}

impl Field {
    fn slug(self) -> &'static str {
        match self {
            Self::Hour => "hour",
            Self::Minute => "minute",
        }
    }
}

impl TempState for State {}

#[derive(Clone, Default)]
struct State {
    keyboard: bool,
    field: Field,
    hour_buf: String,
    minute_buf: String,
    angle: Option<f32>,
}

/// MD3 time picker builder over a jiff civil [`Time`] (seconds are preserved). Clock, day period
/// names, digits and labels follow the active locale (`egui_mineral::i18n`).
#[derive(Debug)]
#[must_use = "call .show(..)"]
pub struct TimePicker<'a> {
    salt: KeyedSalt,
    label: String,
    time: &'a mut Time,
    mode: DisplayMode,
    hour_cycle: Option<HourCycle>,
    axis: Axis,
    labels: dialog::PickerLabels,
    hour_label: Option<String>,
    minute_label: Option<String>,
    am_label: Option<String>,
    pm_label: Option<String>,
    disabled: bool,
}

impl<'a> TimePicker<'a> {
    /// `label` is the accessible name of the dialog, `time` the bound value. Agent key prefix `<id_salt>`.
    pub fn new(id_salt: impl Into<KeyedSalt>, label: impl Into<String>, time: &'a mut Time) -> Self {
        let id_salt: KeyedSalt = id_salt.into();
        Self {
            salt: id_salt,
            label: label.into(),
            time,
            mode: DisplayMode::Picker,
            hour_cycle: None,
            axis: Axis::Vertical,
            labels: dialog::PickerLabels::default(),
            hour_label: None,
            minute_label: None,
            am_label: None,
            pm_label: None,
            disabled: false,
        }
    }

    crate::widget::setters! {
        /// Controls ignore input; the modal does not open.
        disabled
    }

    crate::widget::setters! {
        /// Replaces the `<id_salt>` key prefix.
        agent_key(salt)
    }

    /// [`show`](Self::show) and the initial mode of [`show_modal`](Self::show_modal).
    pub fn input_mode(mut self, input_mode: DisplayMode) -> Self {
        self.mode = input_mode;
        self
    }

    /// Defaults to the locale's cycle.
    pub fn hour_cycle(mut self, hour_cycle: HourCycle) -> Self {
        self.hour_cycle = Some(hour_cycle);
        self
    }

    /// Compose `TimePickerLayoutType`: [`Axis::Horizontal`] puts the clock display beside the dial;
    /// default [`Axis::Vertical`].
    pub fn axis(mut self, axis: Axis) -> Self {
        self.axis = axis;
        self
    }

    dialog::picker_label_setters!(labels);

    /// Hour field name; default the `mineral-time-hour` message.
    pub fn hour_label(mut self, hour_label: impl Into<String>) -> Self {
        self.hour_label = Some(hour_label.into());
        self
    }

    /// Minute field name; default the `mineral-time-minute` message.
    pub fn minute_label(mut self, minute_label: impl Into<String>) -> Self {
        self.minute_label = Some(minute_label.into());
        self
    }

    /// AM toggle text; default the locale day period.
    pub fn am_label(mut self, am_label: impl Into<String>) -> Self {
        self.am_label = Some(am_label.into());
        self
    }

    /// PM toggle text; default the locale day period.
    pub fn pm_label(mut self, pm_label: impl Into<String>) -> Self {
        self.pm_label = Some(pm_label.into());
        self
    }

    fn resolve(&self, rt: &Rt) -> Resolved {
        let loc = rt.i18n().format();
        let text = |over: &Option<String>, id: crate::Msg| crate::tr_or(rt.i18n(), over.as_deref(), id);
        let period_labels = [
            self.am_label.clone().unwrap_or_else(|| loc.day_period(DayPeriod::Am)),
            self.pm_label.clone().unwrap_or_else(|| loc.day_period(DayPeriod::Pm)),
        ];
        Resolved {
            salt: self.salt.clone(),
            label: self.label.clone(),
            hour24: self.hour_cycle.unwrap_or_else(|| loc.hour_cycle()) == HourCycle::H23,
            horizontal: self.axis == Axis::Horizontal,
            headlines: [
                self.labels.headline(rt.i18n(), msg::TIME_HEADLINE),
                self.labels.headline(rt.i18n(), msg::TIME_INPUT_HEADLINE),
            ],
            choice: self.labels.choice(rt.i18n()),
            hour_label: text(&self.hour_label, msg::TIME_HOUR),
            minute_label: text(&self.minute_label, msg::TIME_MINUTE),
            period_labels,
            separator: loc.time_separator(),
            period_first: loc.period_position() == PeriodPosition::BeforeTime,
            disabled: self.disabled,
            loc,
        }
    }

    /// [`DisplayMode::Picker`]: Compose `TimePicker`, the clock display and the dial.
    /// [`DisplayMode::Input`]: Compose `TimeInput`, hour and minute text fields with the period
    /// toggle.
    pub fn show(self, ui: &mut MineralUi<'_, Material>) -> PickerResponse {
        let r = self.resolve(ui.rt());
        let mode = match self.mode {
            DisplayMode::Picker => r.picker_mode(),
            DisplayMode::Input => Mode::Input,
        };
        r.show_inline(ui, self.time, mode)
    }

    /// Compose `TimePickerDialog`: title, `TimePicker` or `TimeInput`, the mode toggle and
    /// Cancel/OK; the time is written on OK.
    pub fn show_modal(self, rt: &Rt, open: &mut bool) -> PickerModalResponse {
        self.resolve(rt)
            .show_modal(rt, open, self.time, self.mode == DisplayMode::Input)
    }
}

struct Resolved {
    salt: KeyedSalt,
    label: String,
    hour24: bool,
    horizontal: bool,
    headlines: [String; 2],
    choice: dialog::ChoiceLabels,
    hour_label: String,
    minute_label: String,
    period_labels: [String; 2],
    separator: String,
    period_first: bool,
    disabled: bool,
    loc: Arc<dyn LocaleFormat>,
}

// Compose VerticalTimePicker, HorizontalTimePicker, TimeInput
#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    Vertical,
    Horizontal,
    Input,
}

impl Resolved {
    fn state(ctx: &Context, id: Id) -> State {
        State::load(ctx, id)
    }

    fn picker_mode(&self) -> Mode {
        if self.horizontal {
            Mode::Horizontal
        } else {
            Mode::Vertical
        }
    }

    fn dialog_mode(&self, st: &State) -> Mode {
        if st.keyboard { Mode::Input } else { self.picker_mode() }
    }

    fn period_slot(&self, width: f32) -> f32 {
        if self.hour24 {
            0.0
        } else {
            ct::PERIOD_TOGGLE_GAP + width
        }
    }

    fn content_size(&self, mode: Mode) -> Vec2 {
        match mode {
            Mode::Vertical => vec2(
                (NUMBERS_W + self.period_slot(tp::PERIOD_SELECTOR_VERTICAL_CONTAINER_WIDTH)).max(DIAL),
                tp::TIME_SELECTOR_CONTAINER_HEIGHT + ct::CLOCK_DISPLAY_MARGIN + DIAL + ct::CLOCK_FACE_BOTTOM_MARGIN,
            ),
            Mode::Horizontal => {
                let period = if self.hour24 {
                    0.0
                } else {
                    ct::PERIOD_TOGGLE_TOP_PADDING + tp::PERIOD_SELECTOR_HORIZONTAL_CONTAINER_HEIGHT
                };
                vec2(
                    NUMBERS_W + ct::CLOCK_DISPLAY_MARGIN + DIAL,
                    (tp::TIME_SELECTOR_CONTAINER_HEIGHT + period).max(DIAL),
                )
            }
            Mode::Input => vec2(
                2.0 * ti::TIME_FIELD_CONTAINER_WIDTH
                    + ct::SEPARATOR_WIDTH
                    + self.period_slot(ti::PERIOD_SELECTOR_CONTAINER_WIDTH),
                ti::TIME_FIELD_CONTAINER_HEIGHT + SUPPORT_H,
            ),
        }
    }

    fn show_inline(&self, ui: &mut MineralUi<'_, Material>, time: &mut Time, mode: Mode) -> PickerResponse {
        let rt = ui.rt();
        let id = ui.make_persistent_id(self.salt.id_salt());
        let out = ui.scope(|ui| {
            ui.spacing_mut().item_spacing = Vec2::ZERO;
            let (area, _) = ui.allocate_exact_size(self.content_size(mode), Sense::hover());
            self.content(&mut rt.ui(ui), id, area, mode, time)
        });
        let mut response = out.response;
        if out.inner {
            response.mark_changed();
        }
        PickerResponse {
            response,
            changed: out.inner,
        }
    }

    // Compose TimePickerDialog, TimePickerCustomLayout
    fn show_modal(&self, rt: &Rt, open: &mut bool, time: &mut Time, input: bool) -> PickerModalResponse {
        let id = egui_mineral::kit::scoped_id(rt.core(), Id::unique(("mineral::time_modal", self.salt.id_salt())));
        let state_id = id.with("state");
        let t = rt.tokens();
        let body = |ui: &mut MineralUi<'_, Material>, work: &mut Time| {
            let ctx = ui.ctx();
            if State::try_load(ctx, state_id).is_none() {
                State {
                    keyboard: input,
                    ..State::default()
                }
                .store(ctx, state_id);
            }
            self.modal_body(ui, state_id, &t, work)
        };
        dialog::PickerModal {
            id,
            salt: &self.salt,
            disabled: self.disabled,
            initial: *time,
        }
        .show(rt, open, time, |work| *work, body, |d| d.remove::<State>(state_id))
    }

    fn modal_body(
        &self,
        ui: &mut MineralUi<'_, Material>,
        state_id: Id,
        t: &Tokens,
        work: &mut Time,
    ) -> (Option<dialog::Choice>, &str) {
        let rt = ui.rt();
        let ctx = &ui.ctx().clone();
        ui.spacing_mut().item_spacing = Vec2::ZERO;
        let mode = self.dialog_mode(&Self::state(ctx, state_id));
        let content = self.content_size(mode);
        let toggle = self.mode_toggle(rt, state_id);
        let actions_h = toggle.footprint(ui.density()).y;
        let pad = ct::DIALOG_PADDING;
        let headline = &self.headlines[usize::from(mode == Mode::Input)];
        let title = parts::type_wrapped(
            ui.painter(),
            headline,
            ct::DIALOG_TITLE_FONT,
            t.role(ct::DIALOG_TITLE_COLOR),
            content.x,
            1,
        );
        let landscape = mode == Mode::Horizontal;
        let (content_top, actions_gap, bottom) = if landscape {
            (
                ct::DIALOG_LANDSCAPE_CONTENT_TOP_PADDING,
                ct::DIALOG_LANDSCAPE_CONTENT_ACTIONS_PADDING,
                ct::DIALOG_LANDSCAPE_ACTIONS_BOTTOM_PADDING,
            )
        } else {
            (pad + title.size().y + ct::DIALOG_TITLE_BOTTOM_PADDING, 0.0, pad)
        };
        let size = vec2(
            content.x + 2.0 * pad,
            content_top + content.y + actions_gap + actions_h + bottom,
        );
        let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
        let title_x = rt
            .i18n()
            .direction()
            .start_x(rect, Rangef::new(pad, pad + title.size().x));
        ui.painter()
            .galley(pos2(title_x, rect.top() + pad), title, Color32::PLACEHOLDER);
        let area = Rect::from_min_size(rect.min + vec2(pad, content_top), content);
        self.content(ui, state_id, area, mode, work);
        let row = Rect::from_min_size(
            pos2(area.left(), area.bottom() + actions_gap),
            vec2(content.x, actions_h),
        );
        Self::place_toggle(ui, state_id, row, toggle);
        let actions = &mut ui.new_child(UiBuilder::new().id_salt("actions").max_rect(row));
        let action = dialog::confirm_cancel(&mut rt.ui(actions), &self.salt, &self.choice, true);
        (action, &self.label)
    }

    // Compose TimePickerDialogDefaults.DisplayModeToggle
    fn mode_toggle(&self, rt: &Rt, id: Id) -> IconButton<'static> {
        let ctx = rt.ctx();
        let (icon, msg) = if Self::state(ctx, id).keyboard {
            (rt.icons().schedule, msg::TIME_TO_DIAL)
        } else {
            (rt.icons().keyboard, msg::TIME_TO_INPUT)
        };
        IconButton::new(self.salt.child("mode"), icon, tr(rt.i18n(), msg)).disabled(self.disabled)
    }

    fn place_toggle(ui: &mut MineralUi<'_, Material>, id: Id, row: Rect, toggle: IconButton) {
        let rt = ui.rt();
        let size = toggle.footprint(ui.density());
        let x = rt.i18n().direction().start_x(row, Rangef::new(0.0, 0.0 + size.x));
        let slot = Rect::from_min_size(pos2(x, row.center().y - size.y / 2.0), size);
        if rt
            .ui(&mut ui.new_child(UiBuilder::new().id_salt("mode").max_rect(slot)))
            .mineral(toggle)
            .clicked()
        {
            let mut st = Self::state(ui.ctx(), id);
            st.keyboard = !st.keyboard;
            st.hour_buf.clear();
            st.minute_buf.clear();
            st.store(ui.ctx(), id);
        }
    }

    fn content(&self, ui: &mut MineralUi<'_, Material>, id: Id, area: Rect, mode: Mode, time: &mut Time) -> bool {
        let rt = ui.rt();
        let ctx = rt.ctx();
        let t = rt.tokens();
        let mut st = Self::state(ctx, id);
        let before = *time;
        match mode {
            Mode::Vertical => self.vertical(ui, id, area, &mut st, time, &t),
            Mode::Horizontal => self.horizontal(ui, id, area, &mut st, time, &t),
            Mode::Input => self.input(ui, id, area, &mut st, time, &t),
        }
        st.store(ctx, id);
        *time != before
    }

    fn period_left(&self, rt: &Rt) -> bool {
        match rt.i18n().direction() {
            Direction::Ltr => self.period_first,
            Direction::Rtl => !self.period_first,
        }
    }

    // Compose VerticalClockDisplay, ClockFace
    fn vertical(
        &self,
        ui: &mut MineralUi<'_, Material>,
        id: Id,
        area: Rect,
        st: &mut State,
        time: &mut Time,
        t: &Tokens,
    ) {
        let period_w = self.period_slot(tp::PERIOD_SELECTOR_VERTICAL_CONTAINER_WIDTH);
        let width = NUMBERS_W + period_w;
        let left = area.center().x - width / 2.0;
        let h = tp::TIME_SELECTOR_CONTAINER_HEIGHT;
        let (numbers_x, period_x) = if self.period_left(ui.rt()) {
            (left + period_w, left)
        } else {
            (left, left + NUMBERS_W + ct::PERIOD_TOGGLE_GAP)
        };
        self.numbers(ui, id, pos2(numbers_x, area.top()), st, *time, t);
        if !self.hour24 {
            let rect = Rect::from_min_size(
                pos2(period_x, area.top()),
                vec2(tp::PERIOD_SELECTOR_VERTICAL_CONTAINER_WIDTH, h),
            );
            self.period_toggle(ui, id, rect, time, false, t);
        }
        let dial = Rect::from_min_size(
            pos2(area.center().x - DIAL / 2.0, area.top() + h + ct::CLOCK_DISPLAY_MARGIN),
            Vec2::splat(DIAL),
        );
        self.dial(ui, id, dial, st, time, t);
    }

    // Compose HorizontalClockDisplay, ClockFace
    fn horizontal(
        &self,
        ui: &mut MineralUi<'_, Material>,
        id: Id,
        area: Rect,
        st: &mut State,
        time: &mut Time,
        t: &Tokens,
    ) {
        let rt = ui.rt();
        let dir = rt.i18n().direction();
        let (col_x, dial_x) = (
            dir.start_x(area, Rangef::new(0.0, 0.0 + NUMBERS_W)),
            dir.reversed().start_x(area, Rangef::new(0.0, 0.0 + DIAL)),
        );
        let period_h = if self.hour24 {
            0.0
        } else {
            ct::PERIOD_TOGGLE_TOP_PADDING + tp::PERIOD_SELECTOR_HORIZONTAL_CONTAINER_HEIGHT
        };
        let height = tp::TIME_SELECTOR_CONTAINER_HEIGHT + period_h;
        let top = area.center().y - height / 2.0;
        self.numbers(ui, id, pos2(col_x, top), st, *time, t);
        if !self.hour24 {
            let rect = Rect::from_min_size(
                pos2(
                    col_x,
                    top + tp::TIME_SELECTOR_CONTAINER_HEIGHT + ct::PERIOD_TOGGLE_TOP_PADDING,
                ),
                vec2(
                    tp::PERIOD_SELECTOR_HORIZONTAL_CONTAINER_WIDTH,
                    tp::PERIOD_SELECTOR_HORIZONTAL_CONTAINER_HEIGHT,
                ),
            );
            self.period_toggle(ui, id, rect, time, true, t);
        }
        let dial = Rect::from_min_size(pos2(dial_x, area.center().y - DIAL / 2.0), Vec2::splat(DIAL));
        self.dial(ui, id, dial, st, time, t);
    }

    // Compose ClockDisplayNumbers
    fn numbers(&self, ui: &mut MineralUi<'_, Material>, id: Id, at: Pos2, st: &mut State, time: Time, t: &Tokens) {
        let size = vec2(tp::TIME_SELECTOR_CONTAINER_WIDTH, tp::TIME_SELECTOR_CONTAINER_HEIGHT);
        let hour = Rect::from_min_size(at, size);
        let minute = hour.translate(vec2(size.x + ct::SEPARATOR_WIDTH, 0.0));
        let (h12, _) = to_12h(time.hour());
        let shown_hour = if self.hour24 { time.hour() } else { h12 };
        for (field, rect, value, name) in [
            (Field::Hour, hour, shown_hour, &self.hour_label),
            (Field::Minute, minute, time.minute(), &self.minute_label),
        ] {
            let text = self.loc.native_digits(&format!("{value:02}"));
            let selector = Selector {
                name,
                salt: &self.salt,
                slug: field.slug(),
                disabled: self.disabled,
                selected: st.field == field,
                value,
                text: &text,
            };
            if selector_box(ui, id, rect, selector, t) {
                st.field = field;
            }
        }
        let sep = Rect::from_min_size(hour.right_top(), vec2(ct::SEPARATOR_WIDTH, size.y));
        self.separator(
            ui,
            sep,
            tp::TIME_SELECTOR_SEPARATOR_FONT,
            tp::TIME_SELECTOR_SEPARATOR_COLOR,
            t,
        );
    }

    // Compose DisplaySeparator
    fn separator(&self, ui: &MineralUi<'_, Material>, rect: Rect, font: TypeScale, color: ColorRole, t: &Tokens) {
        let rt = ui.rt();
        let color = t.role(color);
        let g = parts::type_galley(rt.core(), ui.painter(), &self.separator, font, color, None);
        let at = rect.center() - g.size() / 2.0 + vec2(0.0, ct::SEPARATOR_OFFSET_Y);
        ui.painter().galley(at, g, color);
    }

    // Compose TimeInputImpl
    fn input(&self, ui: &mut MineralUi<'_, Material>, id: Id, area: Rect, st: &mut State, time: &mut Time, t: &Tokens) {
        let fields_w = 2.0 * ti::TIME_FIELD_CONTAINER_WIDTH + ct::SEPARATOR_WIDTH;
        let period_w = self.period_slot(ti::PERIOD_SELECTOR_CONTAINER_WIDTH);
        let (fields_x, period_x) = if self.period_left(ui.rt()) {
            (area.left() + period_w, area.left())
        } else {
            (area.left(), area.left() + fields_w + ct::PERIOD_TOGGLE_GAP)
        };
        let size = vec2(ti::TIME_FIELD_CONTAINER_WIDTH, ti::TIME_FIELD_CONTAINER_HEIGHT);
        let hour = Rect::from_min_size(pos2(fields_x, area.top()), size);
        let minute = hour.translate(vec2(size.x + ct::SEPARATOR_WIDTH, 0.0));
        self.time_field(
            ui,
            &FieldSlot {
                id,
                rect: hour,
                field: Field::Hour,
            },
            st,
            time,
            t,
        );
        self.time_field(
            ui,
            &FieldSlot {
                id,
                rect: minute,
                field: Field::Minute,
            },
            st,
            time,
            t,
        );
        let sep = Rect::from_min_size(
            hour.right_top(),
            vec2(ct::SEPARATOR_WIDTH, ti::PERIOD_SELECTOR_CONTAINER_HEIGHT),
        );
        self.separator(
            ui,
            sep,
            ti::TIME_FIELD_LABEL_TEXT_FONT,
            ti::TIME_FIELD_SEPARATOR_COLOR,
            t,
        );
        if !self.hour24 {
            let rect = Rect::from_min_size(
                pos2(period_x, area.top()),
                vec2(
                    ti::PERIOD_SELECTOR_CONTAINER_WIDTH,
                    ti::PERIOD_SELECTOR_CONTAINER_HEIGHT,
                ),
            );
            self.period_toggle(ui, id, rect, time, false, t);
        }
    }

    // Compose TimePickerTextField, SupportingText
    fn time_field(
        &self,
        ui: &mut MineralUi<'_, Material>,
        slot: &FieldSlot,
        st: &mut State,
        time: &mut Time,
        t: &Tokens,
    ) {
        let rt = ui.rt();
        let FieldSlot { id, rect, field } = *slot;
        let ctx = ui.ctx().clone();
        let loc = self.loc.as_ref();
        let (h12, _) = to_12h(time.hour());
        let (lo, hi, shown, name) = match field {
            Field::Hour if self.hour24 => (0, 23, time.hour(), &self.hour_label),
            Field::Hour => (1, 12, h12, &self.hour_label),
            Field::Minute => (0, 59, time.minute(), &self.minute_label),
        };
        let edit_id = id.with(("field", field.slug()));
        let focused = ctx.memory(|m| m.has_focus(edit_id));
        if !focused {
            let digits = two_digits(loc, shown);
            let buf = field_buf(st, field);
            if !buf.chars().eq(digits) {
                buf.clear();
                buf.extend(digits);
            }
        }
        let valid = {
            let buf = field_buf(st, field);
            buf.is_empty() || parse_digits(loc, lo..=hi, buf).is_some()
        };
        let view = FieldView {
            lo,
            hi,
            valid,
            focused,
            selected: st.field == field,
        };
        let content = self.paint_field_container(ui, rect, &view, t);
        let te = {
            let ui = &mut ui.new_child(UiBuilder::new().id_salt(field.slug()).max_rect(rect));
            ui.visuals_mut().text_cursor.stroke.color = t.role(ct::FIELD_CURSOR_COLOR);
            ui.add_sized(
                rect.size(),
                TextEdit::singleline(field_buf(st, field))
                    .id(edit_id)
                    .frame(Frame::NONE)
                    .margin(Margin::ZERO)
                    .font(ti::TIME_FIELD_LABEL_TEXT_FONT.font(&ctx))
                    .text_color(content)
                    .align(Align2::CENTER_CENTER)
                    .char_limit(2)
                    .interactive(!self.disabled),
            )
        };
        if te.gained_focus() {
            st.field = field;
        }
        // Compose `ImeAction.Next` on the hour field.
        if field == Field::Hour && crate::text_inputs::text_field::submitted(&te) {
            st.field = Field::Minute;
            ctx.memory_mut(|m| m.request_focus(id.with(("field", Field::Minute.slug()))));
        }
        self.paint_field_notes(&rt.ui(ui), rect, &view, name, t);
        if te.changed() {
            self.apply_field_edit(&ctx, slot, st, time, &view);
        }
        let value = field_buf(st, field).as_str();
        semantic::emit(&ctx, Some(te.id), rect, || {
            let mut n = SemanticNode::new(StableId::of(&self.salt, (field.slug(), "field")), Role::TextInput, name)
                .value(value.to_owned());
            n.state.invalid = !view.valid;
            n.state.selected = Some(view.selected);
            n.editable(Availability::disabled_if(self.disabled))
        });
    }

    fn paint_field_container(&self, ui: &Ui, rect: Rect, view: &FieldView, t: &Tokens) -> Color32 {
        let container = if !view.valid {
            t.role(ct::FIELD_ERROR_CONTAINER_COLOR)
        } else if view.focused {
            t.role(tp::TIME_SELECTOR_SELECTED_CONTAINER_COLOR)
        } else {
            t.role(tp::TIME_SELECTOR_UNSELECTED_CONTAINER_COLOR)
        };
        let r = ti::TIME_FIELD_CONTAINER_SHAPE.corner_radius(&t.shape, Direction::Ltr);
        ui.painter().rect_filled(rect, r, container);
        if self.disabled {
            common::disabled_content(t)
        } else if !view.valid {
            t.role(ct::FIELD_ERROR_COLOR)
        } else if view.focused {
            t.role(tp::TIME_SELECTOR_SELECTED_LABEL_TEXT_COLOR)
        } else {
            t.role(tp::TIME_SELECTOR_UNSELECTED_LABEL_TEXT_COLOR)
        }
    }

    fn paint_field_notes(&self, ui: &MineralUi<'_, Material>, rect: Rect, view: &FieldView, name: &str, t: &Tokens) {
        let rt = ui.rt();
        let loc = self.loc.as_ref();
        let r = ti::TIME_FIELD_CONTAINER_SHAPE.corner_radius(&t.shape, Direction::Ltr);
        if view.selected || !view.valid {
            let (width, color) = match (view.valid, view.focused) {
                (false, _) => (ct::FIELD_FOCUS_OUTLINE_WIDTH, t.role(ct::FIELD_ERROR_COLOR)),
                (true, true) => (ct::FIELD_FOCUS_OUTLINE_WIDTH, t.role(ct::FIELD_OUTLINE_COLOR)),
                (true, false) => (ct::FIELD_OUTLINE_WIDTH, t.role(ct::FIELD_OUTLINE_COLOR)),
            };
            ui.painter()
                .rect_stroke(rect, r, Stroke::new(width, color), StrokeKind::Inside);
        }
        let support_color = if view.valid {
            t.role(ti::TIME_FIELD_SUPPORTING_TEXT_COLOR)
        } else {
            t.role(ct::FIELD_ERROR_COLOR)
        };
        let message = if view.valid {
            name.to_owned()
        } else {
            rt.i18n().tr(msg::RangeValue {
                start: loc.integer(view.lo.into()).into(),
                end: loc.integer(view.hi.into()).into(),
            })
        };
        let g = parts::type_wrapped(
            ui.painter(),
            &message,
            ti::TIME_FIELD_SUPPORTING_TEXT_FONT,
            support_color,
            rect.width(),
            ct::SUPPORT_MIN_LINES,
        );
        ui.painter().galley(
            pos2(rect.left(), rect.bottom() + ct::SUPPORT_TOP_PADDING),
            g,
            support_color,
        );
    }

    fn apply_field_edit(&self, ctx: &Context, slot: &FieldSlot, st: &mut State, time: &mut Time, view: &FieldView) {
        let (id, field) = (slot.id, slot.field);
        let loc = self.loc.as_ref();
        let (_, period) = to_12h(time.hour());
        let buf = field_buf(st, field);
        let mut ascii = loc.ascii_digits(buf);
        ascii.retain(|ch| ch.is_ascii_digit());
        ascii.truncate(2);
        *buf = loc.native_digits(&ascii);
        if let Some(v) = parse_digits(loc, view.lo..=view.hi, buf) {
            match field {
                Field::Hour => set(time, if self.hour24 { v } else { from_12h(v, period) }, time.minute()),
                Field::Minute => set(time, time.hour(), v),
            }
            // Compose `shouldSwitchFocusToMinute`: a complete valid hour moves on.
            if field == Field::Hour && ascii.len() == 2 {
                st.field = Field::Minute;
                ctx.memory_mut(|m| m.request_focus(id.with(("field", Field::Minute.slug()))));
            }
        }
    }

    // Compose PeriodToggleImpl
    fn period_toggle(
        &self,
        ui: &mut MineralUi<'_, Material>,
        id: Id,
        rect: Rect,
        time: &mut Time,
        horizontal: bool,
        t: &Tokens,
    ) {
        let rt = ui.rt();
        let (h12, _) = to_12h(time.hour());
        let gap = ct::PERIOD_TOGGLE_GAP;
        let item = if horizontal {
            vec2((rect.width() - gap) / 2.0, rect.height())
        } else {
            vec2(rect.width(), (rect.height() - gap) / 2.0)
        };
        let step = if horizontal {
            vec2(item.x + gap, 0.0)
        } else {
            vec2(0.0, item.y + gap)
        };
        for (i, p) in [DayPeriod::Am, DayPeriod::Pm].into_iter().enumerate() {
            let cell = Rect::from_min_size(rect.min + step * i as f32, item);
            let resp = kit::input::interact(
                &rt.ui(ui),
                cell,
                id.with(("period", p)),
                Sense::click(),
                Availability::disabled_if(self.disabled),
            );
            let checked = to_12h(time.hour()).1 == p;
            let i_state = Interaction::of(&resp, Availability::disabled_if(self.disabled));
            let target = if i_state.pressed || checked {
                ct::PERIOD_ITEM_CHECKED_RADIUS
            } else {
                item.min_elem() / 2.0
            };
            let r = CornerRadius::from(button::morph(ui, resp.id.with("shape"), target, true));
            let (container, content) = match (self.disabled, checked) {
                (true, _) => (common::disabled_container(t), common::disabled_content(t)),
                (false, true) => (
                    t.role(ct::PERIOD_SELECTED_CONTAINER_COLOR),
                    t.role(ct::PERIOD_SELECTED_CONTENT_COLOR),
                ),
                (false, false) => (
                    t.role(ct::PERIOD_CONTAINER_COLOR),
                    t.role(tp::PERIOD_SELECTOR_UNSELECTED_LABEL_TEXT_COLOR),
                ),
            };
            ui.painter().rect_filled(cell, r, container);
            indication::paint(ui, &resp, cell, r, content, i_state, t);
            kit::input::hover_cursor(ui.core(), &resp, Availability::disabled_if(self.disabled));
            let label = &self.period_labels[i];
            let g = parts::type_wrapped(
                ui.painter(),
                label,
                tp::PERIOD_SELECTOR_LABEL_TEXT_FONT,
                content,
                item.x - 2.0 * ct::PERIOD_ITEM_HORIZONTAL_PADDING,
                1,
            );
            ui.painter().galley(cell.center() - g.size() / 2.0, g, content);
            if resp.clicked() && !checked {
                set(time, from_12h(h12, p), time.minute());
            }
            semantic::emit(ui.ctx(), Some(resp.id), cell, || {
                let slug = match p {
                    DayPeriod::Am => "am",
                    DayPeriod::Pm => "pm",
                };
                let mut node = SemanticNode::new(StableId::of(&self.salt, (slug, "button")), Role::Button, label)
                    .clickable(Availability::disabled_if(self.disabled));
                node.state.selected = Some(checked);
                node
            });
        }
    }

    // Compose ClockFace, drawSelector
    fn dial(&self, ui: &mut MineralUi<'_, Material>, id: Id, area: Rect, st: &mut State, time: &mut Time, t: &Tokens) {
        let rt = ui.rt();
        let resp = ui.interact(
            area,
            id.with("dial"),
            if self.disabled {
                Sense::hover()
            } else {
                Sense::click_and_drag()
            },
        );
        let input = self.dial_input(rt, &resp, area, st, time);
        let cx = DialCx {
            id,
            area,
            resp: &resp,
            t,
            input: &input,
        };
        self.paint_dial(ui, &cx, st, *time);
        self.emit_dial(rt, &cx, st, *time);
    }

    fn dial_input(&self, rt: &Rt, resp: &egui::Response, area: Rect, st: &mut State, time: &mut Time) -> DialInput {
        let center = area.center();
        let pointer = resp
            .interact_pointer_pos()
            .filter(|_| !self.disabled && (resp.dragged() || resp.clicked()));
        let dragging = pointer.is_some() && resp.dragged();
        let hour_mode = st.field == Field::Hour;
        if let Some(p) = pointer {
            let angle = dial::angle(center, p);
            if hour_mode {
                let (_, period) = to_12h(time.hour());
                let slot = dial::slot(angle, 12);
                let inner = (p - center).length() < ct::INNER_RING_MAX_DISTANCE;
                let h = if self.hour24 {
                    slot + if inner { 12 } else { 0 }
                } else {
                    from_12h(if slot == 0 { 12 } else { slot }, period)
                };
                set(time, h, time.minute());
            } else {
                // Compose `onTap` snaps minutes to fives; drags pick single minutes.
                let m = if dragging {
                    dial::slot(angle, 60)
                } else {
                    dial::slot(angle, 12) * 5
                };
                set(time, time.hour(), m);
            }
        }
        if hour_mode && (resp.drag_stopped() || resp.clicked()) {
            st.field = Field::Minute;
        }
        // Keys and AccessKit apply after the pointer, to its result.
        let hour_mode = st.field == Field::Hour;
        let (h12, period) = to_12h(time.hour());

        let range = if hour_mode {
            if self.hour24 {
                RangeModel::new(0.0, 23.0).step(1.0)
            } else {
                RangeModel::new(1.0, 12.0).step(1.0)
            }
        } else {
            RangeModel::new(0.0, 59.0).step(1.0).jump(5.0)
        };
        let value = if hour_mode {
            f64::from(if self.hour24 { time.hour() } else { h12 })
        } else {
            f64::from(time.minute())
        };
        let next =
            (!self.disabled).then(|| range.drive(rt.core(), resp.id, value, Track::horizontal(rt.i18n().direction())));
        if let Some(v) = next.filter(|v| (v - value).abs() > f64::EPSILON) {
            if hour_mode {
                let v = v as i8;
                set(time, if self.hour24 { v } else { from_12h(v, period) }, time.minute());
            } else {
                set(time, time.hour(), v as i8);
            }
        }
        DialInput {
            pointer,
            dragging,
            range,
        }
    }

    fn paint_dial(&self, ui: &MineralUi<'_, Material>, cx: &DialCx<'_>, st: &mut State, time: Time) {
        let rt = ui.rt();
        let ctx = ui.ctx();
        let (id, area, t) = (cx.id, cx.area, cx.t);
        let center = area.center();
        let dragging = cx.input.dragging;
        let hour_mode = st.field == Field::Hour;
        let fraction = if hour_mode {
            f32::from(time.hour() % 12) / 12.0
        } else {
            f32::from(time.minute()) / 60.0
        };
        let raw = match cx.input.pointer.filter(|_| dragging) {
            Some(p) => dial::angle(center, p),
            None => fraction * TAU,
        };
        let target = st.angle.map_or(raw, |prev| raw + TAU * ((prev - raw) / TAU).round());
        st.angle = Some(target);
        let angle_id = id.with("dial-angle");
        if dragging {
            egui_mineral::motion::snap(ctx, angle_id, target);
        }
        let shown = egui_mineral::motion::spring(rt.core(), angle_id, target, motion::DEFAULT_SPATIAL);
        // `AnalogTimePickerState.selectorPos`: 24-hour afternoon hours sit on the inner ring.
        let radius = if hour_mode && self.hour24 && time.hour() >= 12 {
            ct::INNER_RADIUS
        } else {
            ct::OUTER_RADIUS
        };
        let handle_r = tp::CLOCK_DIAL_SELECTOR_HANDLE_CONTAINER_SIZE / 2.0;
        let handle = dial::pos(center, shown, radius);
        let selector = t.role(tp::CLOCK_DIAL_SELECTOR_HANDLE_CONTAINER_COLOR);
        let p = ui.painter();
        p.circle_filled(center, DIAL / 2.0, t.role(tp::CLOCK_DIAL_COLOR));
        p.circle_filled(handle, handle_r, selector);
        p.line_segment(
            [center, dial::pos(center, shown, radius - handle_r)],
            Stroke::new(
                tp::CLOCK_DIAL_SELECTOR_TRACK_CONTAINER_WIDTH,
                t.role(tp::CLOCK_DIAL_SELECTOR_TRACK_CONTAINER_COLOR),
            ),
        );
        p.circle_filled(
            center,
            tp::CLOCK_DIAL_SELECTOR_CENTER_CONTAINER_SIZE / 2.0,
            t.role(tp::CLOCK_DIAL_SELECTOR_CENTER_CONTAINER_COLOR),
        );
        // Compose crossfades the hour and minute faces.
        let face = egui_mineral::motion::spring(
            rt.core(),
            id.with("dial-face"),
            f32::from(hour_mode),
            motion::DEFAULT_EFFECTS,
        )
        .clamp(0.0, 1.0);
        let unselected = t.role(tp::CLOCK_DIAL_UNSELECTED_LABEL_TEXT_COLOR);
        let selected = t.role(tp::CLOCK_DIAL_SELECTED_LABEL_TEXT_COLOR);
        let label = |at: Pos2, value: usize, alpha: f32| {
            // Compose clears the handle out of the labels and redraws their overlap in the selected color.
            let on_handle = (at - handle).length() < 1.0;
            let color = common::with_alpha(if on_handle { selected } else { unselected }, alpha);
            let g = parts::type_galley(
                rt.core(),
                p,
                &self.loc.count(value),
                tp::CLOCK_DIAL_LABEL_TEXT_FONT,
                color,
                None,
            );
            p.galley(at - g.size() / 2.0, g, color);
        };
        if face > 0.0 {
            for slot in 0..12 {
                let a = slot as f32 / 12.0 * TAU;
                // Compose `Hours` (12, 1..11); a 24-hour dial shows 0..11 outside and 12..23 inside.
                let outer = if !self.hour24 && slot == 0 { 12 } else { slot };
                label(dial::pos(center, a, ct::OUTER_RADIUS), outer, face);
                if self.hour24 {
                    label(dial::pos(center, a, ct::INNER_RADIUS), slot + 12, face);
                }
            }
        }
        if face < 1.0 {
            for slot in 0..12 {
                label(
                    dial::pos(center, slot as f32 / 12.0 * TAU, ct::OUTER_RADIUS),
                    slot * 5,
                    1.0 - face,
                );
            }
        }
        if Interaction::of(cx.resp, Availability::disabled_if(self.disabled)).focused {
            common::focus_ring(p, area, CornerRadius::from(DIAL / 2.0), t);
        }
        kit::input::hover_cursor(rt.core(), cx.resp, Availability::disabled_if(self.disabled));
    }

    fn emit_dial(&self, rt: &Rt, cx: &DialCx<'_>, st: &State, time: Time) {
        let ctx = rt.ctx();
        let hour_mode = st.field == Field::Hour;
        let (h12, _) = to_12h(time.hour());
        let range = cx.input.range;
        let (name, field) = if hour_mode {
            (&self.hour_label, Field::Hour)
        } else {
            (&self.minute_label, Field::Minute)
        };
        let cur = if hour_mode {
            f64::from(if self.hour24 { time.hour() } else { h12 })
        } else {
            f64::from(time.minute())
        };
        semantic::emit(ctx, Some(cx.resp.id), cx.area, || {
            let node = SemanticNode::new(
                StableId::of(&self.salt, (field.slug(), "dial")),
                Role::Slider,
                rt.i18n().tr(msg::TimeDial { field: name.into() }),
            )
            .value(rt.i18n().format().number_lossy(cur, Some(0)));
            node.ranged(
                NumericRange {
                    value: cur,
                    min: range.min(),
                    max: range.max(),
                    step: Some(1.0),
                },
                Availability::disabled_if(self.disabled),
            )
        });
    }
}

#[derive(Clone, Copy)]
struct FieldSlot {
    id: Id,
    rect: Rect,
    field: Field,
}

struct FieldView {
    lo: i8,
    hi: i8,
    valid: bool,
    focused: bool,
    selected: bool,
}

fn field_buf(st: &mut State, field: Field) -> &mut String {
    match field {
        Field::Hour => &mut st.hour_buf,
        Field::Minute => &mut st.minute_buf,
    }
}

// `v` (0..=99) in locale digits, zero-padded to two.
fn two_digits(loc: &dyn LocaleFormat, v: i8) -> [char; 2] {
    let d = loc.digits();
    let v = usize::from(v.unsigned_abs());
    [d[v / 10 % 10], d[v % 10]]
}

fn parse_digits(loc: &dyn LocaleFormat, range: std::ops::RangeInclusive<i8>, s: &str) -> Option<i8> {
    loc.ascii_digits(s).parse::<i8>().ok().filter(|v| range.contains(v))
}

struct DialInput {
    pointer: Option<Pos2>,
    dragging: bool,
    range: RangeModel,
}

struct DialCx<'a> {
    id: Id,
    area: Rect,
    resp: &'a egui::Response,
    t: &'a Tokens,
    input: &'a DialInput,
}

#[derive(Clone, Copy)]
struct Selector<'a> {
    name: &'a str,
    salt: &'a KeyedSalt,
    slug: &'static str,
    disabled: bool,
    selected: bool,
    value: i8,
    text: &'a str,
}

// Compose TimeSelector
fn selector_box(ui: &mut MineralUi<'_, Material>, id: Id, rect: Rect, selector: Selector<'_>, t: &Tokens) -> bool {
    let rt = ui.rt();
    let Selector {
        name,
        salt,
        slug,
        disabled,
        selected,
        value,
        text,
    } = selector;
    let resp = kit::input::interact(
        &rt.ui(ui),
        rect,
        id.with(("box", slug)),
        Sense::click(),
        Availability::disabled_if(disabled),
    );
    let r = tp::TIME_SELECTOR_CONTAINER_SHAPE.corner_radius(&t.shape, Direction::Ltr);
    let (bg, fg) = if selected {
        (
            t.role(tp::TIME_SELECTOR_SELECTED_CONTAINER_COLOR),
            t.role(tp::TIME_SELECTOR_SELECTED_LABEL_TEXT_COLOR),
        )
    } else {
        (
            t.role(tp::TIME_SELECTOR_UNSELECTED_CONTAINER_COLOR),
            t.role(tp::TIME_SELECTOR_UNSELECTED_LABEL_TEXT_COLOR),
        )
    };
    let fg = if disabled { common::disabled_content(t) } else { fg };
    ui.painter().rect_filled(rect, r, bg);
    indication::paint(
        ui,
        &resp,
        rect,
        r,
        fg,
        Interaction::of(&resp, Availability::disabled_if(disabled)),
        t,
    );
    let g = parts::type_galley(
        rt.core(),
        ui.painter(),
        text,
        tp::TIME_SELECTOR_LABEL_TEXT_FONT,
        fg,
        None,
    );
    ui.painter().galley(rect.center() - g.size() / 2.0, g, fg);
    kit::input::hover_cursor(ui.core(), &resp, Availability::disabled_if(disabled));
    semantic::emit(ui.ctx(), Some(resp.id), rect, || {
        let mut node = SemanticNode::new(StableId::of(salt, (slug, "selector")), Role::Button, name)
            .value(format!("{value:02}"))
            .clickable(Availability::disabled_if(disabled));
        node.state.selected = Some(selected);
        node
    });
    resp.clicked()
}

fn set(time: &mut Time, hour: i8, minute: i8) {
    match Time::new(
        hour.clamp(0, 23),
        minute.clamp(0, 59),
        time.second(),
        time.subsec_nanosecond(),
    ) {
        Ok(t) => *time = t,
        Err(e) => egui_mineral::kit::sanitize::report(format_args!("time {hour}:{minute} is invalid: {e}")),
    }
}

mod dial {
    use egui::{Pos2, Vec2};
    use std::f32::consts::TAU;

    pub(crate) fn angle(center: Pos2, pos: Pos2) -> f32 {
        let v: Vec2 = pos - center;
        v.x.atan2(-v.y).rem_euclid(TAU)
    }

    pub(crate) fn slot(angle: f32, n: i8) -> i8 {
        let n = n.max(1);
        #[expect(clippy::cast_possible_truncation, reason = "angle / TAU < 1, n is an i8")]
        let slot = (angle.rem_euclid(TAU) / TAU * f32::from(n)).round() as i8;
        slot.rem_euclid(n)
    }

    pub(crate) fn pos(center: Pos2, angle: f32, radius: f32) -> Pos2 {
        center + Vec2::new(angle.sin(), -angle.cos()) * radius
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn slot_inverts_pos() {
            let c = Pos2::ZERO;
            let a = 3.0 / 12.0 * TAU;
            assert_eq!(slot(angle(c, pos(c, a, 100.0)), 12), 3);
        }
    }
}

#[cfg(all(test, feature = "bundled-icons"))]
mod tests {
    use egui::{Event, Key, Modifiers, PointerButton};
    use egui_mineral::{input::synth, semantic::SemanticNode};

    use super::*;

    fn pass(ctx: &Context, rt: &crate::Rt, time: &mut Time, events: Vec<Event>) {
        crate::test_support::pass(ctx, rt, vec2(800.0, 800.0), events, |ui| {
            TimePicker::new("tp", "Time", time).show(ui);
        });
    }

    fn node(rt: &crate::Rt, key: &str) -> SemanticNode {
        rt.tree(egui::ViewportId::ROOT)
            .and_then(|t| t.find_key(key).cloned())
            .expect("node in tree")
    }

    #[test]
    fn key_in_the_click_frame_applies_to_the_clicked_result() {
        let ctx = Context::default();
        let rt = crate::test_support::runtime(&ctx);
        rt.set_collect_semantics(true);
        let mut time = Time::constant(9, 41, 0, 0);
        pass(&ctx, &rt, &mut time, Vec::new());
        let dial = node(&rt, "tp-hour-dial");
        ctx.memory_mut(|mem| mem.request_focus(dial.widget_id.expect("dial id")));
        pass(&ctx, &rt, &mut time, Vec::new());
        let at = dial::pos(dial.rect.center(), 0.25 * TAU, dial.rect.width() * 0.4);
        let press = synth::pointer_down(at, PointerButton::Primary);
        pass(&ctx, &rt, &mut time, vec![Event::PointerMoved(at), press]);
        let key = synth::key_down(Key::ArrowUp, Modifiers::NONE);
        let release = synth::pointer_up(at, PointerButton::Primary);
        pass(&ctx, &rt, &mut time, vec![release, key]);
        assert_eq!((time.hour(), time.minute()), (3, 42));
    }
}
