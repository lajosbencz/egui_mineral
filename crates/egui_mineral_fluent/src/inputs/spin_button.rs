use egui::{Color32, CornerRadius, CursorIcon, Galley, Pos2, Rangef, Rect, Sense, emath::Numeric, vec2};
use egui_mineral::{
    edit::{EditEvent, FieldFocus},
    i18n::{Direction, I18n},
    kit::{self, state::TempState},
    layout::{Constraints, Measure, MinTarget},
    number::{self, NumberEdit, NumberRef, NumberSource, ScrubRate},
    range::{RangeModel, Track},
    semantic::{AgentKey, KeyedSalt, Role, SemanticNode},
    widget::{Availability, MineralUi, MineralWidget, WidgetFrame, WidgetResponse},
};
use std::sync::Arc;

use super::field::Validation;
use super::{
    bar::digits,
    textbox::{self, Chrome, Edit, InputAppearance, Live},
};
use crate::spec::ColorToken::{
    ColorNeutralBackground1Hover, ColorNeutralBackground1Pressed, ColorNeutralBackground3Hover,
    ColorNeutralBackground3Pressed, ColorNeutralForeground3, ColorNeutralForeground3Hover,
    ColorNeutralForeground3Pressed, ColorNeutralForegroundDisabled, ColorSubtleBackgroundHover,
    ColorSubtleBackgroundPressed,
};
use crate::{
    Fluent, FluentExt, Icon, Size,
    spec::{
        defaults::{input as inp, spin_button as sp},
        tokens::geometry as g,
    },
    widget::{
        paint,
        state::{Ramp, State},
    },
};

/// Events of [`SpinButton`], read from `WidgetResponse::output`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct SpinButtonOutput {
    /// Typed text, a step or a scrub asked for a value outside the range this pass and got the bound.
    pub clamped: bool,
}

/// Fluent `SpinButton`: a number field with step buttons, bound to `value`, any [`Numeric`]. Typed text commits on Enter or blur,
/// arrow and page keys and the buttons step; `changed` reports a committed value.
#[derive(Debug)]
#[must_use = "show with ui.mineral(..)"]
pub struct SpinButton<'a, N = f64> {
    salt: KeyedSalt,
    key: AgentKey,
    label: String,
    value: NumberRef<'a, N>,
    scrub: Option<ScrubRate>,
    select_on_step: bool,
    range: RangeModel,
    appearance: InputAppearance,
    size: Size,
    invalid: bool,
    disabled: bool,
    placeholder: Option<String>,
    before: Option<egui_mineral::icon::Icon>,
    after: Option<egui_mineral::icon::Icon>,
    unit: Option<String>,
}

impl<'a, N: Numeric> SpinButton<'a, N> {
    /// `range` sets the bounds, `step` (whole for integer `N`) and page `jump`; `label` is the accessible name. Agent key
    /// `<id_salt>-spin-button`.
    pub fn new(
        id_salt: impl Into<KeyedSalt>,
        label: impl Into<String>,
        range: impl Into<RangeModel>,
        value: &'a mut N,
    ) -> Self {
        Self::bound(id_salt, label, range, NumberRef::Value(value))
    }

