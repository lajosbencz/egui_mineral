use crate::msg;
use egui_mineral::select::ItemKey;
use std::sync::Arc;

use egui::{CornerRadius, Galley, IdSalt, Key, Rect, Sense, Stroke, StrokeKind, pos2, text::LayoutJob, vec2};
use egui_mineral::{
    focus::{self, RovingFocus, TabStop},
    i18n::I18n,
    kit,
    layout::{Axis, Constraints, Measure},
    select::Selection,
    semantic::{self, AgentKey, KeyedSalt, Role, SemanticNode},
    widget::{Availability, Interaction, MineralUi, MineralWidget, WidgetFrame, WidgetResponse},
};

use crate::spec::ColorToken::{
    ColorBrandBackground, ColorBrandBackground2, ColorBrandBackground2Hover, ColorBrandBackground2Pressed,
    ColorBrandBackgroundHover, ColorBrandBackgroundPressed, ColorBrandForeground2, ColorBrandStroke1,
    ColorBrandStroke2, ColorCompoundBrandForeground1Hover, ColorCompoundBrandForeground1Pressed,
    ColorNeutralBackground3, ColorNeutralBackground3Hover, ColorNeutralBackground3Pressed,
    ColorNeutralBackgroundDisabled, ColorNeutralForeground2, ColorNeutralForeground2BrandHover,
    ColorNeutralForeground2BrandPressed, ColorNeutralForeground2Hover, ColorNeutralForeground2Pressed,
    ColorNeutralForegroundDisabled, ColorNeutralForegroundOnBrand, ColorNeutralStroke1, ColorNeutralStrokeDisabled,
    ColorNeutralStrokeOnBrand2, ColorStrokeFocus2, ColorSubtleBackground, ColorSubtleBackgroundHover,
    ColorSubtleBackgroundPressed, ColorTransparentStrokeDisabled,
};
use crate::{
    Fluent, FluentExt, Icon, Rt,
    spec::ColorToken,
    spec::{
        defaults::tag as d,
        tokens::geometry::{BORDER_RADIUS_MEDIUM, STROKE_WIDTH_THICK, STROKE_WIDTH_THIN},
    },
    theme::Tokens,
    typography::TypeRamp,
    widget::paint,
};

use super::{Avatar, AvatarShape, AvatarSize};

/// Tag `appearance`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum TagAppearance {
    /// `filled`.
    #[default]
    Filled,
    /// `outline`.
    Outline,
    /// `brand`.
    Brand,
}

impl TagAppearance {
    /// Every variant.
    pub const ALL: &'static [Self] = &[Self::Filled, Self::Outline, Self::Brand];
}

/// Tag `shape`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum TagShape {
    /// `rounded`: `borderRadiusMedium`.
    #[default]
    Rounded,
    /// `circular`: pill.
    Circular,
}

impl TagShape {
    /// Every variant.
    pub const ALL: &'static [Self] = &[Self::Rounded, Self::Circular];
}

/// Tag `size`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum TagSize {
    /// `medium`: 32 high.
    #[default]
    Medium,
    /// `small`: 24 high.
    Small,
    /// `extra-small`: 20 high.
    ExtraSmall,
}

impl TagSize {
    /// Every variant, largest first.
    pub const ALL: &'static [Self] = &[Self::Medium, Self::Small, Self::ExtraSmall];

    const fn metrics(self) -> d::Metrics {
        match self {
            Self::Medium => d::MEDIUM,
            Self::Small => d::SMALL,
            Self::ExtraSmall => d::EXTRA_SMALL,
        }
    }

    const fn avatar(self) -> AvatarSize {
        match self {
            Self::Medium => AvatarSize::S28,
            Self::Small => AvatarSize::S20,
            Self::ExtraSmall => AvatarSize::S16,
        }
    }

