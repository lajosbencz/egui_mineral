// Accompanist placeholder-material3 v0.32.0 (Placeholder.kt, PlaceholderHighlight.kt, PlaceholderDefaults); deprecated upstream
use crate::Material;
use crate::MaterialExt;
use crate::msg;
use egui_mineral::semantic::AgentKey;
use egui_mineral::semantic::KeyedSalt;
use egui_mineral::widget::MineralUi;

use egui::{Color32, Mesh, Pos2, Rect, Ui, pos2, vec2};

use egui_mineral::{
    layout::Constraints,
    semantic::{Role, SemanticNode},
};

use crate::widget::{common, paint::paint_widget};
use crate::{
    spec::{ColorRole, ShapeRole, TypeScale, tokens::motion},
    tokens::Tokens,
};

// Compose PlaceholderDefaults.color
const CONTENT_COLOR: ColorRole = ColorRole::OnSurface;
const BACKGROUND_COLOR: ColorRole = ColorRole::Surface;
const CONTENT_ALPHA: f32 = 0.1;
// Compose PlaceholderDefaults.shimmerHighlightColor
const HIGHLIGHT_COLOR: ColorRole = ColorRole::InverseSurface;
const HIGHLIGHT_ALPHA: f32 = 0.75;
// Compose Modifier.placeholder, MaterialTheme.shapes.small
const SHAPE: ShapeRole = ShapeRole::CornerSmall;
// Compose shimmerAnimationSpec tween(durationMillis = 1700, delayMillis = 200), restarted
const SHIMMER_SECS: f64 = 1.7;
const SHIMMER_DELAY_SECS: f64 = 0.2;
// Compose Shimmer.progressForMaxAlpha
const PROGRESS_FOR_MAX_ALPHA: f32 = 0.6;
// A paragraph's ragged end
const LAST_LINE_FRACTION: f32 = 0.6;
const CELL: f32 = 4.0;

/// Placeholder geometry of a [`Skeleton`].
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub enum SkeletonShape {
    /// Bars of text height; the last of several is shorter.
    #[non_exhaustive]
    Text {
        /// Bar count; 0 draws nothing.
        lines: usize,
        /// Role whose line height each bar takes.
        scale: TypeScale,
    },
    /// Rounded rectangle of this size in dp.
    Rect(egui::Vec2),
    /// Circle of this diameter in dp.
    Circle(f32),
}

impl SkeletonShape {
    /// [`Self::Text`]: `lines` bars at the line height of `scale`.
    pub const fn text(lines: usize, scale: TypeScale) -> Self {
        Self::Text { lines, scale }
    }
}

/// Shimmering loading placeholder; non-MD3.
#[derive(Clone, Debug)]
#[must_use = "show with ui.mineral(..)"]
pub struct Skeleton {
    salt: KeyedSalt,
    key: AgentKey,
    shape: SkeletonShape,
}

// Compose PlaceholderDefaults.color
fn placeholder_color(t: &Tokens) -> Color32 {
    let bg = t.role(BACKGROUND_COLOR);
    let fg = t.role(CONTENT_COLOR);
    bg.lerp_to_gamma(fg, CONTENT_ALPHA)
}

// Compose Shimmer.alpha
fn shimmer_progress(time: f64) -> (f32, f32) {
    let t = (time % (SHIMMER_DELAY_SECS + SHIMMER_SECS) - SHIMMER_DELAY_SECS).max(0.0) / SHIMMER_SECS;
    let progress = motion::EASING_LEGACY_CUBIC_BEZIER.ease(t as f32);
    let alpha = if progress <= PROGRESS_FOR_MAX_ALPHA {
        progress / PROGRESS_FOR_MAX_ALPHA
    } else {
        (1.0 - progress) / (1.0 - PROGRESS_FOR_MAX_ALPHA)
    };
    (progress, alpha)
}

impl Skeleton {
    /// Agent key `<id_salt>-skeleton`.
    pub fn new(id_salt: impl Into<KeyedSalt>, shape: SkeletonShape) -> Self {
        let id_salt: KeyedSalt = id_salt.into();
        Self {
            key: AgentKey::Kind("skeleton"),
            salt: id_salt,
            shape,
        }
    }
    crate::widget::setters! {
        /// Replaces the `<id_salt>-skeleton` agent key.
        agent_key(key)
    }

    fn natural(&self, w: f32) -> egui::Vec2 {
        match self.shape {
            SkeletonShape::Text { lines, scale } => vec2(w, scale.line_height() * lines as f32),
            SkeletonShape::Rect(s) => s,
            SkeletonShape::Circle(d) => vec2(d, d),
        }
    }

