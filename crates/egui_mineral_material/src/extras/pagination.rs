use crate::Material;
use crate::MaterialExt;
use crate::Rt;
use egui::{Color32, CornerRadius, Rect, Sense, Ui, Vec2, vec2};
use egui_mineral::i18n::I18n;
use egui_mineral::semantic::KeyedSalt;
use egui_mineral::widget::Availability;
use egui_mineral::widget::Composite;
use egui_mineral::widget::MineralUi;
use egui_mineral::{
    kit,
    layout::{Constraints, Measure, MinTarget},
    pagination::{PageItem, Spread},
    semantic::{self, Role, SemanticNode, StableId},
    widget::{MineralWidget, WidgetFrame, WidgetResponse},
};

use crate::selection::{Menu, MenuItem};
use crate::widget::{
    common::{self, Interaction},
    parts,
};
use crate::{
    Size,
    actions::{
        ButtonVariant, IconButton,
        button::{self, Metrics},
    },
};
use crate::{
    icons,
    spec::{
        ColorRole,
        tokens::{small_icon_button, standard_icon_button},
    },
    tokens::Tokens,
};

// MD3 has no pagination or breadcrumbs
const PAGE: Size = Size::Small;
const CRUMB: Size = Size::XSmall;
const CURRENT_CRUMB_COLOR: ColorRole = ColorRole::OnSurface;
const SEPARATOR_COLOR: ColorRole = ColorRole::OnSurfaceVariant;

fn min_interactive(rt: &Rt) -> f32 {
    crate::adaptive::MIN_INTERACTIVE.points(rt.core())
}

fn button_width(ui: &MineralUi<'_, Material>, m: Metrics, label: &str) -> f32 {
    let rt = ui.rt();
    let space = m.space.unwrap_or_default();
    let text = parts::type_galley(rt.core(), ui.painter(), label, m.font, Color32::PLACEHOLDER, None)
        .size()
        .x;
    (space.leading + text + space.trailing).max(m.height)
}

fn button_shape(m: Metrics, rect: Rect, t: &Tokens) -> CornerRadius {
    CornerRadius::from(button::radius(t, m.shapes.round.scale(), rect.height()))
}

fn paint_label(ui: &MineralUi<'_, Material>, rect: Rect, label: &str, m: Metrics, color: Color32) {
    let rt = ui.rt();
    let g = parts::type_galley(rt.core(), ui.painter(), label, m.font, color, None);
    ui.painter()
        .galley(rect.center() - g.size() / 2.0, g, Color32::PLACEHOLDER);
}

struct PageButton {
    salt: KeyedSalt,
    key: StableId,
    label: String,
    name: String,
    selected: bool,
    disabled: bool,
}

impl MineralWidget for PageButton {
    type Kit = Material;
    type State = ();
    type Output = ();

    egui_mineral::kit::widget_meta!(salt, enabled);

    fn min_target(&self) -> MinTarget {
        crate::adaptive::MIN_INTERACTIVE
    }

    fn layout(&self, ui: &MineralUi<'_, Material>, c: Constraints, (): &Self::State) -> Measure {
        let m = PAGE.metrics();
        c.constrain(vec2(button_width(ui, m, &self.label), m.height)).into()
    }

    fn update(&mut self, ui: &mut MineralUi<'_, Material>, frame: &WidgetFrame<'_>, (): &mut ()) -> WidgetResponse {
        let t = ui.tokens();
        let m = PAGE.metrics();
        let rect = frame.zones.visual;
        let press = parts::press(ui, frame, Availability::disabled_if(self.disabled));
        let changed = press.clicked && !self.selected;
        let variant = if self.selected {
            ButtonVariant::Tonal
        } else {
            ButtonVariant::Text
        };
        let style = button::style(variant, None, !self.disabled, &t);
        let i = press.interaction;
        button::Surface::of(rect, button_shape(m, rect, &t), style, m.outline).paint(
            ui,
            &frame.response,
            style.content,
            i,
            &t,
        );
        paint_label(ui, rect, &self.label, m, style.content);
        WidgetResponse::new(frame.response.clone(), ()).with_changed(changed)
    }

    fn semantics(&self, _: &I18n, _: &WidgetResponse, (): &()) -> Option<SemanticNode> {
        let mut n = SemanticNode::new(self.key.clone(), Role::Button, &self.name)
            .clickable(Availability::disabled_if(self.disabled));
        n.state.selected = Some(self.selected);
        Some(n)
    }
}

