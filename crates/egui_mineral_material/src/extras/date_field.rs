use crate::Material;
use crate::MaterialExt;
use crate::MaterialUiExt as _;
use crate::text_inputs::text_field::FieldChrome;
use egui::{
    CursorIcon, Event, IMEPurpose, ImeEvent, Key, Rangef, Rect, Ui,
    accesskit::{Action as AkAction, ActionData},
    output::IMEOutput,
    pos2, vec2,
};
use egui_mineral::i18n::I18n;
use egui_mineral::semantic::AgentKey;
use egui_mineral::semantic::KeyedSalt;
use egui_mineral::widget::{Availability, MineralUi};
use egui_mineral::{
    date::{self, Date, DateBounds, DateSegments},
    i18n::{self, DateFields, DateLength, DatePart as Field, Direction},
    kit,
    layout::{Constraints, Measure},
    semantic::{Role, SemanticNode},
    widget::{MineralWidget, WidgetFrame, WidgetResponse},
};

use crate::text_inputs::text_field::{self, Decor, DecorOut, INPUT_FONT, field_h, line_galley};
use crate::{spec::defaults::text_field as tf, tr};

#[doc(hidden)]
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct DateFieldState {
    segments: Option<DateSegments>,
    active: usize,
    last: Option<Date>,
}

/// Locale-ordered segmented date entry; non-MD3. An invalid or out-of-range entry shows its own error before [`Self::error`].
#[derive(Debug)]
#[must_use = "show with ui.mineral(..)"]
pub struct DateField<'a> {
    salt: KeyedSalt,
    key: AgentKey,
    label: String,
    value: &'a mut Option<Date>,
    bounds: DateBounds,
    disabled: bool,
    chrome: FieldChrome,
}

impl<'a> DateField<'a> {
    /// `label` is the field label; `value` is the bound date, `None` while empty or invalid. Agent key `<id_salt>-date`.
    pub fn new(id_salt: impl Into<KeyedSalt>, label: impl Into<String>, value: &'a mut Option<Date>) -> Self {
        let id_salt: KeyedSalt = id_salt.into();
        Self {
            chrome: FieldChrome::default(),
            key: AgentKey::Kind("date"),
            salt: id_salt,
            label: label.into(),
            value,
            bounds: DateBounds::default(),
            disabled: false,
        }
    }

    crate::text_inputs::text_field::field_chrome_setters!();

    /// Allowed range; out-of-range entries show an error. Default unbounded.
    pub fn bounds(mut self, bounds: DateBounds) -> Self {
        self.bounds = bounds;
        self
    }

    crate::widget::setters! {
        /// Greys out the field; the text is read-only.
        disabled
    }

    crate::widget::setters! {
        /// Replaces the `<id_salt>-date` agent key.
        agent_key(key)
    }
}

fn separator(fmt: &dyn i18n::LocaleFormat) -> String {
    let s = fmt.ascii_digits(&fmt.date(
        Date::constant(2001, 11, 22),
        DateFields::YearMonthDay,
        DateLength::Numeric,
    ));
    let rest = s.trim_start_matches(|c: char| !c.is_ascii_digit());
    let tail = rest.trim_start_matches(|c: char| c.is_ascii_digit());
    let sep: String = tail.chars().take_while(|c| !c.is_ascii_digit()).collect();
    if sep.is_empty() {
        egui_mineral::kit::sanitize::report(format_args!("LocaleFormat numeric date has no field separator"));
        return "/".into();
    }
    sep
}

fn hint_id(f: Field) -> &'static str {
    match f {
        Field::Year => "mineral-date-hint-year",
        Field::Month => "mineral-date-hint-month",
        Field::Day => "mineral-date-hint-day",
    }
}

