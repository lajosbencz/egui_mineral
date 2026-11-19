// docs/rules.md rule 10: indices here outlive the frame.
#![deny(clippy::indexing_slicing)]
#![warn(clippy::arithmetic_side_effects)]
use std::{collections::BTreeSet, fmt::Debug, hash::Hash};

use egui::Key;

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
}

/// Node of a tree model; the model is borrowed, never copied.
pub trait TreeData: Sized {
    /// Identity in selection, expansion and moves; its [`KeySalt`] text names the node's agent key.
    type Key: Ord + Clone + Hash + Debug + KeySalt + Send + Sync + 'static;
    /// Must be unique across the whole tree.
    fn key(&self) -> &Self::Key;
    /// Visible text; also the accessible name and typeahead text.
    fn label(&self) -> &str;
    /// Loaded children; empty for leaves and unloaded lazy nodes.
    fn children(&self) -> &[Self];
    /// Children are loaded on demand: shows a chevron while `children()` is still empty.
    fn lazy(&self) -> bool {
        false
    }
    /// Valid [`DropPosition::Inside`] target.
    fn accepts_children(&self) -> bool {
        !self.children().is_empty() || self.lazy()
    }
}

/// Mutable access for [`TreeMove::apply`].
pub trait TreeDataMut: TreeData {
    /// Loaded children, edited by [`TreeMove::apply`].
    fn children_mut(&mut self) -> &mut Vec<Self>;
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

/// Drop of `node` at `frac` (0 top, 1 bottom) of visible row `row`: outer quarters insert
/// before/after, the middle nests into nodes that accept children (halves otherwise). Below an
/// expanded row is before its first child.
pub fn drop_at<N: TreeData>(
    roots: &[N],
    rows: &[TreeRow<'_, N>],
    node: &N::Key,
    row: usize,
    frac: f32,
) -> Option<DropTarget<N::Key>> {
    let r = rows.get(row)?;
    let position = if r.node.accepts_children() {
        match frac {
            f if f < 0.25 => DropPosition::Before,
            f if f > 0.75 => DropPosition::After,
            _ => DropPosition::Inside,
        }
    } else if frac < 0.5 {
        DropPosition::Before
    } else {
        DropPosition::After
    };
    let (row, position) = match position {
        DropPosition::After if r.expanded && rows.get(row.saturating_add(1)).is_some_and(|n| n.parent == Some(row)) => {
            (row.saturating_add(1), DropPosition::Before)
        }
        p => (row, p),
    };
    let m = TreeMove {
        node: node.clone(),
        target: rows.get(row)?.node.key().clone(),
        position,
    };
    m.valid(roots).then_some(DropTarget { row, change: m })
}

fn sibling<N>(rows: &[TreeRow<'_, N>], i: usize, mut steps: impl Iterator<Item = usize>) -> Option<usize> {
    let d = rows.get(i)?.depth;
    steps
        .find(|&j| rows.get(j).is_some_and(|r| r.depth <= d))
        .filter(|&j| rows.get(j).is_some_and(|r| r.depth == d))
}

/// Keyboard move of visible row `current` (Alt+arrow): Up/Down before the previous / after the
/// next sibling, backward after the parent, forward into the previous sibling. Horizontal arrows
/// flip in RTL.
pub fn move_key<N: TreeData>(
    roots: &[N],
    rows: &[TreeRow<'_, N>],
    current: usize,
    key: Key,
    dir: Direction,
) -> Option<TreeMove<N::Key>> {
    let row = rows.get(current)?;
    let prev = || sibling(rows, current, (0..current).rev());
    let (target, position) = match key {
        Key::ArrowUp => (prev()?, DropPosition::Before),
        Key::ArrowDown => (
            sibling(rows, current, current.saturating_add(1)..rows.len())?,
            DropPosition::After,
        ),
        Key::ArrowLeft | Key::ArrowRight if key == dir.forward_key() => (prev()?, DropPosition::Inside),
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

/// Visible rows: roots, plus the children of expanded nodes, depth-first.
pub fn flatten<'a, N: TreeData>(roots: &'a [N], expansion: &Expansion<N::Key>) -> Vec<TreeRow<'a, N>> {
    let mut out = Vec::new();
    let mut stack: Vec<(&'a N, usize, Option<usize>)> = roots.iter().rev().map(|n| (n, 0, None)).collect();
    while let Some((node, depth, parent)) = stack.pop() {
        let kids = node.children();
        let has_children = !kids.is_empty() || node.lazy();
        let expanded = has_children && expansion.is_expanded(node.key());
        let idx = out.len();
        out.push(TreeRow {
            node,
            depth,
            has_children,
            expanded,
            parent,
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
    /// Moves focus to the row index.
    Focus(usize),
    /// Expands the row; focus stays.
    Expand(usize),
    /// Collapses the row; focus stays.
    Collapse(usize),
}

/// <https://www.w3.org/WAI/ARIA/apg/patterns/treeview/>; horizontal arrows flip in RTL.
pub fn on_key<N>(rows: &[TreeRow<'_, N>], current: usize, key: Key, dir: Direction) -> Option<TreeNav> {
    let row = rows.get(current)?;
    let fwd = dir.forward_key();
    Some(match key {
        Key::ArrowDown if current.saturating_add(1) < rows.len() => TreeNav::Focus(current.saturating_add(1)),
        Key::ArrowUp if current > 0 => TreeNav::Focus(current.saturating_sub(1)),
        Key::Home => TreeNav::Focus(0),
        Key::End => TreeNav::Focus(rows.len().saturating_sub(1)),
        Key::ArrowLeft | Key::ArrowRight if key == fwd => match (row.has_children, row.expanded) {
            (true, false) => TreeNav::Expand(current),
            (true, true)
                if rows
                    .get(current.saturating_add(1))
                    .is_some_and(|r| r.parent == Some(current)) =>
            {
                TreeNav::Focus(current.saturating_add(1))
            }
            _ => return None,
        },
        Key::ArrowLeft | Key::ArrowRight => match (row.expanded, row.parent) {
            (true, _) => TreeNav::Collapse(current),
            (false, Some(p)) => TreeNav::Focus(p),
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
        assert_eq!(on_key(&rows, 0, Key::ArrowRight, Direction::Ltr), None);
    }

    #[test]
    fn flatten_and_navigate() {
        let roots = vec![N(1, vec![N(2, vec![]), N(3, vec![])]), N(4, vec![])];
        let mut exp = Expansion::default();
        let rows = flatten(&roots, &exp);
        assert_eq!(rows.iter().map(|r| r.node.0).collect::<Vec<_>>(), [1, 4]);
        assert_eq!(
            on_key(&rows, 0, Key::ArrowRight, Direction::Ltr),
            Some(TreeNav::Expand(0))
        );
        exp.expand(1);
        let rows = flatten(&roots, &exp);
        assert_eq!(
            rows.iter().map(|r| (r.node.0, r.depth)).collect::<Vec<_>>(),
            [(1, 0), (2, 1), (3, 1), (4, 0)]
        );
        assert_eq!(
            on_key(&rows, 2, Key::ArrowLeft, Direction::Ltr),
            Some(TreeNav::Focus(0))
        );
        assert_eq!(
            on_key(&rows, 0, Key::ArrowRight, Direction::Rtl),
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
}
