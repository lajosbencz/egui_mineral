use crate::msg;
use egui::{Color32, FontId, Pos2, Shape, Stroke, pos2, vec2};
use egui_mineral::{
    i18n::I18n,
    kit,
    layout::{Constraints, Measure},
    semantic::{AgentKey, KeyedSalt, Role, SemanticNode},
    widget::{MineralUi, MineralWidget, WidgetFrame, WidgetResponse},
};

use crate::spec::ColorToken::{
    ColorNeutralBackground1, ColorNeutralForeground3, ColorPaletteBerryForeground3, ColorPaletteLightGreenForeground3,
    ColorPaletteMarigoldBackground3, ColorPaletteRedBackground3,
};
use crate::{Fluent, FluentExt, feedback::BadgeSize, spec::ColorToken, spec::defaults::presence_badge as d};

/// Presence `status`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum PresenceStatus {
    /// `available`: green with a check.
    #[default]
    Available,
    /// `away`: marigold with clock hands.
    Away,
    /// `busy`: red.
    Busy,
    /// `do-not-disturb`: red with a bar.
    DoNotDisturb,
    /// `offline`: gray ring with a cross.
    Offline,
    /// `out-of-office`: berry with an arrow.
    OutOfOffice,
    /// `unknown`: gray ring with a question mark.
    Unknown,
    /// `blocked`: red ring with a slash.
    Blocked,
}

impl PresenceStatus {
    /// Every variant.
    pub const ALL: &'static [Self] = &[
        Self::Available,
        Self::Away,
        Self::Busy,
        Self::DoNotDisturb,
        Self::Offline,
        Self::OutOfOffice,
        Self::Unknown,
        Self::Blocked,
    ];

    const fn message(self) -> crate::Msg {
        match self {
            Self::Available => msg::PRESENCE_AVAILABLE,
            Self::Away => msg::PRESENCE_AWAY,
            Self::Busy => msg::PRESENCE_BUSY,
            Self::DoNotDisturb => msg::PRESENCE_DO_NOT_DISTURB,
            Self::Offline => msg::PRESENCE_OFFLINE,
            Self::OutOfOffice => msg::PRESENCE_OUT_OF_OFFICE,
            Self::Unknown => msg::PRESENCE_UNKNOWN,
            Self::Blocked => msg::PRESENCE_BLOCKED,
        }
    }

    const fn busy(self) -> bool {
        matches!(self, Self::Busy | Self::DoNotDisturb | Self::Blocked)
    }

    // react-badge usePresenceBadgeStyles.styles.ts status and outOfOffice colors
    const fn color(self, out_of_office: bool) -> ColorToken {
        match (self, out_of_office) {
            (s, _) if s.busy() => ColorPaletteRedBackground3,
            (Self::Available, _) => ColorPaletteLightGreenForeground3,
            (Self::Away, false) => ColorPaletteMarigoldBackground3,
            (Self::Away | Self::Offline | Self::OutOfOffice, true) | (Self::OutOfOffice, false) => {
                ColorPaletteBerryForeground3
            }
            _ => ColorNeutralForeground3,
        }
    }
}

/// Fluent `PresenceBadge`: a status dot with a ring, drawn as the Fluent presence glyphs.
#[derive(Clone, Debug)]
#[must_use = "show with ui.mineral(..)"]
pub struct PresenceBadge {
    salt: KeyedSalt,
    key: AgentKey,
    status: PresenceStatus,
    out_of_office: bool,
    size: BadgeSize,
}

impl PresenceBadge {
    /// Medium; agent key `<id_salt>-presence-badge`.
    pub fn new(id_salt: impl Into<KeyedSalt>, status: PresenceStatus) -> Self {
        Self {
            salt: id_salt.into(),
            key: egui_mineral::agent_key!("presence-badge"),
            status,
            out_of_office: false,
            size: BadgeSize::Medium,
        }
    }

    /// `outOfOffice`: the outlined variant with an arrow.
    pub fn out_of_office(mut self, out_of_office: bool) -> Self {
        self.out_of_office = out_of_office;
        self
    }

    /// Default [`BadgeSize::Medium`]: tiny 6, extra-small 10, small 12, medium 16, large 20, extra-large 28.
    pub fn size(mut self, size: BadgeSize) -> Self {
        self.size = size;
        self
    }

    crate::widget::setters! {
        /// Replaces the `<id_salt>-presence-badge` agent key.
        agent_key(key)
    }

    pub(crate) const fn extent(&self) -> f32 {
        match self.size {
            BadgeSize::Tiny => d::TINY,
            BadgeSize::ExtraSmall => d::EXTRA_SMALL,
            BadgeSize::Small => d::SMALL,
            BadgeSize::Medium => d::MEDIUM,
            BadgeSize::Large => d::LARGE,
            BadgeSize::ExtraLarge => d::EXTRA_LARGE,
        }
    }

