use egui::{Rangef, Rect, Vec2, pos2, text::LayoutJob, vec2};
use egui_mineral::{
    i18n::I18n,
    kit,
    layout::{Constraints, Measure},
    semantic::{AgentKey, KeyedSalt, Role, SemanticNode},
    widget::{MineralUi, MineralWidget, WidgetFrame, WidgetResponse},
};

use crate::ColorToken::{ColorNeutralForeground1, ColorNeutralForeground2};
use crate::{
    ColorToken, Fluent, FluentExt,
    feedback::{BadgeSize, PresenceBadge, PresenceStatus},
    spec::defaults::persona as d,
    typography::TypeRamp,
};

use super::{Avatar, AvatarSize};

/// Persona `size`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PersonaSize {
    /// `extra-small`: a 20 avatar.
    ExtraSmall,
    /// `small`: a 28 avatar.
    Small,
    /// `medium`: a 32 avatar.
    #[default]
    Medium,
    /// `large`: a 36 avatar.
    Large,
    /// `extra-large`: a 40 avatar.
    ExtraLarge,
    /// `huge`: a 56 avatar.
    Huge,
}

impl PersonaSize {
    /// Every variant, smallest first.
    pub const ALL: [Self; 6] = [
        Self::ExtraSmall,
        Self::Small,
        Self::Medium,
        Self::Large,
        Self::ExtraLarge,
        Self::Huge,
    ];

    fn avatar(self) -> AvatarSize {
        match self {
            Self::ExtraSmall => AvatarSize::S20,
            Self::Small => AvatarSize::S28,
            Self::Medium => AvatarSize::S32,
            Self::Large => AvatarSize::S36,
            Self::ExtraLarge => AvatarSize::S40,
            Self::Huge => AvatarSize::S56,
        }
    }

    fn badge(self) -> BadgeSize {
        match self {
            Self::ExtraSmall => BadgeSize::Tiny,
            Self::Small => BadgeSize::ExtraSmall,
            Self::Medium => BadgeSize::Small,
            Self::Large => BadgeSize::Medium,
            Self::ExtraLarge | Self::Huge => BadgeSize::Large,
        }
    }

    fn spacing(self, presence: bool) -> f32 {
        if presence && self == Self::Small {
            return d::PRESENCE_SMALL_SPACING;
        }
        d::AVATAR_SPACING.get(self as usize).copied().unwrap_or_default()
    }

    const fn large(self) -> bool {
        matches!(self, Self::ExtraLarge | Self::Huge)
    }
}

/// Persona `textPosition`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum PersonaTextPosition {
    /// `after`: the text follows the media.
    #[default]
    After,
    /// `before`: the text precedes the media, end-aligned.
    Before,
    /// `below`: the text is centered under the media.
    Below,
}

impl PersonaTextPosition {
    /// Every variant.
    pub const ALL: [Self; 3] = [Self::After, Self::Before, Self::Below];
}

/// Persona `textAlignment`: how the media sits against the text lines.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum PersonaTextAlignment {
    /// `start`: media and text start at the top.
    #[default]
    Start,
    /// `center`: media and text are centered on each other.
    Center,
}

impl PersonaTextAlignment {
    /// Every variant.
    pub const ALL: [Self; 2] = [Self::Start, Self::Center];
}

struct Line {
    text: String,
    ramp: TypeRamp,
    color: ColorToken,
    offset: f32,
}

// Positions from the start edge (x) and the top (y), in the grid of usePersonaStyles.styles.ts
#[derive(Debug, PartialEq)]
struct Grid {
    size: Vec2,
    media: Option<Rect>,
    lines: Vec<Rect>,
}

struct Inputs<'l> {
    media: Option<Vec2>,
    lines: &'l [Vec2],
    offsets: &'l [f32],
    spacing: f32,
    position: PersonaTextPosition,
    center: bool,
    align_to_primary: bool,
}

