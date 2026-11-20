use egui::{Rangef, Stroke, pos2, text::LayoutJob};
use egui_mineral::{
    i18n::{Direction, I18n},
    kit,
    layout::{Axis, Constraints, Measure},
    semantic::{AgentKey, KeyedSalt, Role, SemanticNode},
    widget::{MineralUi, MineralWidget, WidgetFrame, WidgetResponse},
};

use crate::spec::ColorToken::{
    ColorBrandForeground1, ColorBrandStroke1, ColorNeutralForeground1, ColorNeutralForeground2,
    ColorNeutralForeground3, ColorNeutralStroke1, ColorNeutralStroke2, ColorNeutralStroke3,
};
use crate::{
    Fluent, FluentExt,
    spec::ColorToken,
    spec::{defaults::divider as d, tokens::geometry},
    typography::TypeRamp,
};

/// Divider `appearance`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum DividerAppearance {
    /// `colorNeutralStroke2` line.
    #[default]
    Default,
    /// `brand`.
    Brand,
    /// `subtle`.
    Subtle,
    /// `strong`.
    Strong,
}

/// Placement of a divider's content along its line (`alignContent`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum DividerAlign {
    /// Near the start, after an 8pt line.
    Start,
    /// Centered.
    #[default]
    Center,
    /// Near the end, before an 8pt line.
    End,
}

/// Fluent `Divider`: a line across the main axis, optionally with centered or edge content text.
#[derive(Clone, Debug)]
#[must_use = "show with ui.mineral(..)"]
pub struct Divider {
    salt: KeyedSalt,
    key: AgentKey,
    content: Option<String>,
    axis: Axis,
    appearance: DividerAppearance,
    align: DividerAlign,
    inset: bool,
}

impl Divider {
    /// Horizontal, edge to edge; agent key `<id_salt>-divider` once it has content.
    pub fn new(id_salt: impl Into<KeyedSalt>) -> Self {
        Self {
            salt: id_salt.into(),
            key: egui_mineral::agent_key!("divider"),
            content: None,
            axis: Axis::Horizontal,
            appearance: DividerAppearance::Default,
            align: DividerAlign::Center,
            inset: false,
        }
    }

    /// Text between the line halves.
    pub fn content(mut self, text: impl Into<String>) -> Self {
        self.content = Some(text.into());
        self
    }

    /// Default [`Axis::Horizontal`]; vertical is `vertical`.
    pub fn axis(mut self, axis: Axis) -> Self {
        self.axis = axis;
        self
    }

    /// Default [`DividerAppearance::Default`].
    pub fn appearance(mut self, appearance: DividerAppearance) -> Self {
        self.appearance = appearance;
        self
    }

    /// Default [`DividerAlign::Center`].
    pub fn align_content(mut self, align: DividerAlign) -> Self {
        self.align = align;
        self
    }

    /// `inset`: 12pt free at both ends of the line.
    pub fn inset(mut self, inset: bool) -> Self {
        self.inset = inset;
        self
    }

    crate::widget::setters! {
        /// Replaces the `<id_salt>-divider` agent key.
        agent_key(key)
    }

    // react-divider useDividerStyles.styles.ts useBaseStyles appearances
    const fn colors(&self) -> (ColorToken, ColorToken) {
        match self.appearance {
            DividerAppearance::Default => (ColorNeutralForeground2, ColorNeutralStroke2),
            DividerAppearance::Brand => (ColorBrandForeground1, ColorBrandStroke1),
            DividerAppearance::Subtle => (ColorNeutralForeground3, ColorNeutralStroke3),
            DividerAppearance::Strong => (ColorNeutralForeground1, ColorNeutralStroke1),
        }
    }

    fn galley(&self, ui: &MineralUi<'_, Fluent>, color: egui::Color32) -> Option<std::sync::Arc<egui::Galley>> {
        let text = self.content.as_ref()?;
        let format = TypeRamp::Caption1.format(ui.ctx(), color);
        Some(ui.painter().layout_job(LayoutJob::single_section(text.clone(), format)))
    }
}

