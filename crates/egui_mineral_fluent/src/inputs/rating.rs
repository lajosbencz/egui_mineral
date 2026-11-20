use crate::msg;
use egui::{Color32, CornerRadius, CursorIcon, Painter, Pos2, Rangef, Rect, pos2, vec2};
use egui_mineral::{
    i18n::{Direction, I18n},
    kit,
    layout::{Constraints, Measure, MinTarget},
    range::{RangeModel, Track},
    semantic::{AgentKey, KeyedSalt, Role, SemanticNode},
    widget::{Availability, Interaction, MineralUi, MineralWidget, WidgetFrame, WidgetResponse},
};

use crate::spec::ColorToken::{
    ColorBrandBackground2, ColorBrandForeground1, ColorNeutralBackground6, ColorNeutralForeground1,
    ColorPaletteMarigoldBackground2, ColorPaletteMarigoldBorderActive,
};
use crate::{
    Fluent, FluentExt,
    spec::{
        defaults::{rating as rt, rating_display as rd},
        tokens::{geometry as g, typography as ty},
    },
    theme::Tokens,
    widget::{focus, paint},
};

/// Icon size of a [`Rating`] or [`RatingDisplay`] (`size`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum RatingSize {
    /// `small`: 12.
    Small,
    /// `medium`: 16.
    #[default]
    Medium,
    /// `large`: 20.
    Large,
    /// `extra-large`: 28.
    ExtraLarge,
}

impl RatingSize {
    /// Every variant, smallest first.
    pub const ALL: &'static [Self] = &[Self::Small, Self::Medium, Self::Large, Self::ExtraLarge];

    const fn icon(self) -> f32 {
        match self {
            Self::Small => rt::ITEM_SMALL,
            Self::Medium => rt::ITEM_MEDIUM,
            Self::Large => rt::ITEM_LARGE,
            Self::ExtraLarge => rt::ITEM_EXTRA_LARGE,
        }
    }
}

/// Icon `color` of a [`Rating`] or [`RatingDisplay`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum RatingColor {
    /// `neutral`: `colorNeutralForeground1`.
    #[default]
    Neutral,
    /// `brand`: `colorBrandForeground1`.
    Brand,
    /// `marigold`: `colorPaletteMarigoldBorderActive`.
    Marigold,
}

/// `appearance` of the unselected icons.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum RatingAppearance {
    /// `outline`: outlined glyph.
    #[default]
    Outline,
    /// `filled`: filled glyph in the muted `*Background` color.
    Filled,
}

// react-rating useRatingItemStyles.styles.ts useIndicatorStyles
fn colors(color: RatingColor, appearance: RatingAppearance, t: &Tokens) -> (Color32, Color32) {
    let selected = match color {
        RatingColor::Neutral => ColorNeutralForeground1,
        RatingColor::Brand => ColorBrandForeground1,
        RatingColor::Marigold => ColorPaletteMarigoldBorderActive,
    };
    let unselected = match (appearance, color) {
        (RatingAppearance::Outline, _) => selected,
        (RatingAppearance::Filled, RatingColor::Neutral) => ColorNeutralBackground6,
        (RatingAppearance::Filled, RatingColor::Brand) => ColorBrandBackground2,
        (RatingAppearance::Filled, RatingColor::Marigold) => ColorPaletteMarigoldBackground2,
    };
    (t.color(selected), t.color(unselected))
}

struct Stars {
    max: u8,
    size: RatingSize,
    color: RatingColor,
    appearance: RatingAppearance,
}

impl Stars {
    fn star(&self, rect: Rect, dir: Direction, i: u8) -> Rect {
        let s = self.size.icon();
        let from = f32::from(i) * s;
        Rect::from_min_size(
            pos2(dir.start_x(rect, Rangef::new(from, from + s)), rect.top()),
            vec2(s, s),
        )
    }

