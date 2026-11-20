use egui::{Frame, Sense, vec2};
use egui_mineral::{
    action::Clicked,
    focus::{self as roving, RovingFocus},
    i18n::Direction,
    layout::Axis,
    semantic::{self, AgentKey, KeyedSalt, Role, SemanticNode},
    shortcut::ActionId,
    widget::MineralUi,
};

use super::{
    Menu, MenuItem, MenuUi,
    overflow::{self, Overflow, OverflowItem, OverflowSpace},
};
use crate::{
    ColorToken::ColorNeutralForeground1,
    Fluent, FluentExt, Icon, Rt, Size,
    actions::{ButtonAppearance, PushButton, button::Metrics},
    spec::{defaults::breadcrumb as d, tokens::geometry as g},
    typography::TypeRamp,
    widget::paint,
};

/// Fluent `BreadcrumbButton`: the clickable part of a [`BreadcrumbItem`].
#[derive(Clone, Debug)]
#[must_use]
pub struct BreadcrumbButton {
    label: String,
    icon: Option<Icon>,
    current: bool,
    disabled: bool,
    action_key: Option<ActionId>,
}

impl BreadcrumbButton {
    /// Visible text and accessible name.
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            icon: None,
            current: false,
            disabled: false,
            action_key: None,
        }
    }

    /// `icon` slot before the label.
    pub fn icon(mut self, icon: impl Into<Icon>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    /// Page the trail ends at: strong text, no hover or press colors, reported as disabled (`aria-current`
    /// has no semantic state), clicks ignored.
    pub fn current(mut self, current: bool) -> Self {
        self.current = current;
        self
    }

    crate::widget::setters! {
        action_key
    }

    crate::widget::setters! {
        /// Greys out the button and ignores input.
        disabled
    }
}

impl From<&str> for BreadcrumbButton {
    fn from(s: &str) -> Self {
        Self::new(s)
    }
}

impl From<String> for BreadcrumbButton {
    fn from(s: String) -> Self {
        Self::new(s)
    }
}

/// Fluent `BreadcrumbItem`: one step of a [`Breadcrumb`].
#[derive(Clone, Debug)]
#[must_use]
pub struct BreadcrumbItem {
    button: BreadcrumbButton,
    salt: Option<KeyedSalt>,
}

impl BreadcrumbItem {
    /// Wraps `button`.
    pub fn new(button: impl Into<BreadcrumbButton>) -> Self {
        Self {
            button: button.into(),
            salt: None,
        }
    }

    crate::widget::setters! {
        item_salt
    }
}

impl<T: Into<BreadcrumbButton>> From<T> for BreadcrumbItem {
    fn from(button: T) -> Self {
        Self::new(button)
    }
}

/// Fluent `BreadcrumbDivider`: the glyph between items, a chevron that mirrors in right-to-left layouts.
#[derive(Clone, Copy, Debug, Default)]
#[must_use]
pub struct BreadcrumbDivider {
    icon: Option<Icon>,
}

impl BreadcrumbDivider {
    /// Chevron pointing along the reading direction.
    pub fn new() -> Self {
        Self::default()
    }

    /// Replaces the chevron.
    pub fn icon(mut self, icon: impl Into<Icon>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    fn glyph(self, dir: Direction, icons: &crate::IconSet) -> Icon {
        self.icon
            .unwrap_or_else(|| dir.pick(icons.chevron_right, icons.chevron_left))
    }
}

/// Keyboard model of a [`Breadcrumb`] (`focusMode`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum BreadcrumbFocus {
    /// Every button is a Tab stop.
    #[default]
    Tab,
    /// One Tab stop; arrow keys move between the buttons (wrapping).
    Arrow,
}

// useBreadcrumbButtonStyles.styles.ts `useStyles`, `useIconStyles` and `typographyStyles`
const fn height(size: Size) -> f32 {
    match size {
        Size::Small => d::HEIGHT_SMALL,
        Size::Medium => d::HEIGHT_MEDIUM,
        Size::Large => d::HEIGHT_LARGE,
    }
}

const fn icon_size(size: Size) -> f32 {
    match size {
        Size::Small => d::ICON_SIZE_SMALL,
        Size::Medium => d::ICON_SIZE_MEDIUM,
        Size::Large => d::ICON_SIZE_LARGE,
    }
}

const fn text(size: Size, current: bool) -> TypeRamp {
    match (size, current) {
        (Size::Small, false) => TypeRamp::Caption1,
        (Size::Small, true) => TypeRamp::Caption1Strong,
        (Size::Medium, false) => TypeRamp::Body1,
        (Size::Medium, true) => TypeRamp::Body1Strong,
        (Size::Large, false) => TypeRamp::Body2,
        (Size::Large, true) => TypeRamp::Subtitle2,
    }
}

