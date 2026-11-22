use egui::{
    DragAndDrop, Key, Modifiers, Rangef, Rect, Sense, Stroke, StrokeKind, Ui, UiBuilder, pos2, text::LayoutJob, vec2,
};
use egui_mineral::{
    expand::Expansion,
    focus::{self, Typeahead},
    i18n::I18n,
    kit::{self, state::TempState},
    layout::{Constraints, Measure},
    select::{Check, Selection, SelectionMode},
    semantic::{self, Action, AgentKey, KeyedSalt, Role, SemanticNode},
    tree::{self, DropAccept, DropPosition, TreeData, TreeDrop, TreeNav, TreeRow},
    widget::{Availability, Interaction, MineralUi, MineralWidget, WidgetFrame, WidgetResponse},
};

use crate::spec::ColorToken::{
    ColorCompoundBrandBackground, ColorCompoundBrandBackgroundHover, ColorCompoundBrandBackgroundPressed,
    ColorCompoundBrandForeground1, ColorCompoundBrandStroke, ColorNeutralForeground2, ColorNeutralForeground2Hover,
    ColorNeutralForeground2Pressed, ColorNeutralForeground3, ColorNeutralForeground3Hover,
    ColorNeutralForeground3Pressed, ColorNeutralForegroundDisabled, ColorNeutralForegroundInverted,
    ColorNeutralStrokeAccessible, ColorNeutralStrokeAccessibleHover, ColorNeutralStrokeAccessiblePressed,
    ColorNeutralStrokeDisabled, ColorStrokeFocus2, ColorSubtleBackground, ColorSubtleBackgroundHover,
    ColorSubtleBackgroundLightAlphaHover, ColorSubtleBackgroundLightAlphaPressed, ColorSubtleBackgroundPressed,
    ColorTransparentBackground, ColorTransparentBackgroundHover, ColorTransparentBackgroundPressed,
};
use crate::{
    Fluent, FluentExt, Icon, Size,
    data::{Avatar, AvatarSize},
    spec::{defaults::tree as d, tokens::geometry as g},
    theme::Radius,
    typography::TypeRamp,
    widget::{
        paint,
        state::{Ramp, State},
    },
};

/// Tree `appearance`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum TreeAppearance {
    /// `subtle`: subtle fill states.
    #[default]
    Subtle,
    /// `subtle-alpha`: light alpha hover and pressed fills.
    SubtleAlpha,
    /// `transparent`: transparent fill states.
    Transparent,
}

impl TreeAppearance {
    /// Every variant.
    pub const ALL: &'static [Self] = &[Self::Subtle, Self::SubtleAlpha, Self::Transparent];
}

// Tree `selectionMode`
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TreeSelection {
    None,
    Single,
    Multiple,
}

// core tree::check_state over a Selection
fn check_state<N: TreeData>(node: &N, checked: &Selection<N::Key>) -> Check {
    let mut first = None;
    let mut stack = vec![node];
    while let Some(n) = stack.pop() {
        if n.children().is_empty() {
            let s = Check::from(checked.is_selected(n.key()));
            if *first.get_or_insert(s) != s {
                return Check::Mixed;
            }
        } else {
            stack.extend(n.children());
        }
    }
    first.unwrap_or(Check::Unchecked)
}

// core tree::toggle_check over a Selection
fn toggle_check<N: TreeData>(node: &N, checked: &mut Selection<N::Key>) {
    let on = check_state(node, checked) != Check::Checked;
    let mut stack = vec![node];
    while let Some(n) = stack.pop() {
        if on {
            checked.select(n.key().clone());
        } else {
            checked.deselect(n.key());
        }
        stack.extend(n.children());
    }
}

/// Fluent `TreeItemLayout`: icons around the label and an aside text.
#[derive(Clone, Debug, Default)]
#[must_use]
pub struct TreeItemLayout {
    icon_before: Option<Icon>,
    icon_after: Option<Icon>,
    aside: Option<String>,
}

impl TreeItemLayout {
    /// Label only.
    pub fn new() -> Self {
        Self::default()
    }

    /// `iconBefore` slot; the filled glyph shows while the row is checked.
    pub fn icon_before(mut self, icon: impl Into<Icon>) -> Self {
        self.icon_before = Some(icon.into());
        self
    }

    /// `iconAfter` slot, right behind the label.
    pub fn icon_after(mut self, icon: impl Into<Icon>) -> Self {
        self.icon_after = Some(icon.into());
        self
    }

    /// `aside` slot: caption text at the end of the row.
    pub fn aside(mut self, text: impl Into<String>) -> Self {
        self.aside = Some(text.into());
        self
    }
}

/// Fluent `TreeItemPersonaLayout`: a 32pt [`Avatar`] media slot, the label and a description below it.
#[derive(Clone, Debug, Default)]
#[must_use]
pub struct TreeItemPersonaLayout {
    media: Option<Box<Avatar<'static>>>,
    description: Option<String>,
    aside: Option<String>,
}

impl TreeItemPersonaLayout {
    /// Label only.
    pub fn new() -> Self {
        Self::default()
    }

    /// `media` slot: an [`Avatar`], always shown at 32.
    pub fn media(mut self, avatar: Avatar<'static>) -> Self {
        self.media = Some(Box::new(avatar));
        self
    }

