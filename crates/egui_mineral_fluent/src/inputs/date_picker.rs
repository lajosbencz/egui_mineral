use crate::tokens::ShadowTint;
use egui::{CornerRadius, Frame, Margin, Stroke};
use egui_mineral::{
    anchor::{Align, Anchor, Side},
    date::{Date, DateBounds, Weekday},
    i18n::{DateFields, DateLength, LocaleFormat},
    kit::{self, state::TempState},
    layout::Constraints,
    overlay::Popup,
    semantic::{AgentKey, KeyedSalt, Role},
    widget::{Composite, MineralUi, SizeExt, WidgetResponse},
};

use super::calendar::Calendar;
use super::field::Validation;
use super::input::Input;
use super::textbox::{FieldOutput, InputAppearance};
use crate::ColorToken::{ColorNeutralBackground1, ColorTransparentStroke};
use crate::{
    Fluent, FluentExt, Size,
    spec::{defaults::date_picker as dp, tokens::geometry as g},
    theme::Elevation,
};

#[derive(Clone, Debug, Default)]
struct DateText {
    text: String,
    shown: Option<Date>,
    open: bool,
}

impl TempState for DateText {}

/// Fluent `DatePicker`: an [`Input`] showing the picked day, opening a [`Calendar`] popup. Typed text parses with
/// the locale's date order on blur or Enter.
#[derive(Debug)]
#[must_use = "show with ui.mineral(..)"]
pub struct DatePicker<'a> {
    salt: KeyedSalt,
    key: AgentKey,
    label: String,
    value: &'a mut Option<Date>,
    bounds: DateBounds,
    first_weekday: Option<Weekday>,
    today: Option<Date>,
    week_numbers: bool,
    go_to_today: bool,
    allow_text_input: bool,
    appearance: InputAppearance,
    size: Size,
    placeholder: Option<String>,
    before: Option<egui_mineral::icon::Icon>,
    validation: Option<Validation>,
    disabled: bool,
}

impl<'a> DatePicker<'a> {
    /// `label` is the accessible name; `value` the picked day. Agent key `<id_salt>-date-picker`.
    pub fn new(id_salt: impl Into<KeyedSalt>, label: impl Into<String>, value: &'a mut Option<Date>) -> Self {
        Self {
            salt: id_salt.into(),
            key: AgentKey::Kind("date-picker"),
            label: label.into(),
            value,
            bounds: DateBounds::default(),
            first_weekday: None,
            today: None,
            week_numbers: false,
            go_to_today: true,
            allow_text_input: true,
            appearance: InputAppearance::Outline,
            size: Size::Medium,
            placeholder: None,
            before: None,
            validation: None,
            disabled: false,
        }
    }

    /// `minDate` and `maxDate`.
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

    /// `showWeekNumbers`.
    pub fn week_numbers(mut self, show: bool) -> Self {
        self.week_numbers = show;
        self
    }

    /// `showGoToToday`, default on.
    pub fn go_to_today(mut self, show: bool) -> Self {
        self.go_to_today = show;
        self
    }

    /// `allowTextInput`, default on: typed text is parsed; off, typing is discarded and only the calendar picks.
    pub fn allow_text_input(mut self, allow: bool) -> Self {
        self.allow_text_input = allow;
        self
    }

    /// Default [`InputAppearance::Outline`].
    pub fn appearance(mut self, appearance: InputAppearance) -> Self {
        self.appearance = appearance;
        self
    }

    /// Default [`Size::Medium`].
    pub fn size(mut self, size: Size) -> Self {
        self.size = size;
        self
    }

    /// Hint shown while empty.
    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.placeholder = Some(placeholder.into());
        self
    }

    /// `contentBefore`: icon leading the text.
    pub fn icon_before(mut self, icon: impl Into<egui_mineral::icon::Icon>) -> Self {
        self.before = Some(icon.into());
        self
    }

    /// Validation state, as [`FieldContext::validation`](super::FieldContext::validation) hands it over;
    /// [`Validation::Error`] is `aria-invalid`: red border until focused.
    pub fn validation(mut self, validation: Option<Validation>) -> Self {
        self.validation = validation;
        self
    }

    crate::widget::setters! {
        /// Greys out the field; the calendar cannot open.
        disabled
    }

    crate::widget::setters! {
        /// Replaces the `<id_salt>-date-picker` agent key.
        agent_key(key)
    }

    fn calendar<'v>(&self, value: &'v mut Option<Date>) -> Calendar<'v> {
        let mut c = Calendar::new(self.salt.child("calendar"), self.label.clone(), value)
            .bounds(self.bounds)
            .week_numbers(self.week_numbers)
            .go_to_today(self.go_to_today)
            .agent_key(self.key.id(&self.salt).sub("calendar"))
            .autofocus(!self.allow_text_input);
        if let Some(w) = self.first_weekday {
            c = c.first_weekday(w);
        }
        if let Some(t) = self.today {
            c = c.today(t);
        }
        c
    }
}

