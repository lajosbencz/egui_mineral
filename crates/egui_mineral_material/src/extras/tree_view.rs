use crate::MaterialUiExt as _;
use egui_mineral::kit::state::TempState;

use crate::Material;
use crate::MaterialExt;
use crate::Rt;
use crate::spec::TypeScale;
use egui_mineral::i18n::{I18n, RowCursor};
use egui_mineral::semantic::KeyedSalt;
use egui_mineral::widget::MineralUi;
use std::{collections::BTreeMap, f32::consts::FRAC_PI_2};

use egui::{Color32, DragAndDrop, Id, Key, Modifiers, Rangef, Rect, Response, Sense, StrokeKind, Ui, Vec2, pos2, vec2};
use egui_mineral::widget::Availability;
use egui_mineral::{
    dnd,
    expand::Expansion,
    focus::{self, Typeahead},
    i18n::Direction,
    kit,
    layout::{Constraints, Measure},
    perf::VirtualList,
    select::{Check, Selection},
    semantic::{self, Action, Role, SemanticNode, StableId},
    shortcut,
    tree::{self, DropPosition, TreeData, TreeMove, TreeNav, TreeRow},
    widget::{MineralWidget, WidgetFrame, WidgetResponse},
};

use crate::containment::{
    ListSlot, ListVisual,
    list_slot::{self, RowControl, SlotLayout},
};
use crate::widget::{
    common::{self, Interaction},
    indication, parts,
    row_parts::{self, RowColors, RowState},
};
use crate::{
    IconAction,
    communication::{CircularProgressIndicator, PlainTooltip},
};
use crate::{
    Size,
    actions::{IconButtonWidth, icon_button},
};
use crate::{
    icons, motion,
    spec::{
        defaults,
        tokens::{list, small_icon_button, standard_icon_button},
    },
    tokens::Tokens,
};

const ICON: f32 = list::ITEM_LEADING_ICON_SIZE;
const ACTION_SIZE: Size = Size::Small;
const PREVIEW_MAX_LABEL: f32 = 240.0;
// Tree may never render the row
const PENDING_PASSES: u8 = 4;

#[derive(Clone)]
struct TreeState<K> {
    pending: Option<K>,
    pending_passes: u8,
    stop: Option<K>,
    typeahead: Typeahead,
}

impl<K: Clone + Send + Sync + 'static> TempState for TreeState<K> {}

impl<K> Default for TreeState<K> {
    fn default() -> Self {
        Self {
            pending: None,
            pending_passes: 0,
            stop: None,
            typeahead: Typeahead::default(),
        }
    }
}

/// Trailing action of a [`TreeView`] row.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct TreeAction<K> {
    /// Row node key.
    pub node: K,
    /// Index into the row's [`TreeView::actions`].
    pub action: usize,
    /// Action target, e.g. to anchor a menu with [`Menu::show_at`](crate::selection::Menu::show_at).
    pub rect: Rect,
}

/// Result of [`TreeView::show`].
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct TreeViewResponse<K> {
    /// Tree container response.
    pub response: egui::Response,
    /// Node expanded this frame.
    pub expanded: Option<K>,
    /// Node collapsed this frame.
    pub collapsed: Option<K>,
    /// Node activated (double click) this frame.
    pub activated: Option<K>,
    /// Selection or expansion.
    pub changed: bool,
    /// Responses of the rendered rows, for context menus and modifiers.
    pub rows: BTreeMap<K, Response>,
    /// Row's leading [`ListControl`](crate::containment::ListControl) toggled (click, Space, agent); apply a checkbox with
    /// [`tree::toggle_check`] or app logic.
    pub leading_toggled: Option<K>,
    /// Trailing action without an [`IconAction::command`] clicked this frame.
    pub clicked: Option<TreeAction<K>>,
    /// Drag and drop or Alt+arrow move; apply with [`TreeMove::apply`].
    pub moved: Option<TreeMove<K>>,
}

impl<K> TreeViewResponse<K> {
    fn new(response: egui::Response) -> Self {
        Self {
            response,
            expanded: None,
            collapsed: None,
            activated: None,
            changed: false,
            rows: BTreeMap::new(),
            leading_toggled: None,
            clicked: None,
            moved: None,
        }
    }
}

