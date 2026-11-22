// docs/rules.md rule 10: indices here outlive the frame.
#![deny(clippy::indexing_slicing)]
#![warn(clippy::arithmetic_side_effects)]
use std::{any::Any, collections::BTreeSet, fmt::Debug, hash::Hash, sync::Arc};

use egui::{Key, Modifiers};

use crate::{expand::Expansion, i18n::Direction, select::Check, semantic::KeySalt};

/// Flattened visible row of a tree.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct TreeRow<'a, N> {
    /// Borrowed node of the model.
    pub node: &'a N,
    /// 0 for roots.
    pub depth: usize,
    /// Has children or loads them lazily.
    pub has_children: bool,
    /// Has children and its key is in the [`Expansion`].
    pub expanded: bool,
    /// Row index of the parent in the same row list; `None` for roots.
    pub parent: Option<usize>,
    /// Disabled itself or under a disabled ancestor (see [`flatten_with`]): keyboard focus skips it, it
    /// takes no moves and no drops inside.
    pub disabled: bool,
}

/// Node of a tree model; the model is borrowed, never copied.
pub trait TreeData: Sized {
    /// Identity in selection, expansion and moves; its [`KeySalt`] text names the node's agent key.
    type Key: Ord + Clone + Hash + Debug + KeySalt + Send + Sync + 'static;
    /// Must be unique across the whole tree.
    fn key(&self) -> &Self::Key;
    /// Visible text; also the accessible name and typeahead text.
    fn label(&self) -> &str;
    /// Loaded children; empty for leaves and unloaded lazy nodes. A slice by design: nodes own their children.
    fn children(&self) -> &[Self];
    /// Children are loaded on demand: shows a chevron while `children()` is still empty.
    fn lazy(&self) -> bool {
        false
    }
    /// Valid [`DropPosition::Inside`] target.
    fn accepts_children(&self) -> bool {
        !self.children().is_empty() || self.lazy()
    }
    /// Drop slots of the node's row ([`drop_slot`]); [`DropZones::Quarters`] when it accepts children, else
    /// [`DropZones::Halves`].
    fn drop_zones(&self) -> DropZones {
        if self.accepts_children() {
            DropZones::Quarters
        } else {
            DropZones::Halves
        }
    }
}

/// How a row splits into drop slots ([`TreeData::drop_zones`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DropZones {
    /// Upper half before, lower half after: reorder only.
    Halves,
    /// Outer quarters before and after, the middle inside.
    Quarters,
    /// Whole row inside: nest only, as file manager folders.
    Inside,
}

/// Mutable access for [`TreeMove::apply`].
pub trait TreeDataMut: TreeData {
    /// Loaded children, edited by [`TreeMove::apply`].
    fn children_mut(&mut self) -> &mut Vec<Self>;
}

/// Keys of the ancestors of the node with `key`, root first; `None` when no node has it. Expanding them
/// makes the node visible.
pub fn ancestors<N: TreeData>(roots: &[N], key: &N::Key) -> Option<Vec<N::Key>> {
    let path = path_of(roots, key)?;
    let (_, parents) = path.split_last()?;
    (1..=parents.len())
        .map(|n| node_at(roots, parents.get(..n)?).map(|p| p.key().clone()))
        .collect()
}

impl<K: Ord + Clone> Expansion<K> {
    /// Expands the [`ancestors`] of the node with `key` so a flattened tree shows it; `true` when it is now
    /// visible. Single mode holds one open key, so it reveals only nodes with at most one ancestor and leaves
    /// deeper ones untouched.
    pub fn reveal<N: TreeData<Key = K>>(&mut self, roots: &[N], key: &K) -> bool {
        let Some(path) = ancestors(roots, key) else {
            return false;
        };
        if self.is_single() && path.len() > 1 {
            return false;
        }
        for k in path {
            self.expand(k);
        }
        true
    }
}

