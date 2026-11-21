use crate::Material;
use crate::MaterialExt;
use crate::MaterialUiExt as _;
use crate::Rt;
use crate::adaptive::Density;
use crate::msg;
use crate::text_inputs::text_field::FieldChrome;
use egui::{Align, Context, CursorIcon, Event, Id, Layout, MouseWheelUnit, Ui, Vec2, emath::Numeric, vec2};
use egui_mineral::kit::state::TempState;
use egui_mineral::semantic::KeyedSalt;
use egui_mineral::semantic::StableId;
use egui_mineral::widget::Composite;
use egui_mineral::widget::MineralUi;
use egui_mineral::{
    edit::{EditEvent, FieldFocus},
    input::button_steps,
    layout::Constraints,
    number::{self, NumberEdit, NumberRef, NumberSource, ScrubRate},
    range::{self, RangeModel, Track},
    widget::{SizeExt, WidgetResponse},
};

use crate::actions::IconButton;
use crate::selection::slider::fraction_digits;
use crate::text_inputs::{
    TextField,
    text_field::{self, field_h},
};
use crate::{spec::defaults::text_field as tf, tr};

// No MD3 value; two bodyLarge digits between the paddings
const MIN_FIELD_W: f32 = 2.0 * tf::PADDING + 2.0 * text_field::INPUT_FONT.size();

/// Numeric field with stepper, range and locale formatting over any [`Numeric`]; non-MD3. Integer
/// values step by whole numbers (the range step and increment are at least 1 and rounded) and round typed text.
#[must_use = "show with ui.mineral(..)"]
pub struct NumberField<'a, N: Numeric = f64> {
    salt: KeyedSalt,
    label: String,
    value: NumberRef<'a, N>,
    range: RangeModel,
    format: Option<Box<dyn Fn(N) -> String + 'a>>,
    disabled: bool,
    commit_on_submit: bool,
    scrub: bool,
    scrub_rate: ScrubRate,
    step_buttons: bool,
    snap_typed: bool,
    select_on_step: bool,
    chrome: FieldChrome,
}

impl<N: Numeric> std::fmt::Debug for NumberField<'_, N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NumberField").finish_non_exhaustive()
    }
}

/// Events of [`NumberField`], read from `WidgetResponse::output`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct NumberFieldOutput {
    /// End or flush of an edit: Enter, focus loss, a step while buffered, a scrub release,
    /// [`edit::commit_pending`](egui_mineral::edit::commit_pending), or Escape.
    pub edit: Option<EditEvent>,
    /// Last input that wrote the value, set when the bound value changed or `edit` is
    /// [`EditEvent::Committed`]: typed text apart from steps, the wheel, scrubs and AccessKit.
    /// Within one pass the order is keys, text, buttons, wheel, AccessKit, scrub.
    pub source: Option<NumberSource>,
    /// Scrub drag started this pass, e.g. to open an undo group; its release reports `edit`.
    pub scrub_started: bool,
    /// Text field id, e.g. for `Memory::request_focus`; `None` when not shown.
    pub field: Option<Id>,
    /// Typed text, a step or a scrub asked for a value outside the range this pass and got the bound,
    /// e.g. to flash the outline.
    pub clamped: bool,
}

impl<'a, N: Numeric> NumberField<'a, N> {
    /// `label` is the field label; `range` sets bounds and step; `value` is the bound number. Agent key prefix `<id_salt>`.
    pub fn new(
        id_salt: impl Into<KeyedSalt>,
        label: impl Into<String>,
        range: impl Into<RangeModel>,
        value: &'a mut N,
    ) -> Self {
        Self::bound(id_salt, label, range, NumberRef::Value(value))
    }

    /// [`Self::new`] over a value that may be blank: clearing the text binds `None`, and the first step or
    /// scrub from blank lands on the in-range value nearest 0.
    pub fn optional(
        id_salt: impl Into<KeyedSalt>,
        label: impl Into<String>,
        range: impl Into<RangeModel>,
        value: &'a mut Option<N>,
    ) -> Self {
        Self::bound(id_salt, label, range, NumberRef::Optional(value))
    }