fn grid(i: &Inputs<'_>) -> Grid {
    let tw = i.lines.iter().fold(0.0_f32, |w, l| w.max(l.x));
    let th: f32 = i.lines.iter().map(|l| l.y).sum::<f32>() + i.offsets.iter().sum::<f32>();
    let media = i.media.unwrap_or(Vec2::ZERO);
    let gap = if i.media.is_some() { i.spacing } else { 0.0 };
    let stack = |x_of: &dyn Fn(f32) -> f32, y0: f32| {
        let mut y = y0;
        i.lines
            .iter()
            .zip(i.offsets)
            .map(|(l, off)| {
                y += off;
                let r = Rect::from_min_size(pos2(x_of(l.x), y), *l);
                y += l.y;
                r
            })
            .collect::<Vec<_>>()
    };
    if i.position == PersonaTextPosition::Below {
        let w = tw.max(media.x);
        let h = media.y + gap + th;
        let lines = stack(&|lw| (w - lw) / 2.0, media.y + gap);
        let media_rect = i.media.map(|m| Rect::from_min_size(pos2((w - m.x) / 2.0, 0.0), m));
        return Grid {
            size: vec2(w, h),
            media: media_rect,
            lines,
        };
    }
    let h = media.y.max(th);
    let (media_y, text_y) = if i.center {
        ((h - media.y) / 2.0, (h - th) / 2.0)
    } else if i.align_to_primary {
        let primary = i.lines.first().map_or(0.0, |l| l.y);
        (primary / 2.0 - media.y / 2.0, 0.0)
    } else {
        (0.0, 0.0)
    };
    let (media_x, text_start) = match i.position {
        PersonaTextPosition::Before => (tw + gap, 0.0),
        _ => (0.0, media.x + gap),
    };
    let before = i.position == PersonaTextPosition::Before;
    let lines = stack(&|lw| if before { tw - lw } else { 0.0 } + text_start, text_y);
    Grid {
        size: vec2(media.x + gap + tw, h),
        media: i.media.map(|m| Rect::from_min_size(pos2(media_x, media_y), m)),
        lines,
    }
}

/// Fluent `Persona`: an [`Avatar`] or presence badge beside up to four lines of text.
#[derive(Clone, Debug)]
#[must_use = "show with ui.mineral(..)"]
pub struct Persona<'a> {
    salt: KeyedSalt,
    key: AgentKey,
    name: String,
    size: PersonaSize,
    position: PersonaTextPosition,
    alignment: PersonaTextAlignment,
    avatar: Option<Avatar<'a>>,
    presence: Option<(PresenceStatus, bool)>,
    presence_only: bool,
    text: [Option<String>; 4],
}

impl<'a> Persona<'a> {
    /// `name` is the primary text and the avatar's name; medium, text after, start-aligned.
    /// Agent key `<id_salt>-persona`.
    pub fn new(id_salt: impl Into<KeyedSalt>, name: impl Into<String>) -> Self {
        Self {
            salt: id_salt.into(),
            key: AgentKey::Kind("persona"),
            name: name.into(),
            size: PersonaSize::Medium,
            position: PersonaTextPosition::After,
            alignment: PersonaTextAlignment::Start,
            avatar: None,
            presence: None,
            presence_only: false,
            text: [None, None, None, None],
        }
    }

    /// Default [`PersonaSize::Medium`].
    pub fn size(mut self, size: PersonaSize) -> Self {
        self.size = size;
        self
    }

    /// Default [`PersonaTextPosition::After`].
    pub fn text_position(mut self, position: PersonaTextPosition) -> Self {
        self.position = position;
        self
    }

    /// Default [`PersonaTextAlignment::Start`].
    pub fn text_alignment(mut self, alignment: PersonaTextAlignment) -> Self {
        self.alignment = alignment;
        self
    }

    /// `avatar` slot: replaces the default one of the name; the persona sets its size.
    pub fn avatar(mut self, avatar: Avatar<'a>) -> Self {
        self.avatar = Some(avatar);
        self
    }

    /// `presence` slot: the avatar's badge, or the media itself with [`Self::presence_only`].
    pub fn presence(mut self, status: PresenceStatus) -> Self {
        self.presence = Some((status, false));
        self
    }

    /// `presence` slot as an out-of-office badge.
    pub fn presence_out_of_office(mut self, status: PresenceStatus) -> Self {
        self.presence = Some((status, true));
        self
    }

