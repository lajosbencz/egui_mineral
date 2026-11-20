use crate::Material;
use crate::MaterialExt;
use crate::MaterialUiExt as _;
use egui_mineral::i18n::Sides;
use egui_mineral::semantic::KeyedSalt;
use egui_mineral::semantic::StableId;
use egui_mineral::widget::MineralUi;
use std::{collections::BTreeMap, sync::Arc};

use egui::{Rangef, Rect, Sense, Ui, pos2, vec2};
use egui_mineral::{
    shortcut::{self},
    widget::SizeExt,
};

use crate::selection::{Menu, MenuItem};
use crate::widget::parts;
use crate::{
    Icon,
    adaptive::WindowClass,
    motion,
    spec::{
        ColorRole, TypeScale, defaults,
        tokens::{
            app_bar, app_bar_large, app_bar_large_flexible, app_bar_medium, app_bar_medium_flexible, app_bar_small,
            search_bar,
        },
    },
    tokens::Tokens,
};
use crate::{IconAction, actions::IconButton};

/// Compose `TopAppBar`, `CenterAlignedTopAppBar`, `MediumTopAppBar` and `LargeTopAppBar` styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
pub enum TopAppBarVariant {
    #[default]
    /// `TopAppBar`: one row, title at the start.
    Small,
    /// `CenterAlignedTopAppBar`.
    CenterAligned,
    /// `MediumTopAppBar`: collapsing two-row bar.
    Medium,
    /// `LargeTopAppBar`: collapsing two-row bar, larger title.
    Large,
}

// Compose AppBarRow: at most 3 inline actions on compact windows, 5 elsewhere
use crate::adaptive::Density;

const COMPACT_ACTIONS: usize = 3;
const MAX_ACTIONS: usize = 5;
// 40dp container in the 48dp minimum interactive size

fn inline_actions(count: usize, slots: usize) -> usize {
    if count <= slots { count } else { slots.saturating_sub(1) }
}

/// MD3 top app bar with scroll-collapsing and search modes (Compose `TopAppBar`).
#[derive(Debug)]
#[must_use = "call .show(..)"]
pub struct TopAppBar<'a> {
    salt: KeyedSalt,
    title: String,
    subtitle: Option<String>,
    variant: TopAppBarVariant,
    flexible: bool,
    centered: bool,
    navigation: Option<(Icon, String)>,
    actions: Vec<IconAction>,
    disabled: bool,
    scrolled: bool,
    offset: f32,
    search: Option<&'a mut String>,
}

/// Result of [`TopAppBar::show`].
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct TopAppBarResponse {
    /// Bar container response.
    pub response: egui::Response,
    /// Navigation icon clicked; see [`TopAppBar::navigation`].
    pub leading_clicked: bool,
    /// Index of the clicked action without a [`IconAction::command`], inline or overflowed.
    pub clicked: Option<usize>,
    /// Buttons of the inline actions by index, to anchor popups; overflowed actions are absent.
    pub actions: BTreeMap<usize, egui::Response>,
    /// Overflow menu button; `None` while every action is inline.
    pub overflow: Option<egui::Response>,
    /// Search query edited; only with [`TopAppBar::search`].
    pub changed: bool,
    /// Enter pressed in the search field.
    pub submitted: bool,
}

impl<'a> TopAppBar<'a> {
    /// `title` is the headline, also the search placeholder; agent key prefix `<id_salt>`.
    pub fn new(id_salt: impl Into<KeyedSalt>, title: impl Into<String>) -> Self {
        let id_salt: KeyedSalt = id_salt.into();
        Self {
            salt: id_salt,
            title: title.into(),
            subtitle: None,
            variant: TopAppBarVariant::default(),
            flexible: false,
            centered: false,
            navigation: None,
            actions: Vec::new(),
            disabled: false,
            scrolled: false,
            offset: 0.0,
            search: None,
        }
    }

    crate::widget::setters! {
        /// Replaces the `<id_salt>` key prefix.
        agent_key(salt)
    }

    /// Default [`TopAppBarVariant::Small`].
    pub fn variant(mut self, variant: TopAppBarVariant) -> Self {
        self.variant = variant;
        self
    }

    /// M3 Expressive flexible metrics (`AppBar*FlexibleTokens`).
    pub fn flexible(mut self, flexible: bool) -> Self {
        self.flexible = flexible;
        self
    }