    // `shown` stars filled, the last one partly.
    fn paint(&self, p: &Painter, icons: &crate::IconSet, rect: Rect, dir: Direction, shown: f32, t: &Tokens) {
        let (selected, unselected) = colors(self.color, self.appearance, t);
        let filled_unselected = self.appearance == RatingAppearance::Filled;
        let s = self.size.icon();
        for i in 0..self.max {
            let star = self.star(rect, dir, i);
            let part = (shown - f32::from(i)).clamp(0.0, 1.0);
            if part < 1.0 {
                paint::icon(p, star.center(), icons.star, filled_unselected, s, unselected);
            }
            if part > 0.0 {
                let w = star.width() * part;
                let clip = Rect::from_min_size(
                    pos2(dir.start_x(star, Rangef::new(0.0, w)), star.top()),
                    vec2(w, star.height()),
                );
                paint::icon(&p.with_clip_rect(clip), star.center(), icons.star, true, s, selected);
            }
        }
    }
}

/// Fluent `Rating`: stars bound to `value`, 0 for none; hover previews, a click or arrow key picks.
#[derive(Debug)]
#[must_use = "show with ui.mineral(..)"]
pub struct Rating<'a> {
    salt: KeyedSalt,
    key: AgentKey,
    label: String,
    value: &'a mut f64,
    max: u8,
    half: bool,
    size: RatingSize,
    color: RatingColor,
    appearance: RatingAppearance,
}

impl<'a> Rating<'a> {
    /// Five stars; `label` is the accessible name. Agent key `<id_salt>-rating`.
    pub fn new(id_salt: impl Into<KeyedSalt>, label: impl Into<String>, value: &'a mut f64) -> Self {
        Self {
            salt: id_salt.into(),
            key: egui_mineral::agent_key!("rating"),
            label: label.into(),
            value,
            max: 5,
            half: false,
            size: RatingSize::Medium,
            color: RatingColor::Neutral,
            appearance: RatingAppearance::Outline,
        }
    }

    /// `max`: star count, at least 1; default 5.
    pub fn count(mut self, count: u8) -> Self {
        self.max = egui_mineral::kit::sanitize::at_least(count, 1.., "Rating count");
        self
    }

    /// `step={0.5}`: half-star steps.
    pub fn half(mut self, half: bool) -> Self {
        self.half = half;
        self
    }

    /// Default [`RatingSize::Medium`].
    pub fn size(mut self, size: RatingSize) -> Self {
        self.size = size;
        self
    }

    /// Default [`RatingColor::Neutral`].
    pub fn color(mut self, color: RatingColor) -> Self {
        self.color = color;
        self
    }

    /// Default [`RatingAppearance::Outline`].
    pub fn appearance(mut self, appearance: RatingAppearance) -> Self {
        self.appearance = appearance;
        self
    }

    crate::widget::setters! {
        /// Replaces the `<id_salt>-rating` agent key.
        agent_key(key)
    }

    fn model(&self) -> RangeModel {
        RangeModel::new(0.0, f64::from(self.max)).step(if self.half { 0.5 } else { 1.0 })
    }

    fn stars(&self) -> Stars {
        Stars {
            max: self.max,
            size: self.size,
            color: self.color,
            appearance: self.appearance,
        }
    }

    fn value_at(&self, rect: Rect, p: Pos2, dir: Direction) -> f64 {
        kit::rating::value_at(
            rect,
            p,
            dir,
            self.size.icon(),
            self.max,
            kit::rating::Step::half_if(self.half),
        )
    }
}

