use egui_mineral::{
    i18n::I18n,
    kit,
    layout::{Constraints, Measure, MinTarget},
    semantic::{AgentKey, KeyedSalt, Role, SemanticNode},
    widget::{Availability, MineralUi, MineralWidget, WidgetFrame, WidgetResponse},
};

use super::textbox::{self, Chrome, Content, FieldOutput, Fill, InputAppearance};
use crate::{Fluent, Icon, Size, spec::defaults::input as inp};

/// Fluent `Input`: single-line text bound to `value`; typing reports `changed`.
#[derive(Debug)]
#[must_use = "show with ui.mineral(..)"]
pub struct Input<'a> {
    salt: KeyedSalt,
    key: AgentKey,
    label: String,
    value: &'a mut String,
    appearance: InputAppearance,
    size: Size,
    placeholder: Option<String>,
    before: Option<Icon>,
    after: Option<Icon>,
    password: bool,
    invalid: bool,
    disabled: bool,
}

impl<'a> Input<'a> {
    /// `label` is the accessible name; agent key `<id_salt>-input`.
    pub fn new(id_salt: impl Into<KeyedSalt>, label: impl Into<String>, value: &'a mut String) -> Self {
        Self {
            salt: id_salt.into(),
            key: AgentKey::Kind("input"),
            label: label.into(),
            value,
            appearance: InputAppearance::Outline,
            size: Size::Medium,
            placeholder: None,
            before: None,
            after: None,
            password: false,
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

    /// Hint shown while empty.
    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.placeholder = Some(placeholder.into());
        self
    }

    /// `contentBefore`: icon leading the text.
    pub fn content_before(mut self, icon: impl Into<Icon>) -> Self {
        self.before = Some(icon.into());
        self
    }

    /// `contentAfter`: icon trailing the text.
    pub fn content_after(mut self, icon: impl Into<Icon>) -> Self {
        self.after = Some(icon.into());
        self
    }

    /// `type="password"`: masks the text.
    pub fn password(mut self, password: bool) -> Self {
        self.password = password;
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
        /// Replaces the `<id_salt>-input` agent key.
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
}

impl MineralWidget for Input<'_> {
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
        c.constrain(egui::vec2(width, self.chrome().metrics().height)).into()
    }

    fn update(
        &mut self,
        ui: &mut MineralUi<'_, Fluent>,
        frame: &WidgetFrame<'_>,
        (): &mut (),
    ) -> WidgetResponse<FieldOutput> {
        let base = ui.make_persistent_id(self.salt.id_salt());
        let shown = textbox::show(
            ui,
            self.value,
            &frame.response,
            &Fill {
                base,
                frame: frame.zones.visual,
                chrome: self.chrome(),
                content: Content {
                    before: self.before,
                    after: self.after,
                },
                placeholder: self.placeholder.as_deref(),
                mask: self.password,
                rows: None,
                reserve_end: 0.0,
                text_rect: None,
                metrics: None,
            },
        );
        let out = FieldOutput {
            submitted: textbox::submitted(&shown.response),
            blurred: shown.response.lost_focus(),
        };
        let changed = shown.response.changed();
        WidgetResponse::new(shown.response, out).with_changed(changed)
    }

    fn semantics(&self, _: &I18n, _: &WidgetResponse<FieldOutput>, (): &()) -> Option<SemanticNode> {
        let (role, value) = if self.password {
            (Role::PasswordInput, "*".repeat(self.value.chars().count()))
        } else {
            (Role::TextInput, self.value.clone())
        };
        let mut n = SemanticNode::new(self.key.id(&self.salt), role, &self.label)
            .value(value)
            .editable(Availability::disabled_if(self.disabled));
        n.state.invalid = self.invalid;
        Some(n)
    }
}
