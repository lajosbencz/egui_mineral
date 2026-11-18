use egui::{Pos2, Rect, Vec2, pos2, vec2};

use crate::{i18n::Direction, kit::sanitize, layout::Edge};

/// Logical side of the anchor the floating element goes to. `Start`/`End` follow the reading
/// direction.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Side {
    /// Above the anchor.
    Top,
    /// Below the anchor.
    Bottom,
    /// Left in LTR, right in RTL.
    Start,
    /// Right in LTR, left in RTL.
    End,
}

impl Side {
    /// Flip target of [`Anchor::flip`].
    #[must_use]
    pub fn opposite(self) -> Self {
        match self {
            Self::Top => Self::Bottom,
            Self::Bottom => Self::Top,
            Self::Start => Self::End,
            Self::End => Self::Start,
        }
    }

    /// `Top` or `Bottom`.
    pub fn is_vertical(self) -> bool {
        matches!(self, Self::Top | Self::Bottom)
    }

    fn is_right(self, dir: Direction) -> bool {
        matches!((self, dir), (Self::End, Direction::Ltr) | (Self::Start, Direction::Rtl))
    }
}

/// Alignment along the anchor edge. `Start`/`End` follow the reading direction on horizontal
/// edges and mean top/bottom on vertical edges; `egui::Align` is physical (`Min` is always left).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Align {
    /// Left (LTR) or top edges flush.
    Start,
    /// Centers align.
    Center,
    /// Right (LTR) or bottom edges flush.
    End,
}

impl From<Edge> for Side {
    fn from(edge: Edge) -> Self {
        match edge {
            Edge::Start => Self::Start,
            Edge::End => Self::End,
        }
    }
}

impl From<Edge> for Align {
    fn from(edge: Edge) -> Self {
        match edge {
            Edge::Start => Self::Start,
            Edge::End => Self::End,
        }
    }
}

/// Resolved popup position; `arrow` is the caret position on the edge facing the anchor.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct Placement {
    /// Popup rect in screen points, after flip and shift.
    pub rect: Rect,
    /// Side used; the opposite one after a flip.
    pub side: Side,
    /// Anchor center projected on the edge of `rect` facing the anchor, clamped into `rect`
    /// (arrow/caret position).
    pub arrow: Pos2,
}

/// Space a popup is placed in.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Surroundings {
    /// Area the popup must stay in, before [`Anchor::padding`].
    pub viewport: Rect,
    /// Reading direction that resolves `Start`/`End`.
    pub direction: Direction,
}

/// Popup positioning rules relative to an anchor rect: side, alignment, gap, flip and shift.
#[derive(Clone, Copy, Debug, PartialEq)]
#[must_use]
pub struct Anchor {
    side: Side,
    align: Align,
    gap: f32,
    padding: f32,
    flip: bool,
    shift: bool,
}

impl Anchor {
    /// No gap or padding; flip and shift on.
    pub fn new(side: Side, align: Align) -> Self {
        Self {
            side,
            align,
            gap: 0.0,
            padding: 0.0,
            flip: true,
            shift: true,
        }
    }

    /// Distance from the anchor in points; negative or non-finite counts as 0.
    pub fn gap(mut self, gap: f32) -> Self {
        self.gap = crate::kit::sanitize::at_least(gap, 0.0.., "Anchor gap");
        self
    }

    /// Viewport inset in points that flip and shift keep clear; negative or non-finite counts as 0.
    pub fn padding(mut self, padding: f32) -> Self {
        self.padding = crate::kit::sanitize::at_least(padding, 0.0.., "Anchor padding");
        self
    }

    /// Moves to the opposite side when that overflows less. Default on.
    pub fn flip(mut self, flip: bool) -> Self {
        self.flip = flip;
        self
    }

    /// Clamps into the padded viewport, pinned to its start when larger. Default on.
    pub fn shift(mut self, shift: bool) -> Self {
        self.shift = shift;
        self
    }

