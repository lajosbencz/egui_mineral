use egui::{OpenUrl, Stroke, text::LayoutJob};
use egui_mineral::{
    i18n::I18n,
    kit,
    kit::paint::VisualState,
    layout::{Constraints, Measure, MinTarget},
    semantic::{AgentKey, KeyedSalt, Role, SemanticNode},
    widget::{Availability, Interaction, MineralUi, MineralWidget, WidgetFrame, WidgetResponse},
};

use crate::spec::ColorToken::{
    ColorBrandForegroundLink, ColorBrandForegroundLinkHover, ColorBrandForegroundLinkPressed,
    ColorNeutralForeground2Link, ColorNeutralForeground2LinkHover, ColorNeutralForeground2LinkPressed,
    ColorNeutralForegroundDisabled,
};
use crate::{
    Fluent, FluentExt,
    spec::tokens::geometry,
    typography::TypeRamp,
    widget::{
        focus,
        state::{Pick as _, Ramp},
    },
};

/// Link `appearance`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum LinkAppearance {
    /// Brand colored.
    #[default]
    Default,
    /// `subtle`: neutral colored.
    Subtle,
}

/// Fluent `Link`: brand text, underlined while hovered or pressed. With [`TextLink::url`] activation opens it,
/// without one it is an in-app link read with `.clicked()`.
#[derive(Clone, Debug)]
#[must_use = "show with ui.mineral(..)"]
pub struct TextLink {
    salt: KeyedSalt,
    key: AgentKey,
    label: String,
    url: Option<String>,
    new_tab: bool,
    appearance: LinkAppearance,
    inline: bool,
    ramp: TypeRamp,
    disabled: bool,
}

impl TextLink {
    /// `label` is the visible text and accessible name; agent key `<id_salt>-link`.
    pub fn new(id_salt: impl Into<KeyedSalt>, label: impl Into<String>) -> Self {
        Self {
            salt: id_salt.into(),
            key: egui_mineral::agent_key!("link"),
            label: label.into(),
            url: None,
            new_tab: false,
            appearance: LinkAppearance::Default,
            inline: false,
            ramp: TypeRamp::Body1,
            disabled: false,
        }
    }

    /// Activation opens `url` (egui `OpenUrl`).
    pub fn url(mut self, url: impl Into<String>) -> Self {
        self.url = Some(url.into());
        self
    }

    /// Opens the [`Self::url`] in a new tab (egui `OpenUrl::new_tab`).
    pub fn new_tab(mut self, new_tab: bool) -> Self {
        self.new_tab = new_tab;
        self
    }

    /// Default [`LinkAppearance::Default`].
    pub fn appearance(mut self, appearance: LinkAppearance) -> Self {
        self.appearance = appearance;
        self
    }

    /// `inline`: always underlined, for links inside running text.
    pub fn inline(mut self, inline: bool) -> Self {
        self.inline = inline;
        self
    }

    /// Text style; default [`TypeRamp::Body1`] (`fontSizeBase300`, regular).
    pub fn ramp(mut self, ramp: TypeRamp) -> Self {
        self.ramp = ramp;
        self
    }

    crate::widget::setters! {
        /// Greys out the link; it cannot open.
        disabled
    }

    crate::widget::setters! {
        /// Replaces the `<id_salt>-link` agent key.
        agent_key(key)
    }

    // react-link useLinkStyles.styles.ts root, subtle, disabled
    fn ramp_colors(&self) -> Ramp {
        match self.appearance {
            LinkAppearance::Default => Ramp::some(
                ColorBrandForegroundLink,
                ColorBrandForegroundLinkHover,
                ColorBrandForegroundLinkPressed,
                ColorNeutralForegroundDisabled,
            ),
            LinkAppearance::Subtle => Ramp::some(
                ColorNeutralForeground2Link,
                ColorNeutralForeground2LinkHover,
                ColorNeutralForeground2LinkPressed,
                ColorNeutralForegroundDisabled,
            ),
        }
    }
}

impl MineralWidget for TextLink {
    type Kit = Fluent;
    type State = ();
    type Output = ();

    kit::widget_meta!(salt, enabled);

    fn min_target(&self, _: &MineralUi<'_, Self::Kit>) -> MinTarget {
        MinTarget::None
    }

    fn layout(&self, ui: &MineralUi<'_, Fluent>, constraints: Constraints, (): &()) -> Measure {
        let format = self.ramp.format(ui.ctx(), egui::Color32::PLACEHOLDER);
        let g = ui
            .painter()
            .layout_job(LayoutJob::single_section(self.label.clone(), format));
        constraints.constrain(g.size()).into()
    }

    fn update(&mut self, ui: &mut MineralUi<'_, Fluent>, frame: &WidgetFrame<'_>, (): &mut ()) -> WidgetResponse {
        let t = ui.tokens();
        let r = &frame.response;
        let rect = frame.zones.visual;
        let availability = Availability::disabled_if(self.disabled);
        kit::interact::hover_cursor(ui.core(), r, availability);
        let i = Interaction::of(r, availability);
        if let Some(url) = &self.url {
            if r.clicked() {
                ui.ctx().open_url(if self.new_tab {
                    OpenUrl::new_tab(url)
                } else {
                    OpenUrl::same_tab(url)
                });
            }
            ui.ctx().accesskit_node_builder(r.id, |b| b.set_url(url.as_str()));
        }
        let s = VisualState::of(i);
        let color = self.ramp_colors().pick(s, &t);
        let underline = s != VisualState::Disabled && (self.inline || s.engaged()) && !i.focused;
        let format = egui::TextFormat {
            underline: if underline {
                Stroke::new(geometry::STROKE_WIDTH_THIN, color)
            } else {
                Stroke::NONE
            },
            ..self.ramp.format(ui.ctx(), color)
        };
        let g = ui
            .painter()
            .layout_job(LayoutJob::single_section(self.label.clone(), format));
        let pos = rect.center() - g.size() / 2.0;
        let baseline = pos.y + g.rows.first().map_or(g.size().y, egui::epaint::text::PlacedRow::max_y);
        ui.painter().galley(pos, g, color);
        if i.focused {
            focus::double_underline(ui.painter(), rect.left(), rect.right(), baseline, &t);
        }
        WidgetResponse::new(r.clone(), ())
    }

    fn semantics(&self, _: &I18n, _: &WidgetResponse, (): &()) -> Option<SemanticNode> {
        let mut n = SemanticNode::new(self.key.id(&self.salt), Role::Link, &self.label)
            .clickable(Availability::disabled_if(self.disabled));
        n.value.clone_from(&self.url);
        Some(n)
    }
}