    /// `presenceOnly`: a [`PresenceBadge`] replaces the avatar.
    pub fn presence_only(mut self, presence_only: bool) -> Self {
        self.presence_only = presence_only;
        self
    }

    /// `primaryText` slot: replaces the name as the first line.
    pub fn primary_text(mut self, text: impl Into<String>) -> Self {
        self.text[0] = Some(text.into());
        self
    }

    /// `secondaryText` slot.
    pub fn secondary_text(mut self, text: impl Into<String>) -> Self {
        self.text[1] = Some(text.into());
        self
    }

    /// `tertiaryText` slot.
    pub fn tertiary_text(mut self, text: impl Into<String>) -> Self {
        self.text[2] = Some(text.into());
        self
    }

    /// `quaternaryText` slot.
    pub fn quaternary_text(mut self, text: impl Into<String>) -> Self {
        self.text[3] = Some(text.into());
        self
    }

    crate::widget::setters! {
        /// Replaces the `<id_salt>-persona` agent key.
        agent_key(key)
    }

    fn lines(&self) -> Vec<Line> {
        let [primary, secondary, tertiary, quaternary] = &self.text;
        let primary = primary.clone().unwrap_or_else(|| self.name.clone());
        let optional = [secondary, tertiary, quaternary];
        let count = 1 + optional.iter().filter(|t| t.is_some()).count();
        let (size, only) = (self.size, self.presence_only);
        let primary_ramp = if size.large() {
            TypeRamp::Subtitle2
        } else if only && size == PersonaSize::ExtraSmall && count <= d::CAPTION_PRIMARY_MAX_LINES {
            TypeRamp::Caption1
        } else {
            TypeRamp::Body1
        };
        let optional_ramp = if !only && size == PersonaSize::Huge {
            TypeRamp::Body1
        } else {
            TypeRamp::Caption1
        };
        let mut lines = vec![Line {
            text: primary,
            ramp: primary_ramp,
            color: ColorNeutralForeground1,
            offset: 0.0,
        }];
        for (i, t) in optional.into_iter().enumerate() {
            if let Some(text) = t {
                lines.push(Line {
                    text: text.clone(),
                    ramp: optional_ramp,
                    color: ColorNeutralForeground2,
                    offset: if i == 0 { d::SECOND_LINE_OFFSET } else { 0.0 },
                });
            }
        }
        lines
    }

    fn badge(&self) -> Option<PresenceBadge> {
        let (status, ooo) = self.presence.filter(|_| self.presence_only)?;
        Some(
            PresenceBadge::new("presence", status)
                .out_of_office(ooo)
                .size(self.size.badge()),
        )
    }

    fn media(&self) -> Option<Vec2> {
        if self.presence_only {
            return self.badge().map(|b| vec2(b.extent(), b.extent()));
        }
        let side = f32::from(self.size.avatar().px());
        Some(vec2(side, side))
    }

    fn galleys(ui: &MineralUi<'_, Fluent>, lines: &[Line]) -> Vec<std::sync::Arc<egui::Galley>> {
        lines
            .iter()
            .map(|l| {
                let color = ui.tokens().color(l.color);
                let job = LayoutJob::single_section(l.text.clone(), l.ramp.format(ui.ctx(), color));
                ui.painter().layout_job(job)
            })
            .collect()
    }

    fn grid(&self, lines: &[Line], galleys: &[std::sync::Arc<egui::Galley>]) -> Grid {
        let sizes: Vec<Vec2> = galleys
            .iter()
            .zip(lines)
            .map(|(g, l)| vec2(g.size().x, l.ramp.line_height()))
            .collect();
        let offsets: Vec<f32> = lines.iter().map(|l| l.offset).collect();
        let to_primary = self.presence_only && self.alignment == PersonaTextAlignment::Start && !self.size.large();
        grid(&Inputs {
            media: self.media(),
            lines: &sizes,
            offsets: &offsets,
            spacing: self.size.spacing(self.presence_only),
            position: self.position,
            center: self.alignment == PersonaTextAlignment::Center && self.position != PersonaTextPosition::Below,
            align_to_primary: to_primary,
        })
    }

    fn label(&self) -> String {
        self.lines().into_iter().map(|l| l.text).collect::<Vec<_>>().join(", ")
    }
}