    fn rect_on(&self, side: Side, anchor: Rect, dim: Vec2, dir: Direction) -> Rect {
        let min = if side.is_vertical() {
            let x = match (self.align, dir) {
                (Align::Center, _) => anchor.center().x - dim.x / 2.0,
                (Align::Start, Direction::Ltr) | (Align::End, Direction::Rtl) => anchor.left(),
                (Align::End, Direction::Ltr) | (Align::Start, Direction::Rtl) => anchor.right() - dim.x,
            };
            let y = if side == Side::Top {
                anchor.top() - self.gap - dim.y
            } else {
                anchor.bottom() + self.gap
            };
            pos2(x, y)
        } else {
            let y = match self.align {
                Align::Start => anchor.top(),
                Align::Center => anchor.center().y - dim.y / 2.0,
                Align::End => anchor.bottom() - dim.y,
            };
            let x = if side.is_right(dir) {
                anchor.right() + self.gap
            } else {
                anchor.left() - self.gap - dim.x
            };
            pos2(x, y)
        };
        Rect::from_min_size(min, dim)
    }

    fn overflow(side: Side, r: Rect, area: Rect) -> f32 {
        if side.is_vertical() {
            (area.top() - r.top()).max(0.0) + (r.bottom() - area.bottom()).max(0.0)
        } else {
            (area.left() - r.left()).max(0.0) + (r.right() - area.right()).max(0.0)
        }
    }

    /// Positions a popup of `dim` next to `anchor` within `surroundings`.
    pub fn place(&self, anchor: Rect, dim: Vec2, surroundings: Surroundings) -> Placement {
        let dim = vec2(
            sanitize::at_least(dim.x, 0.0.., "Anchor::place width"),
            sanitize::at_least(dim.y, 0.0.., "Anchor::place height"),
        );
        let Surroundings {
            viewport,
            direction: dir,
        } = surroundings;
        let area = viewport.shrink(self.padding);
        let mut side = self.side;
        let mut rect = self.rect_on(side, anchor, dim, dir);
        if self.flip {
            let over = Self::overflow(side, rect, area);
            if over > 0.0 {
                let alt = self.rect_on(side.opposite(), anchor, dim, dir);
                if Self::overflow(side.opposite(), alt, area) < over {
                    side = side.opposite();
                    rect = alt;
                }
            }
        }
        if self.shift {
            let clamp = |lo: f32, len: f32, min: f32, max: f32| {
                if len >= max - min {
                    min
                } else {
                    lo.clamp(min, max - len)
                }
            };
            let x = clamp(rect.left(), dim.x, area.left(), area.right());
            let y = clamp(rect.top(), dim.y, area.top(), area.bottom());
            rect = Rect::from_min_size(pos2(x, y), dim);
        }
        let c = anchor.center();
        let arrow = if side.is_vertical() {
            let y = if side == Side::Top { rect.bottom() } else { rect.top() };
            pos2(c.x.clamp(rect.left(), rect.right()), y)
        } else {
            let x = if side.is_right(dir) { rect.left() } else { rect.right() };
            pos2(x, c.y.clamp(rect.top(), rect.bottom()))
        };
        Placement { rect, side, arrow }
    }
}

/// Hover-intent corridor from the pointer to an open submenu: while the pointer stays inside,
/// hovering sibling items must not switch the submenu.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SafeTriangle {
    apex: Pos2,
    a: Pos2,
    b: Pos2,
}

impl SafeTriangle {
    /// Corridor from `apex` (pointer) to the near vertical edge of `target`.
    pub fn new(apex: Pos2, target: Rect) -> Self {
        let x = if apex.x <= target.center().x {
            target.left()
        } else {
            target.right()
        };
        Self {
            apex,
            a: pos2(x, target.top()),
            b: pos2(x, target.bottom()),
        }
    }

    /// Inclusive of the edges.
    pub fn contains(&self, p: Pos2) -> bool {
        let cross = |o: Pos2, u: Pos2, v: Pos2| (u.x - o.x) * (v.y - o.y) - (u.y - o.y) * (v.x - o.x);
        let d1 = cross(self.apex, self.a, p);
        let d2 = cross(self.a, self.b, p);
        let d3 = cross(self.b, self.apex, p);
        let neg = d1 < 0.0 || d2 < 0.0 || d3 < 0.0;
        let pos = d1 > 0.0 || d2 > 0.0 || d3 > 0.0;
        !(neg && pos)
    }
}

