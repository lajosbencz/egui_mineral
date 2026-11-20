// FloatingActionButtonMenu.kt
use crate::Material;
use crate::MaterialExt;
use crate::Rt;
use crate::actions::button::{Elevation, Surface};
use egui_mineral::i18n::I18n;
use egui_mineral::kit::state::TempState;
use egui_mineral::overlay::Dismiss;
use egui_mineral::select::ItemKey;
use egui_mineral::semantic::KeyedSalt;
use egui_mineral::widget::MineralUi;

use egui::{
    Align, Align2, Color32, CornerRadius, Id, Layout, Order, Rangef, Rect, Response, epaint::MarginF32, pos2, vec2,
};
use egui_mineral::widget::Availability;
use egui_mineral::{
    focus::RovingFocus,
    i18n::Direction,
    layout::{Axis, Constraints, Measure},
    overlay,
    semantic::{self, Role, SemanticNode, StableId},
    shortcut::{self, ActionId},
    widget::{MineralWidget, WidgetFrame, WidgetResponse},
};

use crate::actions::fab::{self, FabColors, FabSize};
use crate::widget::{
    common::{self},
    indication, parts,
};
use crate::{
    Icon, motion,
    spec::{
        ColorRole,
        defaults::fab_menu as cx,
        tokens::{fab_menu_baseline as m, fab_primary_container},
    },
    tokens::Tokens,
};

/// Compose `FloatingActionButtonMenu` sizes (the toggle FAB has no small size).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
pub enum FabMenuSize {
    #[default]
    /// `FloatingActionButton` (`FabBaselineTokens`).
    Baseline,
    /// `MediumFloatingActionButton` (`FabMediumTokens`).
    Medium,
    /// `LargeFloatingActionButton` (`FabLargeTokens`).
    Large,
}

impl From<FabMenuSize> for FabSize {
    fn from(size: FabMenuSize) -> Self {
        match size {
            FabMenuSize::Baseline => Self::Baseline,
            FabMenuSize::Medium => Self::Medium,
            FabMenuSize::Large => Self::Large,
        }
    }
}

/// M3 FAB menu color sets (no surface set).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
pub enum FabMenuColors {
    #[default]
    /// `primaryContainer` toggle, `primary` when open.
    PrimaryContainer,
    /// `secondaryContainer` toggle, `secondary` when open.
    SecondaryContainer,
    /// `tertiaryContainer` toggle, `tertiary` when open.
    TertiaryContainer,
}

impl From<FabMenuColors> for FabColors {
    fn from(colors: FabMenuColors) -> Self {
        match colors {
            FabMenuColors::PrimaryContainer => Self::PrimaryContainer,
            FabMenuColors::SecondaryContainer => Self::SecondaryContainer,
            FabMenuColors::TertiaryContainer => Self::TertiaryContainer,
        }
    }
}

fn checked_roles(color: FabMenuColors) -> (ColorRole, ColorRole) {
    match color {
        FabMenuColors::PrimaryContainer => (ColorRole::Primary, ColorRole::OnPrimary),
        FabMenuColors::SecondaryContainer => (ColorRole::Secondary, ColorRole::OnSecondary),
        FabMenuColors::TertiaryContainer => (ColorRole::Tertiary, ColorRole::OnTertiary),
    }
}

/// Item of a [`FabMenu`] (`FloatingActionButtonMenuItem`); click-only, the menu pairs it with its value.
#[derive(Clone, Debug)]
#[must_use]
pub struct FabMenuItem {
    icon: Icon,
    label: String,
    command: Option<ActionId>,
    disabled: bool,
    salt: Option<egui_mineral::semantic::KeyedSalt>,
}

impl FabMenuItem {
    crate::widget::setters! {
        /// Default the [`Self::command`] action, else the item's value.
        item_salt
    }

    /// `label` is the visible text and accessible name.
    pub fn new(icon: impl Into<Icon>, label: impl Into<String>) -> Self {
        Self {
            icon: icon.into(),
            label: label.into(),
            command: None,
            disabled: false,
            salt: None,
        }
    }