    fn bound(
        id_salt: impl Into<KeyedSalt>,
        label: impl Into<String>,
        range: impl Into<RangeModel>,
        value: NumberRef<'a, N>,
    ) -> Self {
        Self {
            chrome: FieldChrome::default(),
            salt: id_salt.into(),
            label: label.into(),
            value,
            range: range.into().for_numeric::<N>(),
            format: None,
            disabled: false,
            commit_on_submit: false,
            scrub: false,
            scrub_rate: ScrubRate::Extent,
            step_buttons: true,
            snap_typed: true,
            select_on_step: false,
        }
    }

    crate::text_inputs::text_field::field_chrome_setters!();
    crate::text_inputs::text_field::field_chrome_setters!(text);
    crate::text_inputs::text_field::field_chrome_setters!(align);

    /// Shows the unfocused value with every digit in a cell as wide as the widest one, so its width
    /// holds as digits change (fonts' own `tnum` figures cannot be requested); default `false`.
    pub fn tabular_figures(mut self, tabular_figures: bool) -> Self {
        self.chrome.tabular = tabular_figures;
        self
    }

    pub(crate) fn with_field_chrome(mut self, chrome: FieldChrome) -> Self {
        self.chrome = chrome;
        self
    }

    /// `false` keeps typed values off the step grid, only clamped, and shows the value's own
    /// digits: `0.123` with step 0.01 stays 0.123. Steps and scrubs still land on the grid.
    /// Default `true`.
    pub fn snap_typed(mut self, snap_typed: bool) -> Self {
        self.snap_typed = snap_typed;
        self
    }

    /// Display text of the value; default the locale number format at the range's fraction digits
    /// (else the step's or increment's decimals)
    /// (the value's own digits without [`Self::snap_typed`]).
    pub fn format(mut self, format: impl Fn(N) -> String + 'a) -> Self {
        self.format = Some(Box::new(format));
        self
    }

    crate::widget::setters! {
        /// Greys out the field; text and step buttons are inert.
        disabled
    }

    /// Typed text reaches the bound value on Enter, focus loss or
    /// [`edit::commit_pending`](egui_mineral::edit::commit_pending) instead of live; steps
    /// (buttons, arrow keys, AccessKit) apply at once either way. Escape restores the value from
    /// before the edit in both modes.
    pub fn commit_on_submit(mut self, commit_on_submit: bool) -> Self {
        self.commit_on_submit = commit_on_submit;
        self
    }

    /// Dragging the unfocused field with a mouse scrubs the value: the field width spans the range
    /// through its scale, Shift alone is 10x finer, Ctrl (Cmd on macOS) alone 10x coarser. A click still focuses the text; touch drags scroll.
    /// Scrubbing applies like a step in both commit modes and reports one `edit` at release.
    pub fn scrub(mut self, scrub: bool) -> Self {
        self.scrub = scrub;
        self
    }

    /// How far a [`Self::scrub`] drag moves the value; default [`ScrubRate::Extent`], the field width.
    pub fn scrub_rate(mut self, scrub_rate: ScrubRate) -> Self {
        self.scrub_rate = scrub_rate;
        self
    }

    /// Selects the whole text after a step of the focused field (keys, buttons, wheel), so typing
    /// replaces it; default `false`.
    pub fn select_on_step(mut self, select_on_step: bool) -> Self {
        self.select_on_step = select_on_step;
        self
    }

    /// `false` drops the decrement/increment buttons; arrow keys, the wheel over the focused field,
    /// AccessKit and scrub still step.
    pub fn step_buttons(mut self, step_buttons: bool) -> Self {
        self.step_buttons = step_buttons;
        self
    }

    crate::widget::setters! {
        /// Replaces the `<id_salt>` key prefix.
        agent_key(salt)
    }

    fn buttons(&self, rt: &Rt, v: Option<f64>) -> (IconButton<'_>, IconButton<'_>) {
        let button = |salt: &'static str, icon, msg: crate::Msg, key: &'static str, at_end: bool| {
            IconButton::new(self.salt.child(salt), icon, tr(rt.i18n(), msg))
                .agent_key(StableId::of(&self.salt, key))
                .disabled(self.disabled || at_end)
        };
        (
            button(
                "dec",
                rt.icons().remove,
                msg::NUMBER_DECREMENT,
                "decrement",
                v.is_some_and(|v| v <= self.range.min()),
            ),
            button(
                "inc",
                rt.icons().add,
                msg::NUMBER_INCREMENT,
                "increment",
                v.is_some_and(|v| v >= self.range.max()),
            ),
        )
    }

