// Compose carousel/Carousel.kt, Keylines.kt, KeylineList.kt, Arrangement.kt, Strategy.kt, KeylineSnapPosition.kt
use crate::msg;
use egui_mineral::kit::state::TempState;
use std::sync::Arc;

use crate::Material;
use crate::MaterialExt;
use crate::Rt;
use egui_mineral::semantic::KeyedSalt;
use egui_mineral::table::Rows;
use egui_mineral::widget::MineralUi;

use egui::{
    Color32, CornerRadius, Id, Painter, Pos2, Rangef, Rect, Sense, Shape, UiBuilder, Vec2, epaint::Mesh,
    epaint::RectShape, load::SizedTexture, pos2, vec2,
};
use egui_mineral::widget::Availability;
use egui_mineral::{
    focus::{self, RovingFocus},
    i18n, kit,
    layout::Axis,
    motion::Spring,
    range::Track,
    semantic::{self, Role, SemanticNode, StableId},
};

use crate::spec::{ShapeScale, defaults::carousel as cx};
use crate::widget::common::Interaction;

mod arrangement;
mod keyline_list;
mod keylines;
mod strategy;

use keyline_list::{KeylineList, lerp_keyline};
use keylines::{hero_keylines, multi_browse_keylines, uncontained_keylines};
use strategy::Strategy;

const ARC_STEPS: u32 = 10;
const MIN_PADDED_SIZE: f32 = 1.0;
// Compose HorizontalMultiBrowseCarouselSample height; Compose sizes carousels by modifier only
const SAMPLE_HEIGHT: f32 = 221.0;
// Compose samples maskClip(MaterialTheme.shapes.extraLarge)
const ITEM_SHAPE: ShapeScale = ShapeScale::ExtraLarge;
// Compose CarouselDefaults.singleAdvanceFlingBehavior snap spring
const SNAP_SPRING: Spring = Spring {
    damping_ratio: cx::SNAP_DAMPING,
    stiffness: cx::SNAP_STIFFNESS,
    rest_threshold: crate::motion::REST,
};

/// Keyline strategy of a [`Carousel`].
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub enum CarouselLayout {
    /// `HorizontalMultiBrowseCarousel`: large, medium and small items.
    #[non_exhaustive]
    MultiBrowse {
        /// Large item width in dp; the keylines fit as many as the carousel holds.
        preferred_item_width: f32,
    },
    /// `HorizontalUncontainedCarousel`: same-size items, no snapping.
    #[non_exhaustive]
    Uncontained {
        /// Width of every item in dp.
        item_width: f32,
    },
    /// `heroKeylineList`: one large item and a small one; `centered` is
    /// `HorizontalCenteredHeroCarousel`. The large item fills the carousel without a width.
    #[non_exhaustive]
    Hero {
        /// Large item width in dp; `None` fills the carousel.
        preferred_item_width: Option<f32>,
        /// Centers the large item between two small ones.
        centered: bool,
    },
    /// Vertical, one item filling the carousel (`uncontainedKeylineList` at full size).
    FullScreen,
}

impl CarouselLayout {
    /// [`Self::MultiBrowse`] with large items `preferred_item_width` dp wide.
    pub const fn multi_browse(preferred_item_width: f32) -> Self {
        Self::MultiBrowse { preferred_item_width }
    }

    /// [`Self::Uncontained`] with items `item_width` dp wide.
    pub const fn uncontained(item_width: f32) -> Self {
        Self::Uncontained { item_width }
    }

    /// [`Self::Hero`] with the large item at the start, filling the carousel.
    pub const fn hero() -> Self {
        Self::Hero {
            preferred_item_width: None,
            centered: false,
        }
    }

    /// [`Self::Hero`] with the large item centered between two small ones, filling the carousel.
    pub const fn centered_hero() -> Self {
        Self::Hero {
            preferred_item_width: None,
            centered: true,
        }
    }

