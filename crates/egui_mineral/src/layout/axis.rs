use egui::{Pos2, Rangef, Rect, Vec2, pos2, vec2};

/// Layout orientation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Axis {
    /// x is the main axis.
    Horizontal,
    /// y is the main axis.
    Vertical,
}

impl Axis {
    /// Main-axis component of `v`.
    pub fn main(self, v: Vec2) -> f32 {
        match self {
            Self::Horizontal => v.x,
            Self::Vertical => v.y,
        }
    }

    /// Cross-axis component of `v`.
    pub fn cross(self, v: Vec2) -> f32 {
        match self {
            Self::Horizontal => v.y,
            Self::Vertical => v.x,
        }
    }

    /// Vector from main and cross components.
    pub fn vec2(self, main: f32, cross: f32) -> Vec2 {
        match self {
            Self::Horizontal => vec2(main, cross),
            Self::Vertical => vec2(cross, main),
        }
    }

    /// Point from main and cross coordinates.
    pub fn pos2(self, main: f32, cross: f32) -> Pos2 {
        let v = self.vec2(main, cross);
        pos2(v.x, v.y)
    }

    /// Rect from main and cross extents.
    pub fn rect(self, main: Rangef, cross: Rangef) -> Rect {
        match self {
            Self::Horizontal => Rect::from_x_y_ranges(main, cross),
            Self::Vertical => Rect::from_x_y_ranges(cross, main),
        }
    }

    /// Extent of `rect` along the main axis.
    pub fn main_range(self, rect: Rect) -> Rangef {
        match self {
            Self::Horizontal => rect.x_range(),
            Self::Vertical => rect.y_range(),
        }
    }

    /// Extent of `rect` along the cross axis.
    pub fn cross_range(self, rect: Rect) -> Rangef {
        match self {
            Self::Horizontal => rect.y_range(),
            Self::Vertical => rect.x_range(),
        }
    }
}

/// Logical end of a row or column: `Start`/`End` follow the reading direction horizontally and
/// mean top/bottom vertically.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Edge {
    /// Left in LTR, right in RTL; top vertically.
    Start,
    /// Right in LTR, left in RTL; bottom vertically.
    End,
}
