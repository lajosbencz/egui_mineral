/// Serializers of the semantic tree.
pub mod serialize;

mod key;

pub use egui::accesskit::{Action, Live, Role};
use std::collections::HashMap;

use egui::{Context, Id, KeyboardShortcut, LayerId, Rect, WidgetRects};
pub use egui_mineral_macros::KeySalt;
pub use key::{AgentKey, KeyKind, KeyParts, KeySalt, KeyedSalt, StableId};

use crate::{
    kit::nodes::{push_from, wanted},
    select::Check,
    shortcut::Mnemonic,
    widget::Availability,
};

/// Accessibility states of a [`SemanticNode`].
#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[non_exhaustive]
pub struct SemanticState {
    /// Visible but not operable.
    pub disabled: bool,
    /// Holds keyboard focus this pass.
    pub focused: bool,
    /// Value shown but not editable.
    pub read_only: bool,
    /// Form field that must be filled.
    pub required: bool,
    /// Value failed validation.
    pub invalid: bool,
    /// `Some` for checkable nodes (checkbox, switch, radio).
    pub checked: Option<Check>,
    /// `Some` for selectable items (tabs, list options).
    pub selected: Option<bool>,
    /// `Some` for expandable nodes (menus, tree items, accordions).
    pub expanded: Option<bool>,
    /// `Some` for ranged nodes (slider, progress, spin button).
    pub range: Option<NumericRange>,
    /// `Some` for live regions screen readers announce on change.
    pub live: Option<Live>,
    /// 1-based level of a tree item in its data hierarchy (ARIA `aria-level`); rows of a tree are siblings.
    pub level: Option<usize>,
}

/// Value, bounds and step of a ranged node.
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize)]
pub struct NumericRange {
    /// Current value within `min..=max`.
    pub value: f64,
    /// Lower bound, inclusive.
    pub min: f64,
    /// Upper bound, inclusive.
    pub max: f64,
    /// Increment of AccessKit `Increment`/`Decrement`; `None` when continuous.
    pub step: Option<f64>,
}

/// One accessible element: stable key, role, name, value, state and actions.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct SemanticNode {
    /// Agent-facing key; unique per pass (duplicates warn, agents target the first).
    pub id: StableId,
    /// AccessKit role.
    pub role: Role,
    /// Accessible name; may be translated, so never a key source.
    pub name: String,
    /// Current value as text (field contents, slider value).
    pub value: Option<String>,
    /// Accessibility states.
    pub state: SemanticState,
    /// AccessKit actions the node accepts.
    pub actions: Vec<Action>,
    /// Hit rect in screen points; set by [`emit`].
    pub rect: Rect,
    /// egui id of the widget; `None` for tree-only nodes without an AccessKit node.
    pub widget_id: Option<Id>,
    /// Widget id of the highlighted item a composite (combobox, listbox) controls while it keeps focus.
    pub active_descendant: Option<Id>,
    /// Accelerator; AccessKit gets its `shortcut::label`, the agent markdown platform-neutral names (`Ctrl+S`).
    pub shortcut: Option<KeyboardShortcut>,
    /// Mnemonic letter that activates the node while its menu or dialog is active.
    pub access_key: Option<Mnemonic>,
    /// Ancestors in the [`SemanticTree`]; set when the tree is built.
    pub depth: usize,
}

impl SemanticNode {
    /// Focusable `Role::Button` node.
    pub fn button(id: StableId, name: impl Into<String>, availability: Availability) -> Self {
        Self::new(id, Role::Button, name).clickable(availability)
    }

    /// Click and Focus actions while enabled; `state.disabled` otherwise.
    #[must_use]
    pub fn clickable(mut self, availability: Availability) -> Self {
        self.state.disabled = !availability.is_enabled();
        if availability.is_enabled() {
            self.actions.extend([Action::Click, Action::Focus]);
        }
        self
    }

    /// Click action while enabled, for parts that take no focus of their own; `state.disabled` otherwise.
    #[must_use]
    pub fn pressable(mut self, availability: Availability) -> Self {
        self.state.disabled = !availability.is_enabled();
        if availability.is_enabled() {
            self.actions.push(Action::Click);
        }
        self
    }

