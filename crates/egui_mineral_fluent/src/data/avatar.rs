use crate::msg;
use egui::{Color32, CornerRadius, Pos2, Rect, Shape, Stroke, StrokeKind, Vec2, epaint::RectShape, pos2, vec2};
use egui_mineral::{
    i18n::I18n,
    kit,
    layout::{Constraints, Measure},
    semantic::{AgentKey, KeyedSalt, Role, SemanticNode},
    widget::{MineralUi, MineralWidget, WidgetFrame, WidgetResponse},
};

use crate::{
    ColorToken, Fluent, FluentExt, Icon,
    feedback::{BadgeSize, PresenceBadge, PresenceStatus},
    spec::defaults::avatar as d,
    theme::{Elevation, Radius, Tokens},
    typography,
    widget::paint,
};

/// Avatar `size`: the side in pixels.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum AvatarSize {
    /// `16`.
    S16,
    /// `20`.
    S20,
    /// `24`.
    S24,
    /// `28`.
    S28,
    /// `32`.
    #[default]
    S32,
    /// `36`.
    S36,
    /// `40`.
    S40,
    /// `48`.
    S48,
    /// `56`.
    S56,
    /// `64`.
    S64,
    /// `72`.
    S72,
    /// `96`.
    S96,
    /// `120`.
    S120,
    /// `128`.
    S128,
}

impl AvatarSize {
    /// Every variant, smallest first.
    pub const ALL: &'static [Self] = &[
        Self::S16,
        Self::S20,
        Self::S24,
        Self::S28,
        Self::S32,
        Self::S36,
        Self::S40,
        Self::S48,
        Self::S56,
        Self::S64,
        Self::S72,
        Self::S96,
        Self::S120,
        Self::S128,
    ];

    /// Side in pixels.
    pub const fn px(self) -> u16 {
        match self {
            Self::S16 => 16,
            Self::S20 => 20,
            Self::S24 => 24,
            Self::S28 => 28,
            Self::S32 => 32,
            Self::S36 => 36,
            Self::S40 => 40,
            Self::S48 => 48,
            Self::S56 => 56,
            Self::S64 => 64,
            Self::S72 => 72,
            Self::S96 => 96,
            Self::S120 => 120,
            Self::S128 => 128,
        }
    }
}

/// Avatar `shape`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum AvatarShape {
    /// `circular`.
    #[default]
    Circular,
    /// `square`: rounded by the size.
    Square,
}

impl AvatarShape {
    /// Every variant.
    pub const ALL: &'static [Self] = &[Self::Circular, Self::Square];
}

/// Avatar `active`: the highlighted state of an avatar among others.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum AvatarActive {
    /// `unset`: plain.
    #[default]
    Unset,
    /// `active`: the `activeAppearance` decoration.
    Active,
    /// `inactive`: scaled down and faded.
    Inactive,
}

impl AvatarActive {
    /// Every variant.
    pub const ALL: &'static [Self] = &[Self::Unset, Self::Active, Self::Inactive];
}

/// Avatar `activeAppearance`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum AvatarActiveAppearance {
    /// `ring`.
    #[default]
    Ring,
    /// `shadow`.
    Shadow,
    /// `ring-shadow`.
    RingShadow,
}

impl AvatarActiveAppearance {
    /// Every variant.
    pub const ALL: &'static [Self] = &[Self::Ring, Self::Shadow, Self::RingShadow];

    const fn ring(self) -> bool {
        matches!(self, Self::Ring | Self::RingShadow)
    }

    const fn shadow(self) -> bool {
        matches!(self, Self::Shadow | Self::RingShadow)
    }
}

/// Named color of [`AvatarColor::Named`] and of the `colorful` hash.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum AvatarNamedColor {
    /// `dark-red`.
    DarkRed,
    /// `cranberry`.
    Cranberry,
    /// `red`.
    Red,
    /// `pumpkin`.
    Pumpkin,
    /// `peach`.
    Peach,
    /// `marigold`.
    Marigold,
    /// `gold`.
    Gold,
    /// `brass`.
    Brass,
    /// `brown`.
    Brown,
    /// `forest`.
    Forest,
    /// `seafoam`.
    Seafoam,
    /// `dark-green`.
    DarkGreen,
    /// `light-teal`.
    LightTeal,
    /// `teal`.
    Teal,
    /// `steel`.
    Steel,
    /// `blue`.
    Blue,
    /// `royal-blue`.
    RoyalBlue,
    /// `cornflower`.
    Cornflower,
    /// `navy`.
    Navy,
    /// `lavender`.
    Lavender,
    /// `purple`.
    Purple,
    /// `grape`.
    Grape,
    /// `lilac`.
    Lilac,
    /// `pink`.
    Pink,
    /// `magenta`.
    Magenta,
    /// `plum`.
    Plum,
    /// `beige`.
    Beige,
    /// `mink`.
    Mink,
    /// `platinum`.
    Platinum,
    /// `anchor`.
    Anchor,
}