    fn field<'t>(&'t self, text: &'t mut String, v: Option<f64>) -> TextField<'t> {
        let mut field = TextField::new(self.salt.child("field"), self.label.clone(), text)
            .with_chrome(self.chrome.clone())
            .disabled(self.disabled)
            // Arrows step the value and Escape cancels, so the editor keeps them.
            .event_filter(text_field::editor_keys(true))
            .agent_key(StableId::of(&self.salt, "field"));
        field.spin = Some(self.range.numeric(v.unwrap_or_else(|| self.range.snap(0.0))));
        field
    }

    fn row_metrics(&self, density: Density, foot: Option<Vec2>, total: f32, gap: f32) -> RowMetrics {
        let slot = foot.map_or(0.0, |f| f.x + gap);
        let field_w = (total - 2.0 * slot).max(MIN_FIELD_W);
        let support = self.chrome.supporting.is_some() || self.chrome.error.is_some();
        let measure = text_field::measure(
            vec2(
                field_w,
                field_h(
                    density,
                    self.chrome.variant,
                    !self.label.is_empty() && !self.chrome.label_hidden,
                ),
            ),
            self.chrome.variant,
            !self.label.is_empty(),
            support,
        );
        // Buttons center on the container; the field moves down when they would overhang the top.
        let half = foot.map_or(0.0, |f| f.y / 2.0);
        let field_top = (half - measure.margin.top - measure.visual.y / 2.0).max(0.0);
        let center = field_top + measure.margin.top + measure.visual.y / 2.0;
        let height = (field_top + measure.outer().y).max(center + half);
        RowMetrics {
            field_w,
            field_top,
            center,
            height,
        }
    }

    // Input of an enabled field: text, buttons, wheel, AccessKit, scrub, then the commit; `keys` stepped
    // before the layout.
    fn react(
        &self,
        rt: &Rt,
        pass: &mut number::NumberPass<'_>,
        p: &Parts<'_>,
        (wheel_id, focused, keys): (Id, bool, bool),
        event: &mut Option<EditEvent>,
    ) -> Reaction {
        let ctx = rt.ctx();
        let field = p.field.response().id;
        if p.field.changed() {
            pass.apply_text(rt.i18n().format().as_ref());
        }
        let wheel = wheel_steps(ctx, wheel_id, focused && p.field.response().contains_pointer());
        let buttons_or_wheel = apply_steps(pass, presses(rt, p.inc) - presses(rt, p.dec), wheel);
        let stepped = keys | buttons_or_wheel | pass.accesskit(rt.core(), field);
        let extent = p.scrub.map_or(1.0, |h| h.rect.width());
        let scrubbed = pass.scrub(p.scrub, extent, Track::horizontal(rt.i18n().direction()));
        if p.scrub.is_some_and(egui::Response::clicked) {
            ctx.memory_mut(|m| m.request_focus(field));
        }
        let flush = pass.commit_requested(ctx);
        let blurred = !ctx.memory(|m| m.has_focus(field)) && !pass.scrubbing();
        if event.is_none() && (blurred || flush || (stepped && self.commit_on_submit)) {
            *event = pass.commit();
        }
        if self.select_on_step && (keys || buttons_or_wheel) {
            egui_mineral::edit::select_all(ctx, field, pass.text_mut());
        }
        Reaction { stepped, scrubbed }
    }

    fn load_edit(&self, ctx: &Context, id: Id, before: Option<f64>, fmt: &dyn Fn(f64) -> String) -> NumberEdit {
        let mut edit = NumberEdit::load_or(ctx, id, || NumberEdit::new(self.range, before, fmt));
        edit.set_snap(self.snap_typed);
        edit.set_blank(self.value.optional());
        edit.set_scrub_rate(self.scrub_rate);
        edit
    }