    /// Activation fires the app action `id` ([`shortcut::fire`]) instead of reporting the item as clicked.
    pub fn command(mut self, id: impl Into<ActionId>) -> Self {
        self.command = Some(id.into());
        self
    }

    crate::widget::setters! {
        /// Greys the item out and ignores activation.
        disabled
    }
}

fn stagger(rt: &crate::Rt, id: Id, target: usize) -> usize {
    let goal = target as f32;
    let v = egui_mineral::motion::spring(rt.core(), id, goal, motion::SLOW_EFFECTS);
    if (v - goal).abs() < cx::STAGGER_VISIBILITY_THRESHOLD {
        target
    } else {
        #[expect(clippy::cast_sign_loss, reason = "critically damped between 0 and the item count")]
        let n = v.max(0.0) as usize;
        n
    }
}

/// Result of [`FabMenu::show`].
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct FabMenuResponse<A> {
    /// Toggle FAB response.
    pub response: Response,
    /// Value of the item without a [`FabMenuItem::command`] clicked this frame; clicking closes the menu.
    pub clicked: Option<A>,
    /// Menu state after this frame.
    pub open: bool,
    /// Why the menu closed this frame without an item click.
    pub dismissed: Option<Dismiss>,
}

/// MD3 FAB menu: a FAB that expands into a list of actions (Compose `FloatingActionButtonMenu`).
#[derive(Clone, Debug)]
#[must_use = "call .show(..)"]
pub struct FabMenu<A> {
    salt: KeyedSalt,
    icon: Icon,
    label: String,
    items: Vec<(A, FabMenuItem)>,
    size: FabMenuSize,
    color: FabMenuColors,
    disabled: bool,
}

impl<A: ItemKey> FabMenu<A> {
    /// `icon` shows while closed, `label` is the toggle's accessible name; agent key prefix `<id_salt>`. `A` is the
    /// item values' type.
    pub fn new(id_salt: impl Into<KeyedSalt>, icon: impl Into<Icon>, label: impl Into<String>) -> Self {
        let id_salt: KeyedSalt = id_salt.into();
        Self {
            salt: id_salt,
            icon: icon.into(),
            label: label.into(),
            items: Vec::new(),
            size: FabMenuSize::default(),
            color: FabMenuColors::default(),
            disabled: false,
        }
    }

    crate::widget::setters! {
        /// Replaces the `<id_salt>` key prefix of the toggle and items.
        agent_key(salt)
    }

    /// Appends the item reporting `value`.
    pub fn item(mut self, value: A, item: impl Into<FabMenuItem>) -> Self {
        self.items.push((value, item.into()));
        self
    }

    /// Appends `(value, item)` pairs.
    pub fn items(mut self, items: impl IntoIterator<Item = (A, impl Into<FabMenuItem>)>) -> Self {
        self.items.extend(items.into_iter().map(|(v, i)| (v, i.into())));
        self
    }

    /// Default [`FabMenuSize::Baseline`].
    pub fn size(mut self, size: FabMenuSize) -> Self {
        self.size = size;
        self
    }

    /// Default [`FabMenuColors::PrimaryContainer`].
    pub fn colors(mut self, colors: FabMenuColors) -> Self {
        self.color = colors;
        self
    }

    crate::widget::setters! {
        /// Greys out the toggle; the menu cannot open.
        disabled
    }