/// Rect of `size`, shrunk to fit, centered in `viewport`.
pub fn centered(size: Vec2, viewport: Rect) -> Rect {
    let size = vec2(size.x.min(viewport.width()), size.y.min(viewport.height()));
    Rect::from_center_size(viewport.center(), size)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn screen_rect() -> Rect {
        Rect::from_min_size(Pos2::ZERO, vec2(400.0, 300.0))
    }

    fn screen(direction: Direction) -> Surroundings {
        Surroundings {
            viewport: screen_rect(),
            direction,
        }
    }

    #[test]
    fn flips_to_top_near_bottom_edge() {
        let anchor = Rect::from_min_size(pos2(10.0, 260.0), vec2(40.0, 30.0));
        let p =
            Anchor::new(Side::Bottom, Align::Start)
                .gap(4.0)
                .place(anchor, vec2(100.0, 80.0), screen(Direction::Ltr));
        assert_eq!(p.side, Side::Top);
        assert_eq!(p.rect.bottom(), 256.0);
        assert_eq!(p.arrow, pos2(30.0, 256.0));
    }

    #[test]
    fn shifts_inside_right_edge_and_keeps_side() {
        let anchor = Rect::from_min_size(pos2(380.0, 10.0), vec2(20.0, 20.0));
        let p = Anchor::new(Side::Bottom, Align::Start).padding(8.0).place(
            anchor,
            vec2(100.0, 50.0),
            screen(Direction::Ltr),
        );
        assert_eq!(p.side, Side::Bottom);
        assert_eq!(p.rect.right(), 392.0);
        assert_eq!(p.arrow.x, 390.0);
    }

    #[test]
    fn end_side_follows_direction() {
        let anchor = Rect::from_min_size(pos2(150.0, 100.0), vec2(100.0, 40.0));
        let size = vec2(80.0, 60.0);
        let ltr = Anchor::new(Side::End, Align::Start).place(anchor, size, screen(Direction::Ltr));
        assert_eq!(ltr.rect.left(), 250.0);
        let rtl = Anchor::new(Side::End, Align::Start).place(anchor, size, screen(Direction::Rtl));
        assert_eq!(rtl.rect.right(), 150.0);
        // No room after the anchor: flips to the start side.
        let wide = Rect::from_min_size(pos2(300.0, 100.0), vec2(60.0, 40.0));
        let f = Anchor::new(Side::End, Align::Start).place(wide, size, screen(Direction::Ltr));
        assert_eq!(f.side, Side::Start);
        assert_eq!(f.rect.right(), 300.0);
    }

    #[test]
    fn invalid_dim_never_panics() {
        let anchor = Rect::from_min_size(pos2(10.0, 10.0), vec2(40.0, 30.0));
        for dim in [vec2(-5.0, 20.0), vec2(f32::NAN, f32::NAN), vec2(20.0, f32::INFINITY)] {
            for side in [Side::Top, Side::Bottom, Side::Start, Side::End] {
                let p = Anchor::new(side, Align::Center).place(anchor, dim, screen(Direction::Ltr));
                assert!(p.rect.is_finite() && p.rect.width() >= 0.0 && p.rect.height() >= 0.0);
            }
        }
    }

    #[test]
    fn edge_converts_to_side_and_align() {
        assert_eq!(Side::from(Edge::Start), Side::Start);
        assert_eq!(Side::from(Edge::End), Side::End);
        assert_eq!(Align::from(Edge::Start), Align::Start);
        assert_eq!(Align::from(Edge::End), Align::End);
    }

    #[test]
    fn safe_triangle_corridor() {
        let sub = Rect::from_min_size(pos2(200.0, 0.0), vec2(100.0, 200.0));
        let t = SafeTriangle::new(pos2(150.0, 100.0), sub);
        assert!(t.contains(pos2(180.0, 90.0)));
        assert!(!t.contains(pos2(160.0, 10.0)));
        assert!(!t.contains(pos2(140.0, 100.0)));
    }

    #[test]
    fn invalid_gap_and_padding_count_as_zero() {
        let a = Anchor::new(Side::Bottom, Align::Start).gap(-3.0).padding(f32::NAN);
        assert_eq!(a, Anchor::new(Side::Bottom, Align::Start));
    }
}