#[derive(Default)]
struct RowOut {
    toggle: bool,
    select: Option<Modifiers>,
    activated: bool,
    toggled: bool,
    action: Option<(usize, Rect)>,
}

/// [`DragAndDrop`] payload of a [`reorderable`](TreeView::reorderable) row while it drags; drop zones
/// outside the tree read it with `Response::dnd_release_payload::<TreeDrag<K>>`.
#[derive(Clone, Debug)]
pub struct TreeDrag<K> {
    key: K,
    tree: Id,
}

impl<K> TreeDrag<K> {
    /// Dragged row.
    pub fn key(&self) -> &K {
        &self.key
    }
}

struct RowWidget<'a> {
    salt: KeyedSalt,
    key: StableId,
    label: &'a str,
    leading: ListSlot,
    depth: usize,
    has_children: bool,
    expanded: bool,
    loading: bool,
    selected: bool,
    look: RowLook,
    tab_stop: focus::TabStop,
    disabled: bool,
    actions: Vec<IconAction>,
}

// Row metrics shared by the rows and the drag preview, which renders in its own layer.
#[derive(Clone, Copy)]
struct RowLook {
    height: f32,
    indent: f32,
    // Start, end and inter-slot padding; the list's 16dp, narrowed by density.
    gap: f32,
    font: TypeScale,
}

impl RowLook {
    fn cursor(&self, rect: Rect, indent: f32, dir: Direction) -> RowCursor {
        let mut cur = dir.cursor(rect);
        cur.skip(self.gap + indent);
        cur
    }

    fn take_lead(&self, cur: &mut RowCursor, w: f32, h: f32) -> Rect {
        let r = cur.take(w, h);
        cur.skip(self.gap);
        r
    }
}

// Icon button minimum interactive size
fn action_slot(rt: &Rt) -> f32 {
    let (container, _) = icon_button::metrics(ACTION_SIZE, IconButtonWidth::Default);
    container.x.max(crate::adaptive::MIN_INTERACTIVE.points(rt.core()))
}