    /// Paints the toggle and, while open, the item list above it.
    pub fn show(self, ui: &mut MineralUi<'_, Material>) -> FabMenuResponse<A> {
        let rt = ui.rt();
        let ctx = rt.ctx();
        let base = ui.make_persistent_id(self.salt.id_salt());
        let open_key = base.with("open");
        let was_open = bool::load(ctx, open_key) && !self.disabled;
        let toggle = Toggle {
            salt: self.salt.child("fab"),
            key: StableId::of(&self.salt, "fab"),
            label: self.label.clone(),
            icon: self.icon,
            size: self.size.into(),
            color: self.color,
            progress: egui_mineral::motion::spring(
                rt.core(),
                base.with("checked"),
                f32::from(was_open),
                motion::FAST_SPATIAL,
            ),
            open: was_open,
            disabled: self.disabled,
        };
        let fab = rt.ui(ui).mineral(toggle);
        let fab_rect = fab.zones().visual;
        let mut open = was_open ^ fab.clicked();
        let n = self.items.len();
        let visible = stagger(rt, base.with("stagger"), if open { n } else { 0 });
        let reveal: Vec<(f32, f32)> = (0..n)
            .map(|i| {
                let on = i + visible >= n;
                let id = base.with(("item", i));
                (
                    egui_mineral::motion::spring(rt.core(), id.with("width"), f32::from(on), motion::FAST_SPATIAL),
                    egui_mineral::motion::spring(rt.core(), id.with("alpha"), f32::from(on), motion::FAST_EFFECTS),
                )
            })
            .collect();
        let mut clicked = None;
        let mut dismissed = None;
        let mut menu_rect = Rect::NOTHING;
        if open || reveal.iter().any(|&(_, alpha)| alpha > 0.0) {
            let (rect, ids, hit) = self.items_area(rt, base, fab_rect, &reveal, open);
            menu_rect = rect;
            if let Some(i) = hit {
                match &self.items[i].1.command {
                    Some(c) => shortcut::fire(ctx, c),
                    None => clicked = Some(i),
                }
            }
            if open
                && !was_open
                && let Some(first) = ids.first()
            {
                ctx.memory_mut(|m| m.request_focus(*first));
            }
            if open {
                RovingFocus::new(Axis::Vertical).update(rt.core(), &ids, |i| !self.items[i].1.disabled);
            }
            if open {
                let layer = egui::LayerId::new(Order::Foreground, items_id(base));
                dismissed = overlay::dismissal(rt.core(), layer, &[menu_rect, fab.rect], Some(fab.response().id));
            }
            if open && (hit.is_some() || dismissed.is_some()) {
                open = false;
                if hit.is_some() {
                    fab.response().request_focus();
                }
            }
        }
        if open != was_open {
            open.store(ctx, open_key);
        }
        self.announce(ctx, base, menu_rect, open);
        FabMenuResponse {
            response: fab.into_response(),
            clicked: clicked.and_then(|i| self.items.get(i)).map(|(v, _)| v.clone()),
            open,
            dismissed,
        }
    }

    fn items_area(
        &self,
        rt: &Rt,
        base: Id,
        fab: Rect,
        reveal: &[(f32, f32)],
        open: bool,
    ) -> (Rect, Vec<Id>, Option<usize>) {
        let ctx = rt.ctx();
        let mut clicked = None;
        let dir = rt.i18n().direction();
        let bottom = fab.top() - m::CLOSE_BUTTON_BETWEEN_SPACE;
        let (anchor, pivot, align) = (
            pos2(dir.end_edge(fab), bottom),
            Align2([dir.end_align(), Align::Max]),
            dir.end_align(),
        );
        let area = egui_mineral::kit::layer::area(rt.core(), items_id(base))
            .order(Order::Foreground)
            .fixed_pos(anchor)
            .pivot(pivot)
            .constrain(true)
            .show(ctx, |ui| {
                ui.spacing_mut().item_spacing.y = m::LIST_ITEM_BETWEEN_SPACE;
                ui.with_layout(Layout::top_down(align), |ui| {
                    let mut ids = Vec::with_capacity(self.items.len());
                    for (i, ((value, item), &(width, alpha))) in self.items.iter().zip(reveal).enumerate() {
                        let own = crate::widget::action_salt(&self.salt, item.salt.as_ref(), item.command.as_ref(), value);
                        let r = rt.ui(ui).mineral(Row {
                            key: StableId::of(&own, "fab-menu-item"),
                            salt: own,
                            item,
                            color: self.color,
                            width,
                            alpha,
                            dir,
                            active: open,
                        });
                        if r.clicked() {
                            clicked = Some(i);
                        }
                        ids.push(r.response().id);
                    }
                    ids
                })
                .inner
            });
        (area.response.rect, area.inner, clicked)
    }