impl AvatarNamedColor {
    /// Every name, in the order the `colorful` hash indexes.
    pub const ALL: &'static [Self] = &[
        Self::DarkRed,
        Self::Cranberry,
        Self::Red,
        Self::Pumpkin,
        Self::Peach,
        Self::Marigold,
        Self::Gold,
        Self::Brass,
        Self::Brown,
        Self::Forest,
        Self::Seafoam,
        Self::DarkGreen,
        Self::LightTeal,
        Self::Teal,
        Self::Steel,
        Self::Blue,
        Self::RoyalBlue,
        Self::Cornflower,
        Self::Navy,
        Self::Lavender,
        Self::Purple,
        Self::Grape,
        Self::Lilac,
        Self::Pink,
        Self::Magenta,
        Self::Plum,
        Self::Beige,
        Self::Mink,
        Self::Platinum,
        Self::Anchor,
    ];

    fn palette(self) -> Palette {
        PALETTES.get(self as usize).copied().unwrap_or(NEUTRAL)
    }
}

type Palette = (ColorToken, ColorToken, ColorToken);

const NEUTRAL: Palette = (
    ColorToken::ColorNeutralForeground3,
    ColorToken::ColorNeutralBackground6,
    ColorToken::ColorBrandStroke1,
);

// useColorStyles and useRingColorStyles (foreground, background, active ring) in `AvatarNamedColor::ALL` order
const PALETTES: [Palette; 30] = [
    (
        ColorToken::ColorPaletteDarkRedForeground2,
        ColorToken::ColorPaletteDarkRedBackground2,
        ColorToken::ColorPaletteDarkRedBorderActive,
    ),
    (
        ColorToken::ColorPaletteCranberryForeground2,
        ColorToken::ColorPaletteCranberryBackground2,
        ColorToken::ColorPaletteCranberryBorderActive,
    ),
    (
        ColorToken::ColorPaletteRedForeground2,
        ColorToken::ColorPaletteRedBackground2,
        ColorToken::ColorPaletteRedBorderActive,
    ),
    (
        ColorToken::ColorPalettePumpkinForeground2,
        ColorToken::ColorPalettePumpkinBackground2,
        ColorToken::ColorPalettePumpkinBorderActive,
    ),
    (
        ColorToken::ColorPalettePeachForeground2,
        ColorToken::ColorPalettePeachBackground2,
        ColorToken::ColorPalettePeachBorderActive,
    ),
    (
        ColorToken::ColorPaletteMarigoldForeground2,
        ColorToken::ColorPaletteMarigoldBackground2,
        ColorToken::ColorPaletteMarigoldBorderActive,
    ),
    (
        ColorToken::ColorPaletteGoldForeground2,
        ColorToken::ColorPaletteGoldBackground2,
        ColorToken::ColorPaletteGoldBorderActive,
    ),
    (
        ColorToken::ColorPaletteBrassForeground2,
        ColorToken::ColorPaletteBrassBackground2,
        ColorToken::ColorPaletteBrassBorderActive,
    ),
    (
        ColorToken::ColorPaletteBrownForeground2,
        ColorToken::ColorPaletteBrownBackground2,
        ColorToken::ColorPaletteBrownBorderActive,
    ),
    (
        ColorToken::ColorPaletteForestForeground2,
        ColorToken::ColorPaletteForestBackground2,
        ColorToken::ColorPaletteForestBorderActive,
    ),
    (
        ColorToken::ColorPaletteSeafoamForeground2,
        ColorToken::ColorPaletteSeafoamBackground2,
        ColorToken::ColorPaletteSeafoamBorderActive,
    ),
    (
        ColorToken::ColorPaletteDarkGreenForeground2,
        ColorToken::ColorPaletteDarkGreenBackground2,
        ColorToken::ColorPaletteDarkGreenBorderActive,
    ),
    (
        ColorToken::ColorPaletteLightTealForeground2,
        ColorToken::ColorPaletteLightTealBackground2,
        ColorToken::ColorPaletteLightTealBorderActive,
    ),
    (
        ColorToken::ColorPaletteTealForeground2,
        ColorToken::ColorPaletteTealBackground2,
        ColorToken::ColorPaletteTealBorderActive,
    ),
    (
        ColorToken::ColorPaletteSteelForeground2,
        ColorToken::ColorPaletteSteelBackground2,
        ColorToken::ColorPaletteSteelBorderActive,
    ),
    (
        ColorToken::ColorPaletteBlueForeground2,
        ColorToken::ColorPaletteBlueBackground2,
        ColorToken::ColorPaletteBlueBorderActive,
    ),
    (
        ColorToken::ColorPaletteRoyalBlueForeground2,
        ColorToken::ColorPaletteRoyalBlueBackground2,
        ColorToken::ColorPaletteRoyalBlueBorderActive,
    ),
    (
        ColorToken::ColorPaletteCornflowerForeground2,
        ColorToken::ColorPaletteCornflowerBackground2,
        ColorToken::ColorPaletteCornflowerBorderActive,
    ),
    (
        ColorToken::ColorPaletteNavyForeground2,
        ColorToken::ColorPaletteNavyBackground2,
        ColorToken::ColorPaletteNavyBorderActive,
    ),
    (
        ColorToken::ColorPaletteLavenderForeground2,
        ColorToken::ColorPaletteLavenderBackground2,
        ColorToken::ColorPaletteLavenderBorderActive,
    ),
    (
        ColorToken::ColorPalettePurpleForeground2,
        ColorToken::ColorPalettePurpleBackground2,
        ColorToken::ColorPalettePurpleBorderActive,
    ),
    (
        ColorToken::ColorPaletteGrapeForeground2,
        ColorToken::ColorPaletteGrapeBackground2,
        ColorToken::ColorPaletteGrapeBorderActive,
    ),
    (
        ColorToken::ColorPaletteLilacForeground2,
        ColorToken::ColorPaletteLilacBackground2,
        ColorToken::ColorPaletteLilacBorderActive,
    ),
    (
        ColorToken::ColorPalettePinkForeground2,
        ColorToken::ColorPalettePinkBackground2,
        ColorToken::ColorPalettePinkBorderActive,
    ),
    (
        ColorToken::ColorPaletteMagentaForeground2,
        ColorToken::ColorPaletteMagentaBackground2,
        ColorToken::ColorPaletteMagentaBorderActive,
    ),
    (
        ColorToken::ColorPalettePlumForeground2,
        ColorToken::ColorPalettePlumBackground2,
        ColorToken::ColorPalettePlumBorderActive,
    ),
    (
        ColorToken::ColorPaletteBeigeForeground2,
        ColorToken::ColorPaletteBeigeBackground2,
        ColorToken::ColorPaletteBeigeBorderActive,
    ),
    (
        ColorToken::ColorPaletteMinkForeground2,
        ColorToken::ColorPaletteMinkBackground2,
        ColorToken::ColorPaletteMinkBorderActive,
    ),
    (
        ColorToken::ColorPalettePlatinumForeground2,
        ColorToken::ColorPalettePlatinumBackground2,
        ColorToken::ColorPalettePlatinumBorderActive,
    ),
    (
        ColorToken::ColorPaletteAnchorForeground2,
        ColorToken::ColorPaletteAnchorBackground2,
        ColorToken::ColorPaletteAnchorBorderActive,
    ),
];