impl MineralWidget for Rating<'_> {
    type Kit = Fluent;
    type State = ();
    type Output = ();

    kit::widget_meta!(salt);

    fn min_target(&self, _: &MineralUi<'_, Self::Kit>) -> MinTarget {
        MinTarget::None
    }

    fn layout(&self, _: &MineralUi<'_, Fluent>, c: Constraints, (): &()) -> Measure {
        let s = self.size.icon();
        c.constrain(vec2(f32::from(self.max) * s, s)).into()
    }

    fn update(&mut self, ui: &mut MineralUi<'_, Fluent>, frame: &WidgetFrame<'_>, (): &mut ()) -> WidgetResponse {
        let rt = ui.rt();
        let t = ui.tokens();
        let dir = ui.i18n().direction();
        let rect = frame.zones.visual;
        let r = &frame.response;
        let model = self.model();
        let before = *self.value;
        let mut v = model.snap(before);
        let hover = r.hover_pos().map(|p| self.value_at(rect, p, dir));
        if hover.is_some() {
            ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
        }
        if let Some(p) = frame.press_pos() {
            v = self.value_at(rect, p, dir);
            r.request_focus();
        }
        v = model.drive(rt.core(), r.id, v, Track::horizontal(dir));
        if (v - model.snap(before)).abs() > f64::EPSILON {
            *self.value = v;
        }
        let stars = self.stars();
        stars.paint(ui.painter(), &ui.icons(), rect, dir, hover.unwrap_or(v) as f32, &t);
        if Interaction::of(r, Availability::Enabled).focused {
            let at = (0..self.max).find(|i| f64::from(*i) + 1.0 >= v).unwrap_or(0);
            focus::outline(
                ui.painter(),
                stars.star(rect, dir, at),
                CornerRadius::from(g::BORDER_RADIUS_MEDIUM),
                &t,
            );
        }
        WidgetResponse::new(r.clone(), ()).with_changed((before - *self.value).abs() > f64::EPSILON)
    }

    fn semantics(&self, i18n: &I18n, _: &WidgetResponse, (): &()) -> Option<SemanticNode> {
        let v = *self.value;
        let digits = i16::from(self.half);
        let text = i18n.tr(msg::RatingValue {
            value: i18n.format().number_lossy(v, Some(digits)).into(),
            max: i18n.format().integer(i64::from(self.max)).into(),
        });
        let mut n = SemanticNode::new(self.key.id(&self.salt), Role::Slider, &self.label).value(text);
        n.state.range = Some(self.model().numeric(v));
        n.actions.extend([
            egui::accesskit::Action::Focus,
            egui::accesskit::Action::SetValue,
            egui::accesskit::Action::Increment,
            egui::accesskit::Action::Decrement,
        ]);
        Some(n)
    }
}

/// Fluent `RatingDisplay`: read-only stars with an optional value and count text, or a single star
/// ([`RatingDisplay::compact`]).
#[derive(Clone, Debug)]
#[must_use = "show with ui.mineral(..)"]
pub struct RatingDisplay {
    salt: KeyedSalt,
    key: AgentKey,
    label: String,
    value: f64,
    max: u8,
    count: Option<u64>,
    size: RatingSize,
    color: RatingColor,
    compact: bool,
    value_text: bool,
}

impl RatingDisplay {
    /// Five stars showing `value`; `label` is the accessible name. Agent key `<id_salt>-rating-display`.
    pub fn new(id_salt: impl Into<KeyedSalt>, label: impl Into<String>, value: f64) -> Self {
        Self {
            salt: id_salt.into(),
            key: egui_mineral::agent_key!("rating-display"),
            label: label.into(),
            value,
            max: 5,
            count: None,
            size: RatingSize::Medium,
            color: RatingColor::Neutral,
            compact: false,
            value_text: true,
        }
    }

    /// `max`: star count, at least 1; default 5.
    pub fn max(mut self, max: u8) -> Self {
        self.max = egui_mineral::kit::sanitize::at_least(max, 1.., "RatingDisplay max");
        self
    }

    /// `count`: the number of ratings, shown after the value.
    pub fn count(mut self, count: u64) -> Self {
        self.count = Some(count);
        self
    }

    /// Default [`RatingSize::Medium`].
    pub fn size(mut self, size: RatingSize) -> Self {
        self.size = size;
        self
    }

    /// Default [`RatingColor::Neutral`].
    pub fn color(mut self, color: RatingColor) -> Self {
        self.color = color;
        self
    }

    /// `compact`: one star instead of `max`.
    pub fn compact(mut self, compact: bool) -> Self {
        self.compact = compact;
        self
    }

    /// `false` hides the value text; default shown.
    pub fn value_text(mut self, value_text: bool) -> Self {
        self.value_text = value_text;
        self
    }

    crate::widget::setters! {
        /// Replaces the `<id_salt>-rating-display` agent key.
        agent_key(key)
    }