/// Node with `key`, depth-first.
pub fn find<'a, N: TreeData>(roots: &'a [N], key: &N::Key) -> Option<&'a N> {
    path_of(roots, key).and_then(|p| node_at(roots, &p))
}

fn path_of<N: TreeData>(roots: &[N], key: &N::Key) -> Option<Vec<usize>> {
    let mut stack: Vec<(&N, Vec<usize>)> = roots.iter().enumerate().map(|(i, n)| (n, vec![i])).collect();
    while let Some((node, path)) = stack.pop() {
        if node.key() == key {
            return Some(path);
        }
        stack.extend(node.children().iter().enumerate().map(|(i, c)| {
            let mut p = path.clone();
            p.push(i);
            (c, p)
        }));
    }
    None
}

fn node_at<'a, N: TreeData>(roots: &'a [N], path: &[usize]) -> Option<&'a N> {
    let (first, rest) = path.split_first()?;
    rest.iter().try_fold(roots.get(*first)?, |n, &i| n.children().get(i))
}

fn siblings_mut<'a, N: TreeDataMut>(roots: &'a mut Vec<N>, parent: &[usize]) -> Option<&'a mut Vec<N>> {
    let mut v = roots;
    for &i in parent {
        v = v.get_mut(i)?.children_mut();
    }
    Some(v)
}

/// Leaves (and unloaded lazy nodes) are checked when in `checked`; parents are checked or unchecked when all their
/// loaded leaves are, else mixed. Walks the subtree (stops at the first leaf that differs), so deriving it for every
/// visible row costs O(rows x subtree).
pub fn check_state<N: TreeData>(node: &N, checked: &BTreeSet<N::Key>) -> Check {
    let mut first = None;
    let mut stack = vec![node];
    while let Some(n) = stack.pop() {
        if n.children().is_empty() {
            let s = if checked.contains(n.key()) {
                Check::Checked
            } else {
                Check::Unchecked
            };
            if *first.get_or_insert(s) != s {
                return Check::Mixed;
            }
        } else {
            stack.extend(n.children());
        }
    }
    first.unwrap_or(Check::Unchecked)
}

/// Checked nodes uncheck their subtree; unchecked and mixed ones check it.
pub fn toggle_check<N: TreeData>(node: &N, checked: &mut BTreeSet<N::Key>) {
    let on = check_state(node, checked) != Check::Checked;
    let mut stack = vec![node];
    while let Some(n) = stack.pop() {
        if on {
            checked.insert(n.key().clone());
        } else {
            checked.remove(n.key());
        }
        stack.extend(n.children());
    }
}

/// Drop target position relative to a tree node.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DropPosition {
    /// Previous sibling of the target.
    Before,
    /// Next sibling of the target.
    After,
    /// Last child.
    Inside,
}

/// Completed tree node move.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TreeMove<K> {
    /// Key of the moved node.
    pub node: K,
    /// Key of the reference node `position` is relative to.
    pub target: K,
    /// Where the node lands relative to `target`.
    pub position: DropPosition,
}

impl<K: Ord + Clone + Debug> TreeMove<K> {
    /// Both nodes exist, the target is outside the moved subtree, `Inside` targets accept children,
    /// and the tree changes.
    pub fn valid<N: TreeData<Key = K>>(&self, roots: &[N]) -> bool {
        let (Some(from), Some(to)) = (path_of(roots, &self.node), path_of(roots, &self.target)) else {
            return false;
        };
        if to.starts_with(&from) {
            return false;
        }
        let (Some((&from_i, from_parent)), Some((&to_i, to_parent))) = (from.split_last(), to.split_last()) else {
            return false;
        };
        let siblings = from_parent == to_parent;
        match self.position {
            DropPosition::Inside => node_at(roots, &to).is_some_and(|target| {
                target.accepts_children() && !(from_parent == to && from_i.saturating_add(1) == target.children().len())
            }),
            DropPosition::Before => !(siblings && to_i == from_i.saturating_add(1)),
            DropPosition::After => !(siblings && to_i.saturating_add(1) == from_i),
        }
    }