    /// `description` slot below the label.
    pub fn secondary_text(mut self, text: impl Into<String>) -> Self {
        self.description = Some(text.into());
        self
    }

    /// `aside` slot: caption text at the end of the row.
    pub fn aside(mut self, text: impl Into<String>) -> Self {
        self.aside = Some(text.into());
        self
    }
}

/// Layout of one [`Tree`] row.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub enum ItemLayout {
    /// [`TreeItemLayout`].
    Plain(TreeItemLayout),
    /// [`TreeItemPersonaLayout`].
    Persona(TreeItemPersonaLayout),
}

impl Default for ItemLayout {
    fn default() -> Self {
        Self::Plain(TreeItemLayout::default())
    }
}

impl From<TreeItemLayout> for ItemLayout {
    fn from(layout: TreeItemLayout) -> Self {
        Self::Plain(layout)
    }
}

impl From<TreeItemPersonaLayout> for ItemLayout {
    fn from(layout: TreeItemPersonaLayout) -> Self {
        Self::Persona(layout)
    }
}

impl ItemLayout {
    fn aside(&self) -> Option<&str> {
        match self {
            Self::Plain(l) => l.aside.as_deref(),
            Self::Persona(l) => l.aside.as_deref(),
        }
    }
}

/// Fluent `TreeItem`: a node of the ready-made model of [`Tree::items`]; any [`TreeData`] works with [`Tree::new`].
#[derive(Clone, Debug)]
#[must_use]
pub struct TreeItem {
    key: String,
    label: String,
    layout: ItemLayout,
    children: Vec<TreeItem>,
}

impl TreeItem {
    /// `key` identifies the item in expansion and checks and must be unique across the tree; `label` is the visible
    /// text and accessible name.
    pub fn new(key: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            label: label.into(),
            layout: ItemLayout::default(),
            children: Vec::new(),
        }
    }

    /// Row layout; default a plain [`TreeItemLayout`].
    pub fn layout(mut self, layout: impl Into<ItemLayout>) -> Self {
        self.layout = layout.into();
        self
    }

    /// Nested items; an item with children is a branch.
    pub fn nested(mut self, children: impl IntoIterator<Item = Self>) -> Self {
        self.children = children.into_iter().collect();
        self
    }

    /// `FlatTree` data: items with their level (0 for roots) in depth-first order, nested by level. A level more than
    /// one past its predecessor counts as one past.
    pub fn from_levels(items: impl IntoIterator<Item = (usize, Self)>) -> Vec<Self> {
        fn close(stack: &mut Vec<TreeItem>, roots: &mut Vec<TreeItem>, depth: usize) {
            while stack.len() > depth {
                let Some(done) = stack.pop() else { return };
                match stack.last_mut() {
                    Some(parent) => parent.children.push(done),
                    None => roots.push(done),
                }
            }
        }
        let mut roots = Vec::new();
        let mut stack: Vec<Self> = Vec::new();
        for (level, item) in items {
            let depth = level.min(stack.len());
            close(&mut stack, &mut roots, depth);
            stack.push(item);
        }
        close(&mut stack, &mut roots, 0);
        roots
    }
}

impl TreeData for TreeItem {
    type Key = String;

    fn key(&self) -> &String {
        &self.key
    }

    fn label(&self) -> &str {
        &self.label
    }

    fn children(&self) -> &[Self] {
        &self.children
    }
}

#[derive(Clone, Copy, Debug, Default)]
struct RowOut {
    toggle: bool,
    check: bool,
    activate: bool,
}

struct Row<'a> {
    salt: KeyedSalt,
    label: &'a str,
    layout: ItemLayout,
    depth: usize,
    branch: bool,
    expanded: bool,
    check: Check,
    selection: TreeSelection,
    size: Size,
    appearance: TreeAppearance,
    tab_stop: focus::TabStop,
    disabled: bool,
}

struct Spans {
    expand: Option<Rangef>,
    selector: Option<Rangef>,
    content: f32,
}

fn height_of(layout: &ItemLayout, size: Size) -> f32 {
    let small = size == Size::Small;
    let min = if small { d::MIN_HEIGHT_SMALL } else { d::MIN_HEIGHT };
    match layout {
        ItemLayout::Plain(_) => min,
        ItemLayout::Persona(p) => {
            let (block, ..) = d::PERSONA_MAIN_PADDING;
            let ramp = if small { TypeRamp::Caption1 } else { TypeRamp::Body1 };
            let desc = if p.description.is_some() {
                TypeRamp::Caption1.line_height()
            } else {
                0.0
            };
            min.max(2.0 * block + ramp.line_height() + desc)
        }
    }
}