fn format(fmt: &dyn LocaleFormat, d: Option<Date>) -> String {
    d.map_or_else(String::new, |d| {
        fmt.date(d, DateFields::YearMonthDay, DateLength::Numeric)
    })
}

impl Composite for DatePicker<'_> {
    type Kit = Fluent;
    type Output = FieldOutput;

    kit::widget_meta!(salt);

    fn compose(&mut self, ui: &mut MineralUi<'_, Fluent>, constraints: Constraints) -> WidgetResponse<FieldOutput> {
        let rt = ui.rt();
        let ctx = rt.ctx();
        let fmt = rt.i18n().format();
        let t = ui.tokens();
        let state_id = ui.make_persistent_id(self.salt.id_salt()).with("date-text");
        let mut st = DateText::load(ctx, state_id);
        let input_salt = self.salt.child("input");
        let edit_id = ui.make_persistent_id(input_salt.id_salt()).with("edit");
        if st.shown != *self.value || !ctx.memory(|m| m.has_focus(edit_id)) {
            st.text = format(fmt.as_ref(), *self.value);
            st.shown = *self.value;
        }
        let before = st.text.clone();
        let mut input = Input::new(input_salt, self.label.clone(), &mut st.text)
            .appearance(self.appearance)
            .size(self.size)
            .icon_after(ui.icons().calendar_month)
            .validation(self.validation)
            .disabled(self.disabled)
            .agent_key(self.key.id(&self.salt));
        if let Some(p) = &self.placeholder {
            input = input.placeholder(p.clone());
        }
        if let Some(i) = self.before {
            input = input.icon_before(i);
        }
        let out = ui.mineral(input.constrain(constraints));
        let output = *out.output();
        let visual = out.zones().visual;
        let response = out.into_response();
        let clicked =
            ctx.input(|i| i.pointer.primary_clicked() && i.pointer.interact_pos().is_some_and(|p| visual.contains(p)));
        if clicked && !self.disabled {
            st.open = true;
        }
        if !self.allow_text_input {
            st.text = before;
        }

        let mut changed = false;
        if self.allow_text_input && (output.blurred || output.submitted) {
            let typed = st.text.trim();
            if typed.is_empty() {
                changed = self.value.take().is_some();
            } else if let Some(d) = fmt.parse_date(typed).filter(|d| self.bounds.contains(*d)) {
                changed = *self.value != Some(d);
                *self.value = Some(d);
            }
            st.text = format(fmt.as_ref(), *self.value);
            st.shown = *self.value;
        }

        if st.open && !self.disabled {
            let mut picked = *self.value;
            let shown = Popup::new(self.salt.child("popup"), Anchor::new(Side::Bottom, Align::Start))
                .role(Role::Dialog)
                .name(&self.label)
                .anchor_rect(visual)
                .return_focus(edit_id)
                .show(rt, &response, &mut st.open, |ui, _| {
                    Frame::new()
                        .fill(t.color(ColorNeutralBackground1))
                        .stroke(Stroke::new(dp::POPUP_BORDER, t.color(ColorTransparentStroke)))
                        .corner_radius(CornerRadius::from(g::BORDER_RADIUS_MEDIUM))
                        .inner_margin(Margin::ZERO)
                        .shadow(t.shadow(Elevation::E16, ShadowTint::Neutral).ambient)
                        .show(ui, |ui| rt.ui(ui).mineral(self.calendar(&mut picked)).changed())
                        .inner
                });
            if shown.inner == Some(true) {
                changed |= *self.value != picked;
                *self.value = picked;
                st.open = false;
                st.text = format(fmt.as_ref(), *self.value);
                st.shown = *self.value;
            }
        }
        st.store(ctx, state_id);
        WidgetResponse::new(response, output).with_changed(changed)
    }
}