/// Page selector with siblings, boundaries and gaps; non-MD3 (MUI `Pagination` pattern).
#[derive(Debug)]
#[must_use = "show with ui.mineral(..)"]
pub struct Pagination<'a> {
    salt: KeyedSalt,
    label: Option<String>,
    page: &'a mut usize,
    total: usize,
    siblings: usize,
    boundaries: usize,
    first_last: bool,
    disabled: bool,
}

impl<'a> Pagination<'a> {
    /// `total` pages (0 shows one); `page` is the bound zero-based index, moved onto the last page
    /// when past it. Agent key prefix `<id_salt>`.
    pub fn new(id_salt: impl Into<KeyedSalt>, total: usize, page: &'a mut usize) -> Self {
        let id_salt: KeyedSalt = id_salt.into();
        Self {
            salt: id_salt,
            label: None,
            page,
            total,
            siblings: 1,
            boundaries: 1,
            first_last: false,
            disabled: false,
        }
    }

    crate::widget::setters! {
        /// Replaces the `<id_salt>` key prefix.
        agent_key(salt)
    }

    /// Accessible name of the landmark; default `mineral-pagination`.
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Pages shown on each side of the current one; default 1.
    pub fn siblings(mut self, siblings: usize) -> Self {
        self.siblings = siblings;
        self
    }

    /// Pages always shown at each end; default 1.
    pub fn boundaries(mut self, boundaries: usize) -> Self {
        self.boundaries = boundaries;
        self
    }

    /// First and last page buttons.
    pub fn first_last(mut self, first_last: bool) -> Self {
        self.first_last = first_last;
        self
    }

    crate::widget::setters! {
        /// Greys out every button.
        disabled
    }
}

impl Composite for Pagination<'_> {
    type Kit = Material;
    type Output = ();

    egui_mineral::kit::widget_meta!(salt);

    fn compose(&mut self, ui: &mut MineralUi<'_, Material>, _: Constraints) -> WidgetResponse {
        let rt = ui.rt();
        let ctx = rt.ctx();
        let t = rt.tokens();
        let fmt = rt.i18n().format();
        let dir = rt.i18n().direction();
        let id = ui.make_persistent_id(self.salt.id_salt());
        let total = self.total.max(1);
        let cur = (*self.page).min(total - 1) + 1;
        let mut target = cur;
        semantic::emit_parent(ctx, id, || {
            let name = crate::tr_or(rt.i18n(), self.label.as_deref(), "mineral-pagination");
            let mut n = SemanticNode::new(StableId::of(&self.salt, "pagination"), Role::Navigation, name);
            n.state.disabled = self.disabled;
            n
        });
        let back = dir.pick(icons::ICON_CHEVRON_LEFT, icons::ICON_CHEVRON_RIGHT);
        let fwd = dir.pick(icons::ICON_CHEVRON_RIGHT, icons::ICON_CHEVRON_LEFT);
        let first = dir.pick(icons::ICON_FIRST_PAGE, icons::ICON_LAST_PAGE);
        let last = dir.pick(icons::ICON_LAST_PAGE, icons::ICON_FIRST_PAGE);
        let nav = |ui: &mut Ui, salt: &'static str, icon, msg: &str, enabled: bool| {
            rt.ui(ui)
                .mineral(
                    IconButton::new(self.salt.child(salt), icon, crate::tr(rt.i18n(), msg))
                        .disabled(self.disabled || !enabled),
                )
                .clicked()
        };
        let r = parts::row(ui, dir, min_interactive(rt), |ui| {
            ui.spacing_mut().item_spacing.x = crate::adaptive::grid(1);
            if self.first_last && nav(ui, "first", first, "mineral-pagination-first", cur > 1) {
                target = 1;
            }
            if nav(ui, "prev", back, "mineral-pagination-prev", cur > 1) {
                target = cur - 1;
            }
            let spread = Spread {
                siblings: self.siblings,
                boundaries: self.boundaries,
            };
            for item in spread.items(cur, total) {
                match item {
                    PageItem::Page(n) => {
                        let label = fmt.count(n);
                        let name = rt.i18n().tr("mineral-pagination-page", &[("page", (&label).into())]);
                        let resp = rt.ui(ui).mineral(PageButton {
                            salt: self.salt.child(("page", n)),
                            key: StableId::of(&self.salt, (n, "page-button")),
                            label,
                            name,
                            selected: n == cur,
                            disabled: self.disabled,
                        });
                        if resp.changed() {
                            target = n;
                        }
                    }
                    PageItem::Gap => {
                        let (r, _) = ui.allocate_exact_size(Vec2::splat(PAGE.metrics().height), Sense::hover());
                        common::icon(
                            ui.painter(),
                            r.center(),
                            icons::ICON_MORE_HORIZ,
                            small_icon_button::ICON_SIZE,
                            t.role(standard_icon_button::COLOR),
                        );
                    }
                }
            }
            if nav(ui, "next", fwd, "mineral-pagination-next", cur < total) {
                target = cur + 1;
            }
            if self.first_last && nav(ui, "last", last, "mineral-pagination-last", cur < total) {
                target = total;
            }
        });
        kit::semantic::remember_rect(ctx, id, r.response.rect);
        let changed = target - 1 != *self.page;
        *self.page = target - 1;
        let response = r.response;
        WidgetResponse::new(response, ()).with_changed(changed)
    }
}