impl Row<'_> {
    fn small(&self) -> bool {
        self.size == Size::Small
    }

    fn text(&self) -> TypeRamp {
        if self.small() {
            TypeRamp::Caption1
        } else {
            TypeRamp::Body1
        }
    }

    fn height(&self) -> f32 {
        height_of(&self.layout, self.size)
    }

    // useRootStyles `leaf`/`branch` `paddingLeft`, then the expand icon, selector and content slots, in order
    fn spans(&self) -> Spans {
        let level = self.depth as f32;
        let mut x = if self.branch { level } else { level + 1.0 } * d::LEVEL_INDENT;
        let mut take = |w: f32| {
            let r = Rangef::new(x, x + w);
            x += w;
            r
        };
        let expand = self.branch.then(|| take(d::EXPAND_ICON_WIDTH));
        let selector =
            (self.selection != TreeSelection::None).then(|| take(d::SELECTOR_INDICATOR + 2.0 * d::SELECTOR_MARGIN));
        Spans {
            expand,
            selector,
            content: x,
        }
    }

    fn fill(&self) -> Ramp {
        match self.appearance {
            TreeAppearance::Subtle => Ramp::new(
                ColorSubtleBackground,
                ColorSubtleBackgroundHover,
                ColorSubtleBackgroundPressed,
                ColorSubtleBackground,
            ),
            TreeAppearance::SubtleAlpha => Ramp::new(
                ColorSubtleBackground,
                ColorSubtleBackgroundLightAlphaHover,
                ColorSubtleBackgroundLightAlphaPressed,
                ColorSubtleBackground,
            ),
            TreeAppearance::Transparent => Ramp::new(
                ColorTransparentBackground,
                ColorTransparentBackgroundHover,
                ColorTransparentBackgroundPressed,
                ColorTransparentBackground,
            ),
        }
    }

    fn selector(&self, ui: &MineralUi<'_, Fluent>, s: State, center: egui::Pos2) {
        let t = ui.tokens();
        let p = ui.painter();
        let side = d::SELECTOR_INDICATOR;
        let rect = Rect::from_center_size(center, vec2(side, side));
        let checked = self.check == Check::Checked;
        let accent = Ramp::new(
            ColorCompoundBrandBackground,
            ColorCompoundBrandBackgroundHover,
            ColorCompoundBrandBackgroundPressed,
            ColorNeutralStrokeDisabled,
        );
        let neutral = Ramp::new(
            ColorNeutralStrokeAccessible,
            ColorNeutralStrokeAccessibleHover,
            ColorNeutralStrokeAccessiblePressed,
            ColorNeutralStrokeDisabled,
        );
        let fixed = |token, off| t.color(if self.disabled { off } else { token });
        let brand_stroke = fixed(ColorCompoundBrandStroke, ColorNeutralStrokeDisabled);
        let brand_fg = fixed(ColorCompoundBrandForeground1, ColorNeutralForegroundDisabled);
        if self.selection == TreeSelection::Single {
            let radius = side / 2.0;
            if checked {
                p.circle_stroke(center, radius, Stroke::new(g::STROKE_WIDTH_THIN, brand_stroke));
                p.circle_filled(center, radius * 0.625, brand_fg);
            } else {
                p.circle_stroke(center, radius, Stroke::new(g::STROKE_WIDTH_THIN, neutral.pick(s, &t)));
            }
            return;
        }
        let radius = Radius::Small.on(rect);
        match self.check {
            Check::Checked => {
                p.rect_filled(rect, radius, accent.pick(s, &t));
                paint::icon(
                    p,
                    center,
                    ui.icons().checkmark,
                    true,
                    crate::spec::defaults::checkbox::GLYPH_MEDIUM,
                    fixed(ColorNeutralForegroundInverted, ColorNeutralForegroundDisabled),
                );
            }
            Check::Mixed => {
                p.rect_stroke(
                    rect,
                    radius,
                    Stroke::new(g::STROKE_WIDTH_THIN, brand_stroke),
                    StrokeKind::Inside,
                );
                p.rect_filled(
                    Rect::from_center_size(center, vec2(side / 2.0, side / 2.0)),
                    1,
                    brand_fg,
                );
            }
            Check::Unchecked => {
                p.rect_stroke(
                    rect,
                    radius,
                    Stroke::new(g::STROKE_WIDTH_THIN, neutral.pick(s, &t)),
                    StrokeKind::Inside,
                );
            }
        }
    }
}