/// Avatar `color`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum AvatarColor {
    /// `neutral`.
    #[default]
    Neutral,
    /// `brand`.
    Brand,
    /// `colorful`: one of the [`AvatarNamedColor`]s by a hash of the `idForColor` or the name.
    Colorful,
    /// One of the palette, no hashing.
    Named(AvatarNamedColor),
}

impl AvatarColor {
    fn palette(self, id: &str) -> Palette {
        match self {
            Self::Neutral => NEUTRAL,
            Self::Brand => (
                ColorToken::ColorNeutralForegroundStaticInverted,
                ColorToken::ColorBrandBackgroundStatic,
                ColorToken::ColorBrandStroke1,
            ),
            Self::Colorful => hashed(id).palette(),
            Self::Named(n) => n.palette(),
        }
    }
}

// useAvatar.tsx getHashCode over UTF-16 code units
fn hashed(id: &str) -> AvatarNamedColor {
    let units: Vec<u32> = id.encode_utf16().map(u32::from).collect();
    let hash = units.iter().enumerate().rev().fold(0_u32, |hash, (i, &ch)| {
        let shift = u32::try_from(i % 8).unwrap_or(0);
        hash ^ ((ch << shift).wrapping_add(ch >> (8 - shift)))
    });
    let at = usize::try_from(hash).unwrap_or(0) % AvatarNamedColor::ALL.len();
    AvatarNamedColor::ALL
        .get(at)
        .copied()
        .unwrap_or(AvatarNamedColor::DarkRed)
}