struct Crumb<'a> {
    salt: KeyedSalt,
    key: StableId,
    label: &'a str,
    current: Option<&'a str>,
    disabled: bool,
}

fn crumb_width(ui: &MineralUi<'_, Material>, label: &str) -> f32 {
    button_width(ui, CRUMB.metrics(), label)
}

impl MineralWidget for Crumb<'_> {
    type Kit = Material;
    type State = ();
    type Output = ();

    egui_mineral::kit::widget_meta!(salt);

    fn enabled(&self) -> bool {
        self.current.is_none() && !self.disabled
    }

    fn min_target(&self) -> MinTarget {
        if self.current.is_some() {
            MinTarget::None
        } else {
            crate::adaptive::MIN_INTERACTIVE
        }
    }

    fn layout(&self, ui: &MineralUi<'_, Material>, c: Constraints, (): &Self::State) -> Measure {
        c.constrain(vec2(crumb_width(ui, self.label), CRUMB.metrics().height))
            .into()
    }

    fn update(&mut self, ui: &mut MineralUi<'_, Material>, frame: &WidgetFrame<'_>, (): &mut ()) -> WidgetResponse {
        let t = ui.tokens();
        let m = CRUMB.metrics();
        let r = &frame.response;
        let rect = frame.zones.visual;
        let color = if self.current.is_some() {
            let c = t.role(CURRENT_CRUMB_COLOR);
            if self.disabled { common::disabled_content(&t) } else { c }
        } else {
            kit::input::hover_cursor(ui.core(), r, Availability::disabled_if(self.disabled));
            let style = button::style(ButtonVariant::Text, None, !self.disabled, &t);
            let i = Interaction::of(r, Availability::disabled_if(self.disabled));
            button::Surface::of(rect, button_shape(m, rect, &t), style, m.outline).paint(
                ui,
                &frame.response,
                style.content,
                i,
                &t,
            );
            style.content
        };
        paint_label(ui, rect, self.label, m, color);
        WidgetResponse::new(r.clone(), ())
    }

    fn semantics(&self, _: &I18n, _: &WidgetResponse, (): &()) -> Option<SemanticNode> {
        let mut n = SemanticNode::new(self.key.clone(), Role::Link, self.label);
        if let Some(v) = self.current {
            n.value = Some(v.to_owned());
            n.state.selected = Some(true);
        } else {
            n = n.clickable(Availability::disabled_if(self.disabled));
        }
        Some(n)
    }
}

/// Result of [`Breadcrumbs::show`].
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct BreadcrumbsResponse {
    /// Trail response.
    pub response: egui::Response,
    /// Index of the item clicked this frame.
    pub clicked: Option<usize>,
}

/// Path trail collapsing past `max_items`; non-MD3 (MUI `Breadcrumbs` pattern).
#[derive(Clone, Debug)]
#[must_use = "call .show(..)"]
pub struct Breadcrumbs {
    salt: KeyedSalt,
    label: Option<String>,
    items: Vec<String>,
    max_items: usize,
    disabled: bool,
}

impl Breadcrumbs {
    /// Collapses past 8 items; agent key prefix `<id_salt>`.
    pub fn new(id_salt: impl Into<KeyedSalt>) -> Self {
        let id_salt: KeyedSalt = id_salt.into();
        Self {
            salt: id_salt,
            label: None,
            items: Vec::new(),
            max_items: 8,
            disabled: false,
        }
    }

    crate::widget::setters! {
        /// Replaces the `<id_salt>` key prefix.
        agent_key(salt)
    }

    /// Accessible name of the landmark; default `mineral-breadcrumbs`.
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Appends one path item; the last is marked current.
    pub fn item(mut self, label: impl Into<String>) -> Self {
        self.items.push(label.into());
        self
    }

    /// Appends path items; see [`Self::item`].
    pub fn items(mut self, labels: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.items.extend(labels.into_iter().map(Into::into));
        self
    }

    /// Items shown before the middle collapses into an ellipsis; default 8, minimum 2.
    #[track_caller]
    pub fn max_items(mut self, max_items: usize) -> Self {
        if max_items < 2 {
            egui_mineral::kit::sanitize::report(format_args!("Breadcrumbs max_items must be >= 2, got {max_items}"));
        }
        self.max_items = max_items.max(2);
        self
    }