    fn announce(&self, ctx: &egui::Context, base: Id, rect: Rect, open: bool) {
        if open {
            semantic::emit(ctx, Some(items_id(base)), rect, || {
                SemanticNode::new(StableId::of(&self.salt, "fab-menu"), Role::Menu, &self.label)
            });
        }
    }
}

// Compose ToggleFloatingActionButton, animateIcon
struct Toggle {
    salt: KeyedSalt,
    key: StableId,
    label: String,
    icon: Icon,
    size: FabSize,
    color: FabMenuColors,
    progress: f32,
    open: bool,
    disabled: bool,
}

impl MineralWidget for Toggle {
    type Kit = Material;
    type State = ();
    type Output = ();

    egui_mineral::kit::widget_meta!(salt, enabled);

    fn layout(&self, _ui: &MineralUi<'_, Material>, c: Constraints, (): &Self::State) -> Measure {
        let side = self.size.side();
        Measure::new(c.constrain(vec2(side, side))).margin(MarginF32 {
            left: cx::PADDING_HORIZONTAL,
            right: cx::PADDING_HORIZONTAL,
            top: 0.0,
            bottom: cx::BUTTON_PADDING_BOTTOM,
        })
    }

    fn update(&mut self, ui: &mut MineralUi<'_, Material>, frame: &WidgetFrame<'_>, (): &mut ()) -> WidgetResponse {
        let t = ui.tokens();
        let k = self.progress;
        let full = frame.zones.visual;
        let side = egui::lerp(self.size.side()..=m::CLOSE_BUTTON_CONTAINER_HEIGHT, k);
        // `contentAlignment = Alignment.TopEnd`.
        let x = ui
            .i18n()
            .direction()
            .reversed()
            .start_x(full, Rangef::new(0.0, 0.0 + side));
        let rect = Rect::from_min_size(pos2(x, full.top()), vec2(side, side));
        let (bg, fg) = self.colors(&t, k);
        let corner = egui::lerp(
            f32::from(self.size.corner(&t))..=m::CLOSE_BUTTON_CONTAINER_HEIGHT / 2.0,
            k,
        );
        let r = CornerRadius::from(corner);
        let i = parts::press(ui, frame, Availability::disabled_if(self.disabled)).interaction;
        let surface = Surface {
            rect,
            radius: r,
            fill: bg,
            elevation: if self.disabled {
                Elevation::NONE
            } else {
                Elevation::flat(fab_primary_container::CONTAINER_ELEVATION)
            },
            outline: None,
        };
        surface.paint(ui, &frame.response, fg, i, &t);
        let icon = if k > 0.5 { ui.icons().close } else { self.icon };
        let icon_size = egui::lerp(self.size.icon()..=m::CLOSE_BUTTON_ICON_SIZE, k);
        common::icon(ui.painter(), rect.center(), icon, icon_size, fg);
        WidgetResponse::new(frame.response.clone(), ())
    }

    fn semantics(&self, _: &I18n, _: &WidgetResponse, (): &()) -> Option<SemanticNode> {
        let mut n = SemanticNode::new(self.key.clone(), Role::Button, &self.label)
            .clickable(Availability::disabled_if(self.disabled));
        n.state.expanded = Some(self.open);
        Some(n)
    }
}

impl Toggle {
    fn colors(&self, t: &Tokens, k: f32) -> (Color32, Color32) {
        if self.disabled {
            return (common::disabled_container(t), common::disabled_content(t));
        }
        let (bg0, fg0) = FabColors::from(self.color).colors(t);
        let (bg1, fg1) = checked_roles(self.color);
        (bg0.lerp_to_gamma(t.role(bg1), k), fg0.lerp_to_gamma(t.role(fg1), k))
    }
}

struct Row<'a> {
    salt: KeyedSalt,
    key: StableId,
    item: &'a FabMenuItem,
    color: FabMenuColors,
    width: f32,
    alpha: f32,
    dir: Direction,
    active: bool,
}