// utils/getInitials.ts UNWANTED_ENCLOSURES_REGEX: complete enclosures go, an unterminated tail stays
fn without_enclosures(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    let mut rest = name;
    while let Some(open) = rest.find(['(', '[', '{']) {
        let (before, tail) = rest.split_at(open);
        out.push_str(before);
        if let Some(close) = tail.find([')', ']', '}']) {
            rest = tail.get(close + 1..).unwrap_or_default();
        } else {
            out.push_str(tail);
            rest = "";
        }
    }
    out.push_str(rest);
    out
}

// utils/getInitials.ts UNWANTED_CHARS_REGEX
const fn unwanted(c: char) -> bool {
    matches!(c, '\0'..='\u{1F}' | '!'..='/' | ':'..='@' | '['..='`' | '{'..='\u{BF}' | '\u{250}'..='\u{36F}' | '\u{E000}'..='\u{FFFF}')
}

// utils/getInitials.ts UNSUPPORTED_TEXT_REGEX: Arabic, Hangul, Kana and BMP CJK
const fn unsupported(c: char) -> bool {
    matches!(
        c,
        '\u{600}'..='\u{6FF}'
            | '\u{750}'..='\u{77F}'
            | '\u{8A0}'..='\u{8FF}'
            | '\u{1100}'..='\u{11FF}'
            | '\u{3130}'..='\u{318F}'
            | '\u{A960}'..='\u{A97F}'
            | '\u{AC00}'..='\u{D7FF}'
            | '\u{3040}'..='\u{30FF}'
            | '\u{3400}'..='\u{4DBF}'
            | '\u{4E00}'..='\u{9FFF}'
            | '\u{F900}'..='\u{FAFF}'
    )
}

// utils/getInitials.ts PHONENUMBER_REGEX: digits and spaces, optionally an `ext` or `x` and an extension
fn phone_number(s: &str) -> bool {
    let digits_spaces = |t: &str| t.chars().all(|c| c.is_ascii_digit() || c.is_whitespace());
    let ends_digit = |t: &str| t.chars().next_back().is_some_and(|c| c.is_ascii_digit());
    if s.chars().next().is_none_or(|c| !c.is_ascii_digit()) {
        return false;
    }
    if s.chars().count() >= 2 && digits_spaces(s) && ends_digit(s) {
        return true;
    }
    let lower = s.to_lowercase();
    let Some(at) = lower.find(['x', 'e']) else { return false };
    let (head, tail) = lower.split_at(at);
    let tail = tail
        .strip_prefix("ext")
        .or_else(|| tail.strip_prefix('x'))
        .unwrap_or("e");
    let tail = tail.trim_start();
    digits_spaces(head) && !tail.is_empty() && tail.chars().all(|c| c.is_ascii_digit())
}

fn first_upper(s: &str) -> String {
    s.chars().next().map_or_default(|c| c.to_uppercase().collect())
}

// utils/getInitials.ts getInitials: up to two initials, reversed in rtl, empty for phone numbers and unsupported scripts
pub(crate) fn initials(name: &str, rtl: bool, first_only: bool) -> String {
    let cleaned: String = without_enclosures(name).chars().filter(|c| !unwanted(*c)).collect();
    let cleaned = cleaned.split_whitespace().collect::<Vec<_>>().join(" ");
    if cleaned.chars().next().is_none_or(unsupported) || phone_number(&cleaned) {
        return String::new();
    }
    let parts: Vec<&str> = cleaned.split(' ').collect();
    let mut out = first_upper(parts.first().copied().unwrap_or_default());
    if !first_only {
        let second = match parts.len() {
            2 => parts.get(1),
            3 => parts.get(2),
            _ => None,
        };
        out.push_str(&first_upper(second.copied().unwrap_or_default()));
    }
    if rtl && out.chars().count() > 1 {
        out = out.chars().rev().collect();
    }
    out
}

