use egui::{Rect, Sense, Stroke, StrokeKind, Ui, UiBuilder, pos2, text::LayoutJob, vec2};
use egui_mineral::{
    focus::{self as roving, RovingFocus},
    i18n::I18n,
    kit,
    layout::{Axis, Constraints, Measure, MinTarget},
    select::SelectionResponse,
    semantic::{self, AgentKey, KeyedSalt, Role, SemanticNode},
    widget::{Availability, Interaction, MineralUi, MineralWidget, WidgetFrame, WidgetResponse},
};

use crate::ColorToken::{
    ColorBrandBackground, ColorBrandBackground2, ColorBrandBackground2Hover, ColorBrandBackground2Pressed,
    ColorBrandBackgroundHover, ColorBrandBackgroundPressed, ColorBrandForeground2, ColorBrandForeground2Hover,
    ColorBrandForeground2Pressed, ColorCompoundBrandForeground1, ColorCompoundBrandForeground1Hover,
    ColorCompoundBrandForeground1Pressed, ColorCompoundBrandStroke, ColorCompoundBrandStrokeHover,
    ColorCompoundBrandStrokePressed, ColorNeutralBackground3, ColorNeutralBackground3Hover,
    ColorNeutralBackground3Pressed, ColorNeutralBackgroundDisabled, ColorNeutralForeground1,
    ColorNeutralForeground1Hover, ColorNeutralForeground1Pressed, ColorNeutralForeground2,
    ColorNeutralForeground2Hover, ColorNeutralForeground2Pressed, ColorNeutralForegroundDisabled,
    ColorNeutralForegroundOnBrand, ColorNeutralStroke1Hover, ColorNeutralStroke1Pressed, ColorNeutralStrokeDisabled,
    ColorNeutralStrokeOnBrand, ColorStrokeFocus2, ColorSubtleBackground, ColorSubtleBackgroundHover,
    ColorSubtleBackgroundPressed, ColorTransparentBackground, ColorTransparentBackgroundHover,
    ColorTransparentBackgroundPressed, ColorTransparentStroke,
};
use crate::{
    Fluent, FluentExt, Icon, Size,
    spec::{
        defaults::tab as d,
        tokens::{geometry as g, motion as mo},
    },
    theme::{Elevation, Radius},
    typography::TypeRamp,
    widget::{
        choice::{self, Choice},
        paint,
        state::{Ramp, State},
        tween,
    },
};

/// Tab list `appearance`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum TabAppearance {
    /// `transparent`: no fill until hovered, with a selection indicator.
    #[default]
    Transparent,
    /// `subtle`: subtle fill states, with a selection indicator.
    Subtle,
    /// `subtle-circular`: pill tabs; the selected one is brand tinted.
    SubtleCircular,
    /// `filled-circular`: pill tabs on a neutral fill; the selected one is brand filled.
    FilledCircular,
}

impl TabAppearance {
    /// Every variant.
    pub const ALL: &'static [Self] = &[
        Self::Transparent,
        Self::Subtle,
        Self::SubtleCircular,
        Self::FilledCircular,
    ];

    const fn circular(self) -> bool {
        matches!(self, Self::SubtleCircular | Self::FilledCircular)
    }
}

/// Fluent `Tab`.
#[derive(Clone, Debug)]
#[must_use]
pub struct Tab {
    label: String,
    icon: Option<Icon>,
    disabled: bool,
    salt: Option<KeyedSalt>,
}

impl Tab {
    /// Visible text and accessible name.
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            icon: None,
            disabled: false,
            salt: None,
        }
    }

    /// `icon` slot before the label; the filled glyph shows while selected.
    pub fn icon(mut self, icon: impl Into<Icon>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    crate::widget::setters! {
        item_salt
    }

    crate::widget::setters! {
        /// Not selectable.
        disabled
    }
}

impl From<&str> for Tab {
    fn from(s: &str) -> Self {
        Self::new(s)
    }
}

impl From<String> for Tab {
    fn from(s: String) -> Self {
        Self::new(s)
    }
}