    const fn primary(self) -> TypeRamp {
        match self {
            Self::Medium => TypeRamp::Body1,
            Self::Small | Self::ExtraSmall => TypeRamp::Caption1,
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct Look {
    appearance: TagAppearance,
    shape: TagShape,
    size: TagSize,
    selected: bool,
    disabled: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Level {
    Rest,
    Hover,
    Pressed,
}

impl Level {
    fn of(i: Interaction) -> Self {
        if i.pressed {
            Self::Pressed
        } else if i.hovered {
            Self::Hover
        } else {
            Self::Rest
        }
    }
}

struct Colors {
    bg: ColorToken,
    fg: ColorToken,
    border: Option<ColorToken>,
    divider: Option<ColorToken>,
}

impl Look {
    // Tag, InteractionTagPrimary and InteractionTagSecondary root styles per appearance and state
    fn colors(self, level: Level, secondary: bool) -> Colors {
        let pick = |rest, hover, pressed| match level {
            Level::Rest => rest,
            Level::Hover => hover,
            Level::Pressed => pressed,
        };
        let neutral_fg = if secondary {
            (ColorNeutralForeground2BrandHover, ColorNeutralForeground2BrandPressed)
        } else {
            (ColorNeutralForeground2Hover, ColorNeutralForeground2Pressed)
        };
        if self.disabled {
            return if self.appearance == TagAppearance::Outline {
                Colors {
                    bg: ColorSubtleBackground,
                    fg: ColorNeutralForegroundDisabled,
                    border: Some(ColorNeutralStrokeDisabled),
                    divider: None,
                }
            } else {
                Colors {
                    bg: ColorNeutralBackgroundDisabled,
                    fg: ColorNeutralForegroundDisabled,
                    border: Some(ColorTransparentStrokeDisabled),
                    divider: Some(ColorNeutralStrokeDisabled),
                }
            };
        }
        if self.selected {
            return Colors {
                bg: pick(
                    ColorBrandBackground,
                    ColorBrandBackgroundHover,
                    ColorBrandBackgroundPressed,
                ),
                fg: ColorNeutralForegroundOnBrand,
                border: Some(ColorBrandStroke1),
                divider: Some(ColorNeutralStrokeOnBrand2),
            };
        }
        match self.appearance {
            TagAppearance::Filled => Colors {
                bg: pick(
                    ColorNeutralBackground3,
                    ColorNeutralBackground3Hover,
                    ColorNeutralBackground3Pressed,
                ),
                fg: pick(ColorNeutralForeground2, neutral_fg.0, neutral_fg.1),
                border: None,
                divider: Some(ColorNeutralStroke1),
            },
            TagAppearance::Outline => Colors {
                bg: pick(
                    ColorSubtleBackground,
                    ColorSubtleBackgroundHover,
                    ColorSubtleBackgroundPressed,
                ),
                fg: pick(ColorNeutralForeground2, neutral_fg.0, neutral_fg.1),
                border: Some(ColorNeutralStroke1),
                divider: Some(ColorNeutralStroke1),
            },
            TagAppearance::Brand => Colors {
                bg: pick(
                    ColorBrandBackground2,
                    ColorBrandBackground2Hover,
                    ColorBrandBackground2Pressed,
                ),
                fg: pick(
                    ColorBrandForeground2,
                    ColorCompoundBrandForeground1Hover,
                    ColorCompoundBrandForeground1Pressed,
                ),
                border: None,
                divider: Some(ColorBrandStroke2),
            },
        }
    }

    fn radius(self, rect: Rect, kind: Kind) -> CornerRadius {
        let r = match self.shape {
            TagShape::Rounded => BORDER_RADIUS_MEDIUM,
            TagShape::Circular => rect.height() / 2.0,
        };
        let all = CornerRadius::from(r);
        match kind {
            Kind::PrimaryOfPair => CornerRadius { ne: 0, se: 0, ..all },
            Kind::Secondary => CornerRadius { nw: 0, sw: 0, ..all },
            _ => all,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    // Tag: static, or one dismiss button when dismissible
    Tag { dismissible: bool },
    // InteractionTagPrimary alone, or beside a secondary button
    Primary,
    PrimaryOfPair,
    Secondary,
}

impl Kind {
    const fn interactive(self) -> bool {
        !matches!(self, Self::Tag { dismissible: false })
    }

    const fn dismisses(self) -> bool {
        matches!(self, Self::Tag { dismissible: true } | Self::Secondary)
    }
}

#[derive(Clone, Debug)]
struct Content<'a> {
    media: Option<Avatar<'a>>,
    icon: Option<Icon>,
    primary: String,
    secondary: Option<String>,
}

struct Part<'a> {
    salt: KeyedSalt,
    key: AgentKey,
    name: String,
    look: Look,
    kind: Kind,
    content: Option<Content<'a>>,
    pressed: Option<bool>,
    tab_stop: TabStop,
}

struct Lines {
    primary: Arc<Galley>,
    secondary: Option<Arc<Galley>>,
}

impl Content<'_> {
    fn lines(&self, ui: &MineralUi<'_, Fluent>, size: TagSize, color: egui::Color32) -> Lines {
        let layout = |text: &str, ramp: TypeRamp| {
            ui.painter()
                .layout_job(LayoutJob::single_section(text.to_owned(), ramp.format(ui.ctx(), color)))
        };
        let ramp = if self.secondary.is_some() {
            TypeRamp::Caption1
        } else {
            size.primary()
        };
        Lines {
            primary: layout(&self.primary, ramp),
            secondary: self.secondary.as_deref().map(|s| layout(s, TypeRamp::Caption2)),
        }
    }
}

impl Part<'_> {
    const fn dismiss_icon(&self) -> bool {
        matches!(self.kind, Kind::Tag { dismissible: true })
    }

    fn width(&self, ui: &MineralUi<'_, Fluent>) -> f32 {
        let m = self.look.size.metrics();
        let Some(c) = &self.content else {
            return 2.0 * d::BORDER + 2.0 * m.secondary_padding + m.icon;
        };
        let lines = c.lines(ui, self.look.size, egui::Color32::WHITE);
        let lead = if c.media.is_some() {
            d::MEDIA_PADDING_START + m.media + m.media_padding_end
        } else if c.icon.is_some() {
            m.side_padding + m.icon + m.icon_padding_end
        } else {
            m.side_padding
        };
        let text = lines.primary.size().x.max(lines.secondary.map_or(0.0, |g| g.size().x)) + 2.0 * d::TEXT_PADDING;
        let end = if self.dismiss_icon() {
            m.dismiss_padding_start + m.icon + m.side_padding
        } else if self.kind == Kind::PrimaryOfPair {
            m.primary_padding_end
        } else {
            m.side_padding
        };
        2.0 * d::BORDER + lead + text + end
    }

    fn paint(&self, ui: &MineralUi<'_, Fluent>, rect: Rect, i: Interaction, t: &Tokens) {
        // A dismissible Tag changes only its icon on hover.
        let level = if self.kind.interactive() && !matches!(self.kind, Kind::Tag { .. }) {
            Level::of(i)
        } else {
            Level::Rest
        };
        let colors = self.look.colors(level, self.kind == Kind::Secondary);
        let radius = self.look.radius(rect, self.kind);
        let p = ui.painter();
        p.rect_filled(rect, radius, t.color(colors.bg));
        let bordered = self.look.appearance == TagAppearance::Outline || self.look.selected || self.look.disabled;
        if let Some(border) = colors.border.filter(|_| bordered) {
            p.rect_stroke(
                rect,
                radius,
                Stroke::new(STROKE_WIDTH_THIN, t.color(border)),
                StrokeKind::Inside,
            );
        }
        match self.kind {
            Kind::PrimaryOfPair => {
                let strip = Rect::from_min_max(
                    pos2(rect.right() - d::BORDER, rect.top() + d::BORDER),
                    pos2(rect.right(), rect.bottom() - d::BORDER),
                );
                p.rect_filled(strip, 0.0, t.color(colors.bg));
            }
            Kind::Secondary => {
                if let Some(divider) = colors.divider {
                    let line = Rect::from_min_max(rect.left_top(), pos2(rect.left() + d::BORDER, rect.bottom()));
                    p.rect_filled(line, 0.0, t.color(divider));
                }
            }
            _ => {}
        }
        self.paint_content(ui, rect, (i, level), t.color(colors.fg), t);
    }

    fn paint_content(
        &self,
        ui: &MineralUi<'_, Fluent>,
        rect: Rect,
        (i, level): (Interaction, Level),
        fg: egui::Color32,
        t: &Tokens,
    ) {
        let m = self.look.size.metrics();
        let p = ui.painter();
        let dir = ui.i18n().direction();
        let mid = rect.center().y;
        let x_of = |from: f32, w: f32| dir.start_x(rect, egui::Rangef::new(from, from + w));
        let Some(c) = &self.content else {
            let x = d::BORDER + m.secondary_padding;
            paint::icon(
                p,
                pos2(x_of(x, m.icon) + m.icon / 2.0, mid),
                ui.icons().dismiss,
                false,
                m.icon,
                fg,
            );
            return;
        };
        let lines = c.lines(ui, self.look.size, fg);
        let mut x = d::BORDER;
        if let Some(avatar) = &c.media {
            x += d::MEDIA_PADDING_START;
            let mut a = avatar.clone().size(self.look.size.avatar());
            a.shape = match self.look.shape {
                TagShape::Rounded => AvatarShape::Square,
                TagShape::Circular => AvatarShape::Circular,
            };
            let at = Rect::from_min_size(pos2(x_of(x, m.media), mid - m.media / 2.0), vec2(m.media, m.media));
            a.paint(ui, at, t);
            x += m.media + m.media_padding_end;
        } else {
            x += m.side_padding;
            if let Some(icon) = c.icon {
                // InteractionTag outline: the filled glyph in brand colors while engaged
                let filled = self.look.appearance == TagAppearance::Outline && level != Level::Rest;
                paint::icon(p, pos2(x_of(x, m.icon) + m.icon / 2.0, mid), icon, filled, m.icon, fg);
                x += m.icon + m.icon_padding_end;
            }
        }
        x += d::TEXT_PADDING;
        let line_h = lines.primary.size().y;
        let (top, text_w) = match &lines.secondary {
            Some(s) => (
                mid - (line_h + d::SECONDARY_OFFSET + s.size().y) / 2.0 + d::SECONDARY_OFFSET,
                lines.primary.size().x.max(s.size().x),
            ),
            None => (
                mid - f32::midpoint(line_h, d::PRIMARY_PADDING_BOTTOM),
                lines.primary.size().x,
            ),
        };
        p.galley(pos2(x_of(x, lines.primary.size().x), top), lines.primary, fg);
        if let Some(s) = lines.secondary {
            p.galley(pos2(x_of(x, s.size().x), top + line_h), s, fg);
        }
        x += text_w + d::TEXT_PADDING;
        if self.dismiss_icon() {
            x += m.dismiss_padding_start;
            let hot = i.hovered && !self.look.disabled && !self.look.selected;
            let color = match (hot, Level::of(i)) {
                (true, Level::Pressed) => t.color(ColorCompoundBrandForeground1Pressed),
                (true, _) => t.color(ColorCompoundBrandForeground1Hover),
                _ => fg,
            };
            paint::icon(
                p,
                pos2(x_of(x, m.icon) + m.icon / 2.0, mid),
                ui.icons().dismiss,
                false,
                m.icon,
                color,
            );
        }
    }
}

impl MineralWidget for Part<'_> {
    type Kit = Fluent;
    type State = ();
    type Output = bool;

    kit::widget_meta!(salt, no_target);

    fn sense(&self) -> Sense {
        if self.kind.interactive() {
            focus::roving_sense(Sense::click(), self.tab_stop)
        } else {
            Sense::hover()
        }
    }

    fn layout(&self, ui: &MineralUi<'_, Fluent>, c: Constraints, (): &()) -> Measure {
        c.constrain(vec2(self.width(ui), self.look.size.metrics().height))
            .into()
    }

    fn update(&mut self, ui: &mut MineralUi<'_, Fluent>, frame: &WidgetFrame<'_>, (): &mut ()) -> WidgetResponse<bool> {
        let t = ui.tokens();
        let r = &frame.response;
        let availability = Availability::disabled_if(self.look.disabled);
        let i = Interaction::of(r, availability);
        if self.kind.interactive() {
            kit::interact::hover_cursor(ui.core(), r, availability);
            kit::interact::follow_press(r);
        }
        let rect = frame.zones.visual;
        self.paint(ui, rect, i, &t);
        if i.focused && self.kind.interactive() {
            ui.painter().rect_stroke(
                rect,
                self.look.radius(rect, self.kind),
                Stroke::new(STROKE_WIDTH_THICK, t.color(ColorStrokeFocus2)),
                StrokeKind::Outside,
            );
        }
        let by_key = self.kind.dismisses()
            && r.has_focus()
            && !self.look.disabled
            && ui.input(|inp| inp.key_pressed(Key::Delete) || inp.key_pressed(Key::Backspace));
        let activated = (r.clicked() && !self.look.disabled) || by_key;
        WidgetResponse::new(r.clone(), activated)
    }

    fn semantics(&self, _: &I18n, _: &WidgetResponse<bool>, (): &()) -> Option<SemanticNode> {
        let id = self.key.id(&self.salt);
        if !self.kind.interactive() {
            return Some(SemanticNode::new(id, Role::Label, &self.name));
        }
        let mut n =
            SemanticNode::new(id, Role::Button, &self.name).clickable(Availability::disabled_if(self.look.disabled));
        n.state.checked = self.pressed.map(Into::into);
        Some(n)
    }
}

macro_rules! look_setters {
    () => {
        /// Default [`TagAppearance::Filled`], or the group's.
        pub fn appearance(mut self, appearance: TagAppearance) -> Self {
            self.appearance = Some(appearance);
            self
        }

        /// Default [`TagShape::Rounded`].
        pub fn shape(mut self, shape: TagShape) -> Self {
            self.shape = shape;
            self
        }

        /// Default [`TagSize::Medium`], or the group's.
        pub fn size(mut self, size: TagSize) -> Self {
            self.size = Some(size);
            self
        }

        /// `media` slot: an [`Avatar`] sized by the tag.
        pub fn media(mut self, avatar: Avatar<'a>) -> Self {
            self.media = Some(avatar);
            self
        }

        /// `icon` slot before the text; ignored with `media`.
        pub fn icon(mut self, icon: impl Into<Icon>) -> Self {
            self.icon = Some(icon.into());
            self
        }

        /// `secondaryText` slot below the primary text.
        pub fn secondary_text(mut self, text: impl Into<String>) -> Self {
            self.secondary = Some(text.into());
            self
        }

        /// Greys out the tag and ignores input.
        pub fn disabled(mut self, disabled: bool) -> Self {
            self.disabled = disabled;
            self
        }
    };
}

/// Fluent `Tag`: a label with optional media, icon and secondary text; a dismissible tag is one button whose
/// output is `true` on a click or Delete/Backspace.
#[derive(Clone, Debug)]
#[must_use = "show with ui.mineral(..) or add to a TagGroup"]
pub struct Tag<'a> {
    salt: KeyedSalt,
    label: String,
    appearance: Option<TagAppearance>,
    shape: TagShape,
    size: Option<TagSize>,
    media: Option<Avatar<'a>>,
    icon: Option<Icon>,
    secondary: Option<String>,
    dismissible: Option<bool>,
    selected: bool,
    disabled: bool,
    key: AgentKey,
}

