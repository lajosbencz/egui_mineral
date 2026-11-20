use egui::{
    Color32, ImageSource, Rect, Shape, Stroke, StrokeKind, Vec2, epaint::RectShape, load::TexturePoll, pos2, vec2,
};
use egui_mineral::{
    i18n::I18n,
    kit,
    layout::{Constraints, Measure},
    semantic::{AgentKey, KeyedSalt, Role, SemanticNode},
    widget::{MineralUi, MineralWidget, WidgetFrame, WidgetResponse},
};

use crate::spec::ColorToken::{ColorNeutralBackground3, ColorNeutralForeground3, ColorNeutralStroke1};
use crate::{
    Fluent, FluentExt,
    spec::tokens::geometry,
    theme::{Elevation, Radius},
    widget::paint,
};

pub(super) enum Load {
    Pending,
    Failed,
    // Texture and natural size.
    Ready(egui::TextureId, Vec2),
}

#[track_caller]
pub(super) fn load(ctx: &egui::Context, source: &ImageSource<'_>, hint: Vec2) -> Load {
    match egui::Image::new(source.clone()).load_for_size(ctx, hint) {
        Ok(TexturePoll::Ready { texture }) => Load::Ready(texture.id, texture.size),
        Ok(TexturePoll::Pending { .. }) => Load::Pending,
        Err(e) => {
            kit::sanitize::report(format_args!("image failed to load: {e}"));
            Load::Failed
        }
    }
}

// Texture and natural size once the source has loaded.
pub(super) fn texture(ctx: &egui::Context, source: &ImageSource<'_>, hint: Vec2) -> Option<(egui::TextureId, Vec2)> {
    match load(ctx, source, hint) {
        Load::Ready(id, size) => Some((id, size)),
        Load::Pending | Load::Failed => None,
    }
}

/// Image `fit` (CSS `object-fit`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ImageFit {
    /// `default`: stretched to the box.
    #[default]
    Default,
    /// `none`: natural size, anchored at the top left, cropped.
    None,
    /// `center`: natural size, centered, cropped.
    Center,
    /// `contain`: scaled to fit whole, centered.
    Contain,
    /// `cover`: scaled to fill, centered, cropped.
    Cover,
}

impl ImageFit {
    /// Every variant.
    pub const ALL: &'static [Self] = &[Self::Default, Self::None, Self::Center, Self::Contain, Self::Cover];
}

/// Image `shape`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ImageShape {
    /// `square`: no corner radius.
    #[default]
    Square,
    /// `rounded`: `borderRadiusMedium`.
    Rounded,
    /// `circular`.
    Circular,
}

impl ImageShape {
    /// Every variant.
    pub const ALL: &'static [Self] = &[Self::Square, Self::Rounded, Self::Circular];
}

/// Fluent `Image`: a picture with fit modes, shapes, a border and a shadow.
#[derive(Clone, Debug)]
#[must_use = "show with ui.mineral(..)"]
pub struct Image<'a> {
    salt: KeyedSalt,
    key: AgentKey,
    source: ImageSource<'a>,
    alt: Option<String>,
    fit: ImageFit,
    shape: ImageShape,
    bordered: bool,
    shadow: bool,
    block: bool,
    size: Option<Vec2>,
}

impl<'a> Image<'a> {
    /// Natural size, square, plain; agent key `<id_salt>-image` once it has `alt`.
    pub fn new(id_salt: impl Into<KeyedSalt>, source: impl Into<ImageSource<'a>>) -> Self {
        Self {
            salt: id_salt.into(),
            key: egui_mineral::agent_key!("image"),
            source: source.into(),
            alt: None,
            fit: ImageFit::Default,
            shape: ImageShape::Square,
            bordered: false,
            shadow: false,
            block: false,
            size: None,
        }
    }

    /// `alt` text; without it the image is decorative and has no semantic node.
    pub fn alt(mut self, alt: impl Into<String>) -> Self {
        self.alt = Some(alt.into());
        self
    }

    /// Default [`ImageFit::Default`].
    pub fn fit(mut self, fit: ImageFit) -> Self {
        self.fit = fit;
        self
    }

    /// Default [`ImageShape::Square`].
    pub fn shape(mut self, shape: ImageShape) -> Self {
        self.shape = shape;
        self
    }

    /// `bordered`: 1pt `colorNeutralStroke1` border.
    pub fn bordered(mut self, bordered: bool) -> Self {
        self.bordered = bordered;
        self
    }

    /// `shadow`: `shadow4`.
    pub fn shadow(mut self, shadow: bool) -> Self {
        self.shadow = shadow;
        self
    }

    /// `block`: the full available width, height by aspect ratio unless sized.
    pub fn block(mut self, block: bool) -> Self {
        self.block = block;
        self
    }

