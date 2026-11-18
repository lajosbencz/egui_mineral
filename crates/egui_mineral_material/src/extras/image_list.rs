use crate::Material;
use crate::MaterialExt;
use std::{cmp::Ordering, ops::Range, sync::Arc};

use egui::{Color32, CornerRadius, ImageSource, Rangef, Rect, Sense, UiBuilder, pos2, vec2};
use egui_mineral::i18n::I18n;
use egui_mineral::semantic::KeyedSalt;
use egui_mineral::widget::MineralUi;
use egui_mineral::widget::{Availability, MineralWidget, WidgetFrame, WidgetResponse};
use egui_mineral::{
    focus::{self, RovingFocus},
    kit::{self, state::TempState},
    layout::{Axis, Constraints, Measure},
    semantic::{self, Role, SemanticNode, StableId},
};

use crate::spec::{ColorRole, ShapeRole, TypeScale, tokens::plain_tooltip};
use crate::widget::{
    common::{self, Interaction},
    indication, parts,
};

// No MD3 counterpart
const TILE_SHAPE: ShapeRole = ShapeRole::CornerExtraSmall;
const TITLE_FONT: TypeScale = TypeScale::TitleSmall;
const TITLE_BAR_OPACITY: f32 = 0.8;
const TILE_COLOR: ColorRole = ColorRole::SurfaceContainerHighest;
// Compose ripple() without a color takes LocalContentColor
const RIPPLE_COLOR: ColorRole = ColorRole::OnSurface;

/// Tile arrangement of an [`ImageList`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum ImageListLayout {
    #[default]
    /// Uniform grid; tile height from [`ImageList::row_aspect`].
    Standard,
    /// Columns of tiles with per-index heights.
    Masonry,
}

/// Result of [`ImageList::show`].
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct ImageListResponse {
    /// Grid response.
    pub response: egui::Response,
    /// Index of the tile activated this frame.
    pub activated: Option<usize>,
}

type LabelFn<'a> = Box<dyn Fn(usize) -> String + 'a>;
type AspectFn<'a> = Box<dyn Fn(usize) -> f32 + 'a>;

/// Grid of image tiles, standard or masonry; non-MD3 (MUI `ImageList` pattern).
#[must_use = "call .show(..)"]
pub struct ImageList<'a> {
    salt: KeyedSalt,
    columns: usize,
    gap: Option<f32>,
    layout: ImageListLayout,
    row_aspect: f32,
    aspect: Option<AspectFn<'a>>,
    aspect_version: u64,
    label: Option<String>,
    item_label: Option<LabelFn<'a>>,
    count: usize,
    disabled: bool,
}