// Pie slice of a group: visible clip, the polygon it fills and the scale.
#[derive(Clone, Debug)]
pub(super) struct Place {
    pub(super) clip: Rect,
    pub(super) backdrop: Vec<Pos2>,
    pub(super) scale: f32,
}

/// Fluent `Avatar`: a picture, initials or icon of a person or entity, with an optional presence badge.
#[derive(Clone, Debug)]
#[must_use = "show with ui.mineral(..)"]
pub struct Avatar<'a> {
    pub(super) salt: KeyedSalt,
    key: AgentKey,
    name: String,
    pub(super) size: AvatarSize,
    pub(super) shape: AvatarShape,
    color: AvatarColor,
    id_for_color: Option<String>,
    initials: Option<String>,
    image: Option<egui::ImageSource<'a>>,
    icon: Option<Icon>,
    pub(super) presence: Option<(PresenceStatus, bool)>,
    active: AvatarActive,
    appearance: AvatarActiveAppearance,
    pub(super) place: Option<Place>,
}

impl<'a> Avatar<'a> {
    /// `name` gives the initials and the accessible name; 32, circular, neutral, no badge.
    /// Agent key `<id_salt>-avatar`.
    pub fn new(id_salt: impl Into<KeyedSalt>, name: impl Into<String>) -> Self {
        Self {
            salt: id_salt.into(),
            key: AgentKey::Kind("avatar"),
            name: name.into(),
            size: AvatarSize::S32,
            shape: AvatarShape::Circular,
            color: AvatarColor::Neutral,
            id_for_color: None,
            initials: None,
            image: None,
            icon: None,
            presence: None,
            active: AvatarActive::Unset,
            appearance: AvatarActiveAppearance::Ring,
            place: None,
        }
    }

    /// Default [`AvatarSize::S32`].
    pub fn size(mut self, size: AvatarSize) -> Self {
        self.size = size;
        self
    }

    /// Default [`AvatarShape::Circular`].
    pub fn shape(mut self, shape: AvatarShape) -> Self {
        self.shape = shape;
        self
    }

    /// Default [`AvatarColor::Neutral`].
    pub fn color(mut self, color: AvatarColor) -> Self {
        self.color = color;
        self
    }

    /// `idForColor`: the text [`AvatarColor::Colorful`] hashes instead of the name.
    pub fn id_for_color(mut self, id: impl Into<String>) -> Self {
        self.id_for_color = Some(id.into());
        self
    }

    /// `initials` slot: replaces the initials of the name.
    pub fn initials(mut self, initials: impl Into<String>) -> Self {
        self.initials = Some(initials.into());
        self
    }

    /// `image` slot: covers the initials and icon once loaded.
    pub fn image(mut self, source: impl Into<egui::ImageSource<'a>>) -> Self {
        self.image = Some(source.into());
        self
    }