    /// Explicit `width` and `height`, the box the fit mode applies in.
    pub fn size(mut self, size: Vec2) -> Self {
        self.size = Some(size);
        self
    }

    crate::widget::setters! {
        /// Replaces the `<id_salt>-image` agent key.
        agent_key(key)
    }

    fn load(&self, ui: &MineralUi<'_, Fluent>, hint: Vec2) -> Load {
        load(ui.ctx(), &self.source, hint)
    }

    fn natural(&self, ui: &MineralUi<'_, Fluent>, hint: Vec2) -> Vec2 {
        match self.load(ui, hint) {
            Load::Ready(_, size) => size,
            Load::Pending | Load::Failed => Vec2::ZERO,
        }
    }

    fn box_size(&self, natural: Vec2, avail: f32) -> Vec2 {
        let mut size = self.size.unwrap_or(natural);
        if self.block && self.size.is_none() && natural.x > 0.0 {
            size = vec2(avail, natural.y * avail / natural.x);
        } else if self.block {
            size.x = avail;
        }
        size
    }

    // react-image useImageStyles.styles.ts object-fit; `uv` is the visible part of the texture
    fn placement(&self, bounds: Rect, natural: Vec2) -> (Rect, Rect) {
        let full = Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0));
        if natural.x <= 0.0 || natural.y <= 0.0 {
            return (bounds, full);
        }
        let fit = |scale: f32, center: bool| {
            let content = vec2(natural.x * scale, natural.y * scale);
            let min = if center {
                bounds.center() - content / 2.0
            } else {
                bounds.min
            };
            let content = Rect::from_min_size(min, content);
            let dest = content.intersect(bounds);
            let uv = Rect::from_min_max(
                pos2(
                    (dest.min.x - content.min.x) / content.width(),
                    (dest.min.y - content.min.y) / content.height(),
                ),
                pos2(
                    (dest.max.x - content.min.x) / content.width(),
                    (dest.max.y - content.min.y) / content.height(),
                ),
            );
            (dest, uv)
        };
        let (sx, sy) = (bounds.width() / natural.x, bounds.height() / natural.y);
        match self.fit {
            ImageFit::Default => (bounds, full),
            ImageFit::None => fit(1.0, false),
            ImageFit::Center => fit(1.0, true),
            ImageFit::Contain => fit(sx.min(sy), true),
            ImageFit::Cover => fit(sx.max(sy), true),
        }
    }
}

impl MineralWidget for Image<'_> {
    type Kit = Fluent;
    type State = ();
    type Output = ();

    kit::widget_meta!(salt, hover, no_target);

    fn layout(&self, ui: &MineralUi<'_, Fluent>, c: Constraints, (): &()) -> Measure {
        let avail = if c.max.x.is_finite() {
            c.max.x
        } else {
            ui.available_width()
        };
        let hint = self.size.unwrap_or_else(|| vec2(avail, ui.available_height()));
        c.constrain(self.box_size(self.natural(ui, hint), avail)).into()
    }

    fn update(&mut self, ui: &mut MineralUi<'_, Fluent>, frame: &WidgetFrame<'_>, (): &mut ()) -> WidgetResponse {
        let t = ui.tokens();
        let rect = frame.zones.visual;
        let radius = match self.shape {
            ImageShape::Square => Radius::None,
            ImageShape::Rounded => Radius::Medium,
            ImageShape::Circular => Radius::Circular,
        }
        .on(rect);
        if self.shadow {
            paint::shadow(ui.painter(), rect, radius, Elevation::E4, &t);
        }
        match self.load(ui, rect.size()) {
            Load::Ready(id, natural) => {
                let (dest, uv) = self.placement(rect, natural);
                ui.painter().add(Shape::Rect(
                    RectShape::filled(dest, radius, Color32::WHITE).with_texture(id, uv),
                ));
            }
            Load::Failed => {
                let p = ui.painter();
                p.rect_filled(rect, radius, t.color(ColorNeutralBackground3));
                paint::icon(
                    p,
                    rect.center(),
                    ui.icons().image_off,
                    false,
                    (rect.width().min(rect.height()) / 2.0).clamp(0.0, 24.0),
                    t.color(ColorNeutralForeground3),
                );
            }
            Load::Pending => {}
        }
        if self.bordered {
            ui.painter().rect_stroke(
                rect,
                radius,
                Stroke::new(geometry::STROKE_WIDTH_THIN, t.color(ColorNeutralStroke1)),
                StrokeKind::Inside,
            );
        }
        WidgetResponse::new(frame.response.clone(), ())
    }

    fn semantics(&self, _: &I18n, _: &WidgetResponse, (): &()) -> Option<SemanticNode> {
        let alt = self.alt.as_ref()?;
        Some(SemanticNode::new(self.key.id(&self.salt), Role::Image, alt))
    }
}