impl MineralWidget for Persona<'_> {
    type Kit = Fluent;
    type State = ();
    type Output = ();

    kit::widget_meta!(salt, hover, no_target);

    fn layout(&self, ui: &MineralUi<'_, Fluent>, c: Constraints, (): &()) -> Measure {
        let lines = self.lines();
        let galleys = Self::galleys(ui, &lines);
        c.constrain(self.grid(&lines, &galleys).size).into()
    }

    fn update(&mut self, ui: &mut MineralUi<'_, Fluent>, frame: &WidgetFrame<'_>, (): &mut ()) -> WidgetResponse {
        let t = ui.tokens();
        let dir = ui.i18n().direction();
        let rect = frame.zones.visual;
        let lines = self.lines();
        let galleys = Self::galleys(ui, &lines);
        let grid = self.grid(&lines, &galleys);
        let at = |r: Rect| {
            let x = dir.start_x(rect, Rangef::new(r.min.x, r.max.x));
            Rect::from_min_size(pos2(x, rect.min.y + r.min.y), r.size())
        };
        if let Some(m) = grid.media {
            let m = at(m);
            if let Some(badge) = self.badge() {
                badge.paint(ui.painter(), m, &t, dir.sign());
            } else {
                let mut avatar = self
                    .avatar
                    .clone()
                    .unwrap_or_else(|| Avatar::new("avatar", self.name.clone()))
                    .size(self.size.avatar());
                if let Some(p) = self.presence {
                    avatar.presence = Some(p);
                }
                avatar.paint(ui, m, &t);
            }
        }
        for ((galley, line), r) in galleys.into_iter().zip(&lines).zip(grid.lines) {
            let r = at(r);
            let color = t.color(line.color);
            ui.painter().galley(pos2(r.min.x, r.min.y), galley, color);
        }
        WidgetResponse::new(frame.response.clone(), ())
    }

    fn semantics(&self, _: &I18n, _: &WidgetResponse, (): &()) -> Option<SemanticNode> {
        Some(SemanticNode::new(self.key.id(&self.salt), Role::Group, self.label()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn inputs<'l>(position: PersonaTextPosition, lines: &'l [Vec2], offsets: &'l [f32]) -> Inputs<'l> {
        Inputs {
            media: Some(vec2(32.0, 32.0)),
            lines,
            offsets,
            spacing: 8.0,
            position,
            center: false,
            align_to_primary: false,
        }
    }

    #[test]
    fn text_after_the_media() {
        let lines = [vec2(50.0, 20.0), vec2(40.0, 16.0)];
        let g = grid(&inputs(PersonaTextPosition::After, &lines, &[0.0, -2.0]));
        assert_eq!(g.size, vec2(32.0 + 8.0 + 50.0, 34.0));
        assert_eq!(g.media, Some(Rect::from_min_size(pos2(0.0, 0.0), vec2(32.0, 32.0))));
        assert_eq!(g.lines[1].min, pos2(40.0, 18.0));
    }

    #[test]
    fn text_before_is_end_aligned_and_below_is_centered() {
        let lines = [vec2(50.0, 20.0), vec2(30.0, 16.0)];
        let g = grid(&inputs(PersonaTextPosition::Before, &lines, &[0.0, 0.0]));
        assert_eq!(g.lines[1].min.x, 20.0);
        assert_eq!(g.media.map(|m| m.min.x), Some(58.0));
        let g = grid(&inputs(PersonaTextPosition::Below, &lines, &[0.0, 0.0]));
        assert_eq!(g.size, vec2(50.0, 32.0 + 8.0 + 36.0));
        assert_eq!(g.lines[1].min.x, 10.0);
        assert_eq!(g.media.map(|m| m.min.x), Some(9.0));
    }

    #[test]
    fn centered_media_and_text_share_a_midline() {
        let lines = [vec2(50.0, 20.0)];
        let mut i = inputs(PersonaTextPosition::After, &lines, &[0.0]);
        i.center = true;
        let g = grid(&i);
        assert_eq!(g.lines[0].min.y, 6.0);
        assert_eq!(g.media.map(|m| m.min.y), Some(0.0));
    }
}