    fn text(&self, i18n: &egui_mineral::i18n::I18n, v: f64) -> String {
        match &self.format {
            Some(f) => f(range::to_numeric(v)),
            None => i18n
                .format()
                .number_lossy(v, self.snap_typed.then(|| fraction_digits(&self.range)).flatten()),
        }
    }
}

impl<N: Numeric> Composite for NumberField<'_, N> {
    type Kit = Material;
    type Output = NumberFieldOutput;

    egui_mineral::kit::widget_meta!(salt);

    fn compose(
        &mut self,
        ui: &mut MineralUi<'_, Material>,
        constraints: Constraints,
    ) -> WidgetResponse<NumberFieldOutput> {
        let rt = ui.rt();
        let ctx = rt.ctx();
        let base = ui.make_persistent_id(self.salt.id_salt());
        let edit_key = base.with("edit-id");
        let edit_id = EditId::try_load(ctx, edit_key).map(|e| e.0);
        let state_id = base.with("edit");
        let fmt = |v: f64| self.text(rt.i18n(), v);
        let before = self.value.get();
        let focused = edit_id.is_some_and(|id| ctx.memory(|m| m.has_focus(id)));
        let escape = !self.disabled && edit_id.is_some_and(|id| egui_mineral::kit::layer::consume_escape_from(ctx, id));
        let mut edit = self.load_edit(ctx, state_id, before, &fmt);
        // An Escape pass keeps the edit open so it cancels instead of committing on blur.
        let focus = FieldFocus::of(focused || escape);
        let mut pass = edit.begin(self.range, before, focus, &fmt);
        let keys = focused && !self.disabled && pass.consume_step_keys(ctx);
        let mut event = escape.then(|| cancel_edit(ctx, &mut pass, edit_id)).flatten();
        let v = pass.value();

        let total = constraints.constrain(vec2(tf::MIN_WIDTH, 0.0)).x;
        let gap = crate::adaptive::grid(1);
        let dir = rt.i18n().direction();
        let disabled = self.disabled;
        let buttons = self.step_buttons.then(|| self.buttons(rt, v));
        let foot = buttons.as_ref().map(|(dec, _)| dec.footprint(ui.density()));
        let RowMetrics {
            field_w,
            field_top,
            center,
            height,
        } = self.row_metrics(ui.density(), foot, total, gap);
        let scrub_sense = (self.scrub && !disabled && !focused)
            .then(|| number::scrub_sense(&ui.pass_input()))
            .flatten();
        let step = |ui: &mut Ui, b: IconButton, foot: Vec2| {
            ui.allocate_ui_with_layout(vec2(foot.x, height), Layout::top_down(Align::Center), |ui| {
                ui.add_space(center - foot.y / 2.0);
                rt.ui(ui).mineral(b)
            })
            .inner
        };
        let inner = ui.allocate_ui_with_layout(vec2(total, height), dir.row().with_cross_align(Align::Min), |ui| {
            ui.spacing_mut().item_spacing.x = gap;
            let (dec_button, inc_button) = buttons.map_or((None, None), |(d, i)| (Some(d), Some(i)));
            let dec = dec_button.zip(foot).map(|(b, f)| step(ui, b, f));
            let field = self.field(pass.text_mut(), v);
            let out = ui
                .allocate_ui_with_layout(vec2(field_w, height), dir.column(), |ui| {
                    ui.add_space(field_top);
                    rt.ui(ui).mineral(field.preferred_width(field_w))
                })
                .inner;
            // Over the text edit, so it takes presses until the field has focus.
            let scrub =
                scrub_sense.map(|sense| scrub_handle(&rt.ui(ui), out.zones().visual, base.with("scrub"), sense));
            let inc = inc_button.zip(foot).map(|(b, f)| step(ui, b, f));
            (dec, out, inc, scrub)
        });
        let egui::InnerResponse {
            inner: (dec, out, inc, scrub),
            response: row,
        } = inner;

        let parts = Parts {
            field: &out,
            dec: dec.as_ref(),
            inc: inc.as_ref(),
            scrub: scrub.as_ref(),
        };
        let Reaction { stepped, scrubbed } = if disabled {
            Reaction::default()
        } else {
            self.react(rt, &mut pass, &parts, (base.with("wheel"), focused, keys), &mut event)
        };
        let (v, source, clamped) = (pass.value(), pass.source(), pass.clamped());
        if !self.commit_on_submit || stepped || scrubbed || event.is_some() {
            self.value.set(v);
            edit.wrote(self.value.get());
        }
        edit.store(ctx, state_id);
        EditId(out.response().id).store(ctx, edit_key);
        // The row response: the field's own `changed` flag also fires on unparsed text.
        let changed = differs(before, self.value.get());
        let output = NumberFieldOutput {
            edit: event,
            source: (changed || event == Some(EditEvent::Committed))
                .then_some(source)
                .flatten(),
            scrub_started: !disabled && scrub.as_ref().is_some_and(egui::Response::drag_started),
            field: Some(out.response().id),
            clamped: clamped && !disabled,
        };
        WidgetResponse::new(row, output).with_changed(changed)
    }
}