    /// Centers the title in any variant.
    pub fn centered(mut self, centered: bool) -> Self {
        self.centered = centered;
        self
    }

    /// Subtitle below the title.
    pub fn supporting(mut self, supporting: impl Into<String>) -> Self {
        self.subtitle = Some(supporting.into());
        self
    }

    /// Leading icon button; `label` is its accessible name.
    pub fn navigation(mut self, icon: impl Into<Icon>, label: impl Into<String>) -> Self {
        self.navigation = Some((icon.into(), label.into()));
        self
    }

    /// Appends one action; past the inline limit (3 compact, 5 otherwise) actions go to an overflow menu.
    pub fn action(mut self, action: impl Into<IconAction>) -> Self {
        self.actions.push(action.into());
        self
    }

    /// Appends actions; see [`Self::action`].
    pub fn actions(mut self, actions: impl IntoIterator<Item = impl Into<IconAction>>) -> Self {
        self.actions.extend(actions.into_iter().map(Into::into));
        self
    }

    crate::widget::setters! {
        /// Disables every action and the overflow button.
        disabled
    }

    /// Content scrolled under the bar: single-row bars switch to the scrolled container color.
    pub fn scrolled(mut self, scrolled: bool) -> Self {
        self.scrolled = scrolled;
        self
    }

    /// Content scroll offset in dp; collapses two-row bars; negative values snap to the minimum.
    #[track_caller]
    pub fn scroll_offset(mut self, scroll_offset: f32) -> Self {
        self.offset = egui_mineral::kit::sanitize::at_least(scroll_offset, 0.0.., "scroll_offset");
        self
    }

    /// Replaces the title row with a search bar bound to `query`.
    pub fn search(mut self, query: &'a mut String) -> Self {
        self.search = Some(query);
        self
    }

    fn expanded_height(&self) -> f32 {
        let sub = self.subtitle.is_some();
        match (self.variant, self.flexible, sub) {
            (TopAppBarVariant::Small | TopAppBarVariant::CenterAligned, ..) => app_bar_small::CONTAINER_HEIGHT,
            (TopAppBarVariant::Medium, false, false) => app_bar_medium::CONTAINER_HEIGHT,
            (TopAppBarVariant::Medium, true, false) => app_bar_medium_flexible::CONTAINER_HEIGHT,
            // A subtitle takes the flexible bar's taller height.
            (TopAppBarVariant::Medium, _, true) => app_bar_medium_flexible::LARGE_CONTAINER_HEIGHT,
            (TopAppBarVariant::Large, false, _) => app_bar_large::CONTAINER_HEIGHT,
            (TopAppBarVariant::Large, true, false) => app_bar_large_flexible::CONTAINER_HEIGHT,
            (TopAppBarVariant::Large, true, true) => app_bar_large_flexible::LARGE_CONTAINER_HEIGHT,
        }
    }

    /// Current height in dp at `density` after collapsing by the scroll offset; for padding the content.
    /// The action row is 4dp lower per density step, down to its action slots.
    pub fn bar_height(&self, density: Density) -> f32 {
        let collapsed = collapsed_height(density);
        if self.search.is_some() {
            return collapsed;
        }
        (self.full_height(density) - self.offset).max(collapsed)
    }

    fn full_height(&self, density: Density) -> f32 {
        self.expanded_height() - (app_bar_small::CONTAINER_HEIGHT - collapsed_height(density))
    }

    // Non-flexible bars have no subtitle token; they take the flexible one
    fn fonts(&self) -> TitleFonts {
        match (self.variant, self.flexible) {
            (TopAppBarVariant::Small | TopAppBarVariant::CenterAligned, _) => TitleFonts {
                title: app_bar_small::TITLE_FONT,
                subtitle: app_bar_small::SUBTITLE_FONT,
            },
            (TopAppBarVariant::Medium, false) => TitleFonts {
                title: app_bar_medium::TITLE_FONT,
                subtitle: app_bar_medium_flexible::SUBTITLE_FONT,
            },
            (TopAppBarVariant::Medium, true) => TitleFonts {
                title: app_bar_medium_flexible::TITLE_FONT,
                subtitle: app_bar_medium_flexible::SUBTITLE_FONT,
            },
            (TopAppBarVariant::Large, false) => TitleFonts {
                title: app_bar_large::TITLE_FONT,
                subtitle: app_bar_large_flexible::SUBTITLE_FONT,
            },
            (TopAppBarVariant::Large, true) => TitleFonts {
                title: app_bar_large_flexible::TITLE_FONT,
                subtitle: app_bar_large_flexible::SUBTITLE_FONT,
            },
        }
    }