impl Row<'_> {
    fn paint_content(
        &self,
        ui: &MineralUi<'_, Fluent>,
        rect: Rect,
        content: f32,
        fg: egui::Color32,
        muted: egui::Color32,
    ) {
        let t = ui.tokens();
        let dir = ui.i18n().direction();
        let mid = |x: f32| dir.start_x(rect, Rangef::new(x, x));
        let cy = rect.center().y;
        let checked = self.check == Check::Checked;
        let gap = if self.small() { d::ICON_GAP_SMALL } else { d::ICON_GAP };
        let icon_color = if self.disabled {
            fg
        } else {
            t.color(ColorNeutralForeground2)
        };
        let mut x = content;
        let draw = |ui: &MineralUi<'_, Fluent>, text: &str, ramp: TypeRamp, color, x0: f32, y: f32| {
            let job = LayoutJob::single_section(text.to_owned(), ramp.format(ui.ctx(), color));
            let galley = ui.painter().layout_job(job);
            let w = galley.size().x;
            let px = dir.start_x(rect, Rangef::new(x0, x0 + w));
            ui.painter().galley(pos2(px, y - galley.size().y / 2.0), galley, color);
            w
        };
        match &self.layout {
            ItemLayout::Plain(l) => {
                if let Some(icon) = l.icon_before {
                    let c = pos2(mid(x + d::ICON_SIZE / 2.0), cy);
                    paint::icon(ui.painter(), c, icon, checked, d::ICON_SIZE, icon_color);
                    x += d::ICON_SIZE + gap;
                }
                x += d::MAIN_PADDING;
                x += draw(ui, self.label, self.text(), fg, x, cy) + d::MAIN_PADDING;
                if let Some(icon) = l.icon_after {
                    x += gap;
                    let c = pos2(mid(x + d::ICON_SIZE / 2.0), cy);
                    paint::icon(ui.painter(), c, icon, checked, d::ICON_SIZE, icon_color);
                }
            }
            ItemLayout::Persona(l) => {
                let (pad_start, pad_end) = d::MEDIA_PADDING;
                x += pad_start;
                if let Some(avatar) = &l.media {
                    let c = pos2(mid(x + d::MEDIA_SIZE / 2.0), cy);
                    avatar.as_ref().clone().size(AvatarSize::S32).paint(
                        ui,
                        Rect::from_center_size(c, vec2(d::MEDIA_SIZE, d::MEDIA_SIZE)),
                        &t,
                    );
                }
                x += d::MEDIA_SIZE + pad_end;
                let (block, start, _) = d::PERSONA_MAIN_PADDING;
                let text_x = x + start;
                if let Some(desc) = &l.description {
                    let top = rect.top() + block;
                    draw(
                        ui,
                        self.label,
                        self.text(),
                        fg,
                        text_x,
                        top + self.text().line_height() / 2.0,
                    );
                    let y = top + self.text().line_height() + TypeRamp::Caption1.line_height() / 2.0;
                    draw(ui, desc, TypeRamp::Caption1, muted, text_x, y);
                } else {
                    draw(ui, self.label, self.text(), fg, text_x, cy);
                }
            }
        }
        if let Some(aside) = self.layout.aside() {
            let job = LayoutJob::single_section(aside.to_owned(), TypeRamp::Caption1.format(ui.ctx(), muted));
            let galley = ui.painter().layout_job(job);
            let w = galley.size().x;
            let from = rect.width() - d::ASIDE_PADDING - w;
            let px = dir.start_x(rect, Rangef::new(from, from + w));
            ui.painter().galley(pos2(px, cy - galley.size().y / 2.0), galley, muted);
        }
    }

    fn input(&self, ui: &MineralUi<'_, Fluent>, r: &egui::Response, selector_hit: Option<Rect>) -> RowOut {
        let mut out = RowOut::default();
        if self.disabled {
            return out;
        }
        let by_pointer = r.interact_pointer_pos();
        if r.clicked() {
            let on_selector = by_pointer.is_some_and(|p| selector_hit.is_some_and(|h| h.contains(p)));
            let space = by_pointer.is_none() && ui.input(|i| i.key_pressed(Key::Space));
            if on_selector || (space && selector_hit.is_some()) {
                out.check = true;
            } else if !space {
                out.activate = true;
                out.toggle = self.branch;
            }
        }
        if kit::interact::accesskit_requested(ui.ctx(), r.id, Action::Expand) && self.branch && !self.expanded
            || kit::interact::accesskit_requested(ui.ctx(), r.id, Action::Collapse) && self.expanded
        {
            out.toggle = true;
        }
        out
    }
}
impl MineralWidget for Row<'_> {
    type Kit = Fluent;
    type State = ();
    type Output = RowOut;

    kit::widget_meta!(salt, enabled, no_target);

    fn sense(&self) -> Sense {
        focus::roving_sense(Sense::click(), self.tab_stop)
    }

    fn layout(&self, ui: &MineralUi<'_, Fluent>, c: Constraints, (): &()) -> Measure {
        let width = if c.max.x.is_finite() {
            c.max.x
        } else {
            ui.available_width()
        };
        c.constrain(vec2(width, self.height())).into()
    }

    fn update(
        &mut self,
        ui: &mut MineralUi<'_, Fluent>,
        frame: &WidgetFrame<'_>,
        (): &mut (),
    ) -> WidgetResponse<RowOut> {
        let t = ui.tokens();
        let r = &frame.response;
        let availability = Availability::disabled_if(self.disabled);
        kit::interact::hover_cursor(ui.core(), r, availability);
        kit::interact::follow_press(r);
        let i = Interaction::of(r, availability);
        let s = State::of(i);
        let rect = frame.zones.visual;
        let dir = ui.i18n().direction();
        let at = |range: Rangef| dir.start_x(rect, range);
        let mid = |x: f32| at(Rangef::new(x, x));
        ui.painter().rect_filled(rect, 0, self.fill().pick(s, &t));
        let fg = Ramp::new(
            ColorNeutralForeground2,
            ColorNeutralForeground2Hover,
            ColorNeutralForeground2Pressed,
            ColorNeutralForegroundDisabled,
        )
        .pick(s, &t);
        let muted = Ramp::new(
            ColorNeutralForeground3,
            ColorNeutralForeground3Hover,
            ColorNeutralForeground3Pressed,
            ColorNeutralForegroundDisabled,
        )
        .pick(s, &t);
        let spans = self.spans();
        let cy = rect.center().y;
        if let Some(span) = spans.expand {
            let icon = match (self.expanded, dir) {
                (true, _) => ui.icons().chevron_down,
                (false, egui_mineral::i18n::Direction::Ltr) => ui.icons().chevron_right,
                (false, egui_mineral::i18n::Direction::Rtl) => ui.icons().chevron_left,
            };
            paint::icon(
                ui.painter(),
                pos2(mid(span.center()), cy),
                icon,
                false,
                d::EXPAND_ICON_SIZE,
                muted,
            );
        }
        let mut selector_hit = None;
        if let Some(span) = spans.selector {
            self.selector(ui, s, pos2(mid(span.center()), cy));
            selector_hit = Some(Rect::from_min_size(
                pos2(at(span), rect.top()),
                vec2(span.span(), rect.height()),
            ));
        }
        self.paint_content(ui, rect, spans.content, fg, muted);
        if i.focused {
            ui.painter().rect_stroke(
                rect,
                Radius::Medium.on(rect),
                Stroke::new(g::STROKE_WIDTH_THICK, t.color(ColorStrokeFocus2)),
                StrokeKind::Outside,
            );
        }

        let out = self.input(ui, r, selector_hit);
        let changed = out.check || out.toggle;
        WidgetResponse::new(r.clone(), out).with_changed(changed)
    }

    fn semantics(&self, _: &I18n, response: &WidgetResponse<RowOut>, (): &()) -> Option<SemanticNode> {
        Some(self.node(response.has_focus()))
    }
}

