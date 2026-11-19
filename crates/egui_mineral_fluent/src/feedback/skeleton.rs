use egui::{Color32, CornerRadius, Rect, Sense, vec2};
use egui_mineral::{
    i18n::I18n,
    kit,
    layout::{Constraints, Measure},
    semantic::{self, AgentKey, KeyedSalt, Role, SemanticNode},
    widget::{MineralUi, MineralWidget, WidgetFrame, WidgetResponse},
};

use crate::ColorToken::{ColorNeutralStencil1, ColorNeutralStencil1Alpha, ColorNeutralStencil2};
use crate::{Fluent, FluentExt, spec::defaults::skeleton as d, widget::paint};

/// Skeleton `animation`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum SkeletonAnimation {
    /// `wave`: a highlight sweeps across.
    #[default]
    Wave,
    /// `pulse`: the fill fades in and out.
    Pulse,
}

impl SkeletonAnimation {
    /// Every variant.
    pub const ALL: &'static [Self] = &[Self::Wave, Self::Pulse];
}

/// Skeleton `appearance`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum SkeletonAppearance {
    /// `opaque`: `colorNeutralStencil1`.
    #[default]
    Opaque,
    /// `translucent`: `colorNeutralStencil1Alpha`, for images and colored backgrounds.
    Translucent,
}

impl SkeletonAppearance {
    /// Every variant.
    pub const ALL: &'static [Self] = &[Self::Opaque, Self::Translucent];
}

/// `shape` of a [`SkeletonItem`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum SkeletonShape {
    /// `rectangle`: full width, `size` high, 4 rounded.
    #[default]
    Rectangle,
    /// `square`: `size` by `size`.
    Square,
    /// `circle`: `size` across.
    Circle,
}

impl SkeletonShape {
    /// Every variant.
    pub const ALL: &'static [Self] = &[Self::Rectangle, Self::Square, Self::Circle];
}

/// What a [`Skeleton`] hands its items: the shared animation and appearance.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SkeletonContext {
    animation: SkeletonAnimation,
    appearance: SkeletonAppearance,
}

impl SkeletonContext {
    /// Item sharing this skeleton's animation and appearance.
    pub fn item(self, id_salt: impl Into<KeyedSalt>) -> SkeletonItem {
        SkeletonItem::new(id_salt)
            .animation(self.animation)
            .appearance(self.appearance)
    }
}

/// Fluent `Skeleton`: groups [`SkeletonItem`]s under one animation and appearance and announces loading.
#[derive(Clone, Debug)]
#[must_use = "call .show(..)"]
pub struct Skeleton {
    salt: KeyedSalt,
    key: AgentKey,
    label: Option<String>,
    context: SkeletonContext,
}

impl Skeleton {
    /// Wave, opaque; agent key `<id_salt>-skeleton`.
    pub fn new(id_salt: impl Into<KeyedSalt>) -> Self {
        Self {
            salt: id_salt.into(),
            key: AgentKey::Kind("skeleton"),
            label: None,
            context: SkeletonContext::default(),
        }
    }

    /// Accessible name; default the `mineral-skeleton-loading` message.
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Default [`SkeletonAnimation::Wave`].
    pub fn animation(mut self, animation: SkeletonAnimation) -> Self {
        self.context.animation = animation;
        self
    }

    /// Default [`SkeletonAppearance::Opaque`].
    pub fn appearance(mut self, appearance: SkeletonAppearance) -> Self {
        self.context.appearance = appearance;
        self
    }

    crate::widget::setters! {
        /// Replaces the `<id_salt>-skeleton` agent key.
        agent_key(key)
    }

    /// Lays out `add` in a column, given the [`SkeletonContext`].
    pub fn show<R>(
        self,
        ui: &mut MineralUi<'_, Fluent>,
        add: impl FnOnce(&mut MineralUi<'_, Fluent>, SkeletonContext) -> R,
    ) -> egui::InnerResponse<R> {
        let rt = ui.rt();
        let dir = ui.i18n().direction();
        let cx = self.context;
        let out = ui.push_id(self.salt.id_salt(), |ui| {
            ui.scope_builder(egui::UiBuilder::new().layout(dir.column()), |ui| {
                add(&mut rt.ui(ui), cx)
            })
        });
        let inner = out.inner;
        let id = ui.make_persistent_id((self.salt.id_salt(), "skeleton"));
        let r = ui.interact(inner.response.rect, id, Sense::hover());
        semantic::emit(ui.ctx(), Some(r.id), r.rect, || {
            let name = crate::tr_or(rt.i18n(), self.label.as_deref(), "mineral-skeleton-loading");
            SemanticNode::new(self.key.id(&self.salt), Role::ProgressIndicator, name)
        });
        egui::InnerResponse::new(inner.inner, inner.response)
    }
}

/// Fluent `SkeletonItem`: a placeholder shape for content that is loading.
#[derive(Clone, Debug)]
#[must_use = "show with ui.mineral(..)"]
pub struct SkeletonItem {
    salt: KeyedSalt,
    shape: SkeletonShape,
    size: u16,
    animation: SkeletonAnimation,
    appearance: SkeletonAppearance,
}

impl SkeletonItem {
    /// Rectangle of 16 high, wave, opaque. Decorative: it has no agent node.
    pub fn new(id_salt: impl Into<KeyedSalt>) -> Self {
        Self {
            salt: id_salt.into(),
            shape: SkeletonShape::Rectangle,
            size: d::DEFAULT_SIZE,
            animation: SkeletonAnimation::Wave,
            appearance: SkeletonAppearance::Opaque,
        }
    }

