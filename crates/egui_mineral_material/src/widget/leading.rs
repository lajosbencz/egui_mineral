use std::{fmt, sync::Arc};

use egui::{Color32, Painter, Pos2, Rect, Vec2};

use crate::Icon;
use crate::widget::common;

/// App-painted content of a leading icon slot (colour swatch, preview), for content an icon cannot
/// show. Gets the slot rect (the item's icon size: 24dp, 20dp in expressive menus) and the
/// content colour the item would paint an icon with (state and disabled applied); tokens are at
/// `rt.tokens()` ([`MaterialExt`](crate::MaterialExt)).
#[derive(Clone)]
pub struct LeadingPaint(Arc<Paint>);

type Paint = dyn Fn(&Painter, Rect, Color32) + Send + Sync;

impl LeadingPaint {
    /// `paint` gets the clipped painter, the slot rect and the content color.
    pub fn new(paint: impl Fn(&Painter, Rect, Color32) + Send + Sync + 'static) -> Self {
        Self(Arc::new(paint))
    }

    pub(crate) fn paint(&self, p: &Painter, slot: Rect, color: Color32) {
        (self.0)(&p.with_clip_rect(slot.intersect(p.clip_rect())), slot, color);
    }
}

impl fmt::Debug for LeadingPaint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("LeadingPaint")
    }
}

/// Leading slot content of collection items: an icon or app paint.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub enum Leading {
    /// Material icon.
    Icon(Icon),
    /// App-painted content.
    Paint(LeadingPaint),
}

impl From<Icon> for Leading {
    fn from(icon: Icon) -> Self {
        Self::Icon(icon)
    }
}

impl From<crate::MaterialIcon> for Leading {
    fn from(icon: crate::MaterialIcon) -> Self {
        Self::from(Icon::from(icon))
    }
}

impl From<LeadingPaint> for Leading {
    fn from(paint: LeadingPaint) -> Self {
        Self::Paint(paint)
    }
}

impl Leading {
    pub(crate) fn paint(&self, p: &Painter, center: Pos2, size: f32, color: Color32) {
        match self {
            Self::Icon(icon) => common::icon(p, center, *icon, size, color),
            Self::Paint(f) => f.paint(p, Rect::from_center_size(center, Vec2::splat(size)), color),
        }
    }
}