impl Row<'_> {
    fn node(&self, focused: bool) -> SemanticNode {
        let mut n = SemanticNode::new(
            egui_mineral::agent_key!("treeitem").id(&self.salt),
            Role::TreeItem,
            self.label,
        )
        .clickable(Availability::disabled_if(self.disabled));
        n.state.level = Some(self.depth + 1);
        n.state.focused = focused;
        if self.selection != TreeSelection::None {
            n.state.checked = Some(self.check);
        }
        if self.branch {
            n.state.expanded = Some(self.expanded);
        }
        if self.branch && !self.disabled {
            n.actions.push(if self.expanded {
                Action::Collapse
            } else {
                Action::Expand
            });
        }
        n
    }
}

const ROW_SCOPE: &str = "row";

struct Visible<'a, N> {
    rows: Vec<TreeRow<'a, N>>,
    stop: usize,
    reveal: Option<usize>,
    revealed: bool,
}

struct Drawn {
    row: usize,
    out: RowOut,
    response: egui::Response,
}

#[derive(Clone)]
struct TreeState<K> {
    stop: Option<K>,
    // The Tab stop is focused and scrolled into view on the next pass.
    reveal: bool,
    typeahead: Typeahead,
}

impl<K> Default for TreeState<K> {
    fn default() -> Self {
        Self {
            stop: None,
            reveal: false,
            typeahead: Typeahead::default(),
        }
    }
}

impl<K: Clone + Send + Sync + 'static> TempState for TreeState<K> {}

/// Result of showing a [`Tree`].
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct TreeResponse<K> {
    /// Spans every row; `mark_changed` when `changed`.
    pub response: egui::Response,
    /// Expansion or checks changed this pass.
    pub changed: bool,
    /// Item expanded this pass.
    pub expanded: Option<K>,
    /// Item collapsed this pass.
    pub collapsed: Option<K>,
    /// Item whose selector toggled this pass; the checked set already holds the result.
    pub checked: Option<K>,
    /// Item activated (click or Enter) this pass.
    pub activated: Option<K>,
    /// Row responses of the visible items.
    pub rows: egui_mineral::action::ItemResponses<K, egui::Response>,
    /// Foreign payload released on a row; see [`Tree::accept_drop`].
    pub dropped: Option<TreeDrop<K>>,
}

type LayoutFn<'a, N> = Box<dyn Fn(&N) -> ItemLayout + 'a>;
type DisabledFn<'a, N> = Box<dyn Fn(&N) -> bool + 'a>;

/// Fluent `Tree`: nested or flat ([`TreeItem::from_levels`]) items over a [`TreeData`] model, expandable, with
/// optional checkbox or radio selectors. One row is the Tab stop; Up, Down, Home and End move, Right expands a
/// collapsed branch or enters an open one, Left collapses an open branch or leaves for the parent, Enter
/// activates (and toggles a branch), Space toggles the check and typing a character jumps to a label.
#[must_use = "call .show(..)"]
pub struct Tree<'a, N: TreeData> {
    salt: KeyedSalt,
    key: AgentKey,
    label: String,
    roots: &'a [N],
    expansion: &'a mut Expansion<N::Key>,
    checked: Option<&'a mut Selection<N::Key>>,
    selection: TreeSelection,
    size: Size,
    appearance: TreeAppearance,
    layout: LayoutFn<'a, N>,
    node_disabled: Option<DisabledFn<'a, N>>,
    accept: DropAccept<'a, N>,
    reveal: Option<N::Key>,
}

impl<N: TreeData> std::fmt::Debug for Tree<'_, N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Tree")
            .field("salt", &self.salt)
            .field("label", &self.label)
            .field("selection", &self.selection)
            .finish_non_exhaustive()
    }
}