    /// Default [`SkeletonShape::Rectangle`].
    pub fn shape(mut self, shape: SkeletonShape) -> Self {
        self.shape = shape;
        self
    }

    /// Height of a rectangle, side of a square, diameter of a circle; the nearest of
    /// spec sizes (8 to 128), default 16.
    pub fn size(mut self, size: u16) -> Self {
        self.size = d::SIZES
            .into_iter()
            .min_by_key(|s| s.abs_diff(size))
            .unwrap_or(d::DEFAULT_SIZE);
        self
    }

    /// Default [`SkeletonAnimation::Wave`].
    pub fn animation(mut self, animation: SkeletonAnimation) -> Self {
        self.animation = animation;
        self
    }

    /// Default [`SkeletonAppearance::Opaque`].
    pub fn appearance(mut self, appearance: SkeletonAppearance) -> Self {
        self.appearance = appearance;
        self
    }

    fn extent(&self) -> f32 {
        f32::from(self.size)
    }
}

impl MineralWidget for SkeletonItem {
    type Kit = Fluent;
    type State = ();
    type Output = ();

    kit::widget_meta!(salt, hover, no_target);

    fn layout(&self, ui: &MineralUi<'_, Fluent>, c: Constraints, (): &()) -> Measure {
        let h = self.extent();
        let w = match self.shape {
            SkeletonShape::Rectangle if c.max.x.is_finite() => c.max.x,
            SkeletonShape::Rectangle => ui.available_width(),
            SkeletonShape::Square | SkeletonShape::Circle => h,
        };
        c.constrain(vec2(w, h)).into()
    }

    fn update(&mut self, ui: &mut MineralUi<'_, Fluent>, frame: &WidgetFrame<'_>, (): &mut ()) -> WidgetResponse {
        let t = ui.tokens();
        let rect = frame.zones.visual;
        let radius = match self.shape {
            SkeletonShape::Rectangle => d::RECTANGLE_RADIUS,
            SkeletonShape::Square => 0.0,
            SkeletonShape::Circle => rect.width() / 2.0,
        };
        let r = CornerRadius::from(radius);
        let (solid, edge, middle) = match self.appearance {
            SkeletonAppearance::Opaque => (
                t.color(ColorNeutralStencil1),
                t.color(ColorNeutralStencil1),
                t.color(ColorNeutralStencil2),
            ),
            SkeletonAppearance::Translucent => (
                t.color(ColorNeutralStencil1Alpha),
                Color32::TRANSPARENT,
                t.color(ColorNeutralStencil1Alpha),
            ),
        };
        let reduced = ui.rt().core().motion().reduced();
        let time = ui.input(|i| i.time);
        let animate = |cycle: std::time::Duration| {
            egui_mineral::motion::request_frame(ui.ctx());
            let c = cycle.as_secs_f64();
            (time.rem_euclid(c) / c) as f32
        };
        let p = ui.painter();
        match self.animation {
            SkeletonAnimation::Wave => {
                p.rect_filled(rect, r, solid);
                if !reduced {
                    let k = d::WAVE_EASING.ease(animate(d::WAVE_CYCLE));
                    let w = rect.width();
                    let band = Rect::from_min_size(rect.min + vec2(-w + 2.0 * w * k, 0.0), rect.size());
                    paint::gradient_h(&p.with_clip_rect(rect), band, [edge, middle, edge]);
                }
            }
            SkeletonAnimation::Pulse => {
                let k = if reduced {
                    0.0
                } else {
                    let phase = animate(d::PULSE_CYCLE);
                    1.0 - (phase * 2.0 - 1.0).abs()
                };
                let opacity = 1.0 - k * (1.0 - d::PULSE_MIN_OPACITY);
                p.rect_filled(rect, r, solid.gamma_multiply(opacity));
            }
        }
        WidgetResponse::new(frame.response.clone(), ())
    }

    fn semantics(&self, _: &I18n, _: &WidgetResponse, (): &()) -> Option<SemanticNode> {
        None
    }
}