    pub(crate) fn paint(&self, p: &egui::Painter, rect: egui::Rect, t: &crate::theme::Tokens, sign: f32) {
        let ring = t.color(ColorNeutralBackground1);
        let color = t.color(self.status.color(self.out_of_office));
        let c = rect.center();
        let inner = if self.size == BadgeSize::Tiny {
            rect.width() / 2.0
        } else {
            rect.width() / 2.0 - d::RING
        };
        p.circle_filled(c, rect.width() / 2.0, ring);
        let outlined = self.out_of_office
            || matches!(
                self.status,
                PresenceStatus::Offline | PresenceStatus::Unknown | PresenceStatus::Blocked
            );
        let width = (inner * 0.22).max(1.0);
        if outlined {
            p.circle_stroke(c, inner - width / 2.0, Stroke::new(width, color));
        } else {
            p.circle_filled(c, inner, color);
        }
        if inner >= 5.0 {
            let g = Glyph {
                p,
                c,
                u: inner * 0.8,
                sign,
                stroke: width,
            };
            let mark = if outlined { color } else { ring };
            match (self.out_of_office, self.status) {
                (true, _) | (false, PresenceStatus::OutOfOffice) => g.arrow(mark),
                (false, PresenceStatus::Available) => g.check(mark),
                (false, PresenceStatus::Away) => g.line(&[(0.0, -0.5), (0.0, 0.0), (0.4, 0.25)], mark, width),
                (false, PresenceStatus::DoNotDisturb) => g.line(&[(-0.55, 0.0), (0.55, 0.0)], mark, width * 1.3),
                (false, PresenceStatus::Offline) => g.cross(mark),
                (false, PresenceStatus::Blocked) => g.line(&[(-0.5, 0.5), (0.5, -0.5)], mark, width),
                (false, PresenceStatus::Unknown) => {
                    p.text(
                        c,
                        egui::Align2::CENTER_CENTER,
                        "?",
                        FontId::proportional(inner * 1.3),
                        mark,
                    );
                }
                (false, PresenceStatus::Busy) => {}
            }
        }
    }

    pub(crate) fn name(&self, i18n: &I18n) -> String {
        let status = crate::tr(i18n, self.status.message());
        if self.out_of_office && self.status != PresenceStatus::OutOfOffice {
            i18n.tr(msg::PresenceWithOutOfOffice { status: status.into() })
        } else {
            status
        }
    }
}

struct Glyph<'a> {
    p: &'a egui::Painter,
    c: Pos2,
    u: f32,
    sign: f32,
    stroke: f32,
}

impl Glyph<'_> {
    fn at(&self, x: f32, y: f32) -> Pos2 {
        pos2(self.c.x + self.sign * x * self.u, self.c.y + y * self.u)
    }

    fn line(&self, points: &[(f32, f32)], color: Color32, width: f32) {
        let pts = points.iter().map(|&(x, y)| self.at(x, y)).collect();
        self.p.add(Shape::line(pts, Stroke::new(width, color)));
    }

    fn check(&self, color: Color32) {
        self.line(&[(-0.45, 0.0), (-0.1, 0.35), (0.5, -0.3)], color, self.stroke);
    }

    fn arrow(&self, color: Color32) {
        self.line(&[(-0.5, 0.0), (0.5, 0.0)], color, self.stroke);
        self.line(&[(0.1, -0.4), (0.5, 0.0), (0.1, 0.4)], color, self.stroke);
    }

    fn cross(&self, color: Color32) {
        self.line(&[(-0.4, -0.4), (0.4, 0.4)], color, self.stroke);
        self.line(&[(-0.4, 0.4), (0.4, -0.4)], color, self.stroke);
    }
}

impl MineralWidget for PresenceBadge {
    type Kit = Fluent;
    type State = ();
    type Output = ();

    kit::widget_meta!(salt, hover, no_target);

    fn layout(&self, _: &MineralUi<'_, Fluent>, c: Constraints, (): &()) -> Measure {
        let s = self.extent();
        c.constrain(vec2(s, s)).into()
    }

    fn update(&mut self, ui: &mut MineralUi<'_, Fluent>, frame: &WidgetFrame<'_>, (): &mut ()) -> WidgetResponse {
        let t = ui.tokens();
        self.paint(ui.painter(), frame.zones.visual, &t, ui.i18n().direction().sign());
        WidgetResponse::new(frame.response.clone(), ())
    }

    fn semantics(&self, i18n: &I18n, _: &WidgetResponse, (): &()) -> Option<SemanticNode> {
        Some(SemanticNode::new(self.key.id(&self.salt), Role::Image, self.name(i18n)))
    }
}