impl<'a> Tree<'a, TreeItem> {
    /// [`Tree::new`] over [`TreeItem`]s, each row using the item's own layout.
    pub fn items(
        id_salt: impl Into<KeyedSalt>,
        label: impl Into<String>,
        roots: &'a [TreeItem],
        expansion: &'a mut Expansion<String>,
    ) -> Self {
        Self::new(id_salt, label, roots, expansion).item_layout(|item| item.layout.clone())
    }
}

impl<'a, N: TreeData> Tree<'a, N> {
    /// `label` names the tree for assistive technology; `expansion` holds the expanded keys. Agent key
    /// `<id_salt>-tree`.
    pub fn new(
        id_salt: impl Into<KeyedSalt>,
        label: impl Into<String>,
        roots: &'a [N],
        expansion: &'a mut Expansion<N::Key>,
    ) -> Self {
        Self {
            salt: id_salt.into(),
            key: egui_mineral::agent_key!("tree"),
            label: label.into(),
            roots,
            expansion,
            checked: None,
            selection: TreeSelection::None,
            size: Size::Medium,
            appearance: TreeAppearance::default(),
            layout: Box::new(|_| ItemLayout::default()),
            node_disabled: None,
            accept: DropAccept::default(),
            reveal: None,
        }
    }

    /// Row layout per node; default a plain [`TreeItemLayout`].
    pub fn item_layout(mut self, layout: impl Fn(&N) -> ItemLayout + 'a) -> Self {
        self.layout = Box::new(layout);
        self
    }

    /// `selectionMode` and the checked keys it edits: [`SelectionMode::Single`] shows radio selectors and keeps at
    /// most one key, [`SelectionMode::Multiple`] checkbox selectors, where a branch checks and unchecks its
    /// descendants and shows the mixed state. Without it rows carry no selector.
    pub fn selection(mut self, selection: &'a mut Selection<N::Key>) -> Self {
        self.selection = match selection.mode() {
            SelectionMode::Single => TreeSelection::Single,
            SelectionMode::Multiple => TreeSelection::Multiple,
        };
        self.checked = Some(selection);
        self
    }

    /// [`Size::Medium`] (default) or [`Size::Small`]; [`Size::Large`] is medium, as in React.
    pub fn size(mut self, size: Size) -> Self {
        self.size = size;
        self
    }

    /// Default [`TreeAppearance::Subtle`].
    pub fn appearance(mut self, appearance: TreeAppearance) -> Self {
        self.appearance = appearance;
        self
    }

    crate::widget::setters! {
        /// Replaces the `<id_salt>-tree` agent key.
        agent_key(key)
    }

    /// Disables the nodes `node_disabled` names and their descendants: they take no focus, check, activation,
    /// expansion or drop inside.
    pub fn node_disabled(mut self, node_disabled: impl Fn(&N) -> bool + 'a) -> Self {
        self.node_disabled = Some(Box::new(node_disabled));
        self
    }

    /// Foreign [`DragAndDrop`] payloads of type `P` drop before, after or inside rows where `accept(payload,
    /// node, position)` holds; reported in [`TreeResponse::dropped`]. Each call adds a payload type; the first that
    /// accepts the payload in flight takes it.
    pub fn accept_drop<P: std::any::Any + Send + Sync>(
        mut self,
        accept: impl Fn(&P, &N, DropPosition) -> bool + 'a,
    ) -> Self {
        self.accept.push(accept);
        self
    }

    /// Expands the ancestors of `key` and scrolls its row into view this pass, leaving focus where it is.
    pub fn reveal(mut self, key: N::Key) -> Self {
        self.reveal = Some(key);
        self
    }

    fn check_of(&self, node: &N) -> Check {
        let Some(set) = self.checked.as_deref() else {
            return Check::Unchecked;
        };
        match self.selection {
            TreeSelection::Multiple => check_state(node, set),
            _ => set.is_selected(node.key()).into(),
        }
    }

    fn apply_check(&mut self, node: &N) {
        let Some(set) = self.checked.as_deref_mut() else {
            return;
        };
        if self.selection == TreeSelection::Single {
            set.select(node.key().clone());
        } else {
            toggle_check(node, set);
        }
    }