impl RowWidget<'_> {
    fn indent(&self) -> f32 {
        self.look.indent * self.depth as f32
    }

    fn chevron_rect(&self, rect: Rect, dir: Direction) -> Rect {
        self.look.cursor(rect, self.indent(), dir).peek(ICON, ICON)
    }

    // Leading slot after the chevron.
    fn leading(&self, ui: &MineralUi<'_, Material>, rect: Rect, dir: Direction) -> SlotLayout {
        let base = self.look.gap + self.indent() + ICON + self.look.gap;
        self.leading.layout(ui.rt(), ui.painter(), rect.height(), |at, w, h| {
            let x = dir.start_x(rect, Rangef::new(base + at, base + at + w));
            Rect::from_min_size(pos2(x, rect.center().y - h / 2.0), vec2(w, h))
        })
    }

    fn actions(&self, ui: &MineralUi<'_, Material>, r: &Response, rect: Rect, dir: Direction) -> Vec<Response> {
        let rt = ui.rt();
        let target = crate::adaptive::MIN_INTERACTIVE.points(rt.core()).min(rect.height());
        let slot = action_slot(rt);
        let n = self.actions.len();
        self.actions
            .iter()
            .enumerate()
            .map(|(i, a)| {
                let from_end = self.look.gap + (n - i) as f32 * slot;
                let left = dir.reversed().start_x(rect, Rangef::new(from_end - slot, from_end));
                let hit = Rect::from_min_size(pos2(left, rect.center().y - target / 2.0), vec2(slot, target));
                let enabled = !self.disabled && !a.disabled;
                kit::input::interact(
                    ui,
                    hit,
                    r.id.with(("mineral::tree_action", i)),
                    Sense::CLICK,
                    Availability::enabled_if(enabled),
                )
            })
            .collect()
    }

    fn paint_actions(&self, ui: &MineralUi<'_, Material>, actions: &[Response], color: Color32, t: &Tokens) {
        let rt = ui.rt();
        let (container, icon) = icon_button::metrics(ACTION_SIZE, IconButtonWidth::Default);
        let radius = small_icon_button::CONTAINER_SHAPE_ROUND.corner_radius(&t.shape, rt.i18n().direction());
        for (a, ar) in self.actions.iter().zip(actions) {
            let off = self.disabled || a.disabled;
            let col = if off {
                t.role_alpha(
                    standard_icon_button::DISABLED_COLOR,
                    standard_icon_button::DISABLED_OPACITY,
                )
            } else if a.selected == Some(true) {
                t.role(standard_icon_button::SELECTED_COLOR)
            } else {
                color
            };
            let part = parts::IconPart {
                icon: a.selected.map_or(a.icon, |s| common::selection_icon(a.icon, s)),
                icon_size: icon,
                layer: container,
                radius,
                color: col,
                availability: Availability::disabled_if(off),
            };
            parts::paint_icon_part(ui, ar, ar.rect.center(), &part, t);
            PlainTooltip::new(&a.label).show(rt, ar);
        }
    }

    fn control_semantics(&self, ui: &Ui, control: Option<&RowControl>, actions: &[Response]) {
        let ctx = ui.ctx();
        if let Some(c) = control {
            c.emit(
                ctx,
                self.key.sub(c.control.kind()),
                self.label,
                self.depth + 2,
                self.disabled,
            );
        }
        for (i, (a, r)) in self.actions.iter().zip(actions).enumerate() {
            semantic::emit(ctx, Some(r.id), r.rect, || {
                let mut n = SemanticNode::new(self.key.sub((i, "button")), Role::Button, &a.label)
                    .pressable(Availability::disabled_if(self.disabled || a.disabled));
                n.state.checked = a.selected.map(Check::from);
                n.depth = self.depth + 2;
                n
            });
        }
    }

    fn paint_chevron(
        &self,
        ui: &mut MineralUi<'_, Material>,
        r: &Response,
        chevron: Rect,
        dir: Direction,
        color: Color32,
    ) {
        let rt = ui.rt();
        if self.loading {
            let label = crate::tr(rt.i18n(), "mineral-tree-loading");
            CircularProgressIndicator::indeterminate(self.salt.child("loading"))
                .label(label)
                .agent_key(self.key.sub("progress"))
                .show_in(ui, Rect::from_center_size(chevron.center(), Vec2::splat(ICON)));
        } else if self.has_children {
            let icon = dir.pick(icons::ICON_CHEVRON_RIGHT, icons::ICON_CHEVRON_LEFT);
            let sign = dir.sign();
            let k = egui_mineral::motion::spring(
                ui.core(),
                r.id.with("chevron"),
                f32::from(self.expanded),
                motion::FAST_SPATIAL,
            );
            parts::icon_rotated(ui.painter(), chevron.center(), icon, ICON, color, sign * k * FRAC_PI_2);
        }
    }
}