    crate::widget::setters! {
        /// Greys out every item.
        disabled
    }

    fn collapse(&self, ui: &MineralUi<'_, Material>) -> Option<(usize, usize)> {
        let rt = ui.rt();
        let n = self.items.len();
        let widths: Vec<f32> = self.items.iter().map(|l| crumb_width(ui, l)).collect();
        let avail = ui.available_width();
        let sep_w = CRUMB.metrics().icon;
        // The overflow icon button reserves its minimum interactive size.
        let more_w = min_interactive(rt);
        let row = |idx: &mut dyn Iterator<Item = usize>, more: bool| {
            let (mut w, mut k) = (0.0, 0);
            for i in idx {
                w += widths[i];
                k += 1;
            }
            w + sep_w * (k + usize::from(more)).saturating_sub(1) as f32 + if more { more_w } else { 0.0 }
        };
        if n <= self.max_items && row(&mut (0..n), false) <= avail {
            return None;
        }
        let mut after = n.saturating_sub(1).min(self.max_items.saturating_sub(2)).max(1);
        while after > 1 && row(&mut (0..1).chain(n - after..n), true) > avail {
            after -= 1;
        }
        // Needs at least one crumb between the first and the tail.
        (after + 1 < n).then_some((1, after))
    }

    /// Lays out the trail on one row.
    pub fn show(self, ui: &mut MineralUi<'_, Material>) -> BreadcrumbsResponse {
        let rt = ui.rt();
        let ctx = rt.ctx();
        let t = rt.tokens();
        let dir = rt.i18n().direction();
        let id = ui.make_persistent_id(self.salt.id_salt());
        semantic::emit_parent(ctx, id, || {
            let name = crate::tr_or(rt.i18n(), self.label.as_deref(), "mineral-breadcrumbs");
            let mut n = SemanticNode::new(StableId::of(&self.salt, "breadcrumbs"), Role::Navigation, name);
            n.state.disabled = self.disabled;
            n
        });
        let n = self.items.len();
        let collapse = self.collapse(ui);
        let sep = dir.pick(icons::ICON_CHEVRON_RIGHT, icons::ICON_CHEVRON_LEFT);
        let mut out = None;
        let sep_size = CRUMB.metrics().icon;
        let r = parts::row(ui, dir, min_interactive(rt), |ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
            let mut first = true;
            let mut separator = |ui: &mut Ui| {
                if !first {
                    let (r, _) = ui.allocate_exact_size(Vec2::splat(sep_size), Sense::hover());
                    common::icon(ui.painter(), r.center(), sep, sep_size, t.role(SEPARATOR_COLOR));
                }
                first = false;
            };
            let current_name = crate::tr(rt.i18n(), "mineral-breadcrumbs-current");
            let crumb = |ui: &mut Ui, i: usize| {
                rt.ui(ui)
                    .mineral(Crumb {
                        salt: self.salt.child(("crumb", i)),
                        key: StableId::of(&self.salt, (i, "link")),
                        label: &self.items[i],
                        current: (i + 1 == n).then_some(current_name.as_str()),
                        disabled: self.disabled,
                    })
                    .into_response()
                    .clicked()
                    .then_some(i)
            };
            match collapse {
                None => {
                    for i in 0..n {
                        separator(ui);
                        out = crumb(ui, i).or(out);
                    }
                }
                Some((before, after)) => {
                    for i in 0..before {
                        separator(ui);
                        out = crumb(ui, i).or(out);
                    }
                    separator(ui);
                    let hidden = before..n - after;
                    let label = crate::tr(rt.i18n(), "mineral-breadcrumbs-more");
                    let anchor = rt.ui(ui).mineral(
                        IconButton::new(self.salt.child("more-button"), icons::ICON_MORE_HORIZ, &label)
                            .disabled(self.disabled)
                            .agent_key(StableId::of(&self.salt, "more-button")),
                    );
                    let mut menu = Menu::new(self.salt.child("more"), label).emit_anchor(false);
                    for i in hidden.clone() {
                        menu = menu.item(MenuItem::new(&self.items[i]));
                    }
                    if let Some(k) = menu.show(rt, anchor.response()).clicked {
                        out = Some(hidden.start + k);
                    }
                    for i in n - after..n {
                        separator(ui);
                        out = crumb(ui, i).or(out);
                    }
                }
            }
        });
        kit::semantic::remember_rect(ctx, id, r.response.rect);
        BreadcrumbsResponse {
            response: r.response,
            clicked: out,
        }
    }
}