// useTabStyles.styles.ts `useRootStyles` `*Horizontal`/`*Vertical` padding: block, then inline
const fn padding(size: Size, vertical: bool) -> (f32, f32) {
    match (size, vertical) {
        (Size::Small, false) => (g::SPACING_VERTICAL_S_NUDGE, g::SPACING_HORIZONTAL_S_NUDGE),
        (Size::Small, true) => (g::SPACING_VERTICAL_XXS, g::SPACING_HORIZONTAL_S_NUDGE),
        (Size::Medium, false) => (g::SPACING_VERTICAL_M, g::SPACING_HORIZONTAL_M_NUDGE),
        (Size::Medium, true) => (g::SPACING_VERTICAL_S_NUDGE, g::SPACING_HORIZONTAL_M_NUDGE),
        (Size::Large, false) => (g::SPACING_VERTICAL_L, g::SPACING_HORIZONTAL_M_NUDGE),
        (Size::Large, true) => (g::SPACING_VERTICAL_S, g::SPACING_HORIZONTAL_M_NUDGE),
    }
}

// useTabStyles.styles.ts `columnGap`
const fn icon_gap(size: Size) -> f32 {
    match size {
        Size::Small => g::SPACING_HORIZONTAL_XXS,
        _ => g::SPACING_HORIZONTAL_S_NUDGE,
    }
}

const fn icon_size(size: Size) -> f32 {
    match size {
        Size::Large => d::ICON_SIZE_LARGE,
        _ => d::ICON_SIZE,
    }
}

// useTabStyles.styles.ts `useContentStyles`
const fn text(size: Size, selected: bool) -> TypeRamp {
    match (size, selected) {
        (Size::Large, false) => TypeRamp::Body2,
        (Size::Large, true) => TypeRamp::Subtitle2,
        (_, false) => TypeRamp::Body1,
        (_, true) => TypeRamp::Body1Strong,
    }
}

// useTabStyles.styles.ts `useActiveIndicatorStyles` `*Horizontal`/`*Vertical`
fn indicator_rect(tab: Rect, size: Size, vertical: bool) -> Rect {
    if vertical {
        let inset = match size {
            Size::Small => g::SPACING_VERTICAL_XS,
            Size::Medium => g::SPACING_VERTICAL_S,
            Size::Large => g::SPACING_VERTICAL_M_NUDGE,
        };
        Rect::from_min_max(
            pos2(tab.left(), tab.top() + inset),
            pos2(tab.left() + g::STROKE_WIDTH_THICKER, tab.bottom() - inset),
        )
    } else {
        let (inset, thickness) = match size {
            Size::Small => (g::SPACING_HORIZONTAL_S_NUDGE, g::STROKE_WIDTH_THICK),
            _ => (g::SPACING_HORIZONTAL_M, g::STROKE_WIDTH_THICKER),
        };
        Rect::from_min_max(
            pos2(tab.left() + inset, tab.bottom() - thickness),
            pos2(tab.right() - inset, tab.bottom()),
        )
    }
}

struct Palette {
    fill: Ramp,
    content: Ramp,
    icon: Ramp,
    border: Ramp,
}

fn transparent() -> Ramp {
    Ramp::new(
        ColorTransparentStroke,
        ColorTransparentStroke,
        ColorTransparentStroke,
        ColorTransparentStroke,
    )
}

fn muted() -> Ramp {
    Ramp::new(
        ColorNeutralForeground2,
        ColorNeutralForeground2Hover,
        ColorNeutralForeground2Pressed,
        ColorNeutralForegroundDisabled,
    )
}

fn regular(appearance: TabAppearance, selected: bool) -> Palette {
    let fill = if appearance == TabAppearance::Subtle {
        Ramp::new(
            ColorSubtleBackground,
            ColorSubtleBackgroundHover,
            ColorSubtleBackgroundPressed,
            ColorTransparentBackground,
        )
    } else {
        Ramp::new(
            ColorTransparentBackground,
            ColorTransparentBackgroundHover,
            ColorTransparentBackgroundPressed,
            ColorTransparentBackground,
        )
    };
    let (icon, content) = if selected {
        (
            Ramp::new(
                ColorCompoundBrandForeground1,
                ColorCompoundBrandForeground1Hover,
                ColorCompoundBrandForeground1Pressed,
                ColorNeutralForegroundDisabled,
            ),
            Ramp::new(
                ColorNeutralForeground1,
                ColorNeutralForeground1Hover,
                ColorNeutralForeground1Pressed,
                ColorNeutralForegroundDisabled,
            ),
        )
    } else {
        (muted(), muted())
    };
    Palette {
        fill,
        content,
        icon,
        border: transparent(),
    }
}