impl MineralWidget for RowWidget<'_> {
    type Kit = Material;
    type State = ();
    type Output = RowOut;

    egui_mineral::kit::widget_meta!(salt, enabled, no_target);

    fn sense(&self) -> Sense {
        focus::roving_sense(Sense::click(), self.tab_stop)
    }

    fn layout(&self, _ui: &MineralUi<'_, Material>, c: Constraints, (): &Self::State) -> Measure {
        c.constrain(vec2(c.max.x, self.look.height)).into()
    }

    fn update(
        &mut self,
        ui: &mut MineralUi<'_, Material>,
        frame: &WidgetFrame<'_>,
        (): &mut (),
    ) -> WidgetResponse<RowOut> {
        let t = ui.tokens();
        let r = &frame.response;
        kit::focus::follow_press(r);
        let rect = frame.zones.visual;
        let dir = ui.i18n().direction();
        let mut out = RowOut::default();
        let chevron = self.chevron_rect(rect, dir);
        let chevron_hit = Rect::from_center_size(
            chevron.center(),
            Vec2::splat(crate::adaptive::MIN_INTERACTIVE.points(ui.core()).min(rect.height())),
        );
        let press = frame.press_pos();
        let lead = self.leading(ui, rect, dir);
        let mut control = lead
            .control
            .zip(self.leading.control_ref())
            .map(|(center, c)| RowControl::new(ui, r, "mineral::tree_control", c, center, rect, self.disabled));
        let actions = self.actions(ui, r, rect, dir);
        if let Some(c) = &mut control {
            c.toggled |= !self.disabled && r.has_focus() && ui.input(|i| i.key_pressed(Key::Space));
            out.toggled = c.toggled;
        }
        out.action = actions
            .iter()
            .enumerate()
            .find_map(|(i, a)| a.clicked().then_some((i, a.rect)));
        if let Some(c) = out.action.and_then(|(i, _)| self.actions[i].command.as_ref()) {
            shortcut::fire(ui.ctx(), c);
            out.action = None;
        }
        let on_control = out.toggled || out.action.is_some();
        if parts::press(ui, frame, Availability::disabled_if(self.disabled)).clicked && !on_control {
            if self.has_children && press.is_some_and(|p| chevron_hit.contains(p)) {
                out.toggle = true;
            } else {
                out.select = Some(ui.input(|i| i.modifiers));
            }
        }
        if r.double_clicked() {
            out.activated = true;
            if self.has_children {
                out.toggle = true;
            }
        }
        let ak = |a| kit::input::accesskit_requested(ui.ctx(), r.id, a);
        if self.has_children && ((ak(Action::Expand) && !self.expanded) || (ak(Action::Collapse) && self.expanded)) {
            out.toggle = true;
        }
        let row = RowState {
            selected: self.selected,
            disabled: self.disabled,
            dragged: false,
        };
        let col = RowColors::of(row, &t);
        let shape = row.shape(&t, dir);
        row_parts::paint_container(ui.painter(), rect, row, shape, &t);
        if !self.disabled {
            // A hovered control is still over its row (Compose hover reaches both).
            let on_control = control.iter().map(|c| &c.target).chain(&actions).any(Response::hovered);
            let i = Interaction::of(r, Availability::Enabled);
            let i = Interaction {
                hovered: i.hovered || on_control,
                ..i
            };
            indication::paint(ui, r, rect, shape, col.label, i, &t);
        }
        self.paint_chevron(ui, r, chevron, dir, col.leading);
        if let (Some(v), Some(vr)) = (self.leading.visual_ref(), lead.visual) {
            list_slot::paint_visual(ui, v, vr, self.disabled, col.leading, &t);
        }
        if let Some(c) = &control {
            c.paint(ui, self.disabled, &t);
        }
        let mut cur = self.look.cursor(rect, self.indent(), dir);
        self.look.take_lead(&mut cur, ICON, ICON);
        if lead.extent > 0.0 {
            self.look.take_lead(&mut cur, lead.extent, ICON);
        }
        self.paint_actions(ui, &actions, col.trailing, &t);
        let trail = if actions.is_empty() {
            0.0
        } else {
            self.look.gap + actions.len() as f32 * action_slot(ui.rt())
        };
        let avail = cur.remaining() - trail - self.look.gap;
        let p = ui.painter();
        let g = parts::type_wrapped(p, self.label, self.look.font, col.label, avail, 1);
        p.galley(cur.peek(g.size().x, g.size().y).min, g, Color32::PLACEHOLDER);
        self.control_semantics(ui, control.as_ref(), &actions);
        let changed = out.select.is_some();
        WidgetResponse::new(r.clone(), out).with_changed(changed)
    }

    fn semantics(&self, i18n: &I18n, _: &WidgetResponse<RowOut>, (): &()) -> Option<SemanticNode> {
        let mut n = SemanticNode::new(self.key.clone(), Role::TreeItem, self.label)
            .clickable(Availability::disabled_if(self.disabled));
        n.depth = self.depth + 1;
        n.state.selected = Some(self.selected);
        if self.has_children {
            n.state.expanded = Some(self.expanded);
            if !self.disabled {
                n.actions.push(if self.expanded {
                    Action::Collapse
                } else {
                    Action::Expand
                });
            }
        }
        if self.loading {
            n.value = Some(crate::tr(i18n, "mineral-tree-loading"));
        }
        Some(n)
    }
}

type LeadingFn<'a, N> = Box<dyn Fn(&N) -> ListSlot + 'a>;
type ActionsFn<'a, N> = Box<dyn Fn(&N) -> Vec<IconAction> + 'a>;

/// MD3-styled tree (APG treeview keyboard). Rows carry an optional leading [`ListSlot`] (Space
/// toggles its control on the focused row) and trailing actions. With
/// [`reorderable`](Self::reorderable), rows drag onto, before or after other rows, and Alt+arrows
/// move the focused row ([`tree::move_key`]).
#[must_use = "call .show(..)"]
pub struct TreeView<'a, N: TreeData> {
    salt: KeyedSalt,
    label: String,
    roots: &'a [N],
    expansion: &'a mut Expansion<N::Key>,
    selection: Option<&'a mut Selection<N::Key>>,
    max_height: Option<f32>,
    row_height: Option<f32>,
    indent: Option<f32>,
    label_style: TypeScale,
    leading: Option<LeadingFn<'a, N>>,
    actions: Option<ActionsFn<'a, N>>,
    reorderable: bool,
    disabled: bool,
}