    /// Large item width in dp of the keyed layouts ([`Self::MultiBrowse`], [`Self::Hero`]); others are unchanged.
    #[must_use]
    pub const fn preferred_item_width(self, width: f32) -> Self {
        match self {
            Self::MultiBrowse { .. } => Self::MultiBrowse {
                preferred_item_width: width,
            },
            Self::Hero { centered, .. } => Self::Hero {
                preferred_item_width: Some(width),
                centered,
            },
            Self::Uncontained { .. } | Self::FullScreen => self,
        }
    }

    #[track_caller]
    fn sanitized(self) -> Self {
        let width = |w: f32| egui_mineral::kit::sanitize::at_least(w, cx::MIN_SMALL_ITEM_SIZE.., "carousel item width");
        match self {
            Self::MultiBrowse { preferred_item_width } => Self::MultiBrowse {
                preferred_item_width: width(preferred_item_width),
            },
            Self::Uncontained { item_width } => Self::Uncontained {
                item_width: width(item_width),
            },
            Self::Hero {
                preferred_item_width,
                centered,
            } => Self::Hero {
                preferred_item_width: preferred_item_width.map(width),
                centered,
            },
            Self::FullScreen => Self::FullScreen,
        }
    }

    fn vertical(self) -> bool {
        self == Self::FullScreen
    }

    fn snaps(self) -> bool {
        !matches!(self, Self::Uncontained { .. })
    }

    fn keylines(self, main: f32, spacing: f32, count: usize) -> KeylineList {
        match self {
            Self::MultiBrowse { preferred_item_width } => {
                multi_browse_keylines(main, preferred_item_width, spacing, count)
            }
            Self::Uncontained { item_width } => uncontained_keylines(main, item_width, spacing),
            Self::Hero {
                preferred_item_width,
                centered,
            } => hero_keylines(main, preferred_item_width, spacing, count, centered),
            Self::FullScreen => uncontained_keylines(main, main, spacing),
        }
    }
}

/// Result of [`Carousel::show`] and [`Carousel::show_images`].
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct CarouselResponse<K> {
    /// Viewport response.
    pub response: egui::Response,
    /// Key of the item activated this frame.
    pub activated: Option<K>,
    /// Fractional index of the item at the snap position.
    pub position: f32,
}

impl CarouselResponse<usize> {
    fn keyed<K>(self, items: impl Rows<K> + Copy) -> CarouselResponse<K> {
        CarouselResponse {
            response: self.response,
            activated: self.activated.and_then(|i| items.key(i)),
            position: self.position,
        }
    }
}

#[derive(Clone, Default)]
struct State {
    // 0..=max_scroll
    scroll: f32,
    start: usize,
    gesture: bool,
    claim: egui_mineral::input::PanClaim,
    wheel: f32,
    target: Option<f32>,
}

impl TempState for State {}

enum Content<'f> {
    Ui(&'f mut dyn FnMut(&mut MineralUi<'_, Material>, usize)),
    Image(&'f mut dyn FnMut(usize) -> SizedTexture),
}

#[derive(Clone, Copy, PartialEq)]
struct GeometryKey {
    layout: CarouselLayout,
    extent: f32,
    spacing: f32,
    padding: CarouselPadding,
    count: usize,
}

struct Geometry {
    strategy: Strategy,
    snaps: Vec<f32>,
    max_scroll: f32,
}

impl Geometry {
    fn load(ctx: &egui::Context, id: Id, key: GeometryKey) -> Option<Arc<Self>> {
        let memo = id.with("geometry");
        if let Some((k, g)) = ctx.data(|d| d.get_temp::<(GeometryKey, Arc<Self>)>(memo))
            && k == key
        {
            return Some(g);
        }
        let strategy = Strategy::new(
            key.layout.keylines(key.extent, key.spacing, key.count),
            key.extent,
            key.spacing,
            key.padding,
        );
        if !strategy.is_valid() {
            return None;
        }
        let geometry = Arc::new(Self {
            max_scroll: strategy.max_scroll(key.count),
            snaps: strategy.snap_points(key.count),
            strategy,
        });
        ctx.data_mut(|d| d.insert_temp(memo, (key, geometry.clone())));
        Some(geometry)
    }
}