    /// `icon` slot, shown when there are no initials; default `PersonRegular`.
    pub fn icon(mut self, icon: impl Into<Icon>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    /// `badge` slot: a [`PresenceBadge`] of `status` at the bottom end corner.
    pub fn presence(mut self, status: PresenceStatus) -> Self {
        self.presence = Some((status, false));
        self
    }

    /// `badge` slot as an out-of-office [`PresenceBadge`].
    pub fn presence_out_of_office(mut self, status: PresenceStatus) -> Self {
        self.presence = Some((status, true));
        self
    }

    /// Default [`AvatarActive::Unset`].
    pub fn active(mut self, active: AvatarActive) -> Self {
        self.active = active;
        self
    }

    /// Decoration of the active state; default [`AvatarActiveAppearance::Ring`].
    pub fn active_appearance(mut self, appearance: AvatarActiveAppearance) -> Self {
        self.appearance = appearance;
        self
    }

    crate::widget::setters! {
        /// Replaces the `<id_salt>-avatar` agent key.
        agent_key(key)
    }

    pub(super) fn name(&self) -> &str {
        &self.name
    }

    pub(super) fn key(&self) -> egui_mineral::semantic::StableId {
        self.key.id(&self.salt)
    }

    fn scale(&self) -> f32 {
        self.place.as_ref().map_or(1.0, |p| p.scale)
    }

    fn side(&self) -> f32 {
        f32::from(self.size.px()) * self.scale()
    }

    fn label(&self, i18n: &I18n) -> String {
        let mut parts = Vec::new();
        if self.name.is_empty() {
            parts.push(self.initials_text(i18n.direction().sign() < 0.0));
        } else {
            parts.push(self.name.clone());
        }
        if let Some((status, ooo)) = self.presence {
            parts.push(PresenceBadge::new("name", status).out_of_office(ooo).name(i18n));
        }
        match self.active {
            AvatarActive::Unset => {}
            AvatarActive::Active => parts.push(crate::tr(i18n, msg::AVATAR_ACTIVE)),
            AvatarActive::Inactive => parts.push(crate::tr(i18n, msg::AVATAR_INACTIVE)),
        }
        parts.retain(|p| !p.is_empty());
        parts.join(", ")
    }

    fn initials_text(&self, rtl: bool) -> String {
        self.initials
            .clone()
            .unwrap_or_else(|| initials(&self.name, rtl, self.size.px() <= d::FIRST_INITIAL_ONLY_MAX))
    }

    fn radius(&self, rect: Rect) -> CornerRadius {
        match self.shape {
            AvatarShape::Circular => Radius::Circular.on(rect),
            AvatarShape::Square => CornerRadius::from(d::square_radius(self.size.px()) * self.scale()),
        }
    }

    fn elevation(&self) -> Elevation {
        match d::shadow_level(self.size.px()) {
            4 => Elevation::E4,
            8 => Elevation::E8,
            16 => Elevation::E16,
            _ => Elevation::E28,
        }
    }

    fn badge_size(&self) -> BadgeSize {
        match self.size.px() {
            96.. => BadgeSize::ExtraLarge,
            64..=95 => BadgeSize::Large,
            56..=63 => BadgeSize::Medium,
            40..=55 => BadgeSize::Small,
            28..=39 => BadgeSize::ExtraSmall,
            _ => BadgeSize::Tiny,
        }
    }

    pub(crate) fn paint(&self, ui: &MineralUi<'_, Fluent>, nominal: Rect, t: &Tokens) {
        let id = self.id_for_color.as_deref().unwrap_or(&self.name);
        let (fg, bg, ring) = self.color.palette(id);
        let inactive = self.active == AvatarActive::Inactive;
        let k = if inactive { d::INACTIVE_SCALE } else { 1.0 };
        let fade = if inactive { d::INACTIVE_OPACITY } else { 1.0 };
        let rect = Rect::from_center_size(nominal.center(), nominal.size() * k);
        let unit = self.scale() * k;
        let size = self.size.px();
        let mut p = ui.painter().clone();
        if let Some(place) = &self.place {
            p.set_clip_rect(p.clip_rect().intersect(place.clip));
        }
        let radius = self.radius(rect);
        if self.active == AvatarActive::Active {
            if self.appearance.shadow() {
                paint::shadow(&p, rect, radius, self.elevation(), t);
            }
            if self.appearance.ring() {
                let w = d::ring_width(size) * unit;
                p.rect_stroke(
                    rect.expand(w),
                    radius,
                    Stroke::new(w, t.color(ring)),
                    StrokeKind::Outside,
                );
            }
        }
        let fill = t.color(bg).gamma_multiply(fade);
        match &self.place {
            Some(place) => {
                p.add(Shape::convex_polygon(place.backdrop.clone(), fill, Stroke::NONE));
            }
            None => {
                p.rect_filled(rect, radius, fill);
            }
        }
        let text_color = t.color(fg).gamma_multiply(fade);
        let initials = self.initials_text(ui.i18n().direction().sign() < 0.0);
        if !initials.is_empty() {
            let g = p.layout_no_wrap(
                initials,
                typography::font(ui.ctx(), d::text_size(size) * unit, geometry_semibold()),
                text_color,
            );
            p.galley(rect.center() - g.size() / 2.0, g, text_color);
        } else if self.image.is_none() {
            let icon = self.icon.unwrap_or(ui.icons().person);
            paint::icon(&p, rect.center(), icon, false, d::icon_size(size) * unit, text_color);
        }
        if let Some(source) = &self.image
            && let Some((tex, natural)) = super::image::texture(ui.ctx(), source, rect.size())
        {
            let uv = cover(natural);
            let r = if self.place.is_some() {
                CornerRadius::ZERO
            } else {
                radius
            };
            p.add(Shape::Rect(
                RectShape::filled(rect, r, Color32::WHITE.gamma_multiply(fade)).with_texture(tex, uv),
            ));
        }
        if let Some((status, ooo)) = self.presence {
            let badge = PresenceBadge::new("badge", status)
                .out_of_office(ooo)
                .size(self.badge_size());
            let extent = badge.extent() * unit;
            let dir = ui.i18n().direction();
            let gap = d::badge_gap(badge.extent() / 2.0) * unit;
            let x = dir.start_x(rect, egui::Rangef::new(rect.width() - extent, rect.width()));
            let b = Rect::from_min_size(pos2(x, rect.bottom() - extent), vec2(extent, extent));
            p.circle_filled(
                b.center(),
                extent / 2.0 + gap,
                t.color(ColorToken::ColorNeutralBackground1),
            );
            badge.paint(&p, b, t, dir.sign());
        }
    }
}

const fn geometry_semibold() -> u16 {
    crate::spec::tokens::typography::FONT_WEIGHT_SEMIBOLD
}

// object-fit: cover on a square box
fn cover(natural: Vec2) -> Rect {
    if natural.x <= 0.0 || natural.y <= 0.0 {
        return Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0));
    }
    let (w, h) = if natural.x > natural.y {
        (natural.y / natural.x, 1.0)
    } else {
        (1.0, natural.x / natural.y)
    };
    Rect::from_center_size(pos2(0.5, 0.5), vec2(w, h))
}