impl<'a> Tag<'a> {
    /// `id_salt` is unique within a [`TagGroup`]; `label` is the text and accessible name. Agent key
    /// `<id_salt>-tag`.
    pub fn new(id_salt: impl Into<KeyedSalt>, label: impl Into<String>) -> Self {
        Self {
            salt: id_salt.into(),
            label: label.into(),
            appearance: None,
            shape: TagShape::Rounded,
            size: None,
            media: None,
            icon: None,
            secondary: None,
            dismissible: None,
            selected: false,
            disabled: false,
            key: egui_mineral::agent_key!("tag"),
        }
    }

    look_setters!();

    /// `dismissible`: a dismiss icon after the text; default the group's.
    pub fn dismissible(mut self, dismissible: bool) -> Self {
        self.dismissible = Some(dismissible);
        self
    }

    /// `selected`: the brand-filled look.
    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    crate::widget::setters! {
        /// Replaces the `<id_salt>-tag` agent key.
        agent_key(key)
    }

    fn part(&self, defaults: Defaults, tab_stop: TabStop) -> Part<'a> {
        Part {
            salt: self.salt.clone(),
            key: self.key.clone(),
            name: self.label.clone(),
            look: Look {
                appearance: self.appearance.unwrap_or(defaults.appearance),
                shape: self.shape,
                size: self.size.unwrap_or(defaults.size),
                selected: self.selected,
                disabled: self.disabled || defaults.disabled,
            },
            kind: Kind::Tag {
                dismissible: self.dismissible.unwrap_or(defaults.dismissible),
            },
            content: Some(Content {
                media: self.media.clone(),
                icon: self.icon,
                primary: self.label.clone(),
                secondary: self.secondary.clone(),
            }),
            pressed: None,
            tab_stop,
        }
    }
}