fn apply_events(
    events: Vec<Event>,
    seg: &mut DateSegments,
    active: &mut usize,
    fmt: &dyn i18n::LocaleFormat,
    dir: Direction,
) -> bool {
    let mut edited = false;
    for e in events {
        match e {
            // Soft keyboards deliver text as IME commits and deletions as surrounding deletes.
            Event::Ime(ImeEvent::DeleteSurrounding { before_chars, .. }) => {
                for _ in 0..before_chars {
                    *active = seg.clear(*active);
                }
                edited |= before_chars > 0;
            }
            Event::Text(s) | Event::Ime(ImeEvent::Commit(s)) => {
                for ch in fmt.ascii_digits(&s).chars() {
                    if let Some(d) = ch.to_digit(10) {
                        *active = seg.digit(*active, d as u8);
                    } else if !ch.is_whitespace() {
                        seg.commit();
                        *active = (*active + 1).min(2);
                    }
                    edited = true;
                }
            }
            Event::Key {
                key,
                pressed: true,
                modifiers,
                ..
            } if !modifiers.any() => {
                edited = true;
                match key {
                    Key::Backspace | Key::Delete => *active = seg.clear(*active),
                    Key::ArrowUp => seg.bump(*active, 1, date::today()),
                    Key::ArrowDown => seg.bump(*active, -1, date::today()),
                    Key::ArrowLeft | Key::ArrowRight => {
                        seg.commit();
                        *active = if key == dir.forward_key() {
                            (*active + 1).min(2)
                        } else {
                            (*active).saturating_sub(1)
                        };
                    }
                    Key::Home => *active = 0,
                    Key::End => *active = 2,
                    _ => edited = false,
                }
            }
            _ => {}
        }
    }
    edited
}

fn paint_segments(
    ui: &MineralUi<'_, Material>,
    decor: &DecorOut,
    seg: &DateSegments,
    active_seg: Option<usize>,
    fmt: &dyn i18n::LocaleFormat,
) -> [Rect; 3] {
    let rt = ui.rt();
    let t = rt.tokens();
    let order = fmt.date_order();
    let dir = rt.i18n().direction();
    let colors = decor.colors;
    let mut seg_rects = [Rect::NOTHING; 3];
    if decor.k > 0.0 {
        let sep = separator(fmt);
        let p = ui.painter();
        let mut offset = 0.0;
        let span = decor.span.span();
        // The active segment shows as selected text.
        let selection = text_field::FieldColors::selection(&t);
        let mut place = |text: String, color, selected: bool, idx: Option<usize>| {
            let g = line_galley(p, &text, INPUT_FONT, color, span);
            let w = g.size().x;
            let rr = Rect::from_min_size(
                pos2(dir.span_x(decor.span, Rangef::new(offset, offset + w)), decor.text_top),
                g.size(),
            );
            if selected {
                p.rect_filled(rr, 0, selection);
            }
            p.galley(rr.min, g, color);
            if let Some(i) = idx {
                seg_rects[i] = rr;
            }
            offset += w;
        };
        for (i, f) in order.iter().enumerate() {
            if i > 0 {
                place(sep.clone(), colors.input, false, None);
            }
            let active = active_seg == Some(i);
            match seg.text(i) {
                Some(s) => place(fmt.native_digits(&s), colors.input, active, Some(i)),
                None => place(tr(rt.i18n(), hint_id(*f)), colors.placeholder, active, Some(i)),
            }
        }
    }
    seg_rects
}

// winit set_ime_allowed, eframe web text agent
fn request_ime(ui: &Ui, field: Rect, segment: Rect) {
    let ctx = ui.ctx();
    let to_global = ctx.layer_transform_to_global(ui.layer_id()).unwrap_or_default();
    let at = if segment.is_positive() { segment } else { field };
    ctx.output_mut(|o| {
        o.ime = Some(IMEOutput {
            purpose: IMEPurpose::Normal,
            rect: to_global * field,
            cursor_rect: to_global * Rect::from_x_y_ranges(at.left()..=at.left() + 1.0, at.y_range()),
            should_interrupt_composition: false,
        });
    });
}