struct Strip {
    rect: Rect,
    id: Id,
    track: Track,
    geo: Arc<Geometry>,
}

impl Strip {
    fn item_id(&self, i: usize) -> Id {
        self.id.with(("item", i))
    }
}

#[derive(Default)]
struct Items {
    ids: Vec<(usize, Id)>,
    focus: Option<usize>,
}

// Compose Modifier.carouselItem
struct Placed {
    index: usize,
    item: Rangef,
    mask: Rangef,
    z: f32,
}

/// Main-axis `contentPadding` of a [`Carousel`] in dp.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct CarouselPadding {
    /// Before the first item.
    pub start: f32,
    /// After the last item.
    pub end: f32,
}

/// MD3 carousel (Compose `HorizontalMultiBrowseCarousel`, `HorizontalUncontainedCarousel`, `HorizontalCenteredHeroCarousel`).
#[must_use = "call .show(..)"]
pub struct Carousel<'a> {
    salt: KeyedSalt,
    count: usize,
    layout: CarouselLayout,
    height: f32,
    spacing: f32,
    padding: CarouselPadding,
    label: Option<String>,
    item_label: Option<Box<dyn Fn(usize) -> String + 'a>>,
    surround: Option<Color32>,
    disabled: bool,
}

egui_mineral::kit::opaque_debug!(Carousel<'_>);

impl<'a> Carousel<'a> {
    /// Items placed by `layout`; agent key prefix `<id_salt>`. Item widths below `MIN_SMALL_ITEM_SIZE` (40dp) are
    /// raised to it.
    #[track_caller]
    pub fn new(id_salt: impl Into<KeyedSalt>, layout: CarouselLayout) -> Self {
        let id_salt: KeyedSalt = id_salt.into();
        Self {
            salt: id_salt,
            count: 0,
            layout: layout.sanitized(),
            height: SAMPLE_HEIGHT,
            spacing: cx::ITEM_SPACING,
            padding: CarouselPadding::default(),
            label: None,
            item_label: None,
            surround: None,
            disabled: false,
        }
    }

    crate::widget::setters! {
        /// Ignores clicks; scrolling still works.
        disabled
    }

    crate::widget::setters! {
        /// Replaces the `<id_salt>` key prefix.
        agent_key(salt)
    }

    /// Cross-axis size, or the main-axis size of a [`CarouselLayout::FullScreen`] carousel. Not finite or
    /// below 1 becomes 1.
    #[track_caller]
    pub fn height(mut self, height: f32) -> Self {
        self.height = egui_mineral::kit::sanitize::at_least(height, 1.0.., "height");
        self
    }

    /// Space between items (`itemSpacing`); not finite or negative becomes 0.
    #[track_caller]
    pub fn item_spacing(mut self, spacing: f32) -> Self {
        self.spacing = egui_mineral::kit::sanitize::at_least(spacing, 0.0.., "item_spacing");
        self
    }

    /// Main-axis `contentPadding`; not finite or negative sides become 0, and the keylines limit it to
    /// what keeps every item visible.
    #[track_caller]
    pub fn content_padding(mut self, padding: CarouselPadding) -> Self {
        self.padding = CarouselPadding {
            start: egui_mineral::kit::sanitize::at_least(padding.start, 0.0.., "content_padding start"),
            end: egui_mineral::kit::sanitize::at_least(padding.end, 0.0.., "content_padding end"),
        };
        self
    }

    /// Accessible name of the carousel; default "Carousel".
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Accessible item names by index; default "Item n of count".
    pub fn item_label(mut self, item_label: impl Fn(usize) -> String + 'a) -> Self {
        self.item_label = Some(Box::new(item_label));
        self
    }

    /// Color behind the carousel, painted over the corners outside each item mask.
    pub fn surround(mut self, surround: Color32) -> Self {
        self.surround = Some(surround);
        self
    }

    /// `items` counts and keys the items; calls `add` with each visible item index, inside its masked rect.
    pub fn show<K>(
        mut self,
        ui: &mut MineralUi<'_, Material>,
        items: impl Rows<K> + Copy,
        mut add: impl FnMut(&mut MineralUi<'_, Material>, usize),
    ) -> CarouselResponse<K> {
        self.count = items.len();
        self.run(ui, Content::Ui(&mut add)).keyed(items)
    }

    /// `items` counts and keys the items; paints the texture of each visible item index, cropped to its mask.
    pub fn show_images<K>(
        mut self,
        ui: &mut MineralUi<'_, Material>,
        items: impl Rows<K> + Copy,
        mut image: impl FnMut(usize) -> SizedTexture,
    ) -> CarouselResponse<K> {
        self.count = items.len();
        self.run(ui, Content::Image(&mut image)).keyed(items)
    }

    // Compose Modifier.carouselItem
    fn place(&self, s: &Strategy, scroll: f32, max_scroll: f32, current: usize) -> Vec<Placed> {
        let keylines = s.keylines_at(scroll, max_scroll);
        let (size, pitch) = (s.item_size(), s.pitch());
        let reach = |uo: f32| (uo + scroll - size / 2.0) / pitch;
        #[expect(clippy::cast_sign_loss, reason = "clamped to >= 0")]
        let first = (reach(keylines.first().unadjusted_offset).floor() - 1.0).max(0.0) as usize;
        #[expect(clippy::cast_sign_loss, reason = "clamped to >= 0")]
        let last = (reach(keylines.last().unadjusted_offset).ceil() + 1.0).max(0.0) as usize;
        (first..self.count.min(last + 1))
            .map(|index| {
                let center = index as f32 * pitch + size / 2.0 - scroll;
                let before = keylines.keyline_before(center);
                let after = keylines.keyline_after(center);
                let out_of_bounds = before == after;
                let progress = if out_of_bounds {
                    1.0
                } else {
                    (center - before.unadjusted_offset) / (after.unadjusted_offset - before.unadjusted_offset)
                };
                let k = lerp_keyline(before, after, progress);
                let mut translation = k.offset - center;
                if out_of_bounds {
                    translation += (center - k.unadjusted_offset) / k.size;
                }
                let at = center + translation;
                Placed {
                    index,
                    item: Rangef::new(at - size / 2.0, at + size / 2.0),
                    mask: Rangef::new(at - k.size / 2.0, at + k.size / 2.0),
                    z: 1.0 / (1.0 + index.abs_diff(current) as f32),
                }
            })
            .collect()
    }

    // Compose singleAdvanceFlingBehavior (PagerSnapDistance.atMost(1)), uncontained noSnapFlingBehavior
    fn settle(
        &self,
        core: &egui_mineral::runtime::Core,
        st: &State,
        snaps: &[f32],
        max: f32,
        velocity: f32,
    ) -> Option<f32> {
        let projected = core.policies().fling.project(egui_mineral::motion::Release {
            pos: st.scroll,
            velocity,
        });
        if !self.layout.snaps() {
            return (velocity != 0.0).then(|| projected.clamp(0.0, max));
        }
        let lo = st.start.saturating_sub(1);
        let hi = (st.start + 1).min(snaps.len().saturating_sub(1));
        let nearest = nearest(snaps, projected).clamp(lo, hi);
        let pick = if st.wheel > 0.0 {
            hi
        } else if st.wheel < 0.0 {
            lo
        } else {
            nearest
        };
        snaps.get(pick).copied()
    }

    fn run(self, ui: &mut MineralUi<'_, Material>, mut content: Content<'_>) -> CarouselResponse<usize> {
        let rt = ui.rt();
        let vertical = self.layout.vertical();
        let width = ui.available_width();
        // The strip owns drags (pans), the items clicks; egui hit-tests the two apart. CLICK defers `dragged` to
        // a decided drag, so a click on an item does not flash the grabbing cursor.
        let sense = if self.disabled {
            Sense::hover()
        } else {
            Sense::click_and_drag()
        };
        let (rect, response) = ui.allocate_exact_size(vec2(width, self.height), sense);
        let mut out = CarouselResponse {
            response,
            activated: None,
            position: 0.0,
        };
        if self.count == 0 {
            return out;
        }
        let ctx = ui.ctx().clone();
        let parts = kit::nodes::parts(&ctx, out.response.id, ui.unique_id());
        let id = ui.make_persistent_id(self.salt.id_salt());
        let dir = rt.i18n().direction();
        let track = if vertical {
            Track::vertical_down()
        } else {
            Track::horizontal(dir)
        };
        let extent = track.axis().main(rect.size());
        let key = GeometryKey {
            layout: self.layout,
            extent,
            spacing: self.spacing,
            padding: self.padding,
            count: self.count,
        };
        let Some(geo) = Geometry::load(&ctx, id, key) else {
            return out;
        };
        let strip = Strip { rect, id, track, geo };
        let mut st = State::load(&ctx, id);
        st.scroll = st.scroll.clamp(0.0, strip.geo.max_scroll);
        st.start = st.start.min(strip.geo.snaps.len().saturating_sub(1));
        let focused = ctx
            .memory(egui::Memory::focused)
            .and_then(|f| (0..self.count).find(|&i| strip.item_id(i) == f));
        self.update_scroll(ui, &strip, &mut st, &out.response, focused);
        if out.response.dragged() {
            ctx.set_cursor_icon(egui::CursorIcon::Grabbing);
        } else if out.response.contains_pointer() && !self.disabled {
            ctx.set_cursor_icon(egui::CursorIcon::Grab);
        }

        let current = nearest(&strip.geo.snaps, st.scroll);
        let tab_stop = focus::tab_stop(&ctx, id, Some(current), &focus::Items::all(self.count));
        let items = self.paint_items(ui, &strip, st.scroll, tab_stop, &mut content, &mut out.activated);
        kit::interact::remember(&ctx, id, items.ids);
        self.arrow_focus(rt, &strip, &mut st, items.focus.or(focused));

        drop(parts);
        semantic::emit(&ctx, Some(out.response.id), rect, || {
            let mut n = SemanticNode::new(
                StableId::of(&self.salt, "carousel"),
                Role::List,
                crate::tr_or(rt.i18n(), self.label.as_deref(), msg::CAROUSEL),
            );
            n.state.disabled = self.disabled;
            n
        });
        out.position = position(&strip.geo.snaps, st.scroll);
        st.store(&ctx, id);
        out
    }

    fn update_scroll(
        &self,
        ui: &MineralUi<'_, Material>,
        strip: &Strip,
        st: &mut State,
        response: &egui::Response,
        focused: Option<usize>,
    ) {
        let rt = ui.rt();
        let ctx = ui.ctx();
        let axis = strip.track.axis();
        let flip = strip.track.sign();
        let (snaps, max_scroll) = (&strip.geo.snaps, strip.geo.max_scroll);
        let anim = strip.id.with("scroll");
        let mut delta = 0.0;
        let mut released = None;
        let pan = if self.disabled {
            st.claim.reset();
            None
        } else {
            st.claim.update(ui, egui_mineral::input::PanMode::Axis(axis), response)
        };
        if let Some(pan) = pan {
            let step = -flip * axis.main(pan.delta);
            // Past either end the rest of the pan chains to the enclosing scroll.
            let over = st.scroll + step - (st.scroll + step).clamp(0.0, max_scroll);
            st.claim.chain(rt.core(), ui, axis, &pan, -flip * over);
            delta += step - over;
            if pan.ended {
                released = Some(-flip * axis.main(pan.velocity));
            }
        }
        let mut wheel = 0.0;
        if !self.disabled && ctx.rect_contains_pointer(ui.layer_id(), strip.rect) {
            let w = ctx.input(|i| i.smooth_scroll_delta);
            // Main-axis wheel always; the cross-axis wheel only while an item has focus.
            let (main, cross) = (axis.main(w), axis.cross(w));
            wheel = -flip * (main + if focused.is_some() { cross } else { 0.0 });
            if wheel != 0.0 {
                ctx.input_mut(|i| i.smooth_scroll_delta = Vec2::ZERO);
            }
        }
        let input = delta != 0.0 || wheel != 0.0 || (pan.is_some() && released.is_none());
        if input && !st.gesture {
            st.gesture = true;
            st.start = nearest(snaps, st.scroll);
            st.wheel = 0.0;
        }
        if wheel != 0.0 {
            st.wheel = wheel.signum();
        }
        if input {
            st.scroll = (st.scroll + delta + wheel).clamp(0.0, max_scroll);
            st.target = None;
            egui_mineral::motion::snap(ctx, anim, st.scroll);
        } else if st.gesture {
            st.gesture = false;
            st.target = self.settle(rt.core(), st, snaps, max_scroll, released.unwrap_or(0.0));
            st.wheel = 0.0;
        } else if st.target.is_none() && self.layout.snaps() {
            // A resize moves the snap points.
            if let Some(&snap) = snaps.get(nearest(snaps, st.scroll))
                && (snap - st.scroll).abs() > 0.5
            {
                st.target = Some(snap);
            }
        }
        if let Some(target) = st.target {
            st.scroll = egui_mineral::motion::spring(rt.core(), anim, target, SNAP_SPRING);
            #[expect(clippy::float_cmp, reason = "the spring returns its target exactly at rest")]
            if st.scroll == target {
                st.target = None;
            }
        }
    }

    fn paint_items(
        &self,
        ui: &mut MineralUi<'_, Material>,
        strip: &Strip,
        scroll: f32,
        tab_stop: usize,
        content: &mut Content<'_>,
        activated: &mut Option<usize>,
    ) -> Items {
        let current = nearest(&strip.geo.snaps, scroll);
        let rt = ui.rt();
        let t = rt.tokens();
        let vertical = self.layout.vertical();
        let axis = strip.track.axis();
        let surround = self.surround.unwrap_or(t.color.surface);
        let corner = f32::from(ITEM_SHAPE.radius(&t.shape));
        let mut items = Items::default();
        let dir = if vertical {
            i18n::Direction::Ltr
        } else {
            rt.i18n().direction()
        };
        let to_rect = |span: Rangef| {
            let len = span.span();
            let x0 = dir.span_x(axis.main_range(strip.rect), Rangef::new(span.min, span.min + len));
            let main = Rangef::new(x0, x0 + len);
            axis.rect(main, axis.cross_range(strip.rect))
        };
        let mut placed = self.place(&strip.geo.strategy, scroll, strip.geo.max_scroll, current);
        placed.sort_by(|a, b| a.z.total_cmp(&b.z));
        let p = ui.painter_at(strip.rect);
        for item in placed {
            let i = item.index;
            let (full, mask) = (to_rect(item.item), to_rect(item.mask));
            let shown = mask.intersect(strip.rect);
            if item.mask.span() <= 0.0 || !shown.is_positive() {
                continue;
            }
            let sense = if self.disabled {
                Sense::hover()
            } else {
                focus::roving_sense(Sense::click(), focus::TabStop::at(i, tab_stop))
            };
            let resp = kit::interact::interact(ui, shown, strip.item_id(i), sense, Availability::Enabled);
            kit::interact::follow_press(&resp);
            items.ids.push((i, resp.id));
            if resp.clicked() && !self.disabled {
                *activated = Some(i);
            }
            let r = CornerRadius::from(corner.min(mask.width().min(mask.height()) / 2.0));
            match content {
                Content::Ui(add) => {
                    let mut child =
                        ui.new_child(UiBuilder::new().max_rect(full).layout(rt.i18n().direction().column()));
                    child.set_clip_rect(shown.intersect(ui.clip_rect()));
                    add(&mut rt.ui(&mut child), i);
                    round_corners(&p, mask, f32::from(r.nw), surround);
                }
                Content::Image(image) => {
                    let tex = image(i);
                    let uv = cover_uv(full.size(), tex.size);
                    let sub = Rect::from_min_max(
                        uv.lerp_inside((mask.min - full.min) / full.size()),
                        uv.lerp_inside((mask.max - full.min) / full.size()),
                    );
                    p.add(RectShape::filled(mask, r, Color32::WHITE).with_texture(tex.id, sub));
                }
            }
            let inter = Interaction::of(&resp, Availability::disabled_if(self.disabled));
            crate::widget::common::state_focus_inset(&p, mask, r, t.color.on_surface, inter, &t, 2.0);
            if inter.focused {
                items.focus = Some(i);
            }
            self.emit_item(rt, resp.id, shown, i, current);
        }
        items
    }

    fn arrow_focus(&self, rt: &Rt, strip: &Strip, st: &mut State, focus: Option<usize>) {
        let ctx = rt.ctx();
        let Some(cur) = focus else { return };
        let nav = RovingFocus::new(if self.layout.vertical() {
            Axis::Vertical
        } else {
            Axis::Horizontal
        })
        .wrap(false);
        if let Some(idx) = nav.on_key(rt.core(), strip.item_id(cur), cur, &focus::Items::all(self.count))
            && let Some(&snap) = strip.geo.snaps.get(idx)
        {
            st.target = Some(snap);
            ctx.memory_mut(|m| m.request_focus(strip.item_id(idx)));
        }
    }

    fn emit_item(&self, rt: &Rt, id: Id, rect: Rect, i: usize, current: usize) {
        let ctx = rt.ctx();
        semantic::emit(ctx, Some(id), rect, || {
            let name = self.item_label.as_ref().map_or_else(
                || {
                    rt.i18n().tr(msg::CarouselItem {
                        index: (i + 1).into(),
                        count: self.count.into(),
                    })
                },
                |f| f(i),
            );
            let mut node = SemanticNode::new(StableId::of(&self.salt, (i, "item")), Role::ListItem, name)
                .clickable(Availability::disabled_if(self.disabled));
            node.state.selected = Some(i == current);
            node
        });
    }
}

fn nearest(snaps: &[f32], scroll: f32) -> usize {
    (0..snaps.len())
        .min_by(|&a, &b| (snaps[a] - scroll).abs().total_cmp(&(snaps[b] - scroll).abs()))
        .unwrap_or_default()
}

fn position(snaps: &[f32], scroll: f32) -> f32 {
    for (i, w) in snaps.windows(2).enumerate() {
        if scroll <= w[1] && w[1] > w[0] {
            return i as f32 + ((scroll - w[0]) / (w[1] - w[0])).clamp(0.0, 1.0);
        }
    }
    snaps.len().saturating_sub(1) as f32
}

// Compose ContentScale.Crop
fn cover_uv(rect: Vec2, tex: Vec2) -> Rect {
    if rect.x <= 0.0 || rect.y <= 0.0 || tex.x <= 0.0 || tex.y <= 0.0 {
        return Rect::from_min_max(Pos2::ZERO, pos2(1.0, 1.0));
    }
    let (ar, at) = (rect.x / rect.y, tex.x / tex.y);
    if at > ar {
        let w = ar / at;
        Rect::from_min_max(pos2((1.0 - w) / 2.0, 0.0), pos2(f32::midpoint(1.0, w), 1.0))
    } else {
        let h = at / ar;
        Rect::from_min_max(pos2(0.0, (1.0 - h) / 2.0), pos2(1.0, f32::midpoint(1.0, h)))
    }
}

// egui clips child UIs to rectangles only
fn round_corners(p: &Painter, rect: Rect, r: f32, color: Color32) {
    if r < 1.0 {
        return;
    }
    for (corner, sx, sy) in [
        (rect.left_top(), 1.0, 1.0),
        (rect.right_top(), -1.0, 1.0),
        (rect.right_bottom(), -1.0, -1.0),
        (rect.left_bottom(), 1.0, -1.0),
    ] {
        let center = corner + vec2(sx * r, sy * r);
        let a0 = 0.0_f32.atan2(-sx);
        let a1 = (-sy).atan2(0.0);
        let mut sweep = a1 - a0;
        if sweep > std::f32::consts::PI {
            sweep -= std::f32::consts::TAU;
        } else if sweep < -std::f32::consts::PI {
            sweep += std::f32::consts::TAU;
        }
        let mut mesh = Mesh::default();
        mesh.colored_vertex(corner, color);
        for k in 0..=ARC_STEPS {
            let a = a0 + sweep * k as f32 / ARC_STEPS as f32;
            let at: Pos2 = center + vec2(a.cos(), a.sin()) * r;
            mesh.colored_vertex(at, color);
        }
        for k in 1..=ARC_STEPS {
            mesh.add_triangle(0, k, k + 1);
        }
        p.add(Shape::mesh(mesh));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn strategy(layout: CarouselLayout, main: f32, spacing: f32, count: usize) -> Strategy {
        Strategy::new(
            layout.keylines(main, spacing, count),
            main,
            spacing,
            CarouselPadding::default(),
        )
    }

    #[test]
    fn multi_browse_keylines_fill_the_carousel() {
        let k = multi_browse_keylines(412.0, 186.0, 8.0, 10);
        let sizes: Vec<f32> = k.0.iter().filter(|k| !k.is_anchor).map(|k| k.size).collect();
        let used: f32 = sizes.iter().sum::<f32>() + 8.0 * (sizes.len() - 1) as f32;
        assert!((used - 412.0).abs() < 1e-3, "{sizes:?}");
        assert!(sizes.windows(2).all(|w| w[0] >= w[1]), "large to small: {sizes:?}");
        assert!(
            sizes
                .last()
                .is_some_and(|s| (cx::MIN_SMALL_ITEM_SIZE..=cx::MAX_SMALL_ITEM_SIZE).contains(s))
        );
    }

    #[test]
    fn snap_points_span_the_scroll_range() {
        for layout in [
            CarouselLayout::MultiBrowse {
                preferred_item_width: 186.0,
            },
            CarouselLayout::Hero {
                preferred_item_width: None,
                centered: true,
            },
            CarouselLayout::Hero {
                preferred_item_width: None,
                centered: false,
            },
        ] {
            let s = strategy(layout, 412.0, 8.0, 8);
            let snaps = s.snap_points(8);
            assert_eq!(snaps[0], 0.0, "{layout:?}");
            assert!((snaps[7] - s.max_scroll(8)).abs() < 1.0, "{layout:?}: {snaps:?}");
            assert!(snaps.windows(2).all(|w| w[0] <= w[1]), "{layout:?}: {snaps:?}");
        }
    }

    #[test]
    fn keylines_at_rest_are_the_defaults() {
        let s = strategy(
            CarouselLayout::MultiBrowse {
                preferred_item_width: 186.0,
            },
            412.0,
            8.0,
            8,
        );
        assert_eq!(s.keylines_at(s.start_shift, s.max_scroll(8)), s.default);
    }

    #[test]
    fn oversized_content_padding_keeps_every_step_valid() {
        let layout = CarouselLayout::MultiBrowse {
            preferred_item_width: 186.0,
        };
        for main in [60.0, 412.0] {
            let default = layout.keylines(main, 8.0, 8);
            let s = Strategy::new(
                default,
                main,
                8.0,
                CarouselPadding {
                    start: 100.0,
                    end: 100.0,
                },
            );
            assert!(s.is_valid(), "{main}");
            for step in s.start_steps.iter().chain(&s.end_steps) {
                assert!(!step.0.is_empty(), "{main}");
                assert!(step.0.iter().all(|k| k.size >= MIN_PADDED_SIZE), "{main}: {step:?}");
            }
            assert!(s.snap_points(8).iter().all(|p| p.is_finite()), "{main}");
        }
    }
}