egui_mineral::kit::opaque_debug!(ImageList<'_>);

const MAX_COLUMNS: u8 = 32;

struct Grid {
    cols: usize,
    col_w: f32,
    row_h: f32,
    gap: f32,
}

impl Grid {
    fn new(columns: usize, width: f32, gap: f32) -> Self {
        let cols = columns.max(1);
        Self {
            cols,
            col_w: ((width - gap * (cols - 1) as f32) / cols as f32).max(0.0),
            row_h: 0.0,
            gap,
        }
    }

    fn standard(mut self, row_aspect: f32) -> Self {
        self.row_h = self.col_w * row_aspect;
        self
    }

    fn rect(&self, i: usize) -> Rect {
        let (r, c) = (i / self.cols, i % self.cols);
        Rect::from_min_size(
            pos2((self.col_w + self.gap) * c as f32, (self.row_h + self.gap) * r as f32),
            vec2(self.col_w, self.row_h),
        )
    }

    fn height(&self, len: usize) -> f32 {
        let rows = len.div_ceil(self.cols);
        (self.row_h + self.gap) * rows as f32 - if rows > 0 { self.gap } else { 0.0 }
    }

    // Indices of the tiles whose rows touch `y` (grid coordinates).
    fn visible(&self, y: Rangef, len: usize) -> Range<usize> {
        let pitch = self.row_h + self.gap;
        if pitch.partial_cmp(&0.0) != Some(Ordering::Greater) {
            return 0..0;
        }
        // Float to usize casts saturate and map NaN to 0.
        #[expect(clippy::cast_sign_loss, reason = "max(0.0)")]
        let first = ((y.min - self.row_h) / pitch).floor().max(0.0) as usize;
        #[expect(clippy::cast_sign_loss, reason = "max(0.0)")]
        let last = (y.max / pitch).floor().max(0.0) as usize;
        let end = last.saturating_add(1).saturating_mul(self.cols).min(len);
        first.saturating_mul(self.cols).min(end)..end
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
struct MasonryKey {
    cols: usize,
    width: f32,
    gap: f32,
    len: usize,
    version: u64,
}

#[derive(Clone, Default)]
struct Masonry {
    key: Option<MasonryKey>,
    rects: Arc<[Rect]>,
    height: f32,
}

impl Masonry {
    fn refresh(&mut self, key: MasonryKey, grid: &Grid, aspect: &dyn Fn(usize) -> f32) {
        if self.key.as_ref() == Some(&key) {
            return;
        }
        let mut heights = vec![0.0f32; grid.cols];
        self.rects = (0..key.len)
            .map(|i| {
                let (c, top) = heights
                    .iter()
                    .copied()
                    .enumerate()
                    .min_by(|a, b| a.1.total_cmp(&b.1))
                    .unwrap_or((0, 0.0));
                let h = grid.col_w * aspect(i).max(0.01);
                heights[c] = top + h + grid.gap;
                Rect::from_min_size(pos2((grid.col_w + grid.gap) * c as f32, top), vec2(grid.col_w, h))
            })
            .collect();
        let h = heights.iter().copied().fold(0.0, f32::max);
        self.height = (h - grid.gap).max(0.0);
        self.key = Some(key);
    }
}

#[derive(Clone, Default)]
struct State {
    masonry: Masonry,
    focus: Option<usize>,
}

impl TempState for State {}

impl<'a> ImageList<'a> {
    /// `count` tiles in 3 columns; agent key prefix `<id_salt>`.
    pub fn new(id_salt: impl Into<KeyedSalt>, count: usize) -> Self {
        let id_salt: KeyedSalt = id_salt.into();
        Self {
            salt: id_salt,
            columns: 3,
            gap: None,
            layout: ImageListLayout::default(),
            row_aspect: 1.0,
            aspect: None,
            aspect_version: 0,
            label: None,
            item_label: None,
            count,
            disabled: false,
        }
    }

    crate::widget::setters! {
        /// Disables every tile.
        disabled
    }

    crate::widget::setters! {
        /// Replaces the `<id_salt>` key prefix.
        agent_key(salt)
    }

    /// Column count; default 3, 1 to 32.
    #[track_caller]
    pub fn columns(mut self, columns: usize) -> Self {
        let columns = egui_mineral::kit::sanitize::at_least(columns, 1.., "ImageList columns");
        let capped = u8::try_from(columns).unwrap_or(u8::MAX);
        self.columns = usize::from(egui_mineral::kit::sanitize::at_most(
            capped,
            ..=MAX_COLUMNS,
            "ImageList columns",
        ));
        self
    }

    /// Space between tiles; defaults to the extra small spacing token; negative values snap to the minimum.
    #[track_caller]
    pub fn gap(mut self, gap: f32) -> Self {
        self.gap = Some(egui_mineral::kit::sanitize::at_least(gap, 0.0.., "gap"));
        self
    }

    /// Default [`ImageListLayout::Standard`].
    pub fn layout(mut self, layout: ImageListLayout) -> Self {
        self.layout = layout;
        self
    }

    /// Standard layout tile height over width; default 1; non-positive values snap to the minimum.
    #[track_caller]
    pub fn row_aspect(mut self, row_aspect: f32) -> Self {
        self.row_aspect = egui_mineral::kit::sanitize::at_least(row_aspect, f32::MIN_POSITIVE.., "row_aspect");
        self
    }

    /// [`ImageListLayout::Masonry`] tile height over width by index; default 1; see [`Self::aspect_version`].
    pub fn aspect(mut self, aspect: impl Fn(usize) -> f32 + 'a) -> Self {
        self.aspect = Some(Box::new(aspect));
        self
    }

    /// Version of what [`Self::aspect`] reads; default 0. Masonry tile rects are cached on it, the width, columns,
    /// gap and count: change it whenever the aspects change.
    pub fn aspect_version(mut self, aspect_version: u64) -> Self {
        self.aspect_version = aspect_version;
        self
    }

    /// Accessible name of the grid; default "Images".
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Accessible tile names by index; default "Image n of count".
    pub fn item_label(mut self, item_label: impl Fn(usize) -> String + 'a) -> Self {
        self.item_label = Some(Box::new(item_label));
        self
    }

    fn geometry(&self, st: &mut State, width: f32) -> (Grid, f32) {
        let gap = self.gap.unwrap_or(crate::adaptive::grid(1));
        let grid = Grid::new(self.columns, width, gap);
        match self.layout {
            ImageListLayout::Standard => {
                let grid = grid.standard(self.row_aspect);
                let height = grid.height(self.count);
                (grid, height)
            }
            ImageListLayout::Masonry => {
                let key = MasonryKey {
                    cols: grid.cols,
                    width,
                    gap,
                    len: self.count,
                    version: self.aspect_version,
                };
                st.masonry
                    .refresh(key, &grid, self.aspect.as_deref().unwrap_or(&|_| 1.0));
                (grid, st.masonry.height)
            }
        }
    }

    fn tile_node(&self, i18n: &I18n, i: usize) -> SemanticNode {
        let name = self.item_label.as_ref().map_or_else(
            || {
                i18n.tr(
                    "mineral-image-list-item",
                    &[("index", (i + 1).into()), ("count", self.count.into())],
                )
            },
            |f| f(i),
        );
        let mut n = SemanticNode::new(StableId::of(&self.salt, (i, "tile")), Role::GridCell, name)
            .clickable(Availability::disabled_if(self.disabled));
        n.depth = 1;
        n
    }

    /// `tile` fills each tile's ui, usually with an [`ImageTile`].
    pub fn show(
        self,
        ui: &mut MineralUi<'_, Material>,
        mut tile: impl FnMut(&mut MineralUi<'_, Material>, usize),
    ) -> ImageListResponse {
        let rt = ui.rt();
        let len = self.count;
        let ctx = ui.ctx().clone();
        let t = rt.tokens();
        let id = ui.make_persistent_id(self.salt.id_salt());
        let dir = rt.i18n().direction();
        let availability = Availability::disabled_if(self.disabled);
        semantic::emit_parent(&ctx, id, || {
            let name = crate::tr_or(rt.i18n(), self.label.as_deref(), "mineral-image-list");
            let mut n = SemanticNode::new(StableId::of(&self.salt, "image-list"), Role::Grid, name);
            n.state.disabled = self.disabled;
            n
        });
        let width = ui.available_width();
        let mut st = State::load(&ctx, id);
        let (grid, height) = self.geometry(&mut st, width);
        let (area, response) = ui.allocate_exact_size(vec2(width, height), Sense::hover());
        let mut activated = None;
        let tab = focus::tab_stop(&ctx, id, None, len, |_| !self.disabled);
        let focused = ctx.memory(egui::Memory::focused);
        let held = st.focus.filter(|&i| i < len && focused == Some(id.with(i)));
        let range = match self.layout {
            ImageListLayout::Standard => {
                let clip = ui.clip_rect().y_range();
                grid.visible(Rangef::new(clip.min - area.top(), clip.max - area.top()), len)
            }
            ImageListLayout::Masonry => 0..len,
        };
        let before = held.filter(|&i| i < range.start);
        let after = held.filter(|&i| i >= range.end);
        let mut focus_at = None;
        for i in before.into_iter().chain(range).chain(after) {
            let local = match self.layout {
                ImageListLayout::Standard => grid.rect(i),
                ImageListLayout::Masonry => match st.masonry.rects.get(i) {
                    Some(r) => *r,
                    None => continue,
                },
            };
            let mut rect = local.translate(area.min.to_vec2());
            rect = dir.mirror(rect, area);
            let tid = id.with(i);
            if !ui.is_rect_visible(rect) && held != Some(i) {
                continue;
            }
            let resp = kit::input::interact(
                ui,
                rect,
                tid,
                focus::roving_sense(Sense::click(), focus::TabStop::at(i, tab)),
                availability,
            );
            kit::focus::follow_press(&resp);
            if resp.has_focus() {
                focus_at = Some(i);
            }
            let radius = TILE_SHAPE.corner_radius(&t.shape, dir);
            let mut child = ui.new_child(UiBuilder::new().max_rect(rect).id_salt(i));
            child.set_clip_rect(rect.intersect(ui.clip_rect()));
            tile(&mut rt.ui(&mut child), i);
            indication::paint(
                ui,
                &resp,
                rect,
                radius,
                t.role(RIPPLE_COLOR),
                Interaction::of(&resp, availability),
                &t,
            );
            if focus::visible(&resp.ctx, resp.id) {
                common::focus_ring(ui.painter(), rect, radius, &t);
            }
            if resp.clicked() {
                activated = Some(i);
            }
            semantic::emit(&ctx, Some(resp.id), rect, || self.tile_node(rt.i18n(), i));
        }
        if focus_at.is_some() {
            let ids: Vec<_> = (0..len).map(|i| id.with(i)).collect();
            let nav = match self.layout {
                ImageListLayout::Standard => RovingFocus::grid(self.columns),
                ImageListLayout::Masonry => RovingFocus::new(Axis::Horizontal),
            };
            nav.wrap(false).update_group(rt.core(), id, &ids, |_| !self.disabled);
            let now = ctx.memory(egui::Memory::focused);
            focus_at = ids.iter().position(|&t| Some(t) == now);
        }
        st.focus = focus_at;
        st.store(&ctx, id);
        kit::semantic::remember_rect(&ctx, id, area);
        ImageListResponse { response, activated }
    }
}

/// Tile source and caption for an [`ImageList`].
#[derive(Clone, Debug)]
#[must_use = "show with ui.mineral(..)"]
pub struct ImageTile {
    salt: KeyedSalt,
    source: ImageSource<'static>,
    title: Option<String>,
}

impl ImageTile {
    /// Image painted over the whole tile, rounded.
    pub fn new(id_salt: impl Into<KeyedSalt>, source: impl Into<ImageSource<'static>>) -> Self {
        let id_salt: KeyedSalt = id_salt.into();
        Self {
            salt: id_salt,
            source: source.into(),
            title: None,
        }
    }

    /// Caption over the image.
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    fn paint(&self, ui: &mut MineralUi<'_, Material>, rect: Rect) {
        let rt = ui.rt();
        if !ui.is_rect_visible(rect) {
            return;
        }
        let (source, title) = (self.source.clone(), self.title.as_deref());
        let t = rt.tokens();
        let dir = rt.i18n().direction();
        let radius = TILE_SHAPE.corner_radius(&t.shape, dir);
        ui.painter().rect_filled(rect, radius, t.role(TILE_COLOR));
        egui::Image::new(source).corner_radius(radius).paint_at(ui, rect);
        if let Some(title) = title {
            let h = TITLE_FONT.line_height() + 2.0 * crate::adaptive::grid(2);
            let bar = Rect::from_min_max(pos2(rect.left(), rect.bottom() - h), rect.max);
            ui.painter().rect_filled(
                bar,
                CornerRadius {
                    sw: radius.sw,
                    se: radius.se,
                    ..CornerRadius::ZERO
                },
                t.role_alpha(plain_tooltip::CONTAINER_COLOR, TITLE_BAR_OPACITY),
            );
            let g = parts::type_wrapped(
                ui.painter(),
                title,
                TITLE_FONT,
                t.role(plain_tooltip::SUPPORTING_TEXT_COLOR),
                bar.width() - 2.0 * crate::adaptive::grid(4),
                1,
            );
            let pad = crate::adaptive::grid(4);
            let x = dir.start_x(bar, Rangef::new(pad, pad + g.size().x));
            ui.painter()
                .galley(pos2(x, bar.center().y - g.size().y / 2.0), g, Color32::PLACEHOLDER);
        }
    }
}

/// Fills the available rect; decorative, the [`ImageList`] tile carries the semantics.
impl MineralWidget for ImageTile {
    type Kit = Material;
    type State = ();
    type Output = ();

    egui_mineral::kit::widget_meta!(salt, no_target, hover);

    fn layout(&self, ui: &MineralUi<'_, Material>, c: Constraints, (): &Self::State) -> Measure {
        c.constrain(ui.available_size()).into()
    }

    fn update(&mut self, ui: &mut MineralUi<'_, Material>, frame: &WidgetFrame<'_>, (): &mut ()) -> WidgetResponse {
        self.paint(ui, frame.zones.visual);
        WidgetResponse::new(frame.response.clone(), ())
    }

    fn semantics(&self, _: &I18n, _: &WidgetResponse, (): &()) -> Option<SemanticNode> {
        None
    }
}