    /// Range plus Focus, `SetValue`, Increment and Decrement actions while enabled; `state.disabled` otherwise.
    #[must_use]
    pub fn ranged(mut self, range: NumericRange, availability: Availability) -> Self {
        self.state.range = Some(range);
        self.state.disabled = !availability.is_enabled();
        if availability.is_enabled() {
            self.actions
                .extend([Action::Focus, Action::SetValue, Action::Increment, Action::Decrement]);
        }
        self
    }

    /// Focus and `SetValue` actions while enabled; `state.disabled` otherwise.
    #[must_use]
    pub fn editable(mut self, availability: Availability) -> Self {
        self.state.disabled = !availability.is_enabled();
        if availability.is_enabled() {
            self.actions.extend([Action::Focus, Action::SetValue]);
        }
        self
    }

    /// Text shown to agents; formatted by the caller.
    #[must_use]
    pub fn value(mut self, value: impl Into<String>) -> Self {
        self.value = Some(value.into());
        self
    }

    /// Replaces the action list.
    #[must_use]
    pub fn actions(mut self, actions: impl IntoIterator<Item = Action>) -> Self {
        self.actions = actions.into_iter().collect();
        self
    }

    /// Accelerator that activates the node.
    #[must_use]
    pub fn shortcut(mut self, shortcut: Option<KeyboardShortcut>) -> Self {
        self.shortcut = shortcut;
        self
    }

    /// Starts with no value, default state, no actions and `Rect::NOTHING`.
    pub fn new(id: StableId, role: Role, name: impl Into<String>) -> Self {
        Self {
            id,
            role,
            name: name.into(),
            value: None,
            state: SemanticState::default(),
            actions: Vec::new(),
            rect: Rect::NOTHING,
            widget_id: None,
            active_descendant: None,
            shortcut: None,
            access_key: None,
            depth: 0,
        }
    }
}

/// Nodes of one pass, built only when an agent or inspector asks.
///
/// A node's parent is the nearest node up egui's widget hierarchy (`WidgetRect::parent_id`), so a container
/// node uses the id of the `Ui` holding its children. A popup layer continues at its owner
/// ([`own_layer`](crate::kit::nodes::own_layer)).
#[derive(Clone, Debug, Default, PartialEq)]
#[non_exhaustive]
pub struct SemanticTree {
    /// Pre-order, siblings in emission order: each node is followed by its descendants, which have a greater
    /// [`SemanticNode::depth`].
    pub nodes: Vec<SemanticNode>,
}

impl SemanticTree {
    /// First node with `key`.
    pub fn find_key(&self, key: &str) -> Option<&SemanticNode> {
        self.nodes.iter().find(|n| n.id.key() == key)
    }

    /// First node with `key` followed by its descendants.
    pub fn subtree(&self, key: &str) -> Option<&[SemanticNode]> {
        let start = self.nodes.iter().position(|n| n.id.key() == key)?;
        let depth = self.nodes.get(start)?.depth;
        let rest = self.nodes.get(start + 1..).unwrap_or_default();
        let len = rest.iter().position(|n| n.depth <= depth).unwrap_or(rest.len());
        self.nodes.get(start..=start + len)
    }
}

#[expect(clippy::into_iter_without_iter, reason = "`nodes` is public")]
impl<'a> IntoIterator for &'a SemanticTree {
    type Item = &'a SemanticNode;
    type IntoIter = std::slice::Iter<'a, SemanticNode>;

    fn into_iter(self) -> Self::IntoIter {
        self.nodes.iter()
    }
}

#[derive(Clone, Default)]
pub(crate) struct Collector {
    pub(crate) wanted: bool,
    pub(crate) tree: SemanticTree,
    // Per node: where the hierarchy walk starts when its widget id has no `WidgetRect` this pass.
    pub(crate) anchors: Vec<Option<Id>>,
    // Per node: innermost widget scope open when it was pushed.
    pub(crate) in_scope: Vec<Option<usize>>,
    pub(crate) owners: HashMap<LayerId, Id>,
    pub(crate) scopes: Vec<Scope>,
    pub(crate) open: Vec<usize>,
}

