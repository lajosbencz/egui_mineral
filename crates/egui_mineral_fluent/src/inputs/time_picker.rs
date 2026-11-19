use egui_mineral::{
    date::Time,
    i18n::HourCycle,
    kit::{self, state::TempState},
    layout::Constraints,
    select::Selection,
    semantic::{AgentKey, KeyedSalt},
    widget::{Composite, MineralUi, SizeExt, WidgetResponse},
};

use super::combobox::Combobox;
use super::field::Validation;
use super::listbox::{ListEntry, ListOption};
use super::textbox::{FieldOutput, InputAppearance};
use crate::{Fluent, Size, spec::defaults::time_picker as tp};

#[derive(Clone, Debug, Default)]
struct TimeText {
    text: String,
    shown: Option<Time>,
}

impl TempState for TimeText {}

/// Fluent `TimePicker`: a [`Combobox`] of times between `start_hour` and `end_hour`, `increment` minutes apart.
#[derive(Debug)]
#[must_use = "show with ui.mineral(..)"]
pub struct TimePicker<'a> {
    salt: KeyedSalt,
    key: AgentKey,
    label: String,
    value: &'a mut Option<Time>,
    start_hour: u32,
    end_hour: u32,
    increment: u32,
    hour_cycle: Option<HourCycle>,
    freeform: bool,
    appearance: InputAppearance,
    size: Size,
    placeholder: Option<String>,
    before: Option<egui_mineral::icon::Icon>,
    validation: Option<Validation>,
    disabled: bool,
}

impl<'a> TimePicker<'a> {
    /// `label` is the accessible name; `value` the picked time. Agent key `<id_salt>-time-picker`.
    pub fn new(id_salt: impl Into<KeyedSalt>, label: impl Into<String>, value: &'a mut Option<Time>) -> Self {
        Self {
            salt: id_salt.into(),
            key: AgentKey::Kind("time-picker"),
            label: label.into(),
            value,
            start_hour: tp::START_HOUR,
            end_hour: tp::END_HOUR,
            increment: tp::INCREMENT,
            hour_cycle: None,
            freeform: false,
            appearance: InputAppearance::Outline,
            size: Size::Medium,
            placeholder: None,
            before: None,
            validation: None,
            disabled: false,
        }
    }

    /// First hour listed, default 0.
    pub fn start_hour(mut self, hour: u32) -> Self {
        self.start_hour = hour.min(24);
        self
    }

    /// Hour the list stops before, default 24.
    pub fn end_hour(mut self, hour: u32) -> Self {
        self.end_hour = hour.min(24);
        self
    }

    /// Minutes between listed times, default 30; 0 counts as 1.
    pub fn increment(mut self, minutes: u32) -> Self {
        self.increment = minutes.max(1);
        self
    }

    /// Default: the locale's.
    pub fn hour_cycle(mut self, cycle: HourCycle) -> Self {
        self.hour_cycle = Some(cycle);
        self
    }

    /// `freeform`: a typed time outside the list is accepted.
    pub fn freeform(mut self, freeform: bool) -> Self {
        self.freeform = freeform;
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
        /// Greys out the field; the list cannot open.
        disabled
    }

    crate::widget::setters! {
        /// Replaces the `<id_salt>-time-picker` agent key.
        agent_key(key)
    }

    fn entries(&self, fmt: &dyn egui_mineral::i18n::LocaleFormat, cycle: HourCycle) -> Vec<ListEntry> {
        let (from, to) = (self.start_hour.saturating_mul(60), self.end_hour.saturating_mul(60));
        (from..to)
            .step_by(self.increment as usize)
            .filter_map(|m| {
                let t = Time::new(i8::try_from(m / 60).ok()?, i8::try_from(m % 60).ok()?, 0, 0).ok()?;
                Some(ListOption::new(time_key(t), fmt.time(t, cycle)).into())
            })
            .collect()
    }
}

fn time_key(t: Time) -> String {
    format!("{:02}:{:02}", t.hour(), t.minute())
}