impl MineralWidget for Tag<'_> {
    type Kit = Fluent;
    type State = ();
    type Output = bool;

    kit::widget_meta!(salt, no_target);

    fn sense(&self) -> Sense {
        self.part(Defaults::default(), TabStop::Stop).sense()
    }

    fn layout(&self, ui: &MineralUi<'_, Fluent>, c: Constraints, s: &()) -> Measure {
        self.part(Defaults::default(), TabStop::Stop).layout(ui, c, s)
    }

    fn update(&mut self, ui: &mut MineralUi<'_, Fluent>, frame: &WidgetFrame<'_>, s: &mut ()) -> WidgetResponse<bool> {
        self.part(Defaults::default(), TabStop::Stop).update(ui, frame, s)
    }

    fn semantics(&self, i18n: &I18n, r: &WidgetResponse<bool>, s: &()) -> Option<SemanticNode> {
        self.part(Defaults::default(), TabStop::Stop).semantics(i18n, r, s)
    }
}

/// Fluent `InteractionTag`: a primary button with an optional secondary dismiss button; the primary
/// click selects it in a [`TagGroup`] with a selection.
#[derive(Clone, Debug)]
#[must_use = "call .show(..) or add to a TagGroup"]
pub struct InteractionTag<'a> {
    salt: KeyedSalt,
    label: String,
    appearance: Option<TagAppearance>,
    shape: TagShape,
    size: Option<TagSize>,
    media: Option<Avatar<'a>>,
    icon: Option<Icon>,
    secondary: Option<String>,
    dismissible: Option<bool>,
    disabled: bool,
    key: AgentKey,
}