fn subtle_circular(selected: bool) -> Palette {
    let (fill, fg, border) = if selected {
        (
            Ramp::new(
                ColorBrandBackground2,
                ColorBrandBackground2Hover,
                ColorBrandBackground2Pressed,
                ColorNeutralBackgroundDisabled,
            ),
            Ramp::new(
                ColorBrandForeground2,
                ColorBrandForeground2Hover,
                ColorBrandForeground2Pressed,
                ColorNeutralForegroundDisabled,
            ),
            Ramp::new(
                ColorCompoundBrandStroke,
                ColorCompoundBrandStrokeHover,
                ColorCompoundBrandStrokePressed,
                ColorNeutralStrokeDisabled,
            ),
        )
    } else {
        (
            Ramp::new(
                ColorSubtleBackground,
                ColorSubtleBackgroundHover,
                ColorSubtleBackgroundPressed,
                ColorSubtleBackground,
            ),
            muted(),
            Ramp::new(
                ColorTransparentStroke,
                ColorNeutralStroke1Hover,
                ColorNeutralStroke1Pressed,
                ColorTransparentStroke,
            ),
        )
    };
    Palette {
        fill,
        content: fg,
        icon: fg,
        border,
    }
}

fn filled_circular(selected: bool) -> Palette {
    let (fill, fg) = if selected {
        (
            Ramp::new(
                ColorBrandBackground,
                ColorBrandBackgroundHover,
                ColorBrandBackgroundPressed,
                ColorNeutralBackgroundDisabled,
            ),
            Ramp::new(
                ColorNeutralForegroundOnBrand,
                ColorNeutralForegroundOnBrand,
                ColorNeutralForegroundOnBrand,
                ColorNeutralForegroundDisabled,
            ),
        )
    } else {
        (
            Ramp::new(
                ColorNeutralBackground3,
                ColorNeutralBackground3Hover,
                ColorNeutralBackground3Pressed,
                ColorNeutralBackgroundDisabled,
            ),
            muted(),
        )
    };
    Palette {
        fill,
        content: fg,
        icon: fg,
        border: if selected {
            transparent().disabled(Some(ColorNeutralStrokeDisabled))
        } else {
            transparent()
        },
    }
}

// useTabStyles.styles.ts `useRootStyles` and `useCircularAppearanceStyles`
fn palette(appearance: TabAppearance, selected: bool) -> Palette {
    match appearance {
        TabAppearance::Transparent | TabAppearance::Subtle => regular(appearance, selected),
        TabAppearance::SubtleCircular => subtle_circular(selected),
        TabAppearance::FilledCircular => filled_circular(selected),
    }
}

struct Measured {
    pad: (f32, f32),
    gap: f32,
    icon: f32,
    text_w: f32,
    line: f32,
}

impl Measured {
    fn of(ui: &MineralUi<'_, Fluent>, tab: &Tab, size: Size, vertical: bool) -> Self {
        let ctx = ui.ctx();
        let width = |selected: bool| {
            let job = LayoutJob::single_section(
                tab.label.clone(),
                text(size, selected).format(ctx, egui::Color32::PLACEHOLDER),
            );
            ui.painter().layout_job(job).size().x
        };
        Self {
            pad: padding(size, vertical),
            gap: icon_gap(size),
            icon: if tab.icon.is_some() { icon_size(size) } else { 0.0 },
            text_w: width(false).max(width(true)),
            line: text(size, false).line_height().max(text(size, true).line_height()),
        }
    }

    fn icon_span(&self) -> f32 {
        if self.icon > 0.0 { self.icon + self.gap } else { 0.0 }
    }

    fn content(&self) -> egui::Vec2 {
        vec2(
            self.icon_span() + self.text_w + 2.0 * d::CONTENT_PADDING,
            self.line.max(self.icon),
        )
    }

    fn cross(&self, vertical: bool) -> f32 {
        if vertical {
            self.content().x + 2.0 * self.pad.1
        } else {
            self.content().y + 2.0 * self.pad.0
        }
    }

    fn extent(&self, vertical: bool, stretch: f32) -> egui::Vec2 {
        if vertical {
            vec2(stretch, self.content().y + 2.0 * self.pad.0)
        } else {
            vec2(self.content().x + 2.0 * self.pad.1, stretch)
        }
    }
}