fn metrics(size: Size, current: bool) -> Metrics {
    let spec = text(size, current).spec();
    let pad_x = if size == Size::Large {
        g::SPACING_HORIZONTAL_S
    } else {
        g::SPACING_HORIZONTAL_S_NUDGE
    };
    let pad_y = (height(size) - 2.0 * g::STROKE_WIDTH_THIN - spec.line_height) / 2.0;
    Metrics {
        min_width: 0.0,
        pad_top: pad_y,
        pad_bottom: pad_y,
        pad_x,
        font_size: spec.size,
        line_height: spec.line_height,
        weight: spec.weight,
        icon: icon_size(size),
        icon_gap: g::SPACING_HORIZONTAL_XS,
        icon_only: height(size),
    }
}

/// Result of showing a [`Breadcrumb`].
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct BreadcrumbResponse {
    /// Spans the trail.
    pub response: egui::Response,
    /// Item clicked this frame, also from the overflow menu; the current and disabled items never report.
    pub clicked: Option<Clicked>,
}

/// Fluent `Breadcrumb`: a trail of [`BreadcrumbItem`]s separated by a [`BreadcrumbDivider`].
#[derive(Debug)]
#[must_use = "call .show(..)"]
pub struct Breadcrumb {
    salt: KeyedSalt,
    key: AgentKey,
    label: String,
    size: Size,
    focus: BreadcrumbFocus,
    divider: BreadcrumbDivider,
    overflow: Option<Overflow>,
    items: Vec<BreadcrumbItem>,
}

impl Breadcrumb {
    /// `label` names the trail for assistive technology; agent key `<id_salt>-breadcrumb`.
    pub fn new(id_salt: impl Into<KeyedSalt>, label: impl Into<String>) -> Self {
        Self {
            salt: id_salt.into(),
            key: AgentKey::Kind("breadcrumb"),
            label: label.into(),
            size: Size::Medium,
            focus: if d::ARROW_FOCUS {
                BreadcrumbFocus::Arrow
            } else {
                BreadcrumbFocus::Tab
            },
            divider: BreadcrumbDivider::new(),
            overflow: None,
            items: Vec::new(),
        }
    }

    /// Appends one item.
    pub fn item(mut self, item: impl Into<BreadcrumbItem>) -> Self {
        self.items.push(item.into());
        self
    }

    /// Appends items in order.
    pub fn items(mut self, items: impl IntoIterator<Item = impl Into<BreadcrumbItem>>) -> Self {
        self.items.extend(items.into_iter().map(Into::into));
        self
    }

    /// Default [`Size::Medium`].
    pub fn size(mut self, size: Size) -> Self {
        self.size = size;
        self
    }

    /// Default [`BreadcrumbFocus::Tab`].
    pub fn focus(mut self, focus: BreadcrumbFocus) -> Self {
        self.focus = focus;
        self
    }

    /// Replaces the divider glyph.
    pub fn divider(mut self, divider: BreadcrumbDivider) -> Self {
        self.divider = divider;
        self
    }

    /// Moves the items that do not fit the available width into a menu behind a button at the
    /// [`Overflow`] end; [`OverflowDirection::Start`](super::OverflowDirection::Start) keeps the trail's end visible.
    pub fn overflow(mut self, overflow: Overflow) -> Self {
        self.overflow = Some(overflow);
        self
    }

    crate::widget::setters! {
        /// Replaces the `<id_salt>-breadcrumb` agent key.
        agent_key(key)
    }

    /// Shows the trail.
    pub fn show(self, ui: &mut MineralUi<'_, Fluent>) -> BreadcrumbResponse {
        let rt = ui.rt();
        let Self {
            salt,
            key,
            label,
            size,
            focus,
            divider,
            overflow,
            items,
        } = self;
        let dir = rt.i18n().direction();
        let glyph = divider.glyph(dir, &rt.icons());
        let button = |i: usize| make_button(&items, &salt, size, i);
        let shown = fit(ui, overflow.as_ref(), (&items, &salt, size));
        let at_start = overflow.as_ref().is_some_and(Overflow::at_start);
        let parts = parts(&shown, at_start);
        let enabled = |p: usize| match parts.get(p) {
            Some(Part::Item(i)) => items.get(*i).is_some_and(|it| !it.button.disabled),
            Some(Part::More) => true,
            None => false,
        };
        let id = ui.make_persistent_id(salt.id_salt());
        let stop = roving::tab_stop(ui.ctx(), id, None, parts.len(), enabled);
        let mut ids = Vec::with_capacity(parts.len());
        let mut clicked = None;
        let mut trigger = None;
        let body = |ui: &mut egui::Ui| {
            ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
            for (pos, part) in parts.iter().enumerate() {
                if pos > 0 {
                    show_divider(rt, ui, glyph, size);
                }
                let b = match part {
                    Part::Item(i) => button(*i),
                    Part::More => overflow
                        .as_ref()
                        .map(|o| menu_button(overflow::trigger(salt.child("overflow"), o.label(), &rt.icons()), size)),
                };
                let Some(mut b) = b else { continue };
                if focus == BreadcrumbFocus::Arrow {
                    b.tab_stop = Some(roving::TabStop::at(pos, stop));
                }
                let r = rt.ui(ui).mineral(b);
                match part {
                    Part::Item(i) => {
                        let current = items.get(*i).is_some_and(|it| it.button.current);
                        if r.response().clicked() && !current && enabled(pos) {
                            clicked = Some(*i);
                        }
                    }
                    Part::More => trigger = Some(r.response().clone()),
                }
                ids.push(r.id);
            }
        };
        let group = ui
            .push_id(salt.id_salt(), |ui| {
                Frame::new()
                    .show(ui, |ui| {
                        ui.scope_builder(crate::widget::row(ui, dir, height(size)), body)
                    })
                    .response
            })
            .inner;
        if let (Some(trigger), Some(o)) = (trigger, overflow.as_ref()) {
            let open_id = id.with("mineral::overflow-open");
            let mut open = overflow::is_open(ui.ctx(), open_id);
            Menu::new(salt.child("overflow-menu"), o.label()).show(rt, &trigger, &mut open, |m| {
                clicked = overflow_rows(m, &items, &shown).or(clicked);
            });
            overflow::set_open(ui.ctx(), open_id, open);
        }
        if focus == BreadcrumbFocus::Arrow {
            RovingFocus::new(Axis::Horizontal)
                .wrap(d::WRAP)
                .update_group(ui.core(), id, &ids, enabled);
        }
        semantic::emit(ui.ctx(), Some(group.id), group.rect, || {
            SemanticNode::new(key.id(&salt), Role::Navigation, &label)
        });
        let clicked = clicked.map(|i| {
            let key = items.get(i).and_then(|it| it.button.action_key.clone());
            Clicked::new(i, key)
        });
        BreadcrumbResponse {
            response: group,
            clicked,
        }
    }
}