/// Result of showing an [`InteractionTag`].
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct InteractionTagResponse {
    /// Primary button.
    pub response: egui::Response,
    /// Secondary (dismiss) button when dismissible.
    pub secondary: Option<egui::Response>,
    /// Primary button activated this frame.
    pub clicked: bool,
    /// Secondary button clicked, or Delete or Backspace pressed on it.
    pub dismiss_clicked: bool,
}

impl<'a> InteractionTag<'a> {
    /// `id_salt` is unique within a [`TagGroup`]; `label` is the text and accessible name. Agent key
    /// `<id_salt>-interaction-tag`, and `<id_salt>-dismiss` for the secondary button.
    pub fn new(id_salt: impl Into<KeyedSalt>, label: impl Into<String>) -> Self {
        Self {
            salt: id_salt.into(),
            label: label.into(),
            appearance: None,
            shape: TagShape::Rounded,
            size: None,
            media: None,
            icon: None,
            secondary: None,
            dismissible: None,
            disabled: false,
            key: egui_mineral::agent_key!("interaction-tag"),
        }
    }

    look_setters!();

    /// Adds the secondary dismiss button; default the group's.
    pub fn dismissible(mut self, dismissible: bool) -> Self {
        self.dismissible = Some(dismissible);
        self
    }

