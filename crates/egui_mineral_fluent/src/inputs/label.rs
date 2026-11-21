use crate::msg;
use egui::{TextFormat, text::LayoutJob};
use egui_mineral::{
    i18n::I18n,
    kit,
    layout::{Constraints, Measure},
    semantic::{AgentKey, KeyedSalt, Role, SemanticNode},
    widget::{MineralUi, MineralWidget, WidgetFrame, WidgetResponse},
};

use crate::spec::ColorToken::{ColorNeutralForeground1, ColorNeutralForegroundDisabled, ColorPaletteRedForeground3};
use crate::{
    Fluent, FluentExt, Size,
    spec::tokens::{geometry, typography as ty},
};

/// Fluent `Label`: the name of a form field, with an optional required mark.
#[derive(Clone, Debug)]
#[must_use = "show with ui.mineral(..)"]
pub struct FieldLabel {
    salt: KeyedSalt,
    key: AgentKey,
    text: String,
    size: Size,
    semibold: bool,
    required: bool,
    disabled: bool,
}

impl FieldLabel {
    /// Agent key `<id_salt>-label`.
    pub fn new(id_salt: impl Into<KeyedSalt>, text: impl Into<String>) -> Self {
        Self {
            salt: id_salt.into(),
            key: egui_mineral::agent_key!("label"),
            text: text.into(),
            size: Size::Medium,
            semibold: false,
            required: false,
            disabled: false,
        }
    }

    /// Default [`Size::Medium`]; [`Size::Large`] is semibold.
    pub fn size(mut self, size: Size) -> Self {
        self.size = size;
        self
    }

    /// `weight="semibold"`.
    pub fn semibold(mut self, semibold: bool) -> Self {
        self.semibold = semibold;
        self
    }

    /// Appends the `colorPaletteRedForeground3` required mark.
    pub fn required(mut self, required: bool) -> Self {
        self.required = required;
        self
    }

    crate::widget::setters! {
        /// `colorNeutralForegroundDisabled` text.
        disabled
    }

    crate::widget::setters! {
        /// Replaces the `<id_salt>-label` agent key.
        agent_key(key)
    }

    // react-label useLabelStyles.styles.ts
    fn job(&self, ui: &MineralUi<'_, Fluent>, colors: Option<(egui::Color32, egui::Color32)>) -> LayoutJob {
        let (size, line_height, weight) = match self.size {
            Size::Small => (
                ty::FONT_SIZE_BASE_200,
                ty::LINE_HEIGHT_BASE_200,
                ty::FONT_WEIGHT_REGULAR,
            ),
            Size::Medium => (
                ty::FONT_SIZE_BASE_300,
                ty::LINE_HEIGHT_BASE_300,
                ty::FONT_WEIGHT_REGULAR,
            ),
            Size::Large => (
                ty::FONT_SIZE_BASE_400,
                ty::LINE_HEIGHT_BASE_400,
                ty::FONT_WEIGHT_SEMIBOLD,
            ),
        };
        let weight = if self.semibold {
            ty::FONT_WEIGHT_SEMIBOLD
        } else {
            weight
        };
        let (text, mark) = colors.unwrap_or((egui::Color32::PLACEHOLDER, egui::Color32::PLACEHOLDER));
        let format = crate::typography::format(
            ui.ctx(),
            crate::typography::TypeSpec {
                size,
                line_height,
                weight,
            },
            text,
        );
        let mut job = LayoutJob::default();
        job.append(&self.text, 0.0, format.clone());
        if self.required {
            let mark_text = crate::tr(ui.i18n(), msg::REQUIRED_MARK);
            job.append(
                &mark_text,
                geometry::SPACING_HORIZONTAL_XS,
                TextFormat { color: mark, ..format },
            );
        }
        job
    }
}

#[cfg(feature = "extras")]
impl FieldLabel {
    pub(crate) fn natural_width(&self, ui: &MineralUi<'_, Fluent>) -> f32 {
        ui.painter().layout_job(self.job(ui, None)).size().x
    }
}

impl MineralWidget for FieldLabel {
    type Kit = Fluent;
    type State = ();
    type Output = ();

    kit::widget_meta!(salt, hover, no_target);

    fn layout(&self, ui: &MineralUi<'_, Fluent>, c: Constraints, (): &()) -> Measure {
        let g = ui.painter().layout_job(self.job(ui, None));
        c.constrain(g.size()).into()
    }

    fn update(&mut self, ui: &mut MineralUi<'_, Fluent>, frame: &WidgetFrame<'_>, (): &mut ()) -> WidgetResponse {
        let t = ui.tokens();
        let (text, mark) = if self.disabled {
            (ColorNeutralForegroundDisabled, ColorNeutralForegroundDisabled)
        } else {
            (ColorNeutralForeground1, ColorPaletteRedForeground3)
        };
        let job = self.job(ui, Some((t.color(text), t.color(mark))));
        let g = ui.painter().layout_job(job);
        let rect = frame.zones.visual;
        ui.painter().galley(rect.left_top(), g, t.color(text));
        WidgetResponse::new(frame.response.clone(), ())
    }

    fn semantics(&self, _: &I18n, _: &WidgetResponse, (): &()) -> Option<SemanticNode> {
        let mut n = SemanticNode::new(self.key.id(&self.salt), Role::Label, &self.text);
        n.state.disabled = self.disabled;
        Some(n)
    }
}