    /// Applies the move to `roots`; an invalid move (see [`Self::valid`]) changes nothing.
    pub fn apply<N: TreeDataMut<Key = K>>(&self, roots: &mut Vec<N>) {
        if !self.valid(roots) {
            crate::kit::sanitize::report(format_args!("invalid tree move {self:?}"));
            return;
        }
        let Some(from) = path_of(roots, &self.node) else {
            return;
        };
        let Some((&i, parent)) = from.split_last() else {
            return;
        };
        let Some(node) = siblings_mut(roots, parent).filter(|s| i < s.len()).map(|s| s.remove(i)) else {
            return;
        };
        let Some(to) = path_of(roots, &self.target) else {
            return;
        };
        let Some((&i, parent)) = to.split_last() else {
            return;
        };
        let (siblings, at) = match self.position {
            DropPosition::Inside => (siblings_mut(roots, &to), usize::MAX),
            DropPosition::Before => (siblings_mut(roots, parent), i),
            DropPosition::After => (siblings_mut(roots, parent), i.saturating_add(1)),
        };
        if let Some(siblings) = siblings {
            let at = at.min(siblings.len());
            siblings.insert(at, node);
        }
    }
}

/// Result of [`drop_at`].
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct DropTarget<K> {
    /// Visible row the drop indicator marks.
    pub row: usize,
    /// Valid move the drop performs.
    pub change: TreeMove<K>,
}

/// Drop slot at `frac` (0 top, 1 bottom) of visible row `row`, as the row the indicator marks and the
/// position relative to it, split by the node's [`TreeData::drop_zones`]. Nesting zones fall back to halves on
/// disabled rows and nodes that do not accept children. Below an expanded row is before its first child.
pub fn drop_slot<N: TreeData>(rows: &[TreeRow<'_, N>], row: usize, frac: f32) -> Option<(usize, DropPosition)> {
    let r = rows.get(row)?;
    let zones = match r.node.drop_zones() {
        DropZones::Quarters | DropZones::Inside if r.disabled || !r.node.accepts_children() => DropZones::Halves,
        z => z,
    };
    let position = match zones {
        DropZones::Quarters if frac < 0.25 => DropPosition::Before,
        DropZones::Quarters if frac > 0.75 => DropPosition::After,
        DropZones::Quarters | DropZones::Inside => DropPosition::Inside,
        DropZones::Halves if frac < 0.5 => DropPosition::Before,
        DropZones::Halves => DropPosition::After,
    };
    Some(match position {
        DropPosition::After if r.expanded && rows.get(row.saturating_add(1)).is_some_and(|n| n.parent == Some(row)) => {
            (row.saturating_add(1), DropPosition::Before)
        }
        p => (row, p),
    })
}

/// Drop of `node` at `frac` of visible row `row` ([`drop_slot`]), when the move is valid.
pub fn drop_at<N: TreeData>(
    roots: &[N],
    rows: &[TreeRow<'_, N>],
    node: &N::Key,
    row: usize,
    frac: f32,
) -> Option<DropTarget<N::Key>> {
    let (row, position) = drop_slot(rows, row, frac)?;
    let m = TreeMove {
        node: node.clone(),
        target: rows.get(row)?.node.key().clone(),
        position,
    };
    m.valid(roots).then_some(DropTarget { row, change: m })
}

type Payload = Arc<dyn Any + Send + Sync>;
type AcceptFn<'a, N> = Box<dyn Fn(&egui::Context, &N, DropPosition) -> Option<Payload> + 'a>;

/// Foreign [`egui::DragAndDrop`] payload dropped on a tree row.
#[derive(Clone)]
pub struct TreeDrop<K> {
    target: K,
    position: DropPosition,
    payload: Payload,
}

impl<K: Debug> Debug for TreeDrop<K> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TreeDrop")
            .field("target", &self.target)
            .field("position", &self.position)
            .finish_non_exhaustive()
    }
}