impl MineralWidget for DateField<'_> {
    type Kit = Material;
    type State = DateFieldState;
    type Output = ();

    egui_mineral::kit::widget_meta!(salt, enabled, no_target);

    fn layout(&self, ui: &MineralUi<'_, Material>, c: Constraints, _: &Self::State) -> Measure {
        let w = c.constrain(vec2(tf::MIN_WIDTH, 0.0)).x;
        // The supporting row is always reserved: parse errors appear without a layout shift.
        text_field::measure(
            c.constrain(vec2(w, field_h(ui.density(), self.chrome.variant, !self.label.is_empty()))),
            self.chrome.variant,
            !self.label.is_empty(),
            true,
        )
    }

    fn update(
        &mut self,
        ui: &mut MineralUi<'_, Material>,
        frame: &WidgetFrame<'_>,
        st: &mut DateFieldState,
    ) -> WidgetResponse {
        let rt = ui.rt();
        let ctx = rt.ctx();
        let t = rt.tokens();
        let fmt = rt.i18n().format();
        let order = fmt.date_order();
        let r = frame.response.clone();
        let field = frame.zones.visual;
        let dir = rt.i18n().direction();
        let before = *self.value;

        let mut seg = match st.segments {
            Some(s) if s.order() == order && st.last == *self.value => s,
            _ => {
                st.active = 0;
                DateSegments::from_date(order, *self.value)
            }
        };
        let mut edited = false;
        let focused = r.has_focus();
        if focused && !self.disabled {
            kit::focus::lock_arrows(ctx, r.id, kit::focus::Arrows::Both);
            let events = ctx.input(|i| i.events.clone());
            edited |= apply_events(events, &mut seg, &mut st.active, fmt.as_ref(), dir);
        }
        if !focused {
            seg.commit();
        }
        // AccessKit/agent SetValue with a typed date.
        let typed = ctx.input(|i| {
            i.accesskit_action_requests(r.id, AkAction::SetValue)
                .find_map(|a| match &a.data {
                    Some(ActionData::Value(s)) => Some(s.to_string()),
                    _ => None,
                })
        });
        if let Some(s) = typed.filter(|_| !self.disabled) {
            let date = fmt.parse_date(&s);
            if date.is_some() || s.trim().is_empty() {
                seg = DateSegments::from_date(order, date);
                edited = true;
            } else {
                egui_mineral::kit::sanitize::report(format_args!(
                    "SetValue on {:?}: not a date ({} chars)",
                    r.id,
                    s.chars().count()
                ));
            }
        }

        let parsed = seg.date();
        let out_of_range = parsed.is_some_and(|d| !self.bounds.contains(d));
        if edited {
            *self.value = parsed.filter(|d| self.bounds.contains(*d));
        }
        let complete_invalid = !seg.is_empty() && (0..3).all(|i| seg.value(i).is_some()) && parsed.is_none();
        let error = if out_of_range {
            Some(tr(rt.i18n(), "mineral-date-out-of-range"))
        } else if complete_invalid {
            Some(tr(rt.i18n(), "mineral-date-invalid"))
        } else {
            self.chrome.error.clone()
        };

        let decor = Decor {
            id: r.id,
            variant: self.chrome.variant,
            label: &self.label,
            focused,
            floated: focused || !seg.is_empty(),
            error: error.is_some(),
            disabled: self.disabled,
            leading: self.chrome.leading,
            trailing: false,
        }
        .paint(ui, field, &t);
        let colors = decor.colors;

        let seg_rects = paint_segments(ui, &decor, &seg, focused.then_some(st.active), fmt.as_ref());
        if !self.disabled {
            if r.hovered() {
                ctx.set_cursor_icon(CursorIcon::Text);
            }
            let press = frame.press_pos();
            if let Some(pt) = press {
                r.request_focus();
                seg.commit();
                let near = (0..3).min_by(|a, b| {
                    let d = |i: usize| (seg_rects[i].center().x - pt.x).abs();
                    d(*a).total_cmp(&d(*b))
                });
                st.active = if seg_rects.iter().any(Rect::is_positive) {
                    near.unwrap_or(0)
                } else {
                    0
                };
            }
        }
        if focused && !self.disabled && ctx.memory(|m| m.owns_ime_events(r.id)) {
            request_ime(ui, field, seg_rects[st.active.min(2)]);
        }
        let support = error.as_deref().or(self.chrome.supporting.as_deref());
        text_field::paint_support(ui, field, support, None, colors.supporting, text_field::Elide::End);

        st.segments = Some(seg);
        st.last = *self.value;
        WidgetResponse::new(r, ()).with_changed(*self.value != before)
    }

    fn semantics(&self, i18n: &I18n, _: &WidgetResponse, st: &DateFieldState) -> Option<SemanticNode> {
        let value = self.value.map_or_else(String::new, |d| {
            i18n.format().date(d, DateFields::YearMonthDay, DateLength::Numeric)
        });
        let mut n = SemanticNode::new(self.key.id(&self.salt), Role::DateInput, &self.label).value(value);
        n.state.invalid =
            self.chrome.error.is_some() || st.segments.is_some_and(|s| !s.is_empty() && self.value.is_none());
        Some(n.editable(Availability::disabled_if(self.disabled)))
    }
}
