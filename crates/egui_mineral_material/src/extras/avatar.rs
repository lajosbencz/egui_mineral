use crate::Material;
use crate::MaterialExt;
use egui::{
    Color32, ImageSource, Painter, Pos2, Rangef, Rect, Stroke, Ui, Vec2, load::TexturePoll, text::LayoutJob, vec2,
};
use egui_mineral::semantic::AgentKey;
use egui_mineral::semantic::KeyedSalt;
use egui_mineral::widget::MineralUi;
use egui_mineral::{
    layout::Constraints,
    semantic::{self, Role, SemanticNode},
};

use crate::widget::{common, paint::paint_widget};
use crate::{
    Icon,
    spec::{
        ColorRole,
        tokens::{app_bar, chips, list, search_bar},
    },
};

// The list item leading avatar (`ListTokens.ItemLeadingAvatar*`), scaled with its size.
const AVATAR_SIZE: f32 = list::ITEM_LEADING_AVATAR_SIZE;
const GROUP_STEP: f32 = 0.75;
const RING_WIDTH: f32 = 2.0;
const OVERFLOW_COLOR: ColorRole = ColorRole::SurfaceContainerHighest;
const OVERFLOW_LABEL_COLOR: ColorRole = ColorRole::OnSurfaceVariant;

// Type scaled from the 40dp list avatar
fn paint_label(p: &Painter, center: Pos2, text: String, size: f32, color: Color32) {
    let k = size / AVATAR_SIZE;
    let mut format = list::ITEM_LEADING_AVATAR_LABEL_FONT.format(p.ctx(), color);
    format.font_id.size *= k;
    format.extra_letter_spacing *= k;
    format.line_height = format.line_height.map(|h| h * k);
    let g = p.layout_job(LayoutJob::single_section(text, format));
    p.galley(center - g.size() / 2.0, g, Color32::PLACEHOLDER);
}

/// Avatar diameter of the MD3 component hosting it; any other size goes through
/// [`SizeExt`](egui_mineral::widget::SizeExt).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum AvatarSize {
    /// `ChipsTokens.AvatarSize` (input chips too).
    Chip,
    /// `SearchBarTokens.AvatarSize`.
    SearchBar,
    /// `AppBarTokens.AvatarSize`.
    TopAppBar,
    /// `ListTokens.ItemLeadingAvatarSize`.
    #[default]
    ListItem,
}

impl AvatarSize {
    /// Diameter in dp, from the hosting component's avatar token.
    pub const fn diameter(self) -> f32 {
        match self {
            Self::Chip => chips::AVATAR_SIZE,
            Self::SearchBar => search_bar::AVATAR_SIZE,
            Self::TopAppBar => app_bar::AVATAR_SIZE,
            Self::ListItem => list::ITEM_LEADING_AVATAR_SIZE,
        }
    }
}

fn square(rect: Rect) -> Rect {
    Rect::from_center_size(rect.center(), Vec2::splat(rect.size().min_elem()))
}

/// Circular user image, initials or icon fallback; non-MD3 (MUI `Avatar` pattern).
#[derive(Clone, Debug)]
#[must_use = "show with ui.mineral(..)"]
pub struct Avatar {
    salt: KeyedSalt,
    key: AgentKey,
    name: String,
    image: Option<ImageSource<'static>>,
    icon: Option<Icon>,
    size: AvatarSize,
}