fn gap(appearance: TabAppearance, size: Size) -> f32 {
    if !appearance.circular() {
        0.0
    } else if size == Size::Small {
        g::SPACING_HORIZONTAL_S_NUDGE
    } else {
        g::SPACING_HORIZONTAL_S
    }
}

struct TabItem<'a> {
    salt: KeyedSalt,
    tab: &'a Tab,
    selected: bool,
    appearance: TabAppearance,
    size: Size,
    vertical: bool,
    tab_stop: bool,
    disabled: bool,
    extent: egui::Vec2,
}

impl MineralWidget for TabItem<'_> {
    type Kit = Fluent;
    type State = ();
    type Output = ();

    kit::widget_meta!(salt, enabled);

    fn sense(&self) -> Sense {
        roving::roving_sense(
            Sense::click(),
            if self.tab_stop {
                roving::TabStop::Stop
            } else {
                roving::TabStop::Skip
            },
        )
    }

    fn min_target(&self, _: &MineralUi<'_, Self::Kit>) -> MinTarget {
        MinTarget::None
    }

    fn layout(&self, _: &MineralUi<'_, Fluent>, c: Constraints, (): &()) -> Measure {
        c.constrain(self.extent).into()
    }

    fn update(&mut self, ui: &mut MineralUi<'_, Fluent>, frame: &WidgetFrame<'_>, (): &mut ()) -> WidgetResponse {
        let t = ui.tokens();
        let r = &frame.response;
        let availability = Availability::disabled_if(self.disabled);
        kit::input::hover_cursor(ui.core(), r, availability);
        kit::focus::follow_press(r);
        let i = Interaction::of(r, availability);
        let s = State::of(i);
        let selected = self.selected;
        let pal = palette(self.appearance, selected);
        let rect = frame.zones.visual;
        let circular = self.appearance.circular();
        let radius = if circular {
            Radius::Circular.on(rect)
        } else {
            Radius::Medium.on(rect)
        };
        ui.painter().rect_filled(rect, radius, pal.fill.pick(s, &t));
        if circular {
            ui.painter().rect_stroke(
                rect,
                radius,
                Stroke::new(g::STROKE_WIDTH_THIN, pal.border.pick(s, &t)),
                StrokeKind::Inside,
            );
        } else if !selected && s.engaged() {
            let token = if s == State::Pressed {
                ColorNeutralStroke1Pressed
            } else {
                ColorNeutralStroke1Hover
            };
            let bar = indicator_rect(rect, self.size, self.vertical);
            ui.painter().rect_filled(bar, Radius::Circular.on(bar), t.color(token));
        }

        let m = Measured::of(ui, self.tab, self.size, self.vertical);
        let dir = ui.i18n().direction();
        let content = m.content();
        let offset = if self.vertical {
            m.pad.1
        } else {
            ((rect.width() - content.x) / 2.0).max(0.0)
        };
        let cy = rect.center().y;
        let at = |from: f32, w: f32| dir.start_x(rect, egui::Rangef::new(offset + from, offset + from + w));
        let text_color = pal.content.pick(s, &t);
        if let Some(icon) = self.tab.icon {
            paint::icon(
                ui.painter(),
                pos2(at(0.0, m.icon) + m.icon / 2.0, cy),
                icon,
                selected,
                m.icon,
                pal.icon.pick(s, &t),
            );
        }
        let job = LayoutJob::single_section(
            self.tab.label.clone(),
            text(self.size, selected).format(ui.ctx(), text_color),
        );
        let galley = ui.painter().layout_job(job);
        let tx = at(m.icon_span() + d::CONTENT_PADDING, galley.size().x);
        ui.painter()
            .galley(pos2(tx, cy - galley.size().y / 2.0), galley, text_color);

        if i.focused {
            let p = ui.painter();
            paint::shadow(p, rect, radius, Elevation::E4, &t);
            p.rect_stroke(
                rect,
                radius,
                Stroke::new(g::STROKE_WIDTH_THICK, t.color(ColorStrokeFocus2)),
                StrokeKind::Outside,
            );
            if circular {
                p.rect_stroke(
                    rect,
                    radius,
                    Stroke::new(g::STROKE_WIDTH_THIN, t.color(ColorNeutralStrokeOnBrand)),
                    StrokeKind::Inside,
                );
            }
        }
        WidgetResponse::new(r.clone(), ()).with_changed(r.clicked() && !self.disabled)
    }

    fn semantics(&self, _: &I18n, _: &WidgetResponse, (): &()) -> Option<SemanticNode> {
        let mut n = SemanticNode::new(AgentKey::Kind("tab").id(&self.salt), Role::Tab, &self.tab.label)
            .clickable(Availability::disabled_if(self.disabled));
        n.state.selected = Some(self.selected);
        Some(n)
    }
}

