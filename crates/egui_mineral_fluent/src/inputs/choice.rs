use std::sync::Arc;

use egui::{Align, Galley, Painter, Pos2, Rangef, Rect, Stroke, TextFormat, Vec2, vec2};
use egui_mineral::{
    i18n::Direction,
    runtime::Core,
    shortcut::{self, Mnemonic},
};

use crate::spec::tokens::geometry as g;

// Checkbox, Radio and Switch share one row: an indicator with `spacingVertical/HorizontalS` margins next to a
// label with `spacingVertical/HorizontalS` padding, its near side `spacingHorizontalXS`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Position {
    Before,
    After,
    Below,
    Above,
}

pub(crate) struct Spec {
    pub indicator: Vec2,
    pub position: Position,
    pub line_height: f32,
}

pub(crate) struct Plan {
    pub size: Vec2,
    pub indicator: Pos2,
    pub indicator_size: Vec2,
    pub text: Option<(Arc<Galley>, Pos2)>,
}

pub(crate) struct Placed {
    pub indicator: Rect,
    pub text: Option<(Pos2, Arc<Galley>)>,
}

impl Spec {
    pub(crate) fn wrap_width(&self, max: f32) -> f32 {
        let s = g::SPACING_HORIZONTAL_S;
        match self.position {
            Position::Before | Position::After => max - self.indicator.x - 3.0 * s - g::SPACING_HORIZONTAL_XS,
            Position::Below | Position::Above => max - 2.0 * s,
        }
        .max(0.0)
    }

    pub(crate) fn label(
        &self,
        core: &Core,
        painter: &Painter,
        text: &str,
        format: &TextFormat,
        mnemonic: Option<Mnemonic>,
        max: f32,
    ) -> Arc<Galley> {
        let mut job = shortcut::mnemonic_job(core, text, format, mnemonic, Stroke::new(1.0, format.color));
        job.wrap.max_width = self.wrap_width(max);
        if self.position == Position::Below {
            job.halign = Align::Center;
        }
        painter.layout_job(job)
    }

    pub(crate) fn plan(&self, label: Option<Arc<Galley>>) -> Plan {
        let (s, sv, xs) = (g::SPACING_HORIZONTAL_S, g::SPACING_VERTICAL_S, g::SPACING_HORIZONTAL_XS);
        let ind = self.indicator;
        let cell = vec2(ind.x + 2.0 * s, ind.y + 2.0 * sv);
        let (size, indicator, text) = match (label, self.position) {
            (None, _) => (cell, Pos2::new(s, sv), None),
            (Some(l), Position::After | Position::Before) => {
                let lw = l.size().x + s + xs;
                // The label's vertical margin cancels the difference between indicator and line height.
                let m = (ind.y - self.line_height) / 2.0;
                let size = vec2(cell.x + lw, cell.y.max(l.size().y + 2.0 * sv + 2.0 * m));
                let y = (size.y - l.size().y) / 2.0;
                if self.position == Position::After {
                    (size, Pos2::new(s, sv), Some((l, Pos2::new(cell.x + xs, y))))
                } else {
                    (size, Pos2::new(lw + s, sv), Some((l, Pos2::new(s, y))))
                }
            }
            (Some(l), Position::Below) => {
                let w = cell.x.max(l.size().x + 2.0 * s);
                let y = cell.y + g::SPACING_VERTICAL_XS;
                (
                    vec2(w, y + l.size().y + sv),
                    Pos2::new((w - ind.x) / 2.0, sv),
                    Some((l.clone(), Pos2::new((w - l.size().x) / 2.0, y))),
                )
            }
            (Some(l), Position::Above) => {
                let top = g::SPACING_VERTICAL_XS;
                let h = top + l.size().y + top;
                (
                    vec2(cell.x.max(l.size().x + 2.0 * s), h + ind.y + sv),
                    Pos2::new(s, h),
                    Some((l, Pos2::new(s, top))),
                )
            }
        };
        Plan {
            size,
            indicator,
            indicator_size: ind,
            text,
        }
    }
}

impl Plan {
    pub(crate) fn place(&self, visual: Rect, dir: Direction) -> Placed {
        let x = |at: f32, w: f32| dir.start_x(visual, Rangef::new(at, at + w));
        Placed {
            indicator: Rect::from_min_size(
                Pos2::new(
                    x(self.indicator.x, self.indicator_size.x),
                    visual.top() + self.indicator.y,
                ),
                self.indicator_size,
            ),
            text: self
                .text
                .as_ref()
                .map(|(l, at)| (Pos2::new(x(at.x, l.size().x), visual.top() + at.y), Arc::clone(l))),
        }
    }
}

// A label that is re-wrapped at its own measured width must not break again.
pub(crate) const WRAP_SLACK: f32 = 0.5;
