use std::num::NonZeroU8;

use egui::{Pos2, Rect, vec2};
use egui_mineral::{
    i18n::{I18n, Sides},
    kit,
    layout::{Constraints, Measure, MinTarget},
    semantic::{AgentKey, KeyedSalt, Role, SemanticNode},
    widget::{Availability, MineralUi, MineralWidget, WidgetFrame, WidgetResponse},
};

use super::textbox::{self, Chrome, Content, FieldOutput, Fill, InputAppearance};
use crate::{
    Fluent, Size,
    spec::{
        defaults::{input as inp, textarea as ta},
        tokens::geometry as g,
    },
};

/// Fluent `Textarea`: multi-line text bound to `value`; typing reports `changed`.
#[derive(Debug)]
#[must_use = "show with ui.mineral(..)"]
pub struct Textarea<'a> {
    salt: KeyedSalt,
    key: AgentKey,
    label: String,
    value: &'a mut String,
    appearance: InputAppearance,
    size: Size,
    rows: NonZeroU8,
    placeholder: Option<String>,
    invalid: bool,
    disabled: bool,
}

impl<'a> Textarea<'a> {
    /// `label` is the accessible name; agent key `<id_salt>-textarea`.
    pub fn new(id_salt: impl Into<KeyedSalt>, label: impl Into<String>, value: &'a mut String) -> Self {
        Self {
            salt: id_salt.into(),
            key: AgentKey::Kind("textarea"),
            label: label.into(),
            value,
            appearance: InputAppearance::Outline,
            size: Size::Medium,
            rows: NonZeroU8::MIN.saturating_add(1),
            placeholder: None,
            invalid: false,
            disabled: false,
        }
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

    /// Visible lines before the size's `maxHeight`; default 2, as an HTML `<textarea>`.
    pub fn rows(mut self, rows: NonZeroU8) -> Self {
        self.rows = rows;
        self
    }

    /// Hint shown while empty.
    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.placeholder = Some(placeholder.into());
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
        /// Replaces the `<id_salt>-textarea` agent key.
        agent_key(key)
    }

    fn chrome(&self) -> Chrome {
        Chrome {
            appearance: self.appearance,
            size: self.size,
            invalid: self.invalid,
            disabled: self.disabled,
        }
    }

    // react-textarea useTextareaStyles.styles.ts small, medium, large: min and max height, padding (v, h).
    fn spec(&self) -> (f32, f32, f32, f32) {
        match self.size {
            Size::Small => (
                ta::MIN_HEIGHT_SMALL,
                ta::MAX_HEIGHT_SMALL,
                g::SPACING_VERTICAL_XS,
                g::SPACING_HORIZONTAL_S_NUDGE + g::SPACING_HORIZONTAL_XXS,
            ),
            Size::Medium => (
                ta::MIN_HEIGHT_MEDIUM,
                ta::MAX_HEIGHT_MEDIUM,
                g::SPACING_VERTICAL_S_NUDGE,
                g::SPACING_HORIZONTAL_M_NUDGE + g::SPACING_HORIZONTAL_XXS,
            ),
            Size::Large => (
                ta::MIN_HEIGHT_LARGE,
                ta::MAX_HEIGHT_LARGE,
                g::SPACING_VERTICAL_S,
                g::SPACING_HORIZONTAL_M + g::SPACING_HORIZONTAL_XXS,
            ),
        }
    }

    // Border box: the textarea, `padding-bottom` for the focus underline and both borders.
    fn height(&self) -> f32 {
        let (min, max, pad, _) = self.spec();
        let lines = f32::from(self.rows.get()) * self.chrome().metrics().line + 2.0 * pad;
        lines.clamp(min, max) + ta::FOCUS_UNDERLINE + 2.0 * g::STROKE_WIDTH_THIN
    }
}

impl MineralWidget for Textarea<'_> {
    type Kit = Fluent;
    type State = ();
    type Output = FieldOutput;

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
        c.constrain(vec2(width, self.height())).into()
    }

    fn update(
        &mut self,
        ui: &mut MineralUi<'_, Fluent>,
        frame: &WidgetFrame<'_>,
        (): &mut (),
    ) -> WidgetResponse<FieldOutput> {
        let base = ui.make_persistent_id(self.salt.id_salt());
        let visual = frame.zones.visual;
        let (_, _, pad_v, pad_h) = self.spec();
        let inner = visual.shrink(g::STROKE_WIDTH_THIN);
        let Sides { left, right } = ui.i18n().direction().sides(pad_h, pad_h);
        let text = Rect::from_min_max(
            Pos2::new(inner.left() + left, inner.top() + pad_v),
            Pos2::new(
                (inner.right() - right).max(inner.left() + left),
                (inner.bottom() - ta::FOCUS_UNDERLINE - pad_v).max(inner.top() + pad_v),
            ),
        );
        let lines = usize::from(self.rows.get());
        let shown = textbox::show(
            ui,
            self.value,
            &frame.response,
            &Fill {
                base,
                frame: visual,
                chrome: self.chrome(),
                content: Content {
                    before: None,
                    after: None,
                },
                placeholder: self.placeholder.as_deref(),
                mask: false,
                rows: Some(lines),
                reserve_end: 0.0,
                text_rect: Some(text),
                metrics: None,
            },
        );
        let out = FieldOutput {
            submitted: false,
            blurred: shown.response.lost_focus(),
        };
        let changed = shown.response.changed();
        WidgetResponse::new(shown.response, out).with_changed(changed)
    }

    fn semantics(&self, _: &I18n, _: &WidgetResponse<FieldOutput>, (): &()) -> Option<SemanticNode> {
        let mut n = SemanticNode::new(self.key.id(&self.salt), Role::MultilineTextInput, &self.label)
            .value(self.value.clone())
            .editable(Availability::disabled_if(self.disabled));
        n.state.invalid = self.invalid;
        Some(n)
    }
}