#[derive(Clone, Copy)]
struct Indicator {
    id: egui::Id,
    size: Size,
    vertical: bool,
    enabled: bool,
}

impl Indicator {
    fn paint(&self, ui: &MineralUi<'_, Fluent>, sel: &egui::Response, rects: &[egui::Response]) {
        let Self {
            id,
            size,
            vertical,
            enabled,
        } = *self;
        let origin = rects.iter().fold(sel.rect, |u, r| u.union(r.rect)).min;
        let (from, len) = if vertical {
            (sel.rect.top() - origin.y, sel.rect.height())
        } else {
            (sel.rect.left() - origin.x, sel.rect.width())
        };
        let animate = |name: &str, target: f32| {
            tween::tween(
                ui.core(),
                id.with(name),
                target,
                mo::DURATION_SLOW,
                mo::CURVE_DECELERATE_MAX,
            )
        };
        let (from, len) = (animate("indicator-from", from), animate("indicator-length", len));
        let span = if vertical {
            Rect::from_min_size(pos2(sel.rect.left(), origin.y + from), vec2(sel.rect.width(), len))
        } else {
            Rect::from_min_size(pos2(origin.x + from, sel.rect.top()), vec2(len, sel.rect.height()))
        };
        let t = ui.tokens();
        let s = State::of(Interaction::of(sel, Availability::disabled_if(!enabled)));
        let color = Ramp::new(
            ColorCompoundBrandStroke,
            ColorCompoundBrandStrokeHover,
            ColorCompoundBrandStrokePressed,
            ColorNeutralForegroundDisabled,
        )
        .pick(s, &t);
        let bar = indicator_rect(span, size, vertical);
        ui.painter().rect_filled(bar, Radius::Circular.on(bar), color);
    }
}

/// Fluent `TabList`: a row or column of [`Tab`]s with an animated selection indicator. Arrow keys
/// move focus (wrapping) and Enter or Space selects; [`Self::select_on_focus`] selects on focus.
#[derive(Debug)]
#[must_use = "call .show(..)"]
pub struct TabList<'a> {
    salt: KeyedSalt,
    key: AgentKey,
    label: String,
    selected: Choice<'a>,
    tabs: Vec<Tab>,
    appearance: TabAppearance,
    size: Size,
    vertical: bool,
    select_on_focus: bool,
    disabled: bool,
}

impl<'a> TabList<'a> {
    /// `label` names the list for assistive technology; agent key `<id_salt>-tablist`.
    pub fn new(id_salt: impl Into<KeyedSalt>, label: impl Into<String>, selected: &'a mut usize) -> Self {
        Self::with(id_salt.into(), label.into(), Choice::by_index(selected), Vec::new())
    }