impl<K> TreeDrop<K> {
    /// Row the drop is relative to.
    pub const fn target(&self) -> &K {
        &self.target
    }

    /// Where relative to [`Self::target`].
    pub const fn position(&self) -> DropPosition {
        self.position
    }

    /// `None` when the payload is not a `P`.
    pub fn payload<P: Any + Send + Sync>(&self) -> Option<Arc<P>> {
        Arc::clone(&self.payload).downcast().ok()
    }
}

/// Which foreign payloads a tree takes, and where: one acceptor per payload type, tried in order; kits test
/// them against the hovered [`drop_slot`].
pub struct DropAccept<'a, N: TreeData> {
    accepts: Vec<AcceptFn<'a, N>>,
}

impl<N: TreeData> Default for DropAccept<'_, N> {
    fn default() -> Self {
        Self { accepts: Vec::new() }
    }
}

impl<N: TreeData> Debug for DropAccept<'_, N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DropAccept").finish_non_exhaustive()
    }
}

impl<'a, N: TreeData> DropAccept<'a, N> {
    /// Adds payloads of type `P`: `accept` gets the payload in flight, the target node and the slot position.
    pub fn push<P: Any + Send + Sync>(&mut self, accept: impl Fn(&P, &N, DropPosition) -> bool + 'a) {
        self.accepts.push(Box::new(move |ctx, node, position| {
            let payload = egui::DragAndDrop::payload::<P>(ctx)?;
            let accepted: Payload = payload.clone();
            accept(&payload, node, position).then_some(accepted)
        }));
    }

    /// No payload type is accepted.
    pub fn is_empty(&self) -> bool {
        self.accepts.is_empty()
    }

    /// Drop the payload in flight would make at `frac` of visible row `row` ([`drop_slot`]), with the row
    /// the indicator marks; `None` when nothing acceptable is dragged there.
    pub fn slot(
        &self,
        ctx: &egui::Context,
        rows: &[TreeRow<'_, N>],
        row: usize,
        frac: f32,
    ) -> Option<(usize, TreeDrop<N::Key>)> {
        let (at, position) = drop_slot(rows, row, frac)?;
        let node = rows.get(at)?.node;
        let payload = self.accepts.iter().find_map(|a| a(ctx, node, position))?;
        Some((
            at,
            TreeDrop {
                target: node.key().clone(),
                position,
                payload,
            },
        ))
    }
}

fn sibling<N>(rows: &[TreeRow<'_, N>], i: usize, mut steps: impl Iterator<Item = usize>) -> Option<usize> {
    let d = rows.get(i)?.depth;
    steps
        .find(|&j| rows.get(j).is_some_and(|r| r.depth <= d))
        .filter(|&j| rows.get(j).is_some_and(|r| r.depth == d))
}

/// Keyboard move of visible row `current` (Alt+arrow): Up/Down before the previous / after the
/// next sibling, backward after the parent, forward into the previous enabled sibling. Horizontal
/// arrows flip in RTL. Disabled rows do not move.
pub fn move_key<N: TreeData>(
    roots: &[N],
    rows: &[TreeRow<'_, N>],
    current: usize,
    key: Key,
    dir: Direction,
) -> Option<TreeMove<N::Key>> {
    let row = rows.get(current).filter(|r| !r.disabled)?;
    let prev = || sibling(rows, current, (0..current).rev());
    let (target, position) = match key {
        Key::ArrowUp => (prev()?, DropPosition::Before),
        Key::ArrowDown => (
            sibling(rows, current, current.saturating_add(1)..rows.len())?,
            DropPosition::After,
        ),
        Key::ArrowLeft | Key::ArrowRight if key == dir.forward_key() => (
            prev().filter(|&p| rows.get(p).is_some_and(|r| !r.disabled))?,
            DropPosition::Inside,
        ),
        Key::ArrowLeft | Key::ArrowRight => (row.parent?, DropPosition::After),
        _ => return None,
    };
    let m = TreeMove {
        node: row.node.key().clone(),
        target: rows.get(target)?.node.key().clone(),
        position,
    };
    m.valid(roots).then_some(m)
}

/// Visible rows: roots, plus the children of expanded nodes, depth-first; none disabled.
pub fn flatten<'a, N: TreeData>(roots: &'a [N], expansion: &Expansion<N::Key>) -> Vec<TreeRow<'a, N>> {
    flatten_with(roots, expansion, |_| false)
}

/// [`flatten`], with the nodes `disabled` names and their descendants [`TreeRow::disabled`].
pub fn flatten_with<'a, N: TreeData>(
    roots: &'a [N],
    expansion: &Expansion<N::Key>,
    disabled: impl Fn(&N) -> bool,
) -> Vec<TreeRow<'a, N>> {
    let mut out: Vec<TreeRow<'a, N>> = Vec::new();
    let mut stack: Vec<(&'a N, usize, Option<usize>)> = roots.iter().rev().map(|n| (n, 0, None)).collect();
    while let Some((node, depth, parent)) = stack.pop() {
        let kids = node.children();
        let has_children = !kids.is_empty() || node.lazy();
        let expanded = has_children && expansion.is_expanded(node.key());
        let idx = out.len();
        let inherited = parent.and_then(|p| out.get(p)).is_some_and(|p| p.disabled);
        out.push(TreeRow {
            node,
            depth,
            has_children,
            expanded,
            parent,
            disabled: inherited || disabled(node),
        });
        if expanded {
            stack.extend(kids.iter().rev().map(|c| (c, depth.saturating_add(1), Some(idx))));
        }
    }
    out
}

