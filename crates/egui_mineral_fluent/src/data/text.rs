use std::sync::Arc;

use egui::{Align, FontFamily, FontId, Galley, Stroke, TextFormat, text::LayoutJob};
use egui_mineral::{
    i18n::{Direction, I18n},
    kit,
    layout::{Constraints, Measure},
    semantic::{AgentKey, KeyedSalt, Role, SemanticNode},
    widget::{MineralUi, MineralWidget, WidgetFrame, WidgetResponse},
};

use crate::{
    Fluent, FluentExt,
    spec::ColorToken,
    spec::tokens::{geometry, typography as ty},
    typography::TypeRamp,
};

/// Text `size`: the `fontSizeBase*` / `fontSizeHero*` step with its line height.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum TextSize {
    /// `100`.
    S100,
    /// `200`.
    S200,
    /// `300`.
    #[default]
    S300,
    /// `400`.
    S400,
    /// `500`.
    S500,
    /// `600`.
    S600,
    /// `700`.
    S700,
    /// `800`.
    S800,
    /// `900`.
    S900,
    /// `1000`.
    S1000,
}

/// Font metrics of a [`TextSize`], in points.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct TextMetrics {
    /// Font size.
    pub font_size: f32,
    /// Line height.
    pub line_height: f32,
}

impl TextSize {
    /// Font size and line height.
    pub const fn metrics(self) -> TextMetrics {
        let (font_size, line_height) = match self {
            Self::S100 => (ty::FONT_SIZE_BASE_100, ty::LINE_HEIGHT_BASE_100),
            Self::S200 => (ty::FONT_SIZE_BASE_200, ty::LINE_HEIGHT_BASE_200),
            Self::S300 => (ty::FONT_SIZE_BASE_300, ty::LINE_HEIGHT_BASE_300),
            Self::S400 => (ty::FONT_SIZE_BASE_400, ty::LINE_HEIGHT_BASE_400),
            Self::S500 => (ty::FONT_SIZE_BASE_500, ty::LINE_HEIGHT_BASE_500),
            Self::S600 => (ty::FONT_SIZE_BASE_600, ty::LINE_HEIGHT_BASE_600),
            Self::S700 => (ty::FONT_SIZE_HERO_700, ty::LINE_HEIGHT_HERO_700),
            Self::S800 => (ty::FONT_SIZE_HERO_800, ty::LINE_HEIGHT_HERO_800),
            Self::S900 => (ty::FONT_SIZE_HERO_900, ty::LINE_HEIGHT_HERO_900),
            Self::S1000 => (ty::FONT_SIZE_HERO_1000, ty::LINE_HEIGHT_HERO_1000),
        };
        TextMetrics { font_size, line_height }
    }
}

/// Text `weight`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum TextWeight {
    /// `fontWeightRegular`.
    #[default]
    Regular,
    /// `fontWeightMedium`.
    Medium,
    /// `fontWeightSemibold`.
    Semibold,
    /// `fontWeightBold`.
    Bold,
}

impl TextWeight {
    /// CSS weight.
    pub const fn value(self) -> u16 {
        match self {
            Self::Regular => ty::FONT_WEIGHT_REGULAR,
            Self::Medium => ty::FONT_WEIGHT_MEDIUM,
            Self::Semibold => ty::FONT_WEIGHT_SEMIBOLD,
            Self::Bold => ty::FONT_WEIGHT_BOLD,
        }
    }
}

/// Text `font`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum TextFont {
    /// `fontFamilyBase`.
    #[default]
    Base,
    /// `fontFamilyMonospace`.
    Monospace,
    /// `fontFamilyNumeric`.
    Numeric,
}

/// Text `align`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum TextAlign {
    /// `start`.
    #[default]
    Start,
    /// `center`.
    Center,
    /// `end`.
    End,
    /// `justify`.
    Justify,
}

/// Fluent `Text`: typographic text in `colorNeutralForeground1`, wrapping by default.
#[derive(Clone, Debug)]
#[must_use = "show with ui.mineral(..)"]
pub struct Text {
    salt: KeyedSalt,
    key: AgentKey,
    content: String,
    size: TextSize,
    weight: TextWeight,
    font: TextFont,
    align: TextAlign,
    italic: bool,
    underline: bool,
    strikethrough: bool,
    wrap: bool,
    truncate: bool,
}

impl Text {
    /// Body text; agent key `<id_salt>-text`.
    pub fn new(id_salt: impl Into<KeyedSalt>, text: impl Into<String>) -> Self {
        Self {
            salt: id_salt.into(),
            key: egui_mineral::agent_key!("text"),
            content: text.into(),
            size: TextSize::S300,
            weight: TextWeight::Regular,
            font: TextFont::Base,
            align: TextAlign::Start,
            italic: false,
            underline: false,
            strikethrough: false,
            wrap: true,
            truncate: false,
        }
    }

    /// Size and weight of a type ramp preset (`Body1`, `Title3`, ...).
    pub fn ramp(mut self, ramp: TypeRamp) -> Self {
        let spec = ramp.spec();
        self.size = match ramp {
            TypeRamp::Caption2 | TypeRamp::Caption2Strong => TextSize::S100,
            TypeRamp::Caption1 | TypeRamp::Caption1Strong | TypeRamp::Caption1Stronger => TextSize::S200,
            TypeRamp::Body1 | TypeRamp::Body1Strong | TypeRamp::Body1Stronger => TextSize::S300,
            TypeRamp::Body2 | TypeRamp::Subtitle2 | TypeRamp::Subtitle2Stronger => TextSize::S400,
            TypeRamp::Subtitle1 => TextSize::S500,
            TypeRamp::Title3 => TextSize::S600,
            TypeRamp::Title2 => TextSize::S700,
            TypeRamp::Title1 => TextSize::S800,
            TypeRamp::LargeTitle => TextSize::S900,
            TypeRamp::Display => TextSize::S1000,
        };
        self.weight = match spec.weight {
            w if w >= ty::FONT_WEIGHT_BOLD => TextWeight::Bold,
            w if w >= ty::FONT_WEIGHT_SEMIBOLD => TextWeight::Semibold,
            w if w >= ty::FONT_WEIGHT_MEDIUM => TextWeight::Medium,
            _ => TextWeight::Regular,
        };
        self
    }