    fn title_block(&self, ui: &Ui, fonts: TitleFonts, width: f32, t: &Tokens) -> TitleBlock {
        let TitleFonts { title, subtitle: sub } = fonts;
        let p = ui.painter();
        let line =
            |text: &str, scale: TypeScale, role: ColorRole| parts::type_wrapped(p, text, scale, t.role(role), width, 1);
        TitleBlock {
            title: line(&self.title, title, app_bar::TITLE_COLOR),
            subtitle: self.subtitle.as_deref().map(|s| line(s, sub, app_bar::SUBTITLE_COLOR)),
        }
    }

    /// Paints the bar at the full available width.
    pub fn show(mut self, ui: &mut MineralUi<'_, Material>) -> TopAppBarResponse {
        let rt = ui.rt();
        let t = rt.tokens();
        let id = ui.make_persistent_id(self.salt.id_salt());
        let density = ui.density();
        let collapsed = collapsed_height(density);
        for a in &mut self.actions {
            a.disabled |= self.disabled;
        }
        let height = self.bar_height(density);
        let full = self.full_height(density);
        let small = matches!(self.variant, TopAppBarVariant::Small | TopAppBarVariant::CenterAligned);
        // Compose `TopAppBarState.collapsedFraction`.
        let collapse = if full > collapsed && self.search.is_none() {
            ((full - height) / (full - collapsed)).clamp(0.0, 1.0)
        } else {
            1.0
        };
        let width = ui.available_width();
        let (rect, response) = ui.allocate_exact_size(vec2(width, height), Sense::hover());
        // Single-row bars animate to the scrolled color; two-row bars follow the collapse.
        let on_scroll = if small {
            egui_mineral::motion::spring(
                rt.core(),
                id.with("on-scroll"),
                f32::from(self.scrolled || self.offset > 0.0),
                motion::DEFAULT_EFFECTS,
            )
        } else {
            collapse
        };
        let container = t
            .role(app_bar::CONTAINER_COLOR)
            .lerp_to_gamma(t.role(app_bar::ON_SCROLL_CONTAINER_COLOR), on_scroll);
        ui.painter().rect_filled(rect, 0, container);
        let mut out = TopAppBarResponse {
            response,
            leading_clicked: false,
            clicked: None,
            actions: BTreeMap::new(),
            overflow: None,
            changed: false,
            submitted: false,
        };
        let row = Row {
            rect: Rect::from_min_size(rect.min, vec2(rect.width(), collapsed)),
            dir: rt.i18n().direction(),
            slot: IconButton::slot(density),
        };
        let mut start = app_bar::LEADING_SPACE;
        if let Some((icon, label)) = &self.navigation {
            out.leading_clicked =
                egui_mineral::layout::child(ui, self.salt.child("nav"), row.slot(start), *ui.layout(), |ui| {
                    ui.mineral(IconButton::new(self.salt.child("nav"), *icon, label))
                        .clicked()
                })
                .inner;
            start += row.slot;
        }
        let end = self.show_actions(ui, &row, start, &mut out);
        if let Some(query) = self.search.take() {
            self.show_search(ui, query, &row, Insets { start, end }, &mut out);
            return out;
        }
        self.paint_titles(ui, rect, &row, Insets { start, end }, collapse, &t);
        out
    }