impl MineralWidget for Divider {
    type Kit = Fluent;
    type State = ();
    type Output = ();

    kit::widget_meta!(salt, hover, no_target);

    /// Fills the main axis.
    fn layout(&self, ui: &MineralUi<'_, Fluent>, c: Constraints, (): &()) -> Measure {
        let content = self.galley(ui, egui::Color32::PLACEHOLDER).map(|g| g.size());
        let size = match self.axis {
            Axis::Horizontal => {
                let w = if c.max.x.is_finite() {
                    c.max.x
                } else {
                    ui.available_width()
                };
                egui::vec2(w, content.map_or(geometry::STROKE_WIDTH_THIN, |s| s.y))
            }
            Axis::Vertical => {
                let min = if content.is_some() {
                    d::VERTICAL_MIN_HEIGHT_WITH_CONTENT
                } else {
                    d::VERTICAL_MIN_HEIGHT
                };
                let inset = if self.inset { d::INSET_SPACING * 2.0 } else { 0.0 };
                egui::vec2(
                    content.map_or(geometry::STROKE_WIDTH_THIN, |s| s.x),
                    min.max(c.min.y) + inset,
                )
            }
        };
        c.constrain(size).into()
    }

    fn update(&mut self, ui: &mut MineralUi<'_, Fluent>, frame: &WidgetFrame<'_>, (): &mut ()) -> WidgetResponse {
        let t = ui.tokens();
        let (fg, line) = self.colors();
        let stroke = Stroke::new(geometry::STROKE_WIDTH_THIN, t.color(line));
        let axis = self.axis;
        let rect = frame.zones.visual;
        let inset = if self.inset { d::INSET_SPACING } else { 0.0 };
        let main = axis.main_range(rect);
        let main = Rangef::new(main.min + inset, (main.max - inset).max(main.min + inset));
        let cross = axis.cross_range(rect).center();
        let ltr = ui.i18n().direction() == Direction::Ltr || axis == Axis::Vertical;
        let galley = self.galley(ui, t.color(fg));
        let p = ui.painter();
        let line_at = |p: &egui::Painter, r: Rangef| {
            if r.span() > 0.0 {
                match axis {
                    Axis::Horizontal => p.hline(r, cross, stroke),
                    Axis::Vertical => p.vline(cross, r, stroke),
                };
            }
        };
        let Some(g) = galley else {
            line_at(p, main);
            return WidgetResponse::new(frame.response.clone(), ());
        };
        let len = axis.main(g.size());
        let free = (main.span() - len - d::CONTENT_SPACING * 2.0).max(0.0);
        let lead = match (self.align, ltr) {
            (DividerAlign::Center, _) => free / 2.0,
            (DividerAlign::Start, true) | (DividerAlign::End, false) => d::START_END_LENGTH.min(free),
            (DividerAlign::End, true) | (DividerAlign::Start, false) => (free - d::START_END_LENGTH).max(0.0),
        };
        let text_start = main.min + lead + d::CONTENT_SPACING;
        line_at(p, Rangef::new(main.min, main.min + lead));
        line_at(p, Rangef::new(text_start + len + d::CONTENT_SPACING, main.max));
        let pos = match axis {
            Axis::Horizontal => pos2(text_start, rect.center().y - g.size().y / 2.0),
            Axis::Vertical => pos2(rect.center().x - g.size().x / 2.0, text_start),
        };
        let color = t.color(fg);
        p.galley(pos, g, color);
        WidgetResponse::new(frame.response.clone(), ())
    }

    fn semantics(&self, _: &I18n, _: &WidgetResponse, (): &()) -> Option<SemanticNode> {
        let text = self.content.as_ref()?;
        Some(SemanticNode::new(self.key.id(&self.salt), Role::Label, text))
    }
}