enum Part {
    Item(usize),
    More,
}

// The overflow button takes the size family of the trail: a square of the button height.
fn menu_button(mut b: PushButton<'static>, size: Size) -> PushButton<'static> {
    b = b.size(size);
    b.metrics = Some(metrics(size, false));
    b
}

fn show_divider(rt: &Rt, ui: &mut egui::Ui, icon: Icon, size: Size) {
    let extent = icon_size(size);
    let (rect, _) = ui.allocate_exact_size(vec2(extent, extent), Sense::hover());
    if ui.is_rect_visible(rect) {
        let m = rt.ui(ui);
        let color = m.tokens().color(ColorNeutralForeground1);
        paint::icon(ui.painter(), rect.center(), icon, false, extent, color);
    }
}

fn make_button(items: &[BreadcrumbItem], salt: &KeyedSalt, size: Size, i: usize) -> Option<PushButton<'static>> {
    let item = items.get(i)?;
    let b = &item.button;
    let mut button = PushButton::new(item.salt.clone().unwrap_or_else(|| salt.child(i)), b.label.clone())
        .appearance(ButtonAppearance::Subtle)
        .size(size)
        .disabled(b.disabled);
    if let Some(icon) = b.icon {
        button = button.icon(icon);
    }
    button.metrics = Some(metrics(size, b.current));
    button.inert = b.current;
    Some(button)
}

// Which items stay visible; the overflow button's width is reserved when some hide.
fn fit(
    ui: &MineralUi<'_, Fluent>,
    overflow: Option<&Overflow>,
    (items, salt, size): (&[BreadcrumbItem], &KeyedSalt, Size),
) -> Vec<bool> {
    let Some(o) = overflow else {
        return vec![true; items.len()];
    };
    let widths: Vec<OverflowItem> = (0..items.len())
        .map(|i| {
            let w = make_button(items, salt, size, i).map_or(0.0, |b| b.natural(ui).x);
            OverflowItem::new(w + if i > 0 { icon_size(size) } else { 0.0 })
        })
        .collect();
    let menu = menu_button(overflow::trigger(salt.child("overflow"), o.label(), &ui.icons()), size)
        .natural(ui)
        .x;
    o.fit(
        &widths,
        OverflowSpace {
            available: ui.available_width(),
            menu,
        },
    )
}

fn parts(shown: &[bool], at_start: bool) -> Vec<Part> {
    let mut parts: Vec<Part> = shown
        .iter()
        .enumerate()
        .filter_map(|(i, v)| v.then_some(Part::Item(i)))
        .collect();
    if shown.contains(&false) {
        if at_start {
            parts.insert(0, Part::More);
        } else {
            parts.push(Part::More);
        }
    }
    parts
}

fn overflow_rows(m: &mut MenuUi<'_, '_>, items: &[BreadcrumbItem], shown: &[bool]) -> Option<usize> {
    let mut clicked = None;
    for (i, item) in items.iter().enumerate().filter(|(i, _)| shown.get(*i) == Some(&false)) {
        let b = &item.button;
        let mut row = MenuItem::new(b.label.clone()).disabled(b.disabled || b.current);
        if let Some(icon) = b.icon {
            row = row.icon(icon);
        }
        if m.item(row).clicked {
            clicked = Some(i);
        }
    }
    clicked
}