    fn show_actions(
        &self,
        ui: &mut MineralUi<'_, Material>,
        row: &Row,
        start: f32,
        out: &mut TopAppBarResponse,
    ) -> f32 {
        let rt = ui.rt();
        let ctx = rt.ctx();
        let mut end = app_bar::TRAILING_SPACE;
        let cap = if WindowClass::of(rt.core()) == WindowClass::Compact {
            COMPACT_ACTIONS
        } else {
            MAX_ACTIONS
        };
        #[expect(clippy::cast_sign_loss, reason = ">= 0")]
        let fit = ((row.rect.width() - start - end) / row.slot).floor().max(0.0) as usize;
        let shown = inline_actions(self.actions.len(), fit.min(cap));
        if shown < self.actions.len() {
            let label = crate::tr(rt.i18n(), "mineral-app-bar-more");
            let anchor = egui_mineral::layout::child(
                ui,
                "overflow",
                row.slot(row.rect.width() - end - row.slot),
                *ui.layout(),
                |ui| {
                    ui.mineral(
                        IconButton::new(self.salt.child("overflow"), ui.icons().more_vert, &label)
                            .disabled(self.disabled),
                    )
                    .into_response()
                },
            )
            .inner;
            let mut menu = Menu::new(self.salt.child("overflow"), label).emit_anchor(false);
            for (idx, a) in self.actions.iter().enumerate().skip(shown) {
                let own =
                    crate::widget::item_salt(&self.salt, a.salt.as_ref(), a.command.as_ref(), idx).child("action");
                let mut item = MenuItem::new(&a.label)
                    .leading(a.icon)
                    .disabled(a.disabled)
                    .agent_key(StableId::of(&own, "button"));
                if let Some(c) = &a.command {
                    item = item.command(c);
                }
                if let Some(sel) = a.selected {
                    item = item.selected(sel);
                }
                menu = menu.item(item);
            }
            if let Some(k) = menu.show(rt, &anchor).clicked {
                out.clicked = Some(shown + k);
            }
            out.overflow = Some(anchor);
            end += row.slot;
        }
        for (idx, a) in self.actions.iter().enumerate().take(shown).rev() {
            let r = row.slot(row.rect.width() - end - row.slot);
            let resp = egui_mineral::layout::child(ui, ("action", idx), r, *ui.layout(), |ui| {
                let mut b = IconButton::new(
                    crate::widget::item_salt(&self.salt, a.salt.as_ref(), a.command.as_ref(), idx).child("action"),
                    a.icon,
                    &a.label,
                )
                .disabled(a.disabled);
                if let Some(sel) = a.selected {
                    b = b.selected(sel);
                }
                ui.mineral(b).into_response()
            })
            .inner;
            if resp.clicked() {
                match &a.command {
                    Some(c) => shortcut::fire(ctx, c),
                    None => out.clicked = Some(idx),
                }
            }
            out.actions.insert(idx, resp);
            end += row.slot;
        }
        end
    }

    fn show_search(
        &self,
        ui: &mut MineralUi<'_, Material>,
        query: &mut String,
        row: &Row,
        insets: Insets,
        out: &mut TopAppBarResponse,
    ) {
        use defaults::app_bar as d;
        let Insets { start, end } = insets;
        let Sides { left, right } = row.dir.sides(start, end);
        let (x0, x1) = (row.rect.left() + left, row.rect.right() - right);
        let pad = if self.navigation.is_some() {
            d::TITLE_HORIZONTAL_PADDING
        } else {
            d::TITLE_INSET + d::TITLE_HORIZONTAL_PADDING
        };
        let h = search_bar::CONTAINER_HEIGHT;
        let field = Rect::from_x_y_ranges(
            x0 + pad..=x1 - d::TITLE_HORIZONTAL_PADDING,
            row.rect.center().y - h / 2.0..=row.rect.center().y + h / 2.0,
        );
        let layout = *ui.layout();
        egui_mineral::layout::child(ui, self.salt.child("search"), field, layout, |ui| {
            let bar = crate::navigation::SearchBar::new(self.salt.child("search"), &self.title, query)
                .placeholder(&self.title);
            // Degenerate in very narrow bars; the bar is clipped there anyway.
            let r = ui.mineral(bar.preferred_width(field.width().max(0.0)));
            out.changed = r.changed();
            out.submitted = r.output().submitted;
        });
        if out.changed {
            out.response.mark_changed();
        }
    }