impl<N: TreeData> std::fmt::Debug for TreeView<'_, N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TreeView").finish_non_exhaustive()
    }
}

impl<'a, N: TreeData> TreeView<'a, N> {
    /// `label` names the tree for assistive technology; `expansion` is the bound set of open nodes. Agent key prefix `<id_salt>`.
    pub fn new(
        id_salt: impl Into<KeyedSalt>,
        label: impl Into<String>,
        roots: &'a [N],
        expansion: &'a mut Expansion<N::Key>,
    ) -> Self {
        let id_salt: KeyedSalt = id_salt.into();
        Self {
            salt: id_salt,
            label: label.into(),
            roots,
            expansion,
            selection: None,
            max_height: None,
            row_height: None,
            indent: None,
            label_style: list::ITEM_LABEL_TEXT_FONT,
            leading: None,
            actions: None,
            reorderable: false,
            disabled: false,
        }
    }

    crate::widget::setters! {
        /// Replaces the `<id_salt>` key prefix.
        agent_key(salt)
    }

    /// Rows select on click, Space and Enter; without one rows only activate.
    pub fn selection(mut self, selection: &'a mut Selection<N::Key>) -> Self {
        self.selection = Some(selection);
        self
    }

    crate::widget::setters! {
        /// Greys out every row; nothing toggles or selects.
        disabled
    }

    /// Leading slot per node, e.g. a checkbox derived with [`tree::check_state`] and an icon.
    pub fn leading(mut self, leading: impl Fn(&N) -> ListSlot + 'a) -> Self {
        self.leading = Some(Box::new(leading));
        self
    }

    /// Trailing icon buttons per node; clicks report [`TreeViewResponse::clicked`].
    pub fn actions(mut self, actions: impl Fn(&N) -> Vec<IconAction> + 'a) -> Self {
        self.actions = Some(Box::new(actions));
        self
    }

    /// Drag and Alt+arrow moves; reported in [`TreeViewResponse::moved`].
    pub fn reorderable(mut self, reorderable: bool) -> Self {
        self.reorderable = reorderable;
        self
    }

    /// Body height cap in dp; rows scroll past it; non-positive values snap to the minimum.
    #[track_caller]
    pub fn max_height(mut self, max_height: f32) -> Self {
        self.max_height = Some(egui_mineral::kit::sanitize::at_least(
            max_height,
            f32::MIN_POSITIVE..,
            "max_height",
        ));
        self
    }

    /// Row height in dp; default the one-line list item height at the [`ui.density()`](crate::MaterialUiExt::density).
    /// Clamped to the 24dp icon slot.
    #[track_caller]
    pub fn row_height(mut self, row_height: f32) -> Self {
        self.row_height = Some(egui_mineral::kit::sanitize::at_least(row_height, ICON.., "row_height"));
        self
    }

    /// Inset per depth level in dp; default the icon slot and gap, 40 at default density. Negative
    /// values snap to 0.
    #[track_caller]
    pub fn indent(mut self, indent: f32) -> Self {
        self.indent = Some(egui_mineral::kit::sanitize::at_least(indent, 0.0.., "indent"));
        self
    }

    /// Label type style; default [`TypeScale::BodyLarge`], the list item label.
    pub fn label_style(mut self, label_style: TypeScale) -> Self {
        self.label_style = label_style;
        self
    }

    fn look(&self, ui: &MineralUi<'_, Material>) -> RowLook {
        let density = ui.density();
        // MD3 list paddings are fixed; here they narrow 4dp per density step (docs/divergences.md).
        let gap = density.dense(
            defaults::list::LEADING_CONTENT_END_PADDING,
            crate::adaptive::Density::MIN,
        );
        RowLook {
            height: self.row_height.unwrap_or_else(|| row_parts::row_height(density)),
            indent: self.indent.unwrap_or(ICON + gap),
            gap,
            font: self.label_style,
        }
    }