    /// Binds the value of the selected tab instead of its index (`selectedValue`): each of `tabs` comes with
    /// its value, a pick writes it into `value`, and no tab is selected while `value` matches none. Tabs
    /// added with [`Self::tab`] or [`Self::tabs`] have no value and cannot be picked.
    pub fn bound<T: PartialEq + Clone + 'a>(
        id_salt: impl Into<KeyedSalt>,
        label: impl Into<String>,
        tabs: impl IntoIterator<Item = (T, impl Into<Tab>)>,
        value: &'a mut T,
    ) -> Self {
        let (values, tabs) = choice::split(tabs);
        Self::with(id_salt.into(), label.into(), Choice::by_value(value, values), tabs)
    }

    fn with(salt: KeyedSalt, label: String, selected: Choice<'a>, tabs: Vec<Tab>) -> Self {
        Self {
            salt,
            key: AgentKey::Kind("tablist"),
            label,
            selected,
            tabs,
            appearance: TabAppearance::default(),
            size: Size::Medium,
            vertical: false,
            select_on_focus: d::SELECT_ON_FOCUS,
            disabled: false,
        }
    }

    /// Appends one tab.
    pub fn tab(mut self, tab: impl Into<Tab>) -> Self {
        self.tabs.push(tab.into());
        self
    }

    /// Appends tabs in order.
    pub fn tabs(mut self, tabs: impl IntoIterator<Item = impl Into<Tab>>) -> Self {
        self.tabs.extend(tabs.into_iter().map(Into::into));
        self
    }

    /// Default [`TabAppearance::Transparent`].
    pub fn appearance(mut self, appearance: TabAppearance) -> Self {
        self.appearance = appearance;
        self
    }

    /// Default [`Size::Medium`].
    pub fn size(mut self, size: Size) -> Self {
        self.size = size;
        self
    }

    /// Default [`Axis::Horizontal`]; [`Axis::Vertical`] stacks the tabs in a column.
    pub fn axis(mut self, axis: Axis) -> Self {
        self.vertical = axis == Axis::Vertical;
        self
    }

    /// Selects a tab when arrow keys move focus onto it; default off.
    pub fn select_on_focus(mut self, select_on_focus: bool) -> Self {
        self.select_on_focus = select_on_focus;
        self
    }

    crate::widget::setters! {
        /// Disables every tab.
        disabled
    }

    crate::widget::setters! {
        /// Replaces the `<id_salt>-tablist` agent key.
        agent_key(key)
    }

    /// Shows the list; a pick writes the bound index or value and reports `changed`. A bound index past the
    /// last tab is moved onto it.
    pub fn show(self, ui: &mut MineralUi<'_, Fluent>) -> SelectionResponse {
        let rt = ui.rt();
        let Self {
            salt,
            key,
            label,
            mut selected,
            tabs,
            appearance,
            size,
            vertical,
            select_on_focus,
            disabled,
        } = self;
        selected.clamp(tabs.len(), "TabList selected");
        let id = ui.make_persistent_id(salt.id_salt());
        let enabled = |i: usize| !disabled && tabs.get(i).is_some_and(|t| !t.disabled);
        let tab_stop = roving::tab_stop(ui.ctx(), id, selected.selected(), tabs.len(), enabled);
        let measured: Vec<Measured> = tabs.iter().map(|t| Measured::of(ui, t, size, vertical)).collect();
        let stretch = measured.iter().fold(0.0_f32, |m, l| m.max(l.cross(vertical)));
        let dir = rt.i18n().direction();
        let gap = gap(appearance, size);
        let current = selected.selected();
        let mut picked = None;
        let mut ids = Vec::with_capacity(tabs.len());
        let mut rects = Vec::with_capacity(tabs.len());
        let mut body = |ui: &mut Ui| {
            ui.spacing_mut().item_spacing = vec2(gap, gap);
            for (i, (tab, m)) in tabs.iter().zip(&measured).enumerate() {
                let r = rt.ui(ui).mineral(TabItem {
                    salt: tab.salt.clone().unwrap_or_else(|| salt.child(i)),
                    tab,
                    selected: Some(i) == current,
                    appearance,
                    size,
                    vertical,
                    tab_stop: i == tab_stop,
                    disabled: !enabled(i),
                    extent: m.extent(vertical, stretch),
                });
                if r.changed() {
                    picked = Some(i);
                }
                ids.push(r.id);
                rects.push(r.response().clone());
            }
        };
        let group = ui
            .push_id(salt.id_salt(), |ui| {
                let builder = if vertical {
                    UiBuilder::new().layout(dir.column())
                } else {
                    crate::widget::row(ui, dir, stretch)
                };
                ui.scope_builder(builder, |ui| body(ui)).response
            })
            .inner;
        let axis = if vertical { Axis::Vertical } else { Axis::Horizontal };
        if let Some(i) = RovingFocus::new(axis)
            .wrap(d::WRAP)
            .update_group(ui.core(), id, &ids, enabled)
            && select_on_focus
        {
            picked = Some(i);
        }
        let changed = picked.is_some_and(|i| Some(i) != current && selected.select(i));
        if !appearance.circular()
            && let Some((at, sel)) = selected.selected().and_then(|i| Some((i, rects.get(i)?)))
        {
            let on = Indicator {
                id,
                size,
                vertical,
                enabled: enabled(at),
            };
            on.paint(ui, sel, &rects);
        }
        semantic::emit(ui.ctx(), Some(group.id), group.rect, || {
            let mut n = SemanticNode::new(key.id(&salt), Role::TabList, &label);
            n.state.disabled = disabled;
            n
        });
        let response = group;
        SelectionResponse::new(response, picked.filter(|_| changed), changed)
    }
}
