use crate::MaterialExt;
use crate::Rt;
use egui::{Painter, Rangef, Rect, Vec2, pos2, vec2};

use crate::spec::{defaults, tokens::badge};
use crate::widget::parts;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum BadgeKind {
    Dot,
    Count(u32),
}

/// Compose `Badge`, placed like `BadgedBox`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[must_use]
pub struct Badge {
    kind: BadgeKind,
    max: u32,
}

impl Badge {
    /// Small dot without text (`BadgeTokens.Size`).
    pub fn dot() -> Self {
        Self {
            kind: BadgeKind::Dot,
            max: 999,
        }
    }

    /// Large badge showing `n`, capped at [`Self::max`].
    pub fn count(n: u32) -> Self {
        Self {
            kind: BadgeKind::Count(n),
            max: 999,
        }
    }
    /// Counts above `max` show as `<max>+`; default 999.
    pub fn max(mut self, max: u32) -> Self {
        self.max = max;
        self
    }

    pub(crate) fn text(&self, i18n: &egui_mineral::i18n::I18n) -> Option<String> {
        match self.kind {
            BadgeKind::Dot => None,
            BadgeKind::Count(n) if n > self.max => Some(i18n.tr("mineral-badge-overflow", &[("max", self.max.into())])),
            BadgeKind::Count(n) => Some(i18n.format().integer(n.into())),
        }
    }

    fn place(rt: &Rt, anchor: Rect, size: Vec2, offset: Vec2) -> Rect {
        let y = anchor.top() - size.y + offset.y;
        let dir = rt.i18n().direction();
        let x = dir.offset_x(dir.end_edge(anchor), Rangef::new(-offset.x, -offset.x + size.x));
        Rect::from_min_size(pos2(x, y), size)
    }

    pub(crate) fn paint(&self, rt: &Rt, painter: &Painter, anchor: Rect) {
        let t = rt.tokens();
        let b = defaults::badge::OFFSET;
        match self.text(rt.i18n()) {
            None => {
                let rect = Self::place(rt, anchor, Vec2::splat(badge::SIZE), Vec2::splat(b));
                let r = badge::SHAPE.scale().radius(&t.shape);
                painter.rect_filled(rect, crate::widget::common::radius(rect, r), t.role(badge::COLOR));
            }
            Some(text) => {
                let color = t.role(badge::LARGE_LABEL_TEXT_COLOR);
                let galley = parts::type_galley(rt.core(), painter, &text, badge::LARGE_LABEL_TEXT_FONT, color, None);
                let size = vec2(
                    (galley.size().x + 2.0 * defaults::badge::LARGE_PADDING).max(badge::LARGE_SIZE),
                    badge::LARGE_SIZE,
                );
                let offset = vec2(defaults::badge::LARGE_OFFSET_X, defaults::badge::LARGE_OFFSET_Y);
                let rect = Self::place(rt, anchor, size, offset);
                let r = badge::LARGE_SHAPE.scale().radius(&t.shape);
                painter.rect_filled(rect, crate::widget::common::radius(rect, r), t.role(badge::LARGE_COLOR));
                painter.galley(rect.center() - galley.size() / 2.0, galley, color);
            }
        }
    }
}