    fn row_widget(&self, row: &TreeRow<'a, N>, look: RowLook, tab_stop: focus::TabStop) -> RowWidget<'a> {
        let key = row.node.key();
        let expanded = row.has_children && self.expansion.is_expanded(key);
        let salt = self.salt.child(key);
        RowWidget {
            key: StableId::of(&salt, "treeitem"),
            salt,
            label: row.node.label(),
            leading: self.leading.as_ref().map_or_default(|f| f(row.node)),
            depth: row.depth,
            has_children: row.has_children,
            expanded,
            loading: expanded && row.node.lazy() && row.node.children().is_empty(),
            selected: self.selection.as_ref().is_some_and(|s| s.is_selected(key)),
            look,
            tab_stop,
            disabled: self.disabled,
            actions: self.actions.as_ref().map_or_default(|f| f(row.node)),
        }
    }

    fn drop_target(
        &self,
        ui: &MineralUi<'_, Material>,
        id: Id,
        area: Rect,
        rows: &[TreeRow<'a, N>],
        rendered: &[(usize, Rect)],
    ) -> Option<TreeMove<N::Key>> {
        let rt = ui.rt();
        let ctx = ui.ctx();
        let payload = DragAndDrop::payload::<TreeDrag<N::Key>>(ctx).filter(|p| p.tree == id)?;
        let pos = ctx.pointer_interact_pos().filter(|p| area.contains(*p))?;
        let &(i, rect) = rendered.iter().find(|(_, r)| r.y_range().contains(pos.y))?;
        let tree::DropTarget {
            row: target, change, ..
        } = tree::drop_at(self.roots, rows, &payload.key, i, (pos.y - rect.top()) / rect.height())?;
        let rect = rect.translate(vec2(0.0, (target as f32 - i as f32) * rect.height()));
        let t = rt.tokens();
        let p = ui.painter_at(area);
        let stroke = row_parts::drop_indicator(&t);
        let dir = rt.i18n().direction();
        let look = self.look(ui);
        let from = look.cursor(rect, look.indent * rows[target].depth as f32, dir).x();
        let to = dir.reversed().start_x(rect, Rangef::point(look.gap));
        let line = |y: f32| {
            p.hline(egui::Rangef::new(from.min(to), from.max(to)), y, stroke);
        };
        match change.position {
            DropPosition::Before => line(rect.top()),
            DropPosition::After => line(rect.bottom()),
            DropPosition::Inside => {
                let shape = list::ITEM_CONTAINER_SHAPE.corner_radius(&t.shape, dir);
                p.rect_stroke(rect, shape, stroke, StrokeKind::Inside);
            }
        }
        if !ui.input(|i| i.pointer.any_released()) {
            return None;
        }
        DragAndDrop::clear_payload(ctx);
        Some(change)
    }

    fn drag_preview(ui: &mut MineralUi<'_, Material>, look: RowLook, label: &str, visual: Option<&ListVisual>) {
        let rt = ui.rt();
        let t = rt.tokens();
        let row = RowState {
            dragged: true,
            ..RowState::default()
        };
        let col = RowColors::of(row, &t);
        let g = parts::type_wrapped(ui.painter(), label, look.font, col.label, PREVIEW_MAX_LABEL, 1);
        let slot = visual.cloned().map_or_default(ListSlot::from);
        let row_h = look.height;
        let measured = slot.layout(rt, ui.painter(), row_h, |_, w, h| {
            Rect::from_min_size(egui::Pos2::ZERO, vec2(w, h))
        });
        let lead = if measured.extent > 0.0 {
            measured.extent + look.gap
        } else {
            0.0
        };
        let pad = 2.0 * look.gap;
        let size = vec2(g.size().x + lead + pad, row_h);
        let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
        let p = ui.painter();
        row_parts::paint_container(p, rect, row, row.shape(&t, rt.i18n().direction()), &t);
        let dir = rt.i18n().direction();
        let mut cur = look.cursor(rect, 0.0, dir);
        if let Some(v) = visual {
            let vr = look.take_lead(&mut cur, measured.extent, row_h);
            let vr = Rect::from_center_size(vr.center(), measured.visual.map_or(Vec2::ZERO, |r| r.size()));
            list_slot::paint_visual(ui, v, vr, false, col.leading, &t);
        }
        let p = ui.painter();
        p.galley(cur.peek(g.size().x, g.size().y).min, g, Color32::PLACEHOLDER);
    }

    fn pending_row(state: &mut TreeState<N::Key>, row_of: impl Fn(&N::Key) -> Option<usize>) -> Option<usize> {
        let mut pending = state.pending.as_ref().and_then(row_of);
        if pending.is_some() {
            state.pending_passes = state.pending_passes.saturating_add(1);
            if state.pending_passes > PENDING_PASSES {
                tracing::debug!(row = ?pending, "tree scroll-to-row never rendered; dropped");
                pending = None;
            }
        }
        if pending.is_none() {
            state.pending = None;
        }
        pending
    }

    fn toggle_node(out: &mut TreeViewResponse<N::Key>, exp: &mut Expansion<N::Key>, key: N::Key) {
        if exp.toggle(key.clone()) {
            out.expanded = Some(key);
        } else {
            out.collapsed = Some(key);
        }
    }

    fn apply_events(
        &mut self,
        rows: &[TreeRow<'a, N>],
        events: Vec<(usize, RowOut)>,
        out: &mut TreeViewResponse<N::Key>,
    ) {
        let mut visible: Option<Vec<N::Key>> = None;
        for (i, o) in events {
            let key = rows[i].node.key().clone();
            if o.toggle {
                Self::toggle_node(out, self.expansion, key.clone());
            }
            if let (Some(m), Some(selection)) = (o.select, self.selection.as_deref_mut()) {
                let visible = visible.get_or_insert_with(|| rows.iter().map(|r| r.node.key().clone()).collect());
                selection.click(key.clone(), m, visible);
                out.changed = true;
            }
            if o.toggled {
                out.leading_toggled = Some(key.clone());
            }
            if let Some((action, rect)) = o.action {
                out.clicked = Some(TreeAction {
                    node: key.clone(),
                    action,
                    rect,
                });
            }
            if o.activated {
                out.activated = Some(key);
            }
        }
    }

    fn can_reorder(&self) -> bool {
        self.reorderable && !self.disabled
    }

    fn navigate(
        &mut self,
        rt: &Rt,
        rows: &[TreeRow<'a, N>],
        cur: usize,
        state: &mut TreeState<N::Key>,
        out: &mut TreeViewResponse<N::Key>,
    ) {
        let ctx = rt.ctx();
        let dir = rt.i18n().direction();
        for (k, m) in kit::input::key_presses(ctx) {
            if m.alt && self.can_reorder() {
                if let Some(mv) = tree::move_key(self.roots, rows, cur, k, dir) {
                    state.pending = Some(mv.node.clone());
                    state.pending_passes = 0;
                    out.moved = Some(mv);
                    // Later keys would be computed against the unmoved tree.
                    break;
                }
                continue;
            }
            match tree::on_key(rows, cur, k, dir) {
                Some(TreeNav::Focus(j)) => {
                    state.pending = Some(rows[j].node.key().clone());
                    state.pending_passes = 0;
                }
                Some(TreeNav::Expand(j) | TreeNav::Collapse(j)) => {
                    Self::toggle_node(out, self.expansion, rows[j].node.key().clone());
                }
                _ => {}
            }
        }
        if let Some(j) = state
            .typeahead
            .update(rt.core(), Some(cur), rows.iter().map(|r| r.node.label()))
        {
            state.pending = Some(rows[j].node.key().clone());
            state.pending_passes = 0;
        }
    }

    fn row_response(
        &self,
        ui: &mut MineralUi<'_, Material>,
        w: RowWidget<'_>,
        key: &N::Key,
        id: Id,
        drag_dim: Option<f32>,
    ) -> WidgetResponse<RowOut> {
        let rt = ui.rt();
        if let Some(dim) = drag_dim {
            let (label, visual, look) = (w.label, w.leading.visual_ref().cloned(), w.look);
            dnd::drag_source(
                ui,
                self.salt.child(("drag", key)),
                || TreeDrag {
                    tree: id,
                    key: key.clone(),
                },
                |ui| Self::drag_preview(&mut rt.ui(ui), look, label, visual.as_ref()),
                |ui, dragged| {
                    if dragged {
                        ui.multiply_opacity(dim);
                    }
                    rt.ui(ui).mineral(w)
                },
            )
            .inner
        } else {
            rt.ui(ui).mineral(w)
        }
    }

    /// Paints the visible rows, virtualized.
    pub fn show(mut self, ui: &mut MineralUi<'_, Material>) -> TreeViewResponse<N::Key> {
        let rt = ui.rt();
        let ctx = rt.ctx();
        let id = ui.make_persistent_id(self.salt.id_salt());
        let rows: Vec<TreeRow<'a, N>> = tree::flatten(self.roots, self.expansion);
        let mut state = TreeState::<N::Key>::load(ctx, id);
        let row_of = |k: &N::Key| rows.iter().position(|r| r.node.key() == k);
        let pending = Self::pending_row(&mut state, row_of);
        let present = |k: &N::Key| row_of(k).is_some();
        let stop = state
            .pending
            .clone()
            .or_else(|| state.stop.clone().filter(present))
            .or_else(|| self.selection.as_ref().and_then(|s| s.first().cloned()).filter(present))
            .or_else(|| rows.first().map(|r| r.node.key().clone()));
        let look = self.look(ui);
        let row_h = look.height;
        let height = self
            .max_height
            .unwrap_or_else(|| ui.available_height())
            .min(row_h * rows.len() as f32);
        let width = ui.available_width();
        let (area, response) = ui.allocate_exact_size(vec2(width, height), Sense::hover());
        let mut out = TreeViewResponse::new(response);
        semantic::emit(ctx, None, area, || {
            let mut n = SemanticNode::new(StableId::of(&self.salt, "tree"), Role::Tree, &self.label);
            n.state.disabled = self.disabled;
            n
        });
        let mut focused: Option<(usize, egui::Id)> = None;
        let mut events: Vec<(usize, RowOut)> = Vec::new();
        let mut rendered: Vec<(usize, Rect)> = Vec::new();
        let mut first = true;
        let reorderable = self.can_reorder();
        let dim = rt.tokens().state.disabled_content;
        let scope = self.salt.child("rows");
        egui_mineral::layout::child(ui, scope, area, rt.i18n().direction().column(), |ui| {
            ui.spacing_mut().item_spacing.y = 0.0;
            VirtualList::uniform(self.salt.child("rows"), row_h).show(ui, rows.len(), |ui, i| {
                if std::mem::take(&mut first)
                    && let Some(p) = pending
                {
                    let top = ui.cursor().min.y + (p as f32 - i as f32) * row_h;
                    ui.scroll_to_rect(
                        Rect::from_min_size(pos2(ui.cursor().min.x, top), vec2(1.0, row_h)),
                        None,
                    );
                    #[expect(clippy::disallowed_methods, reason = "one pass to apply the pending scroll-to-row")]
                    ctx.request_repaint();
                }
                let row = &rows[i];
                let key = row.node.key();
                let tab_stop = if stop.as_ref() == Some(key) {
                    focus::TabStop::Stop
                } else {
                    focus::TabStop::Skip
                };
                let w = self.row_widget(row, look, tab_stop);
                let r = self.row_response(&mut rt.ui(ui), w, key, id, reorderable.then_some(dim));
                rendered.push((i, r.rect));
                if pending == Some(i) {
                    ctx.memory_mut(|m| m.request_focus(r.id));
                    state.pending = None;
                }
                if r.has_focus() {
                    focused = Some((i, r.id));
                    state.stop = Some(key.clone());
                }
                let (resp, o) = r.into_parts();
                out.rows.insert(key.clone(), resp);
                if o.toggle || o.select.is_some() || o.activated || o.toggled || o.action.is_some() {
                    events.push((i, o));
                }
            });
        });
        if reorderable {
            out.moved = self.drop_target(ui, id, area, &rows, &rendered);
        }

        self.apply_events(&rows, events, &mut out);
        if let Some((cur, fid)) = focused {
            kit::focus::lock_arrows(ctx, fid, kit::focus::Arrows::Both);
            self.navigate(rt, &rows, cur, &mut state, &mut out);
        }
        if let Some(m) = &out.moved
            && m.position == DropPosition::Inside
            && !self.expansion.is_expanded(&m.target)
        {
            self.expansion.expand(m.target.clone());
            out.expanded = Some(m.target.clone());
        }
        state.store(ctx, id);
        if out.expanded.is_some() || out.collapsed.is_some() {
            out.changed = true;
        }
        if out.changed {
            out.response.mark_changed();
        }
        out
    }
}