    /// Default [`TextSize::S300`].
    pub fn size(mut self, size: TextSize) -> Self {
        self.size = size;
        self
    }

    /// Default [`TextWeight::Regular`].
    pub fn weight(mut self, weight: TextWeight) -> Self {
        self.weight = weight;
        self
    }

    /// Default [`TextFont::Base`].
    pub fn font(mut self, font: TextFont) -> Self {
        self.font = font;
        self
    }

    /// Default [`TextAlign::Start`]; mirrored in right-to-left locales.
    pub fn align(mut self, align: TextAlign) -> Self {
        self.align = align;
        self
    }

    /// `italic`.
    pub fn italic(mut self, italic: bool) -> Self {
        self.italic = italic;
        self
    }

    /// `underline`.
    pub fn underline(mut self, underline: bool) -> Self {
        self.underline = underline;
        self
    }

    /// `strikethrough`.
    pub fn strikethrough(mut self, strikethrough: bool) -> Self {
        self.strikethrough = strikethrough;
        self
    }

    /// `wrap`; `false` keeps one clipped line. Default `true`.
    pub fn wrap(mut self, wrap: bool) -> Self {
        self.wrap = wrap;
        self
    }

    /// `truncate`: an unwrapped line ends in an ellipsis where it is cut.
    pub fn truncate(mut self, truncate: bool) -> Self {
        self.truncate = truncate;
        self
    }

    crate::widget::setters! {
        /// Replaces the `<id_salt>-text` agent key.
        agent_key(key)
    }

    fn galley(&self, ui: &MineralUi<'_, Fluent>, width: f32, color: egui::Color32) -> Arc<Galley> {
        let TextMetrics {
            font_size: size,
            line_height,
        } = self.size.metrics();
        let font = match self.font {
            TextFont::Monospace => FontId::new(size, FontFamily::Monospace),
            TextFont::Base | TextFont::Numeric => crate::typography::font(ui.ctx(), size, self.weight.value()),
        };
        let decoration = |on: bool| {
            if on {
                Stroke::new(geometry::STROKE_WIDTH_THIN, color)
            } else {
                Stroke::NONE
            }
        };
        let format = TextFormat {
            line_height: Some(line_height),
            italics: self.italic,
            underline: decoration(self.underline),
            strikethrough: decoration(self.strikethrough),
            ..TextFormat::simple(font, color)
        };
        let mut job = LayoutJob::single_section(self.content.clone(), format);
        job.wrap.max_width = width;
        if !self.wrap {
            job.wrap.max_rows = 1;
            job.wrap.break_anywhere = true;
            job.wrap.overflow_character = self.truncate.then_some('\u{2026}');
        }
        let rtl = ui.i18n().direction() == Direction::Rtl;
        job.halign = match (self.align, rtl) {
            (TextAlign::Start | TextAlign::Justify, false) | (TextAlign::End, true) => Align::LEFT,
            (TextAlign::Center, _) => Align::Center,
            (TextAlign::End, false) | (TextAlign::Start | TextAlign::Justify, true) => Align::RIGHT,
        };
        job.justify = self.align == TextAlign::Justify;
        ui.painter().layout_job(job)
    }
}

impl TextSize {
    /// Every variant, smallest first.
    pub const ALL: &'static [Self] = &[
        Self::S100,
        Self::S200,
        Self::S300,
        Self::S400,
        Self::S500,
        Self::S600,
        Self::S700,
        Self::S800,
        Self::S900,
        Self::S1000,
    ];
}

impl MineralWidget for Text {
    type Kit = Fluent;
    type State = ();
    type Output = ();

    kit::widget_meta!(salt, hover, no_target);

    fn layout(&self, ui: &MineralUi<'_, Fluent>, c: Constraints, (): &()) -> Measure {
        let width = if c.max.x.is_finite() {
            c.max.x
        } else {
            ui.available_width()
        };
        let g = self.galley(ui, width, egui::Color32::PLACEHOLDER);
        let w = if self.align == TextAlign::Start {
            g.size().x
        } else {
            width.max(g.size().x)
        };
        c.constrain(egui::vec2(w, g.size().y)).into()
    }

    fn update(&mut self, ui: &mut MineralUi<'_, Fluent>, frame: &WidgetFrame<'_>, (): &mut ()) -> WidgetResponse {
        let t = ui.tokens();
        let rect = frame.zones.visual;
        let color = t.color(ColorToken::ColorNeutralForeground1);
        let g = self.galley(ui, rect.width(), color);
        let x = match g.job.halign {
            Align::Min => rect.left(),
            Align::Center => rect.center().x,
            Align::Max => rect.right(),
        };
        ui.painter()
            .with_clip_rect(rect)
            .galley(egui::pos2(x, rect.top()), g, color);
        WidgetResponse::new(frame.response.clone(), ())
    }

    fn semantics(&self, _: &I18n, _: &WidgetResponse, (): &()) -> Option<SemanticNode> {
        Some(SemanticNode::new(self.key.id(&self.salt), Role::Label, &self.content))
    }
}