    crate::widget::setters! {
        /// Replaces the `<id_salt>-interaction-tag` agent key.
        agent_key(key)
    }

    fn parts(
        &self,
        defaults: Defaults,
        selected: bool,
        stops: [TabStop; 2],
        dismiss_label: String,
    ) -> (Part<'a>, Option<Part<'a>>) {
        let dismissible = self.dismissible.unwrap_or(defaults.dismissible);
        let look = Look {
            appearance: self.appearance.unwrap_or(defaults.appearance),
            shape: self.shape,
            size: self.size.unwrap_or(defaults.size),
            selected,
            disabled: self.disabled || defaults.disabled,
        };
        let primary = Part {
            salt: self.salt.clone(),
            key: self.key.clone(),
            name: self.label.clone(),
            look,
            kind: if dismissible {
                Kind::PrimaryOfPair
            } else {
                Kind::Primary
            },
            content: Some(Content {
                media: self.media.clone(),
                icon: self.icon,
                primary: self.label.clone(),
                secondary: self.secondary.clone(),
            }),
            pressed: defaults.selectable.then_some(selected),
            tab_stop: stops[0],
        };
        let secondary = dismissible.then(|| Part {
            salt: self.salt.child("dismiss"),
            key: AgentKey::Custom(egui_mineral::semantic::StableId::of(&self.salt, "dismiss")),
            name: dismiss_label,
            look,
            kind: Kind::Secondary,
            content: None,
            pressed: None,
            tab_stop: stops[1],
        });
        (primary, secondary)
    }

    // Shows both parts; `ids` get the focusable ones in order.
    fn show_in(
        &self,
        rt: &Rt,
        ui: &mut egui::Ui,
        defaults: Defaults,
        selected: bool,
        stops: [TabStop; 2],
        ids: &mut Vec<egui::Id>,
    ) -> InteractionTagResponse {
        let dismiss = rt.i18n().tr(msg::TagDismiss {
            label: self.label.clone().into(),
        });
        let (primary, secondary) = self.parts(defaults, selected, stops, dismiss);
        let height = self.size.unwrap_or(defaults.size).metrics().height;
        let row = crate::widget::row(ui, rt.i18n().direction(), height);
        ui.scope_builder(row, |ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
            let p = rt.ui(ui).mineral(primary);
            ids.push(p.id);
            let s = secondary.map(|s| {
                let r = rt.ui(ui).mineral(s);
                ids.push(r.id);
                r
            });
            InteractionTagResponse {
                clicked: *p.output(),
                dismiss_clicked: s.as_ref().is_some_and(|s| *s.output()),
                secondary: s.map(WidgetResponse::into_response),
                response: p.into_response(),
            }
        })
        .inner
    }

    /// Shows the tag in the current row.
    pub fn show(self, ui: &mut MineralUi<'_, Fluent>) -> InteractionTagResponse {
        let rt = ui.rt();
        self.show_in(rt, ui, Defaults::default(), false, [TabStop::Stop; 2], &mut Vec::new())
    }
}