#[derive(Clone, Copy)]
struct EditId(Id);

impl TempState for EditId {}

fn cancel_edit(ctx: &Context, pass: &mut number::NumberPass<'_>, edit_id: Option<Id>) -> Option<EditEvent> {
    if let Some(id) = edit_id {
        ctx.memory_mut(|m| m.surrender_focus(id));
    }
    pass.cancel()
}

fn presses(rt: &Rt, b: Option<&WidgetResponse>) -> i64 {
    b.map_or(0, |b| i64::from(button_steps(rt.core(), b.response(), b.response().id)))
}

fn differs(before: Option<f64>, after: Option<f64>) -> bool {
    match (before, after) {
        (Some(b), Some(a)) => b.is_nan() != a.is_nan() || (b - a).abs() > f64::EPSILON,
        (b, a) => b.is_some() != a.is_some(),
    }
}

struct Parts<'r> {
    field: &'r WidgetResponse<text_field::TextFieldOutput>,
    dec: Option<&'r WidgetResponse>,
    inc: Option<&'r WidgetResponse>,
    scrub: Option<&'r egui::Response>,
}

#[derive(Default)]
struct Reaction {
    stepped: bool,
    scrubbed: bool,
}

fn apply_steps(pass: &mut number::NumberPass<'_>, buttons: i64, wheel: i64) -> bool {
    let mut stepped = false;
    for (steps, source) in [(buttons, NumberSource::Step), (wheel, NumberSource::Wheel)] {
        if steps != 0 {
            pass.step(steps, source);
            stepped = true;
        }
    }
    stepped
}

// One step per line or page notch; point deltas (trackpads) step once per `line_scroll_speed`.
// Consumes the scroll so an enclosing scroll area stays put.
fn wheel_steps(ctx: &Context, id: Id, over_focused: bool) -> i64 {
    if !over_focused {
        return 0;
    }
    let line = ctx.options(|o| o.input_options.line_scroll_speed);
    let mut acc = Vec2::load(ctx, id).y;
    let mut steps = 0;
    ctx.input_mut(|i| {
        for e in &i.events {
            if let Event::MouseWheel { unit, delta, .. } = e {
                let dy = match unit {
                    MouseWheelUnit::Point => {
                        acc += delta.y;
                        let whole = if acc.abs() >= line { acc } else { 0.0 };
                        acc -= whole;
                        whole
                    }
                    MouseWheelUnit::Line | MouseWheelUnit::Page => delta.y,
                };
                // Positive y moves content down: wheel up, increment.
                steps += i64::from(dy > 0.0) - i64::from(dy < 0.0);
            }
        }
        i.smooth_scroll_delta = Vec2::ZERO;
    });
    Vec2::new(0.0, acc).store(ctx, id);
    steps
}

struct RowMetrics {
    field_w: f32,
    field_top: f32,
    center: f32,
    height: f32,
}

fn scrub_handle(ui: &MineralUi<'_, Material>, rect: egui::Rect, id: Id, sense: egui::Sense) -> egui::Response {
    let mut handle = ui.interact(rect, id, sense);
    ui.pass_input().tap_on(&mut handle);
    if handle.hovered() || handle.dragged() {
        ui.ctx().set_cursor_icon(CursorIcon::ResizeHorizontal);
    }
    handle
}