    // react-rating useRatingDisplayStyles.styles.ts useLabelStyles: text size and start margin.
    fn text_metrics(&self) -> (f32, f32, f32) {
        match self.size {
            RatingSize::Large => (
                ty::FONT_SIZE_BASE_300,
                ty::LINE_HEIGHT_BASE_300,
                g::SPACING_HORIZONTAL_S_NUDGE,
            ),
            RatingSize::ExtraLarge => (
                ty::FONT_SIZE_BASE_400,
                ty::LINE_HEIGHT_BASE_400,
                g::SPACING_HORIZONTAL_S,
            ),
            RatingSize::Small | RatingSize::Medium => (
                ty::FONT_SIZE_BASE_200,
                ty::LINE_HEIGHT_BASE_200,
                g::SPACING_HORIZONTAL_XS,
            ),
        }
    }

    fn texts(&self, ui: &MineralUi<'_, Fluent>) -> Vec<(std::sync::Arc<egui::Galley>, f32)> {
        let (size, line, margin) = self.text_metrics();
        let f = ui.i18n().format();
        let make = |text: String, weight: u16| {
            let format = crate::typography::format(
                ui.ctx(),
                crate::typography::TypeSpec {
                    size,
                    line_height: line,
                    weight,
                },
                Color32::PLACEHOLDER,
            );
            ui.painter()
                .layout_job(egui::text::LayoutJob::single_section(text, format))
        };
        let mut out = Vec::new();
        if self.value_text {
            out.push((
                make(f.number_lossy(self.value, Some(1)), ty::FONT_WEIGHT_SEMIBOLD),
                margin,
            ));
        }
        if let Some(n) = self.count {
            let divider = if self.value_text { rd::DIVIDER } else { "" };
            out.push((
                make(
                    format!("{divider}{}", f.integer(i64::try_from(n).unwrap_or(i64::MAX))),
                    ty::FONT_WEIGHT_REGULAR,
                ),
                margin,
            ));
        }
        out
    }

    fn stars(&self) -> Stars {
        Stars {
            max: if self.compact { 1 } else { self.max },
            size: self.size,
            color: self.color,
            appearance: RatingAppearance::Filled,
        }
    }
}

impl MineralWidget for RatingDisplay {
    type Kit = Fluent;
    type State = ();
    type Output = ();

    kit::widget_meta!(salt, hover, no_target);

    fn layout(&self, ui: &MineralUi<'_, Fluent>, c: Constraints, (): &()) -> Measure {
        let s = self.size.icon();
        let stars = f32::from(self.stars().max) * s;
        let texts = self.texts(ui);
        let w = stars + texts.iter().map(|(g, m)| g.size().x + m).sum::<f32>();
        let h = texts.iter().map(|(g, _)| g.size().y).fold(s, f32::max);
        c.constrain(vec2(w, h)).into()
    }

    fn update(&mut self, ui: &mut MineralUi<'_, Fluent>, frame: &WidgetFrame<'_>, (): &mut ()) -> WidgetResponse {
        let t = ui.tokens();
        let dir = ui.i18n().direction();
        let rect = frame.zones.visual;
        let stars = self.stars();
        let s = self.size.icon();
        let shown = if self.compact {
            f32::from(self.value > 0.0)
        } else {
            self.value.clamp(0.0, f64::from(self.max)) as f32
        };
        let strip_w = f32::from(stars.max) * s;
        let strip = Rect::from_min_size(
            pos2(dir.start_x(rect, Rangef::new(0.0, strip_w)), rect.center().y - s / 2.0),
            vec2(strip_w, s),
        );
        stars.paint(ui.painter(), &ui.icons(), strip, dir, shown, &t);
        let mut at = strip.width();
        let color = t.color(ColorNeutralForeground1);
        for (galley, margin) in self.texts(ui) {
            at += margin;
            let x = dir.start_x(rect, Rangef::new(at, at + galley.size().x));
            ui.painter()
                .galley(pos2(x, rect.center().y - galley.size().y / 2.0), galley.clone(), color);
            at += galley.size().x;
        }
        WidgetResponse::new(frame.response.clone(), ())
    }

    fn semantics(&self, i18n: &I18n, _: &WidgetResponse, (): &()) -> Option<SemanticNode> {
        let f = i18n.format();
        let text = i18n.tr(msg::RatingValue {
            value: f.number_lossy(self.value, Some(1)).into(),
            max: f.integer(i64::from(self.max)).into(),
        });
        Some(SemanticNode::new(self.key.id(&self.salt), Role::Image, &self.label).value(text))
    }
}