// A widget's `update`: a walk from a node pushed inside it that reaches `ui` before any node nests under
// `widget`, so the widget's parts nest under it although egui places them beside it.
#[derive(Clone, Copy)]
pub(crate) struct Scope {
    widget: Id,
    ui: Id,
    outer: Option<usize>,
}

pub(crate) fn enter(ctx: &Context, id: Id, ui: Id) -> Option<usize> {
    Pass::with(ctx, |c| {
        c.wanted.then(|| {
            let i = c.scopes.len();
            c.scopes.push(Scope {
                widget: id,
                ui,
                outer: c.open.last().copied(),
            });
            c.open.push(i);
            i
        })
    })
}

// `id` replaces the owner's id when its node uses another response than the one the scope opened with.
pub(crate) fn leave(ctx: &Context, i: Option<usize>, id: Option<Id>) {
    let Some(i) = i else { return };
    Pass::with(ctx, |c| {
        if let Some(pos) = c.open.iter().rposition(|&o| o == i) {
            c.open.truncate(pos);
        }
        if let (Some(s), Some(id)) = (c.scopes.get_mut(i), id) {
            s.widget = id;
        }
    });
}

pub(crate) type Pass = crate::runtime::PassScope<Collector>;

pub(crate) fn begin_pass(ctx: &Context, wanted: bool) {
    Pass::set(
        ctx,
        Collector {
            wanted,
            ..Collector::default()
        },
    );
}

pub(crate) fn collecting(ctx: &Context) -> bool {
    Pass::read(ctx, |c| c.is_some_and(|c| c.wanted))
}

/// Builds `node` only when [`wanted`], then writes it to AccessKit (with an `id`) and the tree. `id` is the
/// widget's, or for a container the id of the `Ui` holding its children; a node without one has no parent.
pub fn emit(ctx: &Context, id: Option<Id>, rect: Rect, node: impl FnOnce() -> SemanticNode) {
    if wanted(ctx, id) {
        write(ctx, id, rect, None, node);
    }
}

/// [`emit`] of a node with no widget this pass, nested under `ui`'s nearest node.
pub fn emit_in(ui: &egui::Ui, id: Option<Id>, rect: Rect, node: impl FnOnce() -> SemanticNode) {
    if wanted(ui.ctx(), id) {
        write(ui.ctx(), id, rect, Some(ui.unique_id()), node);
    }
}

fn write(ctx: &Context, id: Option<Id>, rect: Rect, anchor: Option<Id>, node: impl FnOnce() -> SemanticNode) {
    let mut node = node();
    node.rect = rect;
    node.widget_id = id;
    if let Some(id) = id {
        serialize::accesskit::write(ctx, id, &node);
    }
    push_from(ctx, node, anchor);
}

pub(crate) fn end_pass(ctx: &Context) -> Option<SemanticTree> {
    let collected = Pass::take(ctx).filter(|c| c.wanted)?;
    let tree = ctx.viewport(|v| nest(collected, &v.this_pass.widgets));
    let mut seen = std::collections::HashSet::with_capacity(tree.nodes.len());
    for n in &tree.nodes {
        if !seen.insert(n.id.key()) {
            crate::kit::sanitize::report(format_args!(
                "duplicate agent key {:?}; agents target the first",
                n.id.key()
            ));
        }
    }
    Some(tree)
}

// egui bounds its own parent walks the same way, against id clashes forming a cycle.
const MAX_WALK: usize = 1000;