    fn paint_titles(&self, ui: &Ui, rect: Rect, row: &Row, insets: Insets, collapse: f32, t: &Tokens) {
        use defaults::app_bar as d;
        let Insets { start, end } = insets;
        let dir = row.dir;
        let small = matches!(self.variant, TopAppBarVariant::Small | TopAppBarVariant::CenterAligned);
        let centered = self.centered || self.variant == TopAppBarVariant::CenterAligned;
        // Compose TopAppBarLayout
        let place = |row: Rect, block: &TitleBlock, start: f32, end: f32| -> f32 {
            let w = block.width() + 2.0 * d::TITLE_HORIZONTAL_PADDING;
            let mut x = if centered { (row.width() - w) / 2.0 } else { 0.0 };
            if x < start {
                x = start;
            } else if x + w > row.width() - end {
                x = row.width() - end - w;
            }
            let from = x + d::TITLE_HORIZONTAL_PADDING;
            dir.start_x(row, Rangef::new(from, from + block.width()))
        };
        let paint = |block: TitleBlock, x: f32, top: f32, clip: Rect, alpha: f32| {
            if alpha <= 0.0 {
                return;
            }
            let mut p = ui.painter().with_clip_rect(clip.intersect(rect));
            p.multiply_opacity(alpha);
            let align = |g: &egui::Galley| {
                if centered {
                    x + (block.width() - g.size().x) / 2.0
                } else {
                    dir.span_x(
                        egui::Rangef::new(x, x + block.width()),
                        Rangef::new(0.0, 0.0 + g.size().x),
                    )
                }
            };
            let th = block.title.size().y;
            p.galley(
                pos2(align(&block.title), top),
                block.title.clone(),
                egui::Color32::PLACEHOLDER,
            );
            if let Some(sub) = &block.subtitle {
                p.galley(pos2(align(sub), top + th), sub.clone(), egui::Color32::PLACEHOLDER);
            }
        };
        let title_start = d::TITLE_INSET.max(start);
        let max_w = (row.rect.width() - title_start - end - 2.0 * d::TITLE_HORIZONTAL_PADDING).max(0.0);
        let top_fonts = TitleFonts {
            title: app_bar_small::TITLE_FONT,
            subtitle: app_bar_small::SUBTITLE_FONT,
        };
        let top = self.title_block(ui, if small { self.fonts() } else { top_fonts }, max_w, t);
        let top_alpha = if small {
            1.0
        } else {
            d::TOP_TITLE_ALPHA_EASING.ease(collapse)
        };
        let x = place(row.rect, &top, title_start, end);
        let y = row.rect.center().y - top.height() / 2.0;
        paint(top, x, y, row.rect, top_alpha);
        if !small {
            // The expanded row has no navigation or actions: its title box starts at the inset.
            let bottom = Rect::from_x_y_ranges(rect.x_range(), row.rect.bottom()..=rect.bottom());
            let max_w =
                (rect.width() - d::TITLE_INSET - 2.0 * d::TITLE_HORIZONTAL_PADDING - app_bar::TRAILING_SPACE).max(0.0);
            let block = self.title_block(ui, self.fonts(), max_w, t);
            let pad = if self.variant == TopAppBarVariant::Large {
                d::LARGE_TITLE_BOTTOM_PADDING
            } else {
                d::MEDIUM_TITLE_BOTTOM_PADDING
            };
            let x = place(bottom, &block, d::TITLE_INSET, app_bar::TRAILING_SPACE);
            let y = rect.bottom() - pad - block.last_baseline();
            paint(block, x, y, bottom, 1.0 - collapse);
        }
    }
}

#[derive(Clone, Copy)]
struct TitleFonts {
    title: TypeScale,
    subtitle: TypeScale,
}

#[derive(Clone, Copy)]
struct Insets {
    start: f32,
    end: f32,
}

struct Row {
    rect: Rect,
    dir: egui_mineral::i18n::Direction,
    slot: f32,
}

fn collapsed_height(density: Density) -> f32 {
    density.dense_to(app_bar_small::CONTAINER_HEIGHT, IconButton::slot(density))
}

impl Row {
    fn slot(&self, from_start: f32) -> Rect {
        crate::widget::parts::action_slot(self.dir, self.slot, self.rect, from_start)
    }
}

struct TitleBlock {
    title: Arc<egui::Galley>,
    subtitle: Option<Arc<egui::Galley>>,
}

impl TitleBlock {
    fn width(&self) -> f32 {
        self.subtitle
            .as_ref()
            .map_or(0.0, |s| s.size().x)
            .max(self.title.size().x)
    }

    fn height(&self) -> f32 {
        self.title.size().y + self.subtitle.as_ref().map_or(0.0, |s| s.size().y)
    }

    // Compose LastBaseline
    fn last_baseline(&self) -> f32 {
        match &self.subtitle {
            Some(s) => self.title.size().y + parts::last_baseline(s),
            None => parts::last_baseline(&self.title),
        }
    }
}