    fn row(&self, i: usize, row: &TreeRow<'a, N>, layout: ItemLayout, stop: usize) -> Row<'a> {
        let node = row.node;
        Row {
            salt: self.salt.child(node.key().clone()),
            label: node.label(),
            layout,
            depth: row.depth,
            branch: row.has_children,
            expanded: row.expanded,
            check: self.check_of(node),
            selection: self.selection,
            size: self.size,
            appearance: self.appearance,
            tab_stop: focus::TabStop::at(i, stop),
            disabled: row.disabled,
        }
    }

    /// Shows the visible rows; rows outside the clip rect are never laid out as widgets.
    pub fn show(mut self, ui: &mut MineralUi<'_, Fluent>) -> TreeResponse<N::Key> {
        let rt = ui.rt();
        let ctx = rt.ctx();
        let id = ui.make_persistent_id(self.salt.id_salt());
        let mut state = TreeState::<N::Key>::load(ctx, id);
        let Visible {
            rows,
            stop,
            reveal,
            revealed,
        } = self.visible(&state);
        let dir = rt.i18n().direction();
        // Row tops from the top of the tree; the last entry is the end of the final gap.
        let mut tops: Vec<f32> = Vec::with_capacity(rows.len() + 1);
        let mut y = 0.0;
        for row in &rows {
            tops.push(y);
            y += height_of(&(self.layout)(row.node), self.size) + d::ROW_GAP;
        }
        tops.push(y);
        let total = (y - d::ROW_GAP).max(0.0);
        let mut drawn: Vec<Drawn> = Vec::new();
        let mut dropped = None;
        let group = ui
            .push_id(self.salt.id_salt(), |ui| {
                let width = ui.available_width();
                let (area, group) = ui.allocate_exact_size(vec2(width, total), Sense::hover());
                let clip = ui.clip_rect();
                let first = tops
                    .get(1..)
                    .unwrap_or_default()
                    .partition_point(|t| area.min.y + t - d::ROW_GAP < clip.min.y);
                let end = tops
                    .get(..rows.len())
                    .unwrap_or_default()
                    .partition_point(|t| area.min.y + t <= clip.max.y);
                let visible = first..end.max(first);
                if let Some(r) = reveal {
                    ui.scroll_to_rect(rect_of_row(&tops, area, r), None);
                }
                // The Tab stop's row is shown wherever it is, so Tab still reaches the tree.
                let extra = Some(stop).filter(|s| *s < rows.len() && !visible.contains(s));
                let rect_of = |i: usize| rect_of_row(&tops, area, i);
                let order = (extra.filter(|s| *s < visible.start).into_iter())
                    .chain(visible.clone())
                    .chain(extra.filter(|s| *s >= visible.end));
                for i in order {
                    let Some(row) = rows.get(i) else { continue };
                    let widget = self.row(i, row, (self.layout)(row.node), stop);
                    let rect = rect_of(i);
                    let mut child =
                        ui.new_child(UiBuilder::new().max_rect(rect).layout(dir.column()).id_salt(ROW_SCOPE));
                    let (response, out) = rt.ui(&mut child).mineral(widget).into_parts();
                    if state.reveal && i == stop {
                        state.reveal = false;
                        ctx.memory_mut(|m| m.request_focus(response.id));
                        ui.scroll_to_rect(rect, None);
                    }
                    drawn.push(Drawn { row: i, out, response });
                }
                if kit::nodes::wanted(ctx, None) {
                    for i in (0..rows.len()).filter(|i| !visible.contains(i) && extra != Some(*i)) {
                        self.emit_culled(ui, &rows, i, stop, rect_of(i));
                    }
                }
                dropped = self.external_drop(rt, ui, area, &rows, &tops);
                group
            })
            .inner;

        let mut out = TreeResponse {
            response: group,
            changed: false,
            expanded: None,
            collapsed: None,
            checked: None,
            activated: None,
            rows: egui_mineral::action::ItemResponses::default(),
            dropped,
        };
        out.changed |= revealed;
        self.apply(&rows, &drawn, &mut out);
        if let Some(cur) = drawn.iter().position(|d| d.response.has_focus()) {
            self.navigate(rt, &rows, &drawn, cur, &mut state, &mut out);
        }
        for (row, d) in drawn.into_iter().filter_map(|d| Some((rows.get(d.row)?, d))) {
            out.rows.push(row.node.key().clone(), d.response);
        }
        state.store(ctx, id);
        let (key, label) = (self.key.id(&self.salt), self.label);
        semantic::emit(ctx, Some(out.response.id), out.response.rect, || {
            SemanticNode::new(key, Role::Tree, label)
        });
        if out.changed {
            out.response.mark_changed();
        }
        out
    }

    // Rows after the reveal expanded its ancestors; the Tab stop is an enabled row.
    fn visible(&mut self, state: &TreeState<N::Key>) -> Visible<'a, N> {
        let revealed = self.expand_revealed();
        let rows: Vec<TreeRow<'a, N>> = match &self.node_disabled {
            Some(f) => tree::flatten_with(self.roots, self.expansion, f),
            None => tree::flatten(self.roots, self.expansion),
        };
        let enabled = |i: &usize| rows.get(*i).is_some_and(|r| !r.disabled);
        let stop = state
            .stop
            .as_ref()
            .and_then(|k| rows.iter().position(|r| r.node.key() == k))
            .filter(enabled)
            .or_else(|| (0..rows.len()).find(enabled))
            .unwrap_or(0);
        let reveal = self
            .reveal
            .as_ref()
            .and_then(|k| rows.iter().position(|r| r.node.key() == k));
        Visible {
            rows,
            stop,
            reveal,
            revealed,
        }
    }

    // `true` when an ancestor of the revealed node opened.
    fn expand_revealed(&mut self) -> bool {
        let Some(key) = &self.reveal else { return false };
        let mut opened = false;
        for k in tree::ancestors(self.roots, key).unwrap_or_default() {
            if !self.expansion.is_expanded(&k) {
                self.expansion.expand(k);
                opened = true;
            }
        }
        opened
    }

    fn external_drop(
        &self,
        rt: &crate::Rt,
        ui: &Ui,
        area: Rect,
        rows: &[TreeRow<'a, N>],
        tops: &[f32],
    ) -> Option<TreeDrop<N::Key>> {
        let accept = Some(&self.accept).filter(|a| !a.is_empty())?;
        let ctx = ui.ctx();
        let pos = ctx
            .pointer_interact_pos()
            .filter(|p| area.contains(*p) && egui_mineral::input::reaches_layer(ctx, ui.layer_id(), *p))?;
        let i = (0..rows.len()).find(|&i| rect_of_row(tops, area, i).y_range().contains(pos.y))?;
        let rect = rect_of_row(tops, area, i);
        let (target, drop) = accept.slot(ctx, rows, i, (pos.y - rect.top()) / rect.height())?;
        let at = rect_of_row(tops, area, target);
        let t = rt.tokens();
        let stroke = Stroke::new(g::STROKE_WIDTH_THICK, t.color(ColorCompoundBrandStroke));
        let p = ui.painter_at(area);
        match drop.position() {
            DropPosition::Before => {
                p.hline(at.x_range(), at.top(), stroke);
            }
            DropPosition::After => {
                p.hline(at.x_range(), at.bottom(), stroke);
            }
            DropPosition::Inside => {
                p.rect_stroke(at, Radius::Medium.on(at), stroke, StrokeKind::Inside);
            }
        }
        if !ui.input(|i| i.pointer.any_released()) {
            return None;
        }
        DragAndDrop::clear_payload(ctx);
        Some(drop)
    }

    // Same id as the drawn row's, whose child ui scope is `ROW_SCOPE` under `ui`.
    fn emit_culled(&self, ui: &Ui, rows: &[TreeRow<'a, N>], i: usize, stop: usize, rect: Rect) {
        let Some(row) = rows.get(i) else { return };
        let widget = self.row(i, row, (self.layout)(row.node), stop);
        let rid = ui.scope_id().with(ROW_SCOPE).with(widget.id_salt());
        kit::nodes::emit_culled(ui, rid, rect, || widget.node(false));
    }

    fn apply(&mut self, rows: &[TreeRow<'a, N>], drawn: &[Drawn], out: &mut TreeResponse<N::Key>) {
        for d in drawn {
            let Some(row) = rows.get(d.row) else { continue };
            let node = row.node;
            if d.out.toggle {
                self.toggle(out, node.key().clone());
            }
            if d.out.check {
                self.apply_check(node);
                out.checked = Some(node.key().clone());
                out.changed = true;
            }
            if d.out.activate {
                out.activated = Some(node.key().clone());
            }
        }
    }

    fn navigate(
        &mut self,
        rt: &crate::Rt,
        rows: &[TreeRow<'a, N>],
        drawn: &[Drawn],
        cur: usize,
        state: &mut TreeState<N::Key>,
        out: &mut TreeResponse<N::Key>,
    ) {
        let ctx = rt.ctx();
        let dir = rt.i18n().direction();
        let Some(&Drawn {
            row: cur_row,
            ref response,
            ..
        }) = drawn.get(cur)
        else {
            return;
        };
        if let Some(row) = rows.get(cur_row) {
            state.stop = Some(row.node.key().clone());
        }
        kit::interact::lock_arrows(ctx, response.id, kit::interact::Arrows::Both);
        let mut jump = None;
        for k in kit::interact::plain_key_presses(ctx) {
            match tree::on_key(rows, jump.unwrap_or(cur_row), k, Modifiers::NONE, dir) {
                Some(TreeNav::Focus { row, .. }) => jump = Some(row),
                Some(TreeNav::Expand(j) | TreeNav::Collapse(j)) => {
                    if let Some(r) = rows.get(j) {
                        self.toggle(out, r.node.key().clone());
                    }
                }
                _ => {}
            }
        }
        if jump.is_none() {
            jump = state
                .typeahead
                .update(rt.core(), Some(cur_row), rows.iter().map(|r| r.node.label()));
        }
        if let Some(j) = jump
            && let Some(r) = rows.get(j)
        {
            state.stop = Some(r.node.key().clone());
            if let Some(target) = drawn.iter().find(|d| d.row == j) {
                ctx.memory_mut(|m| m.request_focus(target.response.id));
            } else {
                state.reveal = true;
                #[expect(clippy::disallowed_methods, reason = "one pass to draw and focus the culled target")]
                ctx.request_repaint();
            }
        }
    }

    fn toggle(&mut self, out: &mut TreeResponse<N::Key>, key: N::Key) {
        if self.expansion.toggle(key.clone()) {
            out.expanded = Some(key);
        } else {
            out.collapsed = Some(key);
        }
        out.changed = true;
    }
}

// Row `i` from the tops of `Tree::show`, without the gap below it.
fn rect_of_row(tops: &[f32], area: Rect, i: usize) -> Rect {
    let top = tops.get(i).copied().unwrap_or_default();
    let bottom = tops.get(i.saturating_add(1)).copied().unwrap_or(top);
    Rect::from_min_max(
        pos2(area.min.x, area.min.y + top),
        pos2(area.max.x, area.min.y + bottom - d::ROW_GAP),
    )
}
