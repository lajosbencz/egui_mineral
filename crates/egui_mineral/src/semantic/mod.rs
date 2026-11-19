/// Serializers of the semantic tree.
pub mod serialize;

mod key;

pub use egui::accesskit::{Action, Live, Role};
use egui::{Context, Id, Rect};
pub use key::{AgentKey, KeyParts, KeySalt, KeyedSalt, StableId};

use crate::{
    kit::semantic::{RECT_KEY, push, wanted},
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
}

/// Value, bounds and step of a ranged node.
#[derive(Clone, Copy, Debug, PartialEq)]
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
    /// Human-readable accelerator label, e.g. `Ctrl+S` (see `shortcut::label`).
    pub shortcut: Option<String>,
    /// Mnemonic letter that activates the node while its menu or dialog is active.
    pub access_key: Option<Mnemonic>,
    /// Indent level in the agent markdown; 0 for top-level nodes.
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

    /// Label as produced by `shortcut::label`.
    #[must_use]
    pub fn shortcut(mut self, shortcut: Option<String>) -> Self {
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
#[derive(Clone, Debug, Default, PartialEq)]
#[non_exhaustive]
pub struct SemanticTree {
    /// Emission order: children before the parents emitted by [`emit_parent`].
    pub nodes: Vec<SemanticNode>,
}

impl SemanticTree {
    /// First node with `key`.
    pub fn find_key(&self, key: &str) -> Option<&SemanticNode> {
        self.nodes.iter().find(|n| n.id.key() == key)
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
}

pub(crate) fn collector_id(ctx: &Context) -> Id {
    crate::runtime::pass_scoped(ctx, COLLECTOR_SCOPE)
}

const COLLECTOR_SCOPE: &str = "mineral::semantic";

pub(crate) fn begin_pass(ctx: &Context, wanted: bool) {
    let id = collector_id(ctx);
    ctx.data_mut(|d| {
        d.insert_temp(
            id,
            Collector {
                wanted,
                tree: SemanticTree::default(),
            },
        )
    });
}

pub(crate) fn collecting(ctx: &Context) -> bool {
    let viewport = ctx.viewport_id();
    ctx.data(|d| {
        let id = crate::runtime::pass_scoped_in(d, viewport, COLLECTOR_SCOPE);
        d.get_temp_raw(egui::util::id_type_map::RawKey::new::<Collector>(id))
            .and_then(|c| c.downcast_ref::<Collector>())
            .is_some_and(|c| c.wanted)
    })
}

/// Builds `node` only when [`wanted`], then writes it to AccessKit (with an `id`) and the tree.
pub fn emit(ctx: &Context, id: Option<Id>, rect: Rect, node: impl FnOnce() -> SemanticNode) {
    if wanted(ctx, id) {
        write(ctx, id, rect, node);
    }
}

fn write(ctx: &Context, id: Option<Id>, rect: Rect, node: impl FnOnce() -> SemanticNode) {
    let mut node = node();
    node.rect = rect;
    node.widget_id = id;
    if let Some(id) = id {
        serialize::accesskit::write(ctx, id, &node);
    }
    push(ctx, node);
}

/// [`emit`] over the rect [`remember_rect`](crate::kit::semantic::remember_rect) stored for `id`: parents emit
/// after their children.
pub fn emit_parent(ctx: &Context, id: Id, node: impl FnOnce() -> SemanticNode) {
    if wanted(ctx, Some(id)) {
        let rect = ctx
            .data(|d| d.get_temp::<Rect>(id.with(RECT_KEY)))
            .unwrap_or(Rect::NOTHING);
        write(ctx, Some(id), rect, node);
    }
}

pub(crate) fn end_pass(ctx: &Context) -> Option<SemanticTree> {
    let id = collector_id(ctx);
    let tree = ctx
        .data_mut(|d| d.remove_temp::<Collector>(id))
        .filter(|c| c.wanted)
        .map(|c| c.tree)?;
    let mut seen = std::collections::HashSet::with_capacity(tree.nodes.len());
    for n in &tree.nodes {
        if !seen.insert(n.id.key()) {
            tracing::warn!(key = n.id.key(), "duplicate agent key; agents target the first");
        }
    }
    Some(tree)
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