fn from_key(key: &str) -> Option<Time> {
    let (h, m) = key.split_once(':')?;
    Time::new(h.parse().ok()?, m.parse().ok()?, 0, 0).ok()
}

// `h`, `h:mm` or `hh:mm` with an optional ASCII `am` / `pm`.
fn parse(text: &str) -> Option<Time> {
    let lower = text.trim().to_lowercase();
    let (body, period) = match lower.strip_suffix("pm") {
        Some(b) => (b, Some(true)),
        None => match lower.strip_suffix("am") {
            Some(b) => (b, Some(false)),
            None => (lower.as_str(), None),
        },
    };
    let (h, m) = body.trim().split_once(':').unwrap_or((body.trim(), "0"));
    let (h, m): (i8, i8) = (h.trim().parse().ok()?, m.trim().parse().ok()?);
    let hour = match period {
        Some(pm) if (1..=12).contains(&h) => h % 12 + if pm { 12 } else { 0 },
        Some(_) => return None,
        None => h,
    };
    Time::new(hour, m, 0, 0).ok()
}

impl Composite for TimePicker<'_> {
    type Kit = Fluent;
    type Output = FieldOutput;

    kit::widget_meta!(salt);

    fn compose(&mut self, ui: &mut MineralUi<'_, Fluent>, constraints: Constraints) -> WidgetResponse<FieldOutput> {
        let rt = ui.rt();
        let ctx = rt.ctx();
        let fmt = rt.i18n().format();
        let cycle = self.hour_cycle.unwrap_or_else(|| fmt.hour_cycle());
        let state_id = ui.make_persistent_id(self.salt.id_salt()).with("time-text");
        let mut st = TimeText::load(ctx, state_id);
        if st.shown != *self.value {
            st.text = self.value.map_or_else(String::new, |t| fmt.time(t, cycle));
            st.shown = *self.value;
        }
        let mut selection = Selection::single();
        if let Some(t) = *self.value {
            selection = selection.with([time_key(t)]);
        }
        let before = selection.first().cloned();
        let entries = self.entries(fmt.as_ref(), cycle);
        let mut combo = Combobox::new(
            self.salt.child("combobox"),
            self.label.clone(),
            entries,
            &mut selection,
            &mut st.text,
        )
        .appearance(self.appearance)
        .size(self.size)
        .freeform(self.freeform)
        .validation(self.validation)
        .disabled(self.disabled)
        .list_max(tp::LISTBOX_MAX_HEIGHT)
        .agent_key(self.key.id(&self.salt));
        if let Some(p) = &self.placeholder {
            combo = combo.placeholder(p.clone());
        }
        if let Some(i) = self.before {
            combo = combo.icon_before(i);
        }
        let out = ui.mineral(combo.constrain(constraints));
        let output = *out.output();
        let mut changed = false;
        if selection.first().cloned() != before {
            *self.value = selection.first().and_then(|k| from_key(k));
            changed = true;
        } else if self.freeform && (output.blurred || output.submitted) {
            let typed = if st.text.trim().is_empty() {
                None
            } else {
                parse(&st.text)
            };
            if (typed.is_some() || st.text.trim().is_empty()) && typed != *self.value {
                *self.value = typed;
                changed = true;
            }
            if let Some(t) = typed {
                st.text = fmt.time(t, cycle);
            }
        }
        st.shown = *self.value;
        st.store(ctx, state_id);
        let response = out.into_response();
        WidgetResponse::new(response, output).with_changed(changed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_twelve_and_twenty_four_hour_text() {
        let t = |h, m| Time::new(h, m, 0, 0).ok();
        assert_eq!(parse("3:45 pm"), t(15, 45));
        assert_eq!(parse("12am"), t(0, 0));
        assert_eq!(parse("12:30PM"), t(12, 30));
        assert_eq!(parse("23:05"), t(23, 5));
        assert_eq!(parse("9"), t(9, 0));
        assert_eq!(parse("13pm"), None);
        assert_eq!(parse("soon"), None);
    }
}