fn nest(c: Collector, widgets: &WidgetRects) -> SemanticTree {
    let mut by_widget = HashMap::new();
    for (i, n) in c.tree.nodes.iter().enumerate() {
        if let Some(id) = n.widget_id {
            by_widget.entry(id).or_insert(i);
        }
    }
    let count = c.tree.nodes.len();
    let mut children = vec![Vec::new(); count];
    let mut roots = Vec::new();
    let mut parents = Vec::with_capacity(count);
    for (i, n) in c.tree.nodes.iter().enumerate() {
        let start = n
            .widget_id
            .filter(|&id| widgets.get(id).is_some())
            .or_else(|| c.anchors.get(i).copied().flatten());
        let walk = Walk {
            own: n.widget_id,
            scope: c.in_scope.get(i).copied().flatten(),
            by_widget: &by_widget,
            widgets,
            c: &c,
        };
        parents.push(start.and_then(|s| walk.ancestor(s)));
    }
    // Reading order: a subtree sorts by its first emitted node, so a parent emitted after its children
    // keeps their place.
    let mut first: Vec<usize> = (0..count).collect();
    for i in 0..count {
        let mut p = parents.get(i).copied().flatten();
        for _ in 0..MAX_WALK {
            let Some(at) = p.and_then(|p| first.get_mut(p)) else {
                break;
            };
            *at = (*at).min(i);
            p = p.and_then(|p| parents.get(p).copied().flatten());
        }
    }
    for (i, p) in parents.iter().enumerate() {
        match p.and_then(|p| children.get_mut(p)) {
            Some(siblings) => siblings.push(i),
            None => roots.push(i),
        }
    }
    let order = |i: &usize| first.get(*i).copied().unwrap_or(*i);
    roots.sort_by_key(order);
    for kids in &mut children {
        kids.sort_by_key(order);
    }
    let mut slots: Vec<Option<SemanticNode>> = c.tree.nodes.into_iter().map(Some).collect();
    let mut nodes = Vec::with_capacity(count);
    let mut stack: Vec<(usize, usize)> = Vec::new();
    // Nodes in a parent cycle are reached by none of the roots; they become roots after them.
    for root in roots.into_iter().chain(0..count) {
        stack.push((root, 0));
        while let Some((i, depth)) = stack.pop() {
            let Some(mut node) = slots.get_mut(i).and_then(Option::take) else {
                continue;
            };
            node.depth = depth;
            nodes.push(node);
            if let Some(kids) = children.get(i) {
                stack.extend(kids.iter().rev().map(|&k| (k, depth + 1)));
            }
        }
    }
    SemanticTree { nodes }
}

struct Walk<'a> {
    own: Option<Id>,
    scope: Option<usize>,
    by_widget: &'a HashMap<Id, usize>,
    widgets: &'a WidgetRects,
    c: &'a Collector,
}