#[derive(Clone, Copy, Debug)]
struct Defaults {
    appearance: TagAppearance,
    size: TagSize,
    dismissible: bool,
    disabled: bool,
    selectable: bool,
}

impl Default for Defaults {
    fn default() -> Self {
        Self {
            appearance: TagAppearance::Filled,
            size: TagSize::Medium,
            dismissible: false,
            disabled: false,
            selectable: false,
        }
    }
}

enum Entry<'a> {
    Tag(Tag<'a>),
    Interaction(InteractionTag<'a>),
}

// Entry to refocus, by its salt, after the entry `gone` leaves the data, and whether its last stop: the next
// entry with a stop, else the previous one.
type Refocus = (IdSalt, bool);

fn neighbor(entries: &[Entry<'_>], d: Defaults, gone: IdSalt) -> Option<Refocus> {
    let at = entries.iter().position(|e| e.id() == gone)?;
    let has_stop = |e: &&Entry<'_>| !e.stops(d).is_empty();
    entries
        .get(at + 1..)
        .and_then(|after| after.iter().find(has_stop))
        .map(|e| (e.id(), false))
        .or_else(|| {
            entries
                .get(..at)
                .and_then(|before| before.iter().rev().find(has_stop))
                .map(|e| (e.id(), true))
        })
}

// Index among all stops of the first (or last) stop of the entry `id`.
fn stop_of(entries: &[Entry<'_>], d: Defaults, id: IdSalt, last: bool) -> Option<usize> {
    let before: usize = entries
        .iter()
        .take_while(|e| e.id() != id)
        .map(|e| e.stops(d).len())
        .sum();
    let own = entries.iter().find(|e| e.id() == id)?.stops(d).len();
    (own > 0).then(|| if last { before + own - 1 } else { before })
}

impl Entry<'_> {
    fn id(&self) -> IdSalt {
        match self {
            Self::Tag(t) => t.salt.id_salt(),
            Self::Interaction(t) => t.salt.id_salt(),
        }
    }

    // Focusable elements and whether each is disabled
    fn stops(&self, d: Defaults) -> Vec<bool> {
        match self {
            Self::Tag(t) => {
                let off = t.disabled || d.disabled;
                if t.dismissible.unwrap_or(d.dismissible) {
                    vec![off]
                } else {
                    Vec::new()
                }
            }
            Self::Interaction(t) => {
                let off = t.disabled || d.disabled;
                if t.dismissible.unwrap_or(d.dismissible) {
                    vec![off, off]
                } else {
                    vec![off]
                }
            }
        }
    }
}

/// Result of showing a [`TagGroup`].
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct TagGroupResponse<K> {
    /// Spans every tag; `mark_changed` when the selection changed.
    pub response: egui::Response,
    /// Key of the tag whose dismiss button was pressed this frame: remove it from the data.
    pub removed: Option<K>,
    /// Agrees with `response.changed()`: a primary click toggled the bound selection.
    pub changed: bool,
}

/// Fluent `TagGroup`: a row of tags with one Tab stop and arrow-key roving focus. Dismissing moves focus to
/// the next tag, else the previous. Each tag is added with the app key `K` that names it in the bound
/// [`Selection`] and in [`TagGroupResponse::removed`].
#[derive(Debug)]
#[must_use = "call .show(..)"]
pub struct TagGroup<'a, K: Ord> {
    salt: KeyedSalt,
    key: AgentKey,
    label: String,
    defaults: Defaults,
    selection: Option<&'a mut Selection<K>>,
    keys: Vec<K>,
    entries: Vec<Entry<'a>>,
}

egui_mineral::kit::opaque_debug!(Entry<'_>);

impl<'a, K: ItemKey> TagGroup<'a, K> {
    /// `label` names the group for assistive technology; agent key `<id_salt>-tag-group`.
    pub fn new(id_salt: impl Into<KeyedSalt>, label: impl Into<String>) -> Self {
        Self {
            salt: id_salt.into(),
            key: egui_mineral::agent_key!("tag-group"),
            label: label.into(),
            defaults: Defaults::default(),
            selection: None,
            keys: Vec::new(),
            entries: Vec::new(),
        }
    }

    /// Appearance of tags without their own; default [`TagAppearance::Filled`].
    pub fn appearance(mut self, appearance: TagAppearance) -> Self {
        self.defaults.appearance = appearance;
        self
    }

    /// Size of tags without their own; default [`TagSize::Medium`].
    pub fn size(mut self, size: TagSize) -> Self {
        self.defaults.size = size;
        self
    }

    /// `dismissible` of tags without their own; default `false`.
    pub fn dismissible(mut self, dismissible: bool) -> Self {
        self.defaults.dismissible = dismissible;
        self
    }

    /// Greys out every tag.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.defaults.disabled = disabled;
        self
    }

    /// `selectedValues` and `onTagSelect`: primary clicks of [`InteractionTag`]s toggle their key.
    pub fn selection(mut self, selection: &'a mut Selection<K>) -> Self {
        self.defaults.selectable = true;
        self.selection = Some(selection);
        self
    }

    /// Appends a [`Tag`] named `key`.
    pub fn tag(mut self, key: K, tag: Tag<'a>) -> Self {
        self.keys.push(key);
        self.entries.push(Entry::Tag(tag));
        self
    }

    /// Appends an [`InteractionTag`] named `key`.
    pub fn interaction_tag(mut self, key: K, tag: InteractionTag<'a>) -> Self {
        self.keys.push(key);
        self.entries.push(Entry::Interaction(tag));
        self
    }

    crate::widget::setters! {
        /// Replaces the `<id_salt>-tag-group` agent key.
        agent_key(key)
    }

    /// Shows the group.
    pub fn show(self, ui: &mut MineralUi<'_, Fluent>) -> TagGroupResponse<K> {
        let rt = ui.rt();
        let Self {
            salt,
            key,
            label,
            defaults,
            mut selection,
            keys,
            entries,
        } = self;
        let id = ui.make_persistent_id(salt.id_salt());
        let disabled: Vec<bool> = entries.iter().flat_map(|e| e.stops(defaults)).collect();
        let enabled = |i: usize| disabled.get(i).is_some_and(|d| !d);
        let refocus_id = id.with("mineral::refocus");
        let pending = ui
            .ctx()
            .data_mut(|d| d.remove_temp::<Option<Refocus>>(refocus_id))
            .flatten();
        let target = pending.and_then(|(at, last)| stop_of(&entries, defaults, at, last));
        let stop = focus::tab_stop(ui.ctx(), id, target, disabled.len(), enabled);
        let gap = defaults.size.metrics().group_gap;
        let mut ids: Vec<egui::Id> = Vec::with_capacity(disabled.len());
        let mut dismissed: Option<(&K, IdSalt)> = None;
        let mut toggled = false;
        let group = ui
            .push_id(salt.id_salt(), |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = gap;
                    for (key, entry) in keys.iter().zip(&entries) {
                        let first = ids.len();
                        let stops = |n: usize| TabStop::at(first + n, stop);
                        match entry {
                            Entry::Tag(t) => {
                                let dismissible = t.dismissible.unwrap_or(defaults.dismissible);
                                let r = rt.ui(ui).mineral(t.part(defaults, stops(0)));
                                if dismissible {
                                    ids.push(r.id);
                                }
                                if *r.output() && dismissible {
                                    dismissed = Some((key, entry.id()));
                                }
                            }
                            Entry::Interaction(t) => {
                                let selected = selection.as_ref().is_some_and(|s| s.is_selected(key));
                                let r = t.show_in(rt, ui, defaults, selected, [stops(0), stops(1)], &mut ids);
                                if r.clicked
                                    && let Some(s) = selection.as_mut()
                                {
                                    toggled |= s.toggle(key.clone());
                                }
                                if r.dismiss_clicked {
                                    dismissed = Some((key, entry.id()));
                                }
                            }
                        }
                    }
                })
                .response
            })
            .inner;
        if let Some((_, gone)) = dismissed {
            let in_group = ids.iter().any(|i| ui.ctx().memory(|m| m.has_focus(*i)));
            if let (true, Some(next)) = (in_group, neighbor(&entries, defaults, gone)) {
                ui.ctx().data_mut(|d| d.insert_temp(refocus_id, Some(next)));
            }
        }
        if let Some(at) = target
            && let Some(i) = ids.get(at)
        {
            ui.ctx().memory_mut(|m| m.request_focus(*i));
        }
        RovingFocus::new(Axis::Horizontal).update_group(ui.core(), id, &ids, enabled);
        semantic::emit(ui.ctx(), Some(group.id), group.rect, || {
            SemanticNode::new(key.id(&salt), Role::Toolbar, &label)
        });
        let mut response = group;
        if toggled {
            response.mark_changed();
        }
        TagGroupResponse {
            response,
            removed: dismissed.map(|(key, _)| key.clone()),
            changed: toggled,
        }
    }
}