    /// [`Self::new`] over a value that may be blank (React `value={null}`): clearing the text binds `None`, and
    /// the first step from blank lands on the in-range value nearest 0.
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
            salt: id_salt.into(),
            key: egui_mineral::agent_key!("spin-button"),
            label: label.into(),
            value,
            scrub: None,
            select_on_step: false,
            range: range.into().for_numeric::<N>(),
            appearance: InputAppearance::Outline,
            size: Size::Medium,
            invalid: false,
            disabled: false,
            placeholder: None,
            before: None,
            after: None,
            unit: None,
        }
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

    /// `contentAfter`: icon trailing the text, before the step buttons.
    pub fn icon_after(mut self, icon: impl Into<egui_mineral::icon::Icon>) -> Self {
        self.after = Some(icon.into());
        self
    }

    /// Unit text after the value, e.g. `mm`.
    pub fn suffix(mut self, suffix: impl Into<String>) -> Self {
        self.unit = Some(suffix.into());
        self
    }

    /// Default [`InputAppearance::Outline`].
    pub fn appearance(mut self, appearance: InputAppearance) -> Self {
        self.appearance = appearance;
        self
    }

    /// [`Size::Small`] or [`Size::Medium`] (default); [`Size::Large`] is medium, as in React.
    pub fn size(mut self, size: Size) -> Self {
        self.size = size;
        self
    }

    /// Validation state, as [`FieldContext::validation`](super::FieldContext::validation) hands it over;
    /// [`Validation::Error`] is `aria-invalid`: red border until focused.
    pub fn validation(mut self, validation: Option<Validation>) -> Self {
        self.invalid = validation == Some(Validation::Error);
        self
    }

    crate::widget::setters! {
        /// Greys out the field and ignores input.
        disabled
    }

    /// Dragging the unfocused text with a mouse scrubs the value at `rate`; Shift alone is 10x finer, Ctrl
    /// (Cmd on macOS) alone 10x coarser. A click still focuses the text; touch drags scroll. Not in Fluent
    /// (docs/divergences.md).
    pub fn scrub(mut self, rate: ScrubRate) -> Self {
        self.scrub = Some(rate);
        self
    }

    /// Selects the whole text after a step of the focused field, so typing replaces it; default `false`.
    pub fn select_on_step(mut self, select_on_step: bool) -> Self {
        self.select_on_step = select_on_step;
        self
    }

    crate::widget::setters! {
        /// Replaces the `<id_salt>-spin-button` agent key.
        agent_key(key)
    }

    fn chrome(&self) -> Chrome {
        Chrome {
            appearance: self.appearance,
            size: self.field_size(),
            invalid: self.invalid,
            disabled: self.disabled,
        }
    }

    // Over the unfocused text, so a press-drag scrubs and a click focuses.
    fn scrub_handle(
        &self,
        ui: &MineralUi<'_, Fluent>,
        rect: Rect,
        id: egui::Id,
        focused: bool,
    ) -> Option<egui::Response> {
        let sense = (self.scrub.is_some() && !self.disabled && !focused)
            .then(|| number::scrub_sense(&ui.pass_input()))
            .flatten()?;
        let mut h = ui.interact(rect, id, sense);
        ui.pass_input().tap_on(&mut h);
        if h.hovered() || h.dragged() {
            ui.ctx().set_cursor_icon(CursorIcon::ResizeHorizontal);
        }
        Some(h)
    }

    fn field_size(&self) -> Size {
        match self.size {
            Size::Small => Size::Small,
            Size::Medium | Size::Large => Size::Medium,
        }
    }

    // Net clicks of the increment and decrement buttons, painted over the frame.
    fn step_buttons(
        &self,
        ui: &mut MineralUi<'_, Fluent>,
        base: egui::Id,
        parts: [(Rect, bool); 2],
        t: &crate::theme::Tokens,
    ) -> i64 {
        let dir = ui.i18n().direction();
        let (fore, fill) = self.button_ramps();
        let mut steps = 0_i64;
        for (rect, up) in parts {
            let id = base.with(if up { "increment" } else { "decrement" });
            let sense = if self.disabled {
                Sense::hover()
            } else {
                Sense::click() - Sense::FOCUSABLE
            };
            let r = ui.interact(rect, id, sense);
            let s = State::of(egui_mineral::widget::Interaction::of(
                &r,
                Availability::disabled_if(self.disabled),
            ));
            ui.painter().rect_filled(rect, corner(up, dir), fill.pick(s, t));
            let icon: Icon = if up {
                ui.icons().chevron_up
            } else {
                ui.icons().chevron_down
            };
            paint::icon(ui.painter(), rect.center(), icon, false, self.glyph(), fore.pick(s, t));
            if r.clicked() {
                steps += if up { 1 } else { -1 };
            }
        }
        steps
    }

    fn text(&self, i18n: &I18n, v: f64) -> String {
        i18n.format().number_lossy(v, digits(&self.range))
    }

    // react-spinbutton useSpinButtonStyles.styles.ts: root padding start, then the 24 wide button column.
    fn pad(&self) -> f32 {
        match self.field_size() {
            Size::Small => g::SPACING_HORIZONTAL_S,
            _ => g::SPACING_HORIZONTAL_M_NUDGE,
        }
    }

    fn button_height(&self) -> f32 {
        match self.field_size() {
            Size::Small => sp::BUTTON_HEIGHT_SMALL,
            _ => sp::BUTTON_HEIGHT,
        }
    }

    fn glyph(&self) -> f32 {
        match self.field_size() {
            Size::Small => sp::GLYPH_SMALL,
            _ => sp::GLYPH,
        }
    }

    fn unit_galley(&self, ui: &MineralUi<'_, Fluent>, chrome: Chrome, t: &crate::theme::Tokens) -> Option<Arc<Galley>> {
        self.unit.as_deref().map(|u| {
            ui.painter().layout_no_wrap(
                u.to_owned(),
                chrome.text_format(ui.ctx(), t).font_id,
                chrome.placeholder_color(t),
            )
        })
    }

    // The unit follows the typed text on the start side, kept inside the reserved width.
    fn paint_unit(
        &self,
        ui: &MineralUi<'_, Fluent>,
        chrome: Chrome,
        text_rect: Rect,
        text: &str,
        t: &crate::theme::Tokens,
    ) {
        let Some(unit) = self.unit_galley(ui, chrome, t) else {
            return;
        };
        let dir = ui.i18n().direction();
        let gap = chrome.metrics().gap;
        let typed = ui
            .painter()
            .layout_no_wrap(
                text.to_owned(),
                chrome.text_format(ui.ctx(), t).font_id,
                Color32::PLACEHOLDER,
            )
            .size()
            .x
            .min(text_rect.width());
        let y = text_rect.center().y - unit.size().y / 2.0;
        let x = match dir {
            Direction::Ltr => text_rect.left() + typed + gap,
            Direction::Rtl => text_rect.right() - typed - gap - unit.size().x,
        };
        ui.painter().galley(Pos2::new(x, y), unit, chrome.placeholder_color(t));
    }

    // Icons and placeholder around the editor; the editor rect they leave.
    fn paint_content(
        &self,
        ui: &MineralUi<'_, Fluent>,
        chrome: Chrome,
        visual: Rect,
        column: Rect,
        empty: bool,
        t: &crate::theme::Tokens,
    ) -> Rect {
        let ctx = ui.ctx();
        let dir = ui.i18n().direction();
        let inner = visual.shrink(g::STROKE_WIDTH_THIN);
        let pad = self.pad();
        let m = chrome.metrics();
        let lead = if self.before.is_some() {
            textbox::pads(
                &m,
                textbox::Slots {
                    before: true,
                    after: false,
                },
            )
            .start
        } else {
            pad
        };
        let start = dir.start_x(inner, Rangef::new(lead - g::STROKE_WIDTH_THIN, lead));
        let (text_left, text_right) = match dir {
            Direction::Ltr => (start + g::STROKE_WIDTH_THIN, column.min.x - g::SPACING_HORIZONTAL_XS),
            Direction::Rtl => (column.max.x + g::SPACING_HORIZONTAL_XS, start),
        };
        let p = ui.painter().clone();
        let icon_at = |x: f32| Rect::from_center_size(Pos2::new(x, inner.center().y), vec2(m.icon, m.icon));
        if let Some(icon) = self.before {
            let x = dir.start_x(inner, Rangef::new(m.root, m.root + m.icon)) + m.icon / 2.0;
            paint::icon(&p, icon_at(x).center(), icon, false, m.icon, chrome.icon_color(t));
        }
        // Reserved beside the editor: the trailing icon, then the unit.
        let after_w = if self.after.is_some() { m.icon + m.gap } else { 0.0 };
        let unit = self.unit_galley(ui, chrome, t);
        let unit_w = unit.as_ref().map_or(0.0, |g| g.size().x + m.gap);
        let (edit_left, edit_right) = match dir {
            Direction::Ltr => (text_left, text_right - after_w - unit_w),
            Direction::Rtl => (text_left + after_w + unit_w, text_right),
        };
        let text_rect = Rect::from_min_max(
            Pos2::new(edit_left, inner.top()),
            Pos2::new(edit_right.max(edit_left), inner.bottom()),
        );
        if let Some(icon) = self.after {
            let edge = match dir {
                Direction::Ltr => text_right - m.icon / 2.0,
                Direction::Rtl => text_left + m.icon / 2.0,
            };
            paint::icon(&p, icon_at(edge).center(), icon, false, m.icon, chrome.icon_color(t));
        }
        if let (Some(ph), true) = (&self.placeholder, empty) {
            let color = chrome.placeholder_color(t);
            let g = p.layout_no_wrap(ph.clone(), chrome.text_format(ctx, t).font_id, color);
            let x = dir.start_x(text_rect, Rangef::new(0.0, g.size().x));
            p.galley(Pos2::new(x, text_rect.center().y - g.size().y / 2.0), g, color);
        }
        text_rect
    }

    // useBaseButtonClassName / useButtonStyles `:enabled` colors per appearance.
    fn button_ramps(&self) -> (Ramp, Ramp) {
        let fore = Ramp::new(
            ColorNeutralForeground3,
            ColorNeutralForeground3Hover,
            ColorNeutralForeground3Pressed,
            ColorNeutralForegroundDisabled,
        );
        let fill = match self.appearance {
            InputAppearance::Outline | InputAppearance::Underline => Ramp {
                rest: None,
                hover: Some(ColorSubtleBackgroundHover),
                pressed: Some(ColorSubtleBackgroundPressed),
                disabled: None,
            },
            InputAppearance::FilledDarker => Ramp {
                rest: None,
                hover: Some(ColorNeutralBackground3Hover),
                pressed: Some(ColorNeutralBackground3Pressed),
                disabled: None,
            },
            InputAppearance::FilledLighter => Ramp {
                rest: None,
                hover: Some(ColorNeutralBackground1Hover),
                pressed: Some(ColorNeutralBackground1Pressed),
                disabled: None,
            },
        };
        (fore, fill)
    }
}