/// Row to focus, expand or collapse from a key press.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum TreeNav {
    /// Moves focus to `row` with the key's `modifiers`, for
    /// [`Selection::follow_focus`](crate::select::Selection::follow_focus).
    Focus {
        /// Row index.
        row: usize,
        /// Modifiers of the key.
        modifiers: Modifiers,
    },
    /// Expands the row; focus stays.
    Expand(usize),
    /// Collapses the row; focus stays.
    Collapse(usize),
}

/// <https://www.w3.org/WAI/ARIA/apg/patterns/treeview/>; horizontal arrows flip in RTL. Focus moves skip
/// disabled rows. Plain, Shift and Ctrl/Cmd keys move; others are left alone.
pub fn on_key<N>(
    rows: &[TreeRow<'_, N>],
    current: usize,
    key: Key,
    modifiers: Modifiers,
    dir: Direction,
) -> Option<TreeNav> {
    let row = rows.get(current)?;
    if !(modifiers.is_none() || modifiers.shift_only() || modifiers.command_only()) {
        return None;
    }
    let focus = |row: usize| TreeNav::Focus { row, modifiers };
    let fwd = dir.forward_key();
    let enabled = |i: &usize| rows.get(*i).is_some_and(|r| !r.disabled);
    let next = current.saturating_add(1);
    Some(match key {
        Key::ArrowDown => focus((next..rows.len()).find(enabled)?),
        Key::ArrowUp => focus((0..current).rev().find(enabled)?),
        Key::Home => focus((0..rows.len()).find(enabled)?),
        Key::End => focus((0..rows.len()).rev().find(enabled)?),
        Key::ArrowLeft | Key::ArrowRight if key == fwd => match (row.has_children, row.expanded) {
            (true, false) => TreeNav::Expand(current),
            (true, true) if rows.get(next).is_some_and(|r| r.parent == Some(current)) && enabled(&next) => focus(next),
            _ => return None,
        },
        Key::ArrowLeft | Key::ArrowRight => match (row.expanded, row.parent) {
            (true, _) => TreeNav::Collapse(current),
            (false, Some(p)) => focus(p),
            (false, None) => return None,
        },
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    struct N(u32, Vec<N>);

    impl TreeData for N {
        type Key = u32;

        fn key(&self) -> &u32 {
            &self.0
        }

        fn label(&self) -> &'static str {
            ""
        }

        fn children(&self) -> &[Self] {
            &self.1
        }

        fn drop_zones(&self) -> DropZones {
            match self.0 {
                20 => DropZones::Inside,
                21 => DropZones::Halves,
                _ if self.accepts_children() => DropZones::Quarters,
                _ => DropZones::Halves,
            }
        }
    }

    impl TreeDataMut for N {
        fn children_mut(&mut self) -> &mut Vec<Self> {
            &mut self.1
        }
    }

    fn keys(roots: &[N]) -> Vec<(u32, usize)> {
        let mut exp = Expansion::default();
        for k in 0..10 {
            exp.expand(k);
        }
        flatten(roots, &exp).iter().map(|r| (r.node.0, r.depth)).collect()
    }

    fn mv(node: u32, target: u32, position: DropPosition) -> TreeMove<u32> {
        TreeMove { node, target, position }
    }

    #[test]
    fn tri_state_checks_cascade() {
        let roots = [N(1, vec![N(2, vec![]), N(3, vec![])])];
        let mut checked = BTreeSet::new();
        toggle_check(&roots[0].1[0], &mut checked);
        assert_eq!(check_state(&roots[0], &checked), Check::Mixed);
        toggle_check(&roots[0], &mut checked);
        assert_eq!(check_state(&roots[0], &checked), Check::Checked);
        toggle_check(&roots[0], &mut checked);
        assert!(checked.is_empty());
    }

    #[test]
    fn check_state_survives_unbounded_depth() {
        const DEPTH: u32 = 100_000;
        let root = (1..DEPTH).fold(N(0, vec![]), |n, key| N(key, vec![n]));
        let mut checked = BTreeSet::from([0]);
        assert_eq!(check_state(&root, &checked), Check::Checked);
        checked.clear();
        assert_eq!(check_state(&root, &checked), Check::Unchecked);
        // Dropping the chain recurses as deep as the chain.
        std::mem::forget(root);
    }

    #[test]
    fn moves_reject_own_subtree_and_no_ops() {
        let mut roots = vec![N(1, vec![N(2, vec![N(5, vec![])]), N(3, vec![])]), N(4, vec![])];
        assert!(!mv(1, 2, DropPosition::Inside).valid(&roots));
        assert!(!mv(2, 5, DropPosition::After).valid(&roots));
        assert!(!mv(2, 3, DropPosition::Before).valid(&roots));
        assert!(
            !mv(4, 3, DropPosition::Inside).valid(&roots),
            "leaf accepts no children"
        );
        let mut exp = Expansion::default();
        exp.expand(1);
        let rows = flatten(&roots, &exp);
        assert_eq!(
            drop_at(&roots, &rows, &4, 0, 0.9),
            Some(DropTarget {
                row: 1,
                change: mv(4, 2, DropPosition::Before)
            }),
            "below an expanded row is before its first child"
        );
        assert_eq!(drop_at(&roots, &rows, &1, 1, 0.5), None);
        assert_eq!(
            move_key(&roots, &rows, 2, Key::ArrowLeft, Direction::Ltr),
            Some(mv(3, 1, DropPosition::After))
        );
        assert_eq!(
            move_key(&roots, &rows, 2, Key::ArrowRight, Direction::Ltr),
            Some(mv(3, 2, DropPosition::Inside))
        );
        mv(3, 1, DropPosition::After).apply(&mut roots);
        mv(4, 2, DropPosition::Inside).apply(&mut roots);
        assert_eq!(keys(&roots), [(1, 0), (2, 1), (5, 2), (4, 2), (3, 0)]);
    }

    #[test]
    fn forward_on_an_expanded_row_without_children_loaded_stays() {
        let roots = [N(1, vec![]), N(2, vec![])];
        let mut rows = flatten(&roots, &Expansion::default());
        rows[0].has_children = true;
        rows[0].expanded = true;
        assert_eq!(on_key(&rows, 0, Key::ArrowRight, Modifiers::NONE, Direction::Ltr), None);
    }

    #[test]
    fn flatten_and_navigate() {
        let roots = vec![N(1, vec![N(2, vec![]), N(3, vec![])]), N(4, vec![])];
        let mut exp = Expansion::default();
        let rows = flatten(&roots, &exp);
        assert_eq!(rows.iter().map(|r| r.node.0).collect::<Vec<_>>(), [1, 4]);
        assert_eq!(
            on_key(&rows, 0, Key::ArrowRight, Modifiers::NONE, Direction::Ltr),
            Some(TreeNav::Expand(0))
        );
        exp.expand(1);
        let rows = flatten(&roots, &exp);
        assert_eq!(
            rows.iter().map(|r| (r.node.0, r.depth)).collect::<Vec<_>>(),
            [(1, 0), (2, 1), (3, 1), (4, 0)]
        );
        assert_eq!(
            on_key(&rows, 2, Key::ArrowLeft, Modifiers::NONE, Direction::Ltr),
            Some(TreeNav::Focus {
                row: 0,
                modifiers: Modifiers::NONE
            })
        );
        assert_eq!(
            on_key(&rows, 0, Key::ArrowRight, Modifiers::NONE, Direction::Rtl),
            Some(TreeNav::Collapse(0))
        );
    }

    #[test]
    fn invalid_move_leaves_the_tree_unchanged() {
        let mut roots = vec![N(1, vec![N(2, vec![])]), N(4, vec![])];
        let before = keys(&roots);
        mv(1, 2, DropPosition::Inside).apply(&mut roots);
        mv(9, 1, DropPosition::After).apply(&mut roots);
        assert_eq!(keys(&roots), before);
    }

    #[test]
    fn focus_moves_carry_their_modifiers() {
        let roots = vec![N(1, vec![]), N(2, vec![]), N(3, vec![])];
        let rows = flatten(&roots, &Expansion::default());
        let ltr = Direction::Ltr;
        assert_eq!(
            on_key(&rows, 0, Key::End, Modifiers::SHIFT, ltr),
            Some(TreeNav::Focus {
                row: 2,
                modifiers: Modifiers::SHIFT
            })
        );
        assert_eq!(
            on_key(&rows, 1, Key::ArrowUp, Modifiers::COMMAND, ltr),
            Some(TreeNav::Focus {
                row: 0,
                modifiers: Modifiers::COMMAND
            })
        );
        assert_eq!(
            on_key(&rows, 1, Key::ArrowUp, Modifiers::ALT, ltr),
            None,
            "Alt moves rows instead"
        );
    }

    #[test]
    fn disabled_nodes_disable_their_subtree_and_focus_skips_them() {
        let roots = vec![
            N(1, vec![]),
            N(2, vec![N(3, vec![]), N(4, vec![])]),
            N(5, vec![N(6, vec![])]),
            N(7, vec![]),
        ];
        let mut exp = Expansion::default();
        exp.expand(2);
        exp.expand(5);
        let rows = flatten_with(&roots, &exp, |n| n.0 == 2 || n.0 == 6);
        let off: Vec<u32> = rows.iter().filter(|r| r.disabled).map(|r| r.node.0).collect();
        assert_eq!(off, [2, 3, 4, 6]);
        let ltr = Direction::Ltr;
        assert_eq!(
            on_key(&rows, 0, Key::ArrowDown, Modifiers::NONE, ltr),
            Some(TreeNav::Focus {
                row: 4,
                modifiers: Modifiers::NONE
            }),
            "skips 2, 3 and 4"
        );
        assert_eq!(
            on_key(&rows, 4, Key::ArrowUp, Modifiers::NONE, ltr),
            Some(TreeNav::Focus {
                row: 0,
                modifiers: Modifiers::NONE
            })
        );
        assert_eq!(
            on_key(&rows, 4, Key::ArrowRight, Modifiers::NONE, ltr),
            None,
            "the first child is disabled"
        );
        assert_eq!(on_key(&rows, 6, Key::ArrowDown, Modifiers::NONE, ltr), None);
        assert_eq!(
            move_key(&roots, &rows, 1, Key::ArrowDown, ltr),
            None,
            "disabled rows do not move"
        );
        assert_eq!(
            drop_slot(&rows, 1, 0.5),
            Some((2, DropPosition::Before)),
            "no drop inside a disabled node"
        );
    }

    #[test]
    fn ancestors_lists_the_path_root_first() {
        let roots = vec![N(1, vec![N(2, vec![N(3, vec![])])]), N(4, vec![])];
        assert_eq!(ancestors(&roots, &3), Some(vec![1, 2]));
        assert_eq!(ancestors(&roots, &4), Some(vec![]));
        assert_eq!(ancestors(&roots, &9), None);
    }

    #[test]
    fn drop_zones_split_the_row() {
        let roots = vec![
            N(20, vec![N(1, vec![])]),
            N(21, vec![N(2, vec![])]),
            N(22, vec![N(3, vec![])]),
            N(23, vec![]),
        ];
        let rows = flatten(&roots, &Expansion::default());
        let at = |row: usize, frac: f32| drop_slot(&rows, row, frac).map(|s| s.1);
        assert_eq!(at(0, 0.05), Some(DropPosition::Inside), "a folder row nests everywhere");
        assert_eq!(at(1, 0.5), Some(DropPosition::After), "halves never nest");
        assert_eq!(at(2, 0.1), Some(DropPosition::Before));
        assert_eq!(at(2, 0.5), Some(DropPosition::Inside));
        assert_eq!(at(3, 0.5), Some(DropPosition::After), "a leaf takes halves");
    }

    #[test]
    fn reveal_expands_the_ancestors() {
        let roots = vec![N(1, vec![N(2, vec![N(3, vec![])])]), N(4, vec![])];
        let mut exp = Expansion::default();
        assert!(exp.reveal(&roots, &3));
        assert!(flatten(&roots, &exp).iter().any(|r| r.node.0 == 3));
        assert!(!exp.reveal(&roots, &9));
        let mut single = Expansion::single();
        assert!(!single.reveal(&roots, &3), "two ancestors need two open keys");
        assert!(single.reveal(&roots, &2));
        assert_eq!(single.expanded().collect::<Vec<_>>(), [&1]);
    }

    #[test]
    fn drop_accept_filters_by_payload_type_and_slot() {
        let ctx = egui::Context::default();
        let roots = vec![N(1, vec![N(2, vec![])]), N(3, vec![])];
        let rows = flatten(&roots, &Expansion::default());
        let mut accept = DropAccept::default();
        assert!(accept.is_empty());
        accept.push(|p: &String, n: &N, pos| p == "file" && (n.0 != 3 || pos != DropPosition::Inside));
        assert!(accept.slot(&ctx, &rows, 0, 0.5).is_none(), "nothing in flight");
        egui::DragAndDrop::set_payload(&ctx, 7u8);
        assert!(accept.slot(&ctx, &rows, 0, 0.5).is_none(), "foreign type");
        accept.push(|p: &u8, _: &N, _| *p == 7);
        let (_, drop) = accept.slot(&ctx, &rows, 0, 0.5).expect("a second payload type");
        assert_eq!(drop.payload::<u8>().as_deref(), Some(&7));
        egui::DragAndDrop::set_payload(&ctx, String::from("file"));
        let (row, drop) = accept.slot(&ctx, &rows, 0, 0.5).expect("accepted");
        assert_eq!((row, *drop.target(), drop.position()), (0, 1, DropPosition::Inside));
        assert_eq!(drop.payload::<String>().as_deref().map(String::as_str), Some("file"));
        assert!(drop.payload::<u8>().is_none());
        assert_eq!(
            accept.slot(&ctx, &rows, 1, 0.9).map(|(_, d)| d.position()),
            Some(DropPosition::After)
        );
    }
}