impl Walk<'_> {
    fn node(&self, id: Id) -> Option<usize> {
        (Some(id) != self.own)
            .then(|| self.by_widget.get(&id).copied())
            .flatten()
    }

    fn scope_at(&self, ui: Id) -> Option<usize> {
        let mut s = self.scope;
        for _ in 0..MAX_WALK {
            let scope = self.c.scopes.get(s?)?;
            if scope.ui == ui
                && let Some(i) = self.node(scope.widget)
            {
                return Some(i);
            }
            s = scope.outer;
        }
        None
    }

    fn ancestor(&self, start: Id) -> Option<usize> {
        let mut id = start;
        for _ in 0..MAX_WALK {
            if let Some(i) = self.node(id).or_else(|| self.scope_at(id)) {
                return Some(i);
            }
            let w = self.widgets.get(id)?;
            id = if w.parent_id == id {
                *self.c.owners.get(&w.layer_id)?
            } else {
                w.parent_id
            };
        }
        None
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn tree_iterates_by_reference() {
        use super::{Role, SemanticNode, SemanticTree, StableId};

        let tree = SemanticTree {
            nodes: vec![
                SemanticNode::new(StableId::new("a"), Role::Button, "a"),
                SemanticNode::new(StableId::new("b"), Role::Button, "b"),
            ],
        };
        let keys: Vec<&str> = (&tree).into_iter().map(|n| n.id.key()).collect();
        assert_eq!(keys, ["a", "b"]);
        let mut count = 0;
        for _ in &tree {
            count += 1;
        }
        assert_eq!(count, 2);
    }

    fn tree_of(add: impl Fn(&mut egui::Ui)) -> Vec<(String, usize)> {
        use crate::test_support::{ctx, input, run, runtime};

        let ctx = ctx();
        let rt = runtime(&ctx);
        rt.set_collect_semantics(true);
        let out = run(&ctx, input(vec![]), |ui| rt.pass(ui, |ui| add(ui)));
        drop(out);
        let tree = rt.tree(egui::ViewportId::ROOT).expect("tree collected");
        tree.nodes.iter().map(|n| (n.id.key().to_owned(), n.depth)).collect()
    }

    fn node(ui: &egui::Ui, id: Option<egui::Id>, key: &'static str) {
        super::emit(ui.ctx(), id, egui::Rect::ZERO, || {
            super::SemanticNode::new(super::StableId::new(key), super::Role::Group, key)
        });
    }

    fn keyed(nodes: &[(&str, usize)]) -> Vec<(String, usize)> {
        nodes.iter().map(|&(k, d)| (k.to_owned(), d)).collect()
    }

    #[test]
    fn a_container_nests_the_nodes_of_its_ui_wherever_it_emits() {
        let tree = tree_of(|ui| {
            ui.scope(|ui| {
                let b = ui.interact(egui::Rect::ZERO, egui::Id::unique("b"), egui::Sense::click());
                node(ui, Some(b.id), "button");
                super::emit_in(ui, None, egui::Rect::ZERO, || {
                    super::SemanticNode::new(super::StableId::new("text"), super::Role::Label, "t")
                });
                node(ui, Some(ui.unique_id()), "group");
            });
            node(ui, Some(egui::Id::unique("unplaced")), "unplaced");
        });
        assert_eq!(
            tree,
            keyed(&[("group", 0), ("button", 1), ("text", 1), ("unplaced", 0)])
        );
    }

    #[test]
    fn parts_beside_their_owner_nest_under_it() {
        let tree = tree_of(|ui| {
            let owner = ui.interact(egui::Rect::ZERO, egui::Id::unique("o"), egui::Sense::hover());
            let parts = crate::kit::nodes::parts(ui.ctx(), owner.id, ui.unique_id());
            let p = ui.interact(egui::Rect::ZERO, egui::Id::unique("p"), egui::Sense::click());
            node(ui, Some(p.id), "part");
            drop(parts);
            let s = ui.interact(egui::Rect::ZERO, egui::Id::unique("s"), egui::Sense::click());
            node(ui, Some(s.id), "sibling");
            node(ui, Some(owner.id), "owner");
        });
        assert_eq!(tree, keyed(&[("owner", 0), ("part", 1), ("sibling", 0)]));
    }

    #[test]
    fn a_popup_nests_under_its_trigger() {
        use crate::{
            anchor::{Align, Anchor, Side},
            overlay::Popup,
            test_support::{ctx, input, run, runtime},
        };

        let ctx = ctx();
        let rt = runtime(&ctx);
        rt.set_collect_semantics(true);
        let out = run(&ctx, input(vec![]), |ui| {
            rt.pass(ui, |ui| {
                let trigger = ui.interact(egui::Rect::ZERO, egui::Id::unique("t"), egui::Sense::click());
                node(ui, Some(trigger.id), "trigger");
                let mut open = true;
                Popup::new("p", Anchor::new(Side::Bottom, Align::Start)).show(&rt, &trigger, &mut open, |ui, _| {
                    let o = ui.interact(egui::Rect::ZERO, egui::Id::unique("o"), egui::Sense::click());
                    node(ui, Some(o.id), "option");
                });
            });
        });
        drop(out);
        let tree = rt.tree(egui::ViewportId::ROOT).expect("tree collected");
        let depths: Vec<_> = tree.nodes.iter().map(|n| (n.id.key(), n.depth)).collect();
        assert_eq!(depths, [("trigger", 0), ("option", 1)]);
        let sub = tree.subtree("trigger").expect("subtree");
        assert_eq!(sub.len(), 2, "the trigger and its popup");
    }

    #[test]
    fn emit_builds_the_node_only_when_collecting() {
        use egui::{Context, Rect};

        use super::{Role, SemanticNode, StableId, begin_pass, emit, end_pass};

        let ctx = Context::default();
        let built = std::cell::Cell::new(false);
        let node = || {
            built.set(true);
            SemanticNode::new(StableId::new("n"), Role::Button, "n")
        };
        begin_pass(&ctx, false);
        emit(&ctx, None, Rect::NOTHING, node);
        assert!(end_pass(&ctx).is_none());
        assert!(!built.get());

        begin_pass(&ctx, true);
        emit(&ctx, None, Rect::NOTHING, node);
        let tree = end_pass(&ctx).expect("tree collected");
        assert!(built.get());
        assert!(tree.find_key("n").is_some());
    }
}