impl Avatar {
    /// Shows initials of `name`, also the accessible name; agent key `<id_salt>-avatar`.
    pub fn new(id_salt: impl Into<KeyedSalt>, name: impl Into<String>) -> Self {
        let id_salt: KeyedSalt = id_salt.into();
        Self {
            key: AgentKey::Kind("avatar"),
            salt: id_salt,
            name: name.into(),
            image: None,
            icon: None,
            size: AvatarSize::default(),
        }
    }
    /// Image in place of the initials, cropped to the circle.
    pub fn image(mut self, source: impl Into<ImageSource<'static>>) -> Self {
        self.image = Some(source.into());
        self
    }
    /// Fallback icon when `name` has no initials; default [`IconSet::person`].
    pub fn icon(mut self, icon: impl Into<Icon>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    /// Default [`AvatarSize::ListItem`].
    pub fn size(mut self, size: AvatarSize) -> Self {
        self.size = size;
        self
    }
    crate::widget::setters! {
        /// Replaces the `<id_salt>-avatar` agent key.
        agent_key(key)
    }

    fn initials(name: &str) -> String {
        let mut words = name
            .split_whitespace()
            .filter_map(|w| w.chars().find(|c| c.is_alphanumeric()));
        let first = words.next();
        let last = words.next_back();
        first.into_iter().chain(last).flat_map(char::to_uppercase).collect()
    }

    fn extent(&self, _: &Ui, c: Constraints) -> Vec2 {
        c.constrain(Vec2::splat(self.size.diameter()))
    }

    fn paint(&self, ui: &mut MineralUi<'_, Material>, rect: Rect) {
        let rt = ui.rt();
        if ui.is_rect_visible(rect) {
            self.paint_at(&rt.ui(ui), square(rect), None);
        }
    }

    fn node(&self, _: &egui_mineral::i18n::I18n) -> SemanticNode {
        SemanticNode::new(self.key.id(&self.salt), Role::Image, &self.name)
    }

    fn paint_at(&self, ui: &MineralUi<'_, Material>, rect: Rect, ring: Option<egui::Color32>) {
        let rt = ui.rt();
        let t = rt.tokens();
        let p = ui.painter();
        let center = rect.center();
        let radius = rect.width() / 2.0;
        let ready = self.image.as_ref().and_then(|src| {
            match egui::Image::new(src.clone()).load_for_size(ui.ctx(), rect.size()) {
                Ok(TexturePoll::Ready { .. }) => Some(src.clone()),
                _ => None,
            }
        });
        if let Some(src) = ready {
            egui::Image::new(src)
                .corner_radius(common::radius(rect, t.shape.full))
                .paint_at(ui, rect);
        } else {
            let fg = t.role(list::ITEM_LEADING_AVATAR_LABEL_COLOR);
            p.circle_filled(center, radius, t.role(list::ITEM_LEADING_AVATAR_COLOR));
            let initials = Self::initials(&self.name);
            if initials.is_empty() {
                let icon = list::ITEM_LEADING_ICON_SIZE * rect.width() / AVATAR_SIZE;
                common::icon(p, center, self.icon.unwrap_or(ui.icons().person), icon, fg);
            } else {
                paint_label(p, center, initials, rect.width(), fg);
            }
        }
        if let Some(ring) = ring {
            p.circle_stroke(center, radius + RING_WIDTH / 2.0, Stroke::new(RING_WIDTH, ring));
        }
    }
}

/// Overlapping [`Avatar`]s collapsed past `max` into a count.
#[derive(Clone, Debug)]
#[must_use = "show with ui.mineral(..)"]
pub struct AvatarGroup {
    salt: KeyedSalt,
    key: AgentKey,
    avatars: Vec<Avatar>,
    max: usize,
    size: AvatarSize,
}

impl AvatarGroup {
    /// Overlapping avatars; agent key `<id_salt>-avatar-group`.
    pub fn new(id_salt: impl Into<KeyedSalt>) -> Self {
        let id_salt: KeyedSalt = id_salt.into();
        Self {
            key: AgentKey::Kind("avatar-group"),
            salt: id_salt,
            avatars: Vec::new(),
            max: 5,
            size: AvatarSize::default(),
        }
    }
    /// Appends one avatar.
    pub fn avatar(mut self, avatar: impl Into<Avatar>) -> Self {
        self.avatars.push(avatar.into());
        self
    }
    /// Appends avatars.
    pub fn avatars(mut self, avatars: impl IntoIterator<Item = impl Into<Avatar>>) -> Self {
        self.avatars.extend(avatars.into_iter().map(Into::into));
        self
    }
    /// Avatars shown before a `+N` overflow; default 5, minimum 1.
    #[track_caller]
    pub fn max_visible(mut self, max: usize) -> Self {
        self.max = egui_mineral::kit::sanitize::at_least(max, 1.., "AvatarGroup max_visible");
        self
    }

    /// Default [`AvatarSize::ListItem`].
    pub fn size(mut self, size: AvatarSize) -> Self {
        self.size = size;
        self
    }
    crate::widget::setters! {
        /// Replaces the `<id_salt>-avatar-group` agent key.
        agent_key(key)
    }

    fn split(&self) -> (usize, usize) {
        let total = self.avatars.len();
        if total > self.max {
            (self.max - 1, total - (self.max - 1))
        } else {
            (total, 0)
        }
    }

    fn extent(&self, _: &Ui, c: Constraints) -> Vec2 {
        let d = c.constrain(Vec2::splat(self.size.diameter())).y;
        let (shown, hidden) = self.split();
        let bubbles = shown + usize::from(hidden > 0);
        let width = if bubbles == 0 {
            0.0
        } else {
            d * GROUP_STEP * (bubbles - 1) as f32 + d
        };
        c.constrain(vec2(width, d))
    }

    fn node(&self, i18n: &egui_mineral::i18n::I18n) -> SemanticNode {
        let (_, hidden) = self.split();
        let name = i18n.tr("mineral-avatar-group", &[("count", self.avatars.len().into())]);
        let mut n = SemanticNode::new(self.key.id(&self.salt), Role::Group, name);
        if hidden > 0 {
            n.value = Some(i18n.tr("mineral-avatar-more", &[("count", hidden.into())]));
        }
        n
    }

    fn paint(&self, ui: &mut MineralUi<'_, Material>, rect: Rect) {
        let rt = ui.rt();
        if !ui.is_rect_visible(rect) {
            return;
        }
        let t = ui.tokens();
        let ctx = ui.ctx().clone();
        let (shown, hidden) = self.split();
        let d = rect.height();
        let step = d * GROUP_STEP;
        let dir = rt.i18n().direction();
        let slot = |i: usize| {
            let from = step * i as f32;
            let x = dir.start_x(rect, Rangef::new(from, from + d));
            Rect::from_min_size(egui::pos2(x, rect.top()), vec2(d, d))
        };
        for (i, a) in self.avatars.iter().take(shown).enumerate() {
            let r = slot(i).shrink(RING_WIDTH / 2.0);
            a.paint_at(ui, r, Some(t.color.surface));
            semantic::emit(&ctx, None, r, || {
                let mut n = SemanticNode::new(a.key.id(&a.salt), Role::Image, &a.name);
                n.depth = 1;
                n
            });
        }
        if hidden > 0 {
            let r = slot(shown).shrink(RING_WIDTH / 2.0);
            let p = ui.painter();
            p.circle_filled(r.center(), r.width() / 2.0, t.role(OVERFLOW_COLOR));
            p.circle_stroke(
                r.center(),
                f32::midpoint(r.width(), RING_WIDTH),
                Stroke::new(RING_WIDTH, t.color.surface),
            );
            let count = rt.i18n().format().count(hidden);
            let text = rt.i18n().tr("mineral-avatar-overflow", &[("count", (&count).into())]);
            paint_label(p, r.center(), text, r.width(), t.role(OVERFLOW_LABEL_COLOR));
        }
    }
}

paint_widget!(Avatar);
paint_widget!(AvatarGroup);