    fn bars(&self, t: &Tokens, size: egui::Vec2) -> Vec<(Rect, u8)> {
        let small = SHAPE.scale().radius(&t.shape);
        match self.shape {
            SkeletonShape::Text { lines, scale } => {
                let glyph = scale.size();
                let lh = size.y / lines as f32;
                (0..lines)
                    .map(|i| {
                        let bw = if i + 1 == lines && lines > 1 {
                            size.x * LAST_LINE_FRACTION
                        } else {
                            size.x
                        };
                        let top = i as f32 * lh + (lh - glyph).max(0.0) / 2.0;
                        (Rect::from_min_size(pos2(0.0, top), vec2(bw, glyph.min(lh))), small)
                    })
                    .collect()
            }
            SkeletonShape::Rect(_) => vec![(Rect::from_min_size(Pos2::ZERO, size), small)],
            SkeletonShape::Circle(_) => {
                let d = size.min_elem();
                vec![(Rect::from_min_size(Pos2::ZERO, vec2(d, d)), t.shape.full)]
            }
        }
    }

    fn extent(&self, ui: &Ui, c: Constraints) -> egui::Vec2 {
        let w = if c.max.x.is_finite() {
            c.max.x
        } else {
            ui.available_width()
        };
        c.constrain(self.natural(w))
    }

    fn paint(&self, ui: &mut MineralUi<'_, Material>, rect: Rect) {
        let rt = ui.rt();
        if !ui.is_rect_visible(rect) {
            return;
        }
        let t = ui.tokens();
        let bars = self.bars(&t, rect.size());
        let ctx = ui.ctx().clone();
        let dir = rt.i18n().direction();
        let base = placeholder_color(&t);
        let highlight = t.role_alpha(HIGHLIGHT_COLOR, HIGHLIGHT_ALPHA);
        let animate = !rt.core().motion().reduced();
        let (progress, alpha) = shimmer_progress(ctx.input(|i| i.time));
        let p = ui.painter();
        for (bar, r) in &bars {
            let bar = dir.mirror(bar.translate(rect.min.to_vec2()), rect);
            let r = common::radius(bar, *r);
            p.rect_filled(bar, r, base);
            if animate && alpha > 0.0 {
                shimmer(p, bar, f32::from(r.nw), progress, highlight.gamma_multiply(alpha));
            }
        }
        if animate {
            egui_mineral::motion::request_frame(&ctx);
        }
    }

    fn node(&self, i18n: &egui_mineral::i18n::I18n) -> SemanticNode {
        let name = crate::tr(i18n, msg::SKELETON);
        let mut n = SemanticNode::new(self.key.id(&self.salt), Role::Status, name);
        n.state.live = Some(egui_mineral::semantic::Live::Polite);
        n
    }
}

paint_widget!(Skeleton);

// Compose Shimmer.brush
fn shimmer(p: &egui::Painter, bar: Rect, radius: f32, progress: f32, glow: Color32) {
    let reach = (bar.width().max(bar.height()) * progress * 2.0).max(f32::EPSILON);
    let color_at = |q: Pos2| {
        let d = (q - bar.min).length() / reach;
        glow.gamma_multiply((1.0 - (2.0 * d - 1.0).abs()).max(0.0))
    };
    // Vertical extent lost to the rounded corners at `x`.
    let inset = |x: f32| {
        let d = (x - bar.left()).min(bar.right() - x);
        if d >= radius {
            0.0
        } else {
            let k = radius - d.max(0.0);
            radius - (radius * radius - k * k).max(0.0).sqrt()
        }
    };
    #[expect(clippy::cast_sign_loss, reason = ">= 1")]
    let cells = |len: f32| (len / CELL).ceil().max(1.0) as u32;
    let (cols, rows) = (cells(bar.width()), cells(bar.height()));
    let mut mesh = Mesh::default();
    for c in 0..=cols {
        let x = egui::lerp(bar.x_range(), c as f32 / cols as f32);
        let i = inset(x);
        for r in 0..=rows {
            let y = egui::lerp(bar.top() + i..=bar.bottom() - i, r as f32 / rows as f32);
            let q = pos2(x, y);
            mesh.colored_vertex(q, color_at(q));
        }
    }
    let at = |c: u32, r: u32| c * (rows + 1) + r;
    for c in 0..cols {
        for r in 0..rows {
            mesh.add_triangle(at(c, r), at(c + 1, r), at(c, r + 1));
            mesh.add_triangle(at(c + 1, r), at(c + 1, r + 1), at(c, r + 1));
        }
    }
    p.add(mesh);
}