impl MineralWidget for Avatar<'_> {
    type Kit = Fluent;
    type State = ();
    type Output = ();

    kit::widget_meta!(salt, hover, no_target);

    fn layout(&self, _: &MineralUi<'_, Fluent>, c: Constraints, (): &()) -> Measure {
        c.constrain(vec2(self.side(), self.side())).into()
    }

    fn update(&mut self, ui: &mut MineralUi<'_, Fluent>, frame: &WidgetFrame<'_>, (): &mut ()) -> WidgetResponse {
        let t = ui.tokens();
        self.paint(ui, frame.zones.visual, &t);
        WidgetResponse::new(frame.response.clone(), ())
    }

    fn semantics(&self, i18n: &I18n, _: &WidgetResponse, (): &()) -> Option<SemanticNode> {
        Some(SemanticNode::new(self.key(), Role::Image, self.label(i18n)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initials_follow_get_initials() {
        for (name, rtl, expected) in [
            ("Kat Larrson", false, "KL"),
            ("Kat Larrson", true, "LK"),
            ("Irissa Thordar-Dottir", false, "IT"),
            ("David (The man) Goff", false, "DG"),
            ("David {The man} Goff", false, "DG"),
            (
                " !@#$%^&*()=+ (Alpha) David   (The man) `~<>,./?[]{}|   Goff   (Gamma)    ",
                false,
                "DG",
            ),
            ("A B C D", false, "A"),
            ("A B C", false, "AC"),
            ("4lex 5loo", false, "45"),
            ("[Team {Inner) Grace] Hopper", false, "GH"),
            ("[Ada [Grace] Hopper", false, "H"),
            ("Ada (Grace [Hopper", false, "AH"),
            ("Ada [Team\n[Inner] Hopper]", false, "AH"),
            ("[Team] Ada [Grace Hopper", false, "AH"),
            ("\u{6842}\u{82f1}", false, ""),
            ("\u{ac15}\u{d604}", false, ""),
            (
                "\u{62e}\u{633}\u{631}\u{648} \u{631}\u{62d}\u{64a}\u{645}\u{64a}",
                true,
                "",
            ),
            ("555 1234", false, ""),
            ("555 1234 x 12", false, ""),
            ("", false, ""),
        ] {
            assert_eq!(initials(name, rtl, false), expected, "{name:?} rtl={rtl}");
        }
        assert_eq!(initials("Kat Larrson", false, true), "K");
    }

    #[test]
    fn colorful_hash_matches_get_hash_code() {
        // getHashCode("a"): ch 97, shift 0: 97 + (97 >> 8) = 97; 97 % 30 = 7
        assert_eq!(hashed("a"), AvatarNamedColor::ALL[7]);
        assert_eq!(hashed(""), AvatarNamedColor::ALL[0]);
    }
}