impl<N: Numeric> MineralWidget for SpinButton<'_, N> {
    type Kit = Fluent;
    type State = ();
    type Output = SpinButtonOutput;

    kit::widget_meta!(salt, enabled, frame_sense);

    fn min_target(&self, _: &MineralUi<'_, Self::Kit>) -> MinTarget {
        MinTarget::None
    }

    fn layout(&self, _: &MineralUi<'_, Fluent>, c: Constraints, (): &()) -> Measure {
        let width = if c.max.x.is_finite() {
            c.max.x
        } else {
            inp::DEFAULT_WIDTH
        };
        let height = match self.field_size() {
            Size::Small => sp::HEIGHT_SMALL,
            _ => sp::HEIGHT_MEDIUM,
        };
        c.constrain(vec2(width, height)).into()
    }

    fn update(
        &mut self,
        ui: &mut MineralUi<'_, Fluent>,
        frame: &WidgetFrame<'_>,
        (): &mut (),
    ) -> WidgetResponse<SpinButtonOutput> {
        let rt = ui.rt();
        let ctx = rt.ctx();
        let t = ui.tokens();
        let dir = ui.i18n().direction();
        let base = ui.make_persistent_id(self.salt.id_salt());
        let (edit_id, state_id) = (base.with("edit"), base.with("number"));
        let visual = frame.zones.visual;
        let before = self.value.get();
        let focused = ctx.memory(|m| m.has_focus(edit_id));
        let escape = focused && !self.disabled && kit::layer::consume_escape_from(ctx, edit_id);
        let fmt = |v: f64| self.text(rt.i18n(), v);
        let mut edit = NumberEdit::load_or(ctx, state_id, || NumberEdit::new(self.range, before, fmt));
        edit.set_scrub_rate(self.scrub.unwrap_or_default());
        let mut pass = edit.begin(self.range, &self.value, FieldFocus::of(focused || escape), &fmt);
        let keys = focused && !self.disabled && pass.consume_step_keys(ctx);
        let mut stepped = keys;
        let mut event = escape
            .then(|| {
                ctx.memory_mut(|m| m.surrender_focus(edit_id));
                pass.cancel()
            })
            .flatten();

        let chrome = self.chrome();
        let live = Live {
            hovered: frame.response.contains_pointer(),
            pressed: frame.response.contains_pointer() && ctx.input(|i| i.pointer.primary_down()),
            focused,
        };
        textbox::paint_frame(ui, base, visual, chrome, live, &t);
        let column = button_column(visual, dir);
        let h = self.button_height();
        let parts = [
            (Rect::from_min_size(column.min, vec2(column.width(), h)), true),
            (
                Rect::from_min_size(Pos2::new(column.min.x, column.max.y - h), vec2(column.width(), h)),
                false,
            ),
        ];
        let text_rect = self.paint_content(ui, chrome, visual, column, pass.text_mut().is_empty(), &t);
        let te = Edit {
            id: edit_id,
            rect: text_rect,
            format: chrome.text_format(ctx, &t),
            t: &t,
            mask: false,
            rows: None,
        }
        .show(ui, pass.text_mut(), |e| {
            e.interactive(!self.disabled).event_filter(egui::EventFilter {
                tab: false,
                horizontal_arrows: true,
                vertical_arrows: true,
                escape: true,
            })
        });
        let scrub = self.scrub_handle(ui, text_rect, base.with("scrub"), focused);
        if !focused
            && !self.disabled
            && (frame.response.clicked() || scrub.as_ref().is_some_and(egui::Response::clicked))
        {
            ctx.memory_mut(|m| m.request_focus(edit_id));
        }
        self.paint_unit(ui, chrome, text_rect, pass.text_mut(), &t);
        let steps = self.step_buttons(ui, base, parts, &t);
        if !self.disabled {
            if te.changed() {
                pass.apply_text(rt.i18n().format().as_ref());
            }
            if steps != 0 {
                pass.step(steps, NumberSource::Step);
                stepped = true;
            }
            let select = keys || steps != 0;
            stepped |= pass.accesskit(rt.core(), te.id);
            let extent = scrub.as_ref().map_or(1.0, |h| h.rect.width());
            stepped |= pass.scrub(scrub.as_ref(), extent, Track::horizontal(dir));
            let flush = pass.commit_requested(ctx);
            let blurred = !ctx.memory(|m| m.has_focus(edit_id)) && !pass.scrubbing();
            if event.is_none() && (blurred || flush || stepped) {
                event = pass.commit();
            }
            if self.select_on_step && select {
                egui_mineral::edit::select_all(ctx, edit_id, pass.text_mut());
            }
        }
        let clamped = pass.clamped() && !self.disabled;
        let changed = edit.finish(&mut self.value, stepped || event == Some(EditEvent::Committed));
        edit.store(ctx, state_id);
        WidgetResponse::new(te, SpinButtonOutput { clamped }).with_changed(changed)
    }

    fn semantics(&self, i18n: &I18n, _: &WidgetResponse<SpinButtonOutput>, (): &()) -> Option<SemanticNode> {
        let v = self.value.get();
        let mut n = SemanticNode::new(self.key.id(&self.salt), Role::SpinButton, &self.label)
            .value(v.map_or_default(|v| self.text(i18n, v)))
            .ranged(
                self.range.numeric(v.unwrap_or_else(|| self.range.snap(0.0))),
                Availability::disabled_if(self.disabled),
            );
        n.state.invalid = self.invalid;
        Some(n)
    }
}

fn button_column(visual: Rect, dir: Direction) -> Rect {
    match dir {
        Direction::Ltr => Rect::from_min_max(Pos2::new(visual.right() - sp::BUTTON_WIDTH, visual.top()), visual.max),
        Direction::Rtl => Rect::from_min_max(visual.min, Pos2::new(visual.left() + sp::BUTTON_WIDTH, visual.bottom())),
    }
}

// useButtonStyles.increment `borderTopRightRadius`, decrement `borderBottomRightRadius`: the outer corners.
fn corner(up: bool, dir: Direction) -> CornerRadius {
    let r = CornerRadius::from(g::BORDER_RADIUS_MEDIUM).nw;
    let (nw, ne, sw, se) = match (up, dir) {
        (true, Direction::Ltr) => (0, r, 0, 0),
        (false, Direction::Ltr) => (0, 0, 0, r),
        (true, Direction::Rtl) => (r, 0, 0, 0),
        (false, Direction::Rtl) => (0, 0, r, 0),
    };
    CornerRadius { nw, ne, sw, se }
}