impl Row<'_> {
    fn content_width(&self, ui: &MineralUi<'_, Material>) -> f32 {
        let rt = ui.rt();
        let text = parts::type_galley(
            rt.core(),
            ui.painter(),
            &self.item.label,
            cx::ITEM_LABEL_FONT,
            Color32::PLACEHOLDER,
            None,
        );
        (m::LIST_ITEM_LEADING_SPACE
            + m::LIST_ITEM_ICON_SIZE
            + m::LIST_ITEM_ICON_LABEL_SPACE
            + text.size().x
            + m::LIST_ITEM_TRAILING_SPACE)
            .max(m::LIST_ITEM_CONTAINER_HEIGHT)
    }
}

impl MineralWidget for Row<'_> {
    type Kit = Material;
    type State = ();
    type Output = ();

    egui_mineral::kit::widget_meta!(salt);

    fn enabled(&self) -> bool {
        self.active && self.alpha > 0.0 && !self.item.disabled
    }

    fn layout(&self, ui: &MineralUi<'_, Material>, c: Constraints, (): &Self::State) -> Measure {
        let w = self.content_width(ui) * self.width.max(0.0);
        c.constrain(vec2(w, m::LIST_ITEM_CONTAINER_HEIGHT)).into()
    }

    fn update(&mut self, ui: &mut MineralUi<'_, Material>, frame: &WidgetFrame<'_>, (): &mut ()) -> WidgetResponse {
        let rt = ui.rt();
        let r = &frame.response;
        let disabled = !self.enabled();
        let press = parts::press(ui, frame, Availability::disabled_if(disabled));
        let rect = frame.zones.visual;
        if self.alpha <= 0.0 || rect.width() <= 0.0 {
            return WidgetResponse::new(r.clone(), ());
        }
        let t = ui.tokens();
        ui.multiply_opacity(self.alpha.min(1.0));
        let (bg, fg) = fab::fab_colors(self.color.into(), self.item.disabled, &t);
        let cr = m::LIST_ITEM_CONTAINER_SHAPE.corner_radius(&t.shape, ui.i18n().direction());
        let i = press.interaction;
        ui.painter().rect_filled(rect, cr, bg);
        indication::paint(ui, r, rect, cr, fg, i, &t);
        // The row keeps its full width, placed at the end edge and clipped by the container.
        let full = self.content_width(ui);
        let p = ui.painter().with_clip_rect(rect.intersect(ui.clip_rect()));
        let x0 = self.dir.reversed().start_x(rect, Rangef::new(0.0, 0.0 + full));
        let block = Rect::from_min_size(pos2(x0, rect.top()), vec2(full, rect.height()));
        let text = parts::type_galley(rt.core(), &p, &self.item.label, cx::ITEM_LABEL_FONT, fg, None);
        let mid = rect.center().y;
        let inner = full - m::LIST_ITEM_LEADING_SPACE - m::LIST_ITEM_TRAILING_SPACE;
        let used = m::LIST_ITEM_ICON_SIZE + m::LIST_ITEM_ICON_LABEL_SPACE + text.size().x;
        // `Arrangement.spacedBy(.., CenterHorizontally)` within the minimum width.
        let lead = m::LIST_ITEM_LEADING_SPACE + (inner - used).max(0.0) / 2.0;
        let icon_x = self
            .dir
            .start_x(block, Rangef::point(lead + m::LIST_ITEM_ICON_SIZE / 2.0));
        let text_from = lead + m::LIST_ITEM_ICON_SIZE + m::LIST_ITEM_ICON_LABEL_SPACE;
        let text_x = self
            .dir
            .start_x(block, Rangef::new(text_from, text_from + text.size().x));
        common::icon(&p, pos2(icon_x, mid), self.item.icon, m::LIST_ITEM_ICON_SIZE, fg);
        p.galley(pos2(text_x, mid - text.size().y / 2.0), text, fg);
        WidgetResponse::new(r.clone(), ())
    }

    fn semantics(&self, _: &I18n, _: &WidgetResponse, (): &()) -> Option<SemanticNode> {
        Some(
            SemanticNode::new(self.key.clone(), Role::MenuItem, &self.item.label)
                .clickable(Availability::enabled_if(self.active && !self.item.disabled)),
        )
    }
}

fn items_id(base: egui::Id) -> egui::Id {
    base.with("items")
}
