use egui::{CornerRadius, Pos2, Rangef, Rect, Sense, vec2};
use egui_mineral::{
    edit::{EditEvent, FieldFocus},
    i18n::{Direction, I18n},
    kit::{self, state::TempState},
    layout::{Constraints, Measure, MinTarget},
    number::{NumberEdit, NumberSource},
    range::RangeModel,
    semantic::{AgentKey, KeyedSalt, Role, SemanticNode},
    widget::{Availability, MineralUi, MineralWidget, WidgetFrame, WidgetResponse},
};

use super::{
    bar::digits,
    textbox::{self, Chrome, Edit, InputAppearance, Live},
};
use crate::ColorToken::{
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

/// Fluent `SpinButton`: a number field with step buttons, bound to `value`. Typed text commits on Enter or blur,
/// arrow and page keys and the buttons step; `changed` reports a committed value.
#[derive(Debug)]
#[must_use = "show with ui.mineral(..)"]
pub struct SpinButton<'a> {
    salt: KeyedSalt,
    key: AgentKey,
    label: String,
    value: &'a mut f64,
    range: RangeModel,
    appearance: InputAppearance,
    size: Size,
    invalid: bool,
    disabled: bool,
}

impl<'a> SpinButton<'a> {
    /// `range` sets the bounds, `step` and page `jump`; `label` is the accessible name. Agent key
    /// `<id_salt>-spin-button`.
    pub fn new(
        id_salt: impl Into<KeyedSalt>,
        label: impl Into<String>,
        range: impl Into<RangeModel>,
        value: &'a mut f64,
    ) -> Self {
        Self {
            salt: id_salt.into(),
            key: AgentKey::Kind("spin-button"),
            label: label.into(),
            value,
            range: range.into(),
            appearance: InputAppearance::Outline,
            size: Size::Medium,
            invalid: false,
            disabled: false,
        }
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

    /// `aria-invalid`: red border until focused.
    pub fn invalid(mut self, invalid: bool) -> Self {
        self.invalid = invalid;
        self
    }

    crate::widget::setters! {
        /// Greys out the field and ignores input.
        disabled
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

impl MineralWidget for SpinButton<'_> {
    type Kit = Fluent;
    type State = ();
    type Output = ();

    kit::widget_meta!(salt, enabled, frame_sense);

    fn min_target(&self) -> MinTarget {
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

    fn update(&mut self, ui: &mut MineralUi<'_, Fluent>, frame: &WidgetFrame<'_>, (): &mut ()) -> WidgetResponse {
        let rt = ui.rt();
        let ctx = rt.ctx();
        let t = ui.tokens();
        let dir = ui.i18n().direction();
        let base = ui.make_persistent_id(self.salt.id_salt());
        let (edit_id, state_id) = (base.with("edit"), base.with("number"));
        let visual = frame.zones.visual;
        let before = *self.value;
        let focused = ctx.memory(|m| m.has_focus(edit_id));
        let escape = focused && !self.disabled && kit::overlay::consume_escape_from(ctx, edit_id);
        let fmt = |v: f64| self.text(rt.i18n(), v);
        let mut edit = NumberEdit::load_or(ctx, state_id, || NumberEdit::new(self.range, before, fmt));
        let mut pass = edit.begin(self.range, before, FieldFocus::of(focused || escape), &fmt);
        let mut stepped = focused && !self.disabled && pass.consume_step_keys(ctx);
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
        let inner = visual.shrink(g::STROKE_WIDTH_THIN);
        let pad = self.pad();
        let start = dir.start_x(inner, Rangef::new(pad - g::STROKE_WIDTH_THIN, pad));
        let (text_left, text_right) = match dir {
            Direction::Ltr => (start + g::STROKE_WIDTH_THIN, column.min.x - g::SPACING_HORIZONTAL_XS),
            Direction::Rtl => (column.max.x + g::SPACING_HORIZONTAL_XS, start),
        };
        let text_rect = Rect::from_min_max(
            Pos2::new(text_left, inner.top()),
            Pos2::new(text_right.max(text_left), inner.bottom()),
        );
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
        if !focused && frame.response.clicked() && !self.disabled {
            ctx.memory_mut(|m| m.request_focus(edit_id));
        }
        let steps = self.step_buttons(ui, base, parts, &t);
        if !self.disabled {
            if te.changed() {
                pass.apply_text(rt.i18n().format().as_ref());
            }
            if steps != 0 {
                pass.step(steps, NumberSource::Step);
                stepped = true;
            }
            stepped |= pass.accesskit(rt.core(), te.id);
            let flush = pass.commit_requested(ctx);
            let blurred = !ctx.memory(|m| m.has_focus(edit_id));
            if event.is_none() && (blurred || flush || stepped) {
                event = pass.commit();
            }
        }
        let value = pass.value();
        edit.store(ctx, state_id);
        if stepped || event == Some(EditEvent::Committed) {
            *self.value = self.range.clamp(value);
        }
        let changed = (before - *self.value).abs() > f64::EPSILON;
        WidgetResponse::new(te, ()).with_changed(changed)
    }

    fn semantics(&self, i18n: &I18n, _: &WidgetResponse, (): &()) -> Option<SemanticNode> {
        let mut n = SemanticNode::new(self.key.id(&self.salt), Role::SpinButton, &self.label)
            .value(self.text(i18n, *self.value))
            .ranged(
                self.range.numeric(*self.value),
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
