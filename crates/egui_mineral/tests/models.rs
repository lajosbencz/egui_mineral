//! Property tests of the stateful core models: random operation sequences, including hostile ones
//! (stale indices, resized collections, NaN/inf, crossed bounds, extreme dates), keep each model's
//! invariants and never panic (docs/rules.md rule 10).

use std::{cmp::Ordering, sync::LazyLock};

use egui::{Key, Modifiers, Rangef};
use egui_mineral::{
    combobox::{ComboAction, ComboItem, ComboKind, ComboList, ComboState, combo_items, creatable_combo_items},
    date::{self, Date, DateBounds, DateRange, DateSegments, MonthGrid, Weekday},
    dnd::Reorder,
    expand::Expansion,
    focus::Cell,
    i18n::{DatePart, DayPeriod, Direction, LocaleFormat},
    pagination::{PageItem, Paging, Spread},
    range::{RangeModel, Scale, Track, nearest_thumb},
    select::Selection,
    table::{ColumnSize, ColumnWidths, SortColumn, SortOrder, SortState, sort_order},
    tree::{self, DropPosition, TreeData, TreeDataMut, TreeMove, TreeNav},
};
use jiff::ToSpan;
use proptest::{prelude::*, sample::select};

fn same(a: f64, b: f64) -> bool {
    (a - b).abs() <= 0.0
}

fn config() -> ProptestConfig {
    ProptestConfig {
        failure_persistence: None,
        ..ProptestConfig::default()
    }
}

fn index() -> impl Strategy<Value = usize> {
    prop_oneof![8 => 0..12usize, 1 => Just(usize::MAX), 1 => any::<usize>()]
}

fn opt_index() -> impl Strategy<Value = Option<usize>> {
    prop::option::of(index())
}

fn f32s() -> impl Strategy<Value = f32> {
    prop_oneof![
        4 => -1e4f32..1e4,
        2 => select(vec![
            0.0,
            -0.0,
            f32::NAN,
            f32::INFINITY,
            f32::NEG_INFINITY,
            f32::MAX,
            f32::MIN,
            f32::MIN_POSITIVE,
            f32::EPSILON,
        ]),
        1 => any::<f32>(),
    ]
}

fn f64s() -> impl Strategy<Value = f64> {
    prop_oneof![
        4 => -1e4f64..1e4,
        2 => select(vec![
            0.0,
            -0.0,
            f64::NAN,
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::MAX,
            f64::MIN,
            f64::MIN_POSITIVE,
            f64::EPSILON,
        ]),
        1 => any::<f64>(),
    ]
}

fn dates() -> impl Strategy<Value = Date> {
    prop_oneof![
        1 => select(vec![
            Date::MIN,
            Date::MAX,
            Date::constant(1, 1, 1),
            Date::constant(0, 1, 1),
            Date::constant(9999, 12, 31),
            Date::constant(2024, 2, 29),
        ]),
        4 => (-9999i16..=9999, 1i8..=12, 1i8..=31).prop_map(|(y, m, d)| {
            let dim = Date::new(y, m, 1).map_or(28, Date::days_in_month);
            Date::new(y, m, d.min(dim)).unwrap_or(Date::constant(2000, 1, 1))
        }),
    ]
}

fn weekdays() -> impl Strategy<Value = Weekday> {
    select(vec![
        Weekday::Monday,
        Weekday::Tuesday,
        Weekday::Wednesday,
        Weekday::Thursday,
        Weekday::Friday,
        Weekday::Saturday,
        Weekday::Sunday,
    ])
}

fn directions() -> impl Strategy<Value = Direction> {
    select(vec![Direction::Ltr, Direction::Rtl])
}

fn keys() -> impl Strategy<Value = Key> {
    select(vec![
        Key::ArrowDown,
        Key::ArrowUp,
        Key::ArrowLeft,
        Key::ArrowRight,
        Key::Enter,
        Key::Tab,
        Key::Escape,
        Key::Space,
        Key::Home,
        Key::End,
        Key::PageUp,
        Key::PageDown,
        Key::A,
        Key::F1,
    ])
}

fn modifiers() -> impl Strategy<Value = Modifiers> {
    (any::<bool>(), any::<bool>()).prop_map(|(alt, shift)| Modifiers {
        alt,
        shift,
        ..Modifiers::NONE
    })
}

fn texts() -> impl Strategy<Value = String> {
    prop_oneof![
        "\\PC{0,12}",
        "[ a-zA-Z]{0,8}",
        "[-+0-9eE.,/ \u{2212}']{0,14}",
        "[0-9]{1,4}[-/. ][0-9]{1,2}[-/. ][0-9]{1,6}",
    ]
}

#[derive(Clone, Debug)]
enum ComboOp {
    Key(Key, Modifiers, ComboKind),
    Open(Option<usize>),
    OpenSelected(ComboKind),
    SetActive(Option<usize>),
    Retain,
    Close,
    Resize(usize),
    Select(Option<usize>),
    Mask(u64),
    Jump(usize),
}

fn combo_op() -> impl Strategy<Value = ComboOp> {
    prop_oneof![
        6 => (keys(), modifiers(), combo_kind()).prop_map(|(k, m, kind)| ComboOp::Key(k, m, kind)),
        1 => opt_index().prop_map(ComboOp::Open),
        1 => combo_kind().prop_map(ComboOp::OpenSelected),
        1 => opt_index().prop_map(ComboOp::SetActive),
        1 => Just(ComboOp::Retain),
        1 => Just(ComboOp::Close),
        2 => (0..40usize).prop_map(ComboOp::Resize),
        1 => opt_index().prop_map(ComboOp::Select),
        1 => any::<u64>().prop_map(ComboOp::Mask),
        1 => index().prop_map(ComboOp::Jump),
    ]
}

fn combo_kind() -> impl Strategy<Value = ComboKind> {
    prop_oneof![
        Just(ComboKind::SelectOnly),
        Just(ComboKind::Editable),
        Just(ComboKind::Embedded)
    ]
}

fn clamped(i: Option<usize>, len: usize) -> Option<usize> {
    i.and_then(|i| len.checked_sub(1).map(|last| i.min(last)))
}

proptest! {
    #![proptest_config(config())]

    #[test]
    fn combobox_state_survives_hostile_sequences(ops in prop::collection::vec(combo_op(), 0..48)) {
        let mut s = ComboState::default();
        let (mut len, mut selected, mut mask) = (5usize, None, 0u64);
        for op in ops {
            let enabled = move |i: usize| (mask >> (i % 64)) & 1 == 0;
            let list = |kind, selected, len| ComboList::new(kind, len, &enabled).selected(selected);
            let usable = |i: Option<usize>| i.is_none_or(|i| i < len && enabled(i));
            match op {
                ComboOp::Key(key, mods, kind) => {
                    let was_open = s.is_open();
                    let act = s.on_key(key, mods, list(kind, selected, len));
                    if kind == ComboKind::Embedded && was_open {
                        prop_assert!(act != Some(ComboAction::Closed), "an embedded list leaves closing to its host");
                    }
                    match act {
                        Some(ComboAction::Opened) => {
                            prop_assert!(!was_open && s.is_open(), "Opened from closed");
                            prop_assert!(usable(s.active()), "opened on a usable option: {:?}", s.active());
                        }
                        Some(ComboAction::Moved(i)) => {
                            prop_assert!(was_open && s.is_open(), "Moved while open");
                            prop_assert!(i < len && enabled(i), "moved to usable option {i}");
                            prop_assert_eq!(s.active(), Some(i));
                        }
                        Some(ComboAction::Commit(i)) => {
                            prop_assert!(was_open && !s.is_open(), "Commit closes");
                            prop_assert!(i < len && enabled(i), "committed usable option {i}");
                            prop_assert_eq!(s.active(), None);
                        }
                        Some(ComboAction::Closed) => {
                            prop_assert!(was_open && !s.is_open(), "Closed closes");
                            prop_assert_eq!(s.active(), None);
                        }
                        None => prop_assert_eq!(s.is_open(), was_open, "no action keeps the open state"),
                        Some(_) => {}
                    }
                    if s.is_open() {
                        prop_assert!(usable(s.active()), "active stays usable after a key: {:?}", s.active());
                    }
                }
                ComboOp::Open(a) => {
                    s.open(a, len);
                    prop_assert!(s.is_open(), "open opens");
                    prop_assert_eq!(s.active(), clamped(a, len));
                }
                ComboOp::OpenSelected(kind) => {
                    s.open_selected(list(kind, selected, len));
                    prop_assert!(s.is_open(), "open_selected opens");
                    prop_assert!(usable(s.active()), "opened on a usable option: {:?}", s.active());
                }
                ComboOp::SetActive(a) => {
                    s.set_active(a, len);
                    prop_assert_eq!(s.active(), clamped(a, len));
                }
                ComboOp::Retain => {
                    let before = s.active();
                    s.retain_active(len);
                    prop_assert_eq!(s.active(), before.filter(|i| *i < len));
                }
                ComboOp::Close => {
                    s.close();
                    prop_assert!(!s.is_open() && s.active().is_none(), "close clears");
                }
                ComboOp::Resize(n) => len = n,
                ComboOp::Select(i) => selected = i,
                ComboOp::Mask(m) => mask = m,
                ComboOp::Jump(j) => s = s.jump(j),
            }
        }
    }

    #[test]
    fn combo_items_reference_the_labels(
        query in texts(),
        labels in prop::collection::vec(texts(), 0..8),
    ) {
        let refs = || labels.iter().map(String::as_str);
        for items in [combo_items(&query, refs()), creatable_combo_items(&query, refs())] {
            let mut seen = vec![false; labels.len()];
            for (n, item) in items.iter().enumerate() {
                match item {
                    ComboItem::Option { index, hits, .. } => {
                        prop_assert!(*index < labels.len(), "option {index} indexes the labels");
                        prop_assert!(!std::mem::replace(&mut seen[*index], true), "option {index} listed once");
                        let chars = labels[*index].chars().count();
                        prop_assert!(hits.iter().all(|h| *h < chars), "hits inside the label: {hits:?}");
                        prop_assert!(hits.windows(2).all(|w| w[0] < w[1]), "hits ascending: {hits:?}");
                    }
                    ComboItem::Create => prop_assert_eq!(n + 1, items.len(), "Create comes last"),
                    other => prop_assert!(false, "unknown item {other:?}"),
                }
            }
        }
        let plain = combo_items(&query, refs());
        prop_assert!(!plain.contains(&ComboItem::Create), "combo_items never creates");
        if query.trim().is_empty() {
            prop_assert_eq!(plain.len(), labels.len(), "empty query lists every option");
        }
    }
}

#[derive(Clone, Debug)]
enum WidthOp {
    Rebuild(Vec<(f32, f32)>),
    Set(usize, f32),
}

fn width_op() -> impl Strategy<Value = WidthOp> {
    prop_oneof![
        1 => prop::collection::vec((f32s(), f32s()), 0..6).prop_map(WidthOp::Rebuild),
        4 => (index(), f32s()).prop_map(|(c, w)| WidthOp::Set(c, w)),
    ]
}

fn check_widths(w: &ColumnWidths) -> Result<(), TestCaseError> {
    let mut prev = 0.0f32;
    for c in 0..w.len() {
        let (width, min) = (w.width(c), w.min(c));
        prop_assert!(min.is_finite() && min >= 0.0, "min {min} finite and non-negative");
        prop_assert!(
            width.is_finite() && width >= min,
            "width {width} finite and >= min {min}"
        );
        let offset = w.offset(c);
        prop_assert!(offset >= prev, "offsets grow: {prev} then {offset}");
        prev = offset;
    }
    prop_assert!(
        !w.total().is_nan() && w.total() >= prev,
        "total {} covers the offsets",
        w.total()
    );
    prop_assert!(same(w.width(w.len()).into(), 0.0), "stale column reads 0");
    prop_assert!(same(w.min(usize::MAX).into(), 0.0), "stale column reads 0");
    prop_assert_eq!(
        w.offset(usize::MAX).to_bits(),
        w.total().to_bits(),
        "past the end sums everything"
    );
    Ok(())
}

#[derive(Clone, Debug)]
enum SortOp {
    Cycle(usize),
    Sort(usize),
}

proptest! {
    #![proptest_config(config())]

    #[test]
    fn column_widths_stay_valid(ops in prop::collection::vec(width_op(), 0..32)) {
        let mut w = ColumnWidths::default();
        check_widths(&w)?;
        for op in ops {
            match op {
                WidthOp::Rebuild(cols) => w = ColumnWidths::new(cols.into_iter().map(|(width, min)| ColumnSize { width, min })),
                WidthOp::Set(c, width) => {
                    let (len, min) = (w.len(), w.min(c));
                    let stored = w.set(c, width);
                    if c < len {
                        prop_assert!(stored.is_finite() && stored >= min, "stored {stored} finite and >= min {min}");
                        prop_assert_eq!(w.width(c).to_bits(), stored.to_bits());
                        if width.is_finite() {
                            prop_assert!(same(stored.into(), width.max(min).into()), "stored {stored} for {width} over min {min}");
                        }
                    } else {
                        prop_assert!(same(stored.into(), 0.0), "stale column reads 0");
                        prop_assert_eq!(w.len(), len, "stale column changes nothing");
                    }
                }
            }
            check_widths(&w)?;
        }
    }

    #[test]
    fn sort_cycle_and_order(
        ops in prop::collection::vec(
            prop_oneof![index().prop_map(SortOp::Cycle), (0..24usize).prop_map(SortOp::Sort)],
            0..24,
        ),
        data in prop::collection::vec(prop::array::uniform3(0u8..4), 0..24),
    ) {
        let mut s = SortState::default();
        for op in ops {
            match op {
                SortOp::Cycle(c) => {
                    s.cycle(c);
                    prop_assert!(s.order(c).is_some() || s.column().is_none(), "cycling selects the column or clears");
                    for other in [0, 1, c.wrapping_add(1)] {
                        if other != c {
                            prop_assert!(s.order(other).is_none(), "only the sorted column reports an order");
                        }
                    }
                }
                SortOp::Sort(len) => {
                    // `len` may exceed the data: rows past it compare as missing.
                    let key = |c: Cell| data.get(c.row).and_then(|row| row.get(c.column)).copied();
                    let order = sort_order(len, s, key);
                    let mut sorted = order.clone();
                    sorted.sort_unstable();
                    prop_assert_eq!(sorted, (0..len).collect::<Vec<_>>(), "order is a permutation of 0..len");
                    match s.column() {
                        None => prop_assert_eq!(order, (0..len).collect::<Vec<_>>(), "unsorted keeps data order"),
                        Some(SortColumn { index: column, order: dir }) => {
                            for pair in order.windows(2) {
                                let o = key(Cell { row: pair[0], column }).cmp(&key(Cell { row: pair[1], column }));
                                let o = if dir == SortOrder::Descending { o.reverse() } else { o };
                                prop_assert!(o != Ordering::Greater, "rows {pair:?} out of order");
                                if o == Ordering::Equal {
                                    prop_assert!(pair[0] < pair[1], "ties keep data order: {pair:?}");
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
struct Node {
    key: u32,
    lazy: bool,
    kids: Vec<Node>,
}

impl TreeData for Node {
    type Key = u32;

    fn key(&self) -> &u32 {
        &self.key
    }

    fn label(&self) -> &'static str {
        ""
    }

    fn children(&self) -> &[Self] {
        &self.kids
    }

    fn lazy(&self) -> bool {
        self.lazy
    }
}

impl TreeDataMut for Node {
    fn children_mut(&mut self) -> &mut Vec<Self> {
        &mut self.kids
    }
}

fn forest(raw: &[(usize, bool)], colliding: bool) -> Vec<Node> {
    let key = |i: usize| u32::try_from(if colliding { i % 4 } else { i }).unwrap_or(0);
    let parent = |i: usize| raw.get(i).map(|(p, _)| p % (i + 1)).filter(|p| *p < i);
    let mut slots: Vec<Option<Node>> = raw
        .iter()
        .enumerate()
        .map(|(i, (_, lazy))| {
            Some(Node {
                key: key(i),
                lazy: *lazy,
                kids: Vec::new(),
            })
        })
        .collect();
    let mut roots = Vec::new();
    for i in (0..slots.len()).rev() {
        let Some(node) = slots[i].take() else { continue };
        match parent(i).and_then(|p| slots[p].as_mut()) {
            Some(p) => p.kids.insert(0, node),
            None => roots.insert(0, node),
        }
    }
    roots
}

fn nodes(roots: &[Node]) -> Vec<(u32, bool)> {
    let mut out = Vec::new();
    let mut stack: Vec<&Node> = roots.iter().collect();
    while let Some(n) = stack.pop() {
        out.push((n.key, n.lazy));
        stack.extend(&n.kids);
    }
    out.sort_unstable();
    out
}

fn raw_forest() -> impl Strategy<Value = Vec<(usize, bool)>> {
    prop::collection::vec((any::<usize>(), any::<bool>()), 0..14)
}

fn tree_key() -> impl Strategy<Value = u32> {
    prop_oneof![8 => 0..16u32, 1 => any::<u32>()]
}

fn positions() -> impl Strategy<Value = DropPosition> {
    select(vec![DropPosition::Before, DropPosition::After, DropPosition::Inside])
}

fn tree_move() -> impl Strategy<Value = TreeMove<u32>> {
    (tree_key(), tree_key(), positions()).prop_map(|(node, target, position)| TreeMove { node, target, position })
}

fn expansion(open: &[u32]) -> Expansion<u32> {
    let mut e = Expansion::default();
    for k in open {
        e.expand(*k);
    }
    e
}

fn locate(roots: &[Node], key: u32) -> Option<(Option<u32>, usize)> {
    fn walk(level: &[Node], parent: Option<u32>, key: u32) -> Option<(Option<u32>, usize)> {
        level.iter().enumerate().find_map(|(i, n)| {
            if n.key == key {
                Some((parent, i))
            } else {
                walk(&n.kids, Some(n.key), key)
            }
        })
    }
    walk(roots, None, key)
}

fn children_of(roots: &[Node], parent: Option<u32>) -> Option<&[Node]> {
    match parent {
        None => Some(roots),
        Some(k) => tree::find(roots, &k).map(|n| n.kids.as_slice()),
    }
}

proptest! {
    #![proptest_config(config())]

    #[test]
    fn tree_moves_preserve_the_nodes(
        raw in raw_forest(),
        moves in prop::collection::vec(tree_move(), 0..24),
    ) {
        let mut roots = forest(&raw, false);
        let all = nodes(&roots);
        for m in moves {
            let before = roots.clone();
            let valid = m.valid(&roots);
            m.apply(&mut roots);
            prop_assert_eq!(nodes(&roots), all.clone(), "moves neither add nor drop nodes");
            if valid {
                prop_assert!(roots != before, "a valid move changes the tree: {m:?}");
                let Some((parent, at)) = locate(&roots, m.node) else {
                    return Err(TestCaseError::fail("moved node vanished"));
                };
                let siblings = children_of(&roots, parent).unwrap_or_default();
                let moved_key = |i: Option<usize>| i.and_then(|i| siblings.get(i)).map(|n| n.key);
                match m.position {
                    DropPosition::Before => {
                        prop_assert_eq!(moved_key(at.checked_add(1)), Some(m.target), "lands before the target");
                    }
                    DropPosition::After => {
                        prop_assert_eq!(moved_key(at.checked_sub(1)), Some(m.target), "lands after the target");
                    }
                    DropPosition::Inside => {
                        prop_assert_eq!(parent, Some(m.target), "lands inside the target");
                        prop_assert_eq!(at + 1, siblings.len(), "lands as the last child");
                    }
                }
            } else {
                prop_assert!(roots == before, "an invalid move changes nothing: {m:?}");
            }
        }
    }

    #[test]
    fn tree_moves_never_panic_on_colliding_keys(
        raw in raw_forest(),
        moves in prop::collection::vec(tree_move(), 0..24),
    ) {
        let mut roots = forest(&raw, true);
        let all = nodes(&roots);
        for m in moves {
            m.apply(&mut roots);
            prop_assert_eq!(nodes(&roots), all.clone(), "moves neither add nor drop nodes");
        }
    }

    #[test]
    fn tree_rows_and_keys_stay_in_bounds(
        raw in raw_forest(),
        stale in raw_forest(),
        open in prop::collection::vec(tree_key(), 0..8),
        probes in prop::collection::vec(
            (index(), keys(), directions(), tree_key(), prop_oneof![-1.0f32..2.0, f32s()]),
            1..12,
        ),
    ) {
        let (roots, other) = (forest(&raw, false), forest(&stale, false));
        let exp = expansion(&open);
        let rows = tree::flatten(&roots, &exp);
        let stale_rows = tree::flatten(&other, &exp);
        for (i, r) in rows.iter().enumerate() {
            prop_assert!(r.parent.is_none_or(|p| p < i), "parents precede their rows");
            prop_assert_eq!(r.depth == 0, r.parent.is_none());
            prop_assert!(!r.expanded || r.has_children, "only nodes with children expand");
            prop_assert_eq!(r.parent.and_then(|p| rows.get(p)).map_or(0, |p| p.depth + 1), r.depth);
        }
        // Rows of one model against another model's tree: stale keys and indices.
        for (rows, roots) in [(&rows, &roots), (&stale_rows, &roots), (&rows, &other)] {
            for (current, key, dir, node, frac) in probes.iter().copied() {
                if let Some(TreeNav::Focus(i) | TreeNav::Expand(i) | TreeNav::Collapse(i)) =
                    tree::on_key::<Node>(rows, current, key, dir)
                {
                    prop_assert!(i < rows.len(), "row {i} of {}", rows.len());
                }
                if let Some(m) = tree::move_key(roots, rows, current, key, dir) {
                    prop_assert!(m.valid(roots), "keyboard moves are valid: {m:?}");
                }
                if let Some(tree::DropTarget { row, change: m, .. }) = tree::drop_at(roots, rows, &node, current, frac) {
                    prop_assert!(row < rows.len(), "indicator row {row} of {}", rows.len());
                    prop_assert!(m.valid(roots), "drops are valid: {m:?}");
                }
            }
        }
    }

    #[test]
    fn tree_checks_derive_from_the_leaves(
        raw in raw_forest(),
        toggles in prop::collection::vec(tree_key(), 0..10),
    ) {
        use egui_mineral::select::Check;
        let roots = forest(&raw, false);
        let mut checked = std::collections::BTreeSet::new();
        for k in toggles {
            if let Some(n) = tree::find(&roots, &k) {
                tree::toggle_check(n, &mut checked);
                prop_assert!(
                    tree::check_state(n, &checked) != Check::Mixed,
                    "a toggled node is checked or unchecked, never mixed"
                );
            }
            for n in &roots {
                prop_assert!(
                    !n.kids.is_empty() || tree::check_state(n, &checked) != Check::Mixed,
                    "a leaf is never mixed"
                );
            }
        }
    }
}

#[derive(Clone, Debug)]
enum PagingOp {
    SetIndex(usize, usize),
    SetSize(usize),
    Clamp(usize),
    Range(usize),
}

fn counts() -> impl Strategy<Value = usize> {
    prop_oneof![8 => 0..60usize, 1 => Just(usize::MAX), 1 => any::<usize>()]
}

fn sizes() -> impl Strategy<Value = usize> {
    prop_oneof![8 => 0..12usize, 1 => Just(usize::MAX), 1 => any::<usize>()]
}

fn paging_op() -> impl Strategy<Value = PagingOp> {
    prop_oneof![
        3 => (index(), counts()).prop_map(|(i, n)| PagingOp::SetIndex(i, n)),
        2 => sizes().prop_map(PagingOp::SetSize),
        2 => counts().prop_map(PagingOp::Clamp),
        2 => counts().prop_map(PagingOp::Range),
    ]
}

fn spread_total() -> impl Strategy<Value = (Spread, usize, usize)> {
    let small = (0..8usize, 0..8usize);
    prop_oneof![
        // Any spread over few pages (huge spreads list every page).
        (
            prop_oneof![small.clone(), (any::<usize>(), any::<usize>())],
            0..80usize,
            index()
        ),
        // Few slots over a huge page count.
        (
            small,
            prop_oneof![Just(usize::MAX), (usize::MAX - 3..=usize::MAX), 1000..1_000_000usize],
            index()
        ),
    ]
    .prop_map(|((siblings, boundaries), total, current)| (Spread { siblings, boundaries }, total, current))
}

proptest! {
    #![proptest_config(config())]

    #[test]
    fn paging_stays_inside_the_rows(
        size in sizes(),
        ops in prop::collection::vec(paging_op(), 0..32),
    ) {
        let mut p = Paging::new(size);
        prop_assert!(p.size() >= 1, "size is at least 1");
        for op in ops {
            match op {
                PagingOp::SetIndex(i, len) => {
                    p.set_index(i, len);
                    prop_assert!(p.index() < p.page_count(len), "index {} of {} pages", p.index(), p.page_count(len));
                    prop_assert_eq!(p.index(), i.min(p.page_count(len) - 1));
                }
                PagingOp::SetSize(size) => {
                    let (index, old, len) = (p.index(), p.size(), usize::MAX);
                    let first = index.saturating_mul(old);
                    p.set_size(size);
                    prop_assert!(p.size() >= 1, "size is at least 1");
                    if index.checked_mul(old).is_some() {
                        let start = p.range(len).start;
                        prop_assert!(start <= first && first - start < p.size(), "the page keeps the old first row {first}");
                    }
                }
                PagingOp::Clamp(len) => {
                    p.clamp_index(len);
                    prop_assert!(p.index() < p.page_count(len), "index {} of {} pages", p.index(), p.page_count(len));
                }
                PagingOp::Range(len) => {
                    let r = p.range(len);
                    prop_assert!(r.start <= r.end && r.end <= len, "{r:?} inside 0..{len}");
                    prop_assert!(r.len() <= p.size(), "at most one page of rows");
                    if p.index() < p.page_count(len) && len > 0 {
                        prop_assert!(!r.is_empty(), "an in-range page of a non-empty list has rows");
                    }
                }
            }
            prop_assert!(p.size() >= 1, "size is at least 1");
        }
    }

    #[test]
    fn page_ranges_tile_the_rows(size in 0..9usize, len in 0..70usize) {
        let mut p = Paging::new(size);
        let mut next = 0;
        for page in 0..p.page_count(len) {
            p.set_index(page, len);
            let r = p.range(len);
            prop_assert_eq!(r.start, next, "page {} starts where the last ended", page);
            next = r.end;
        }
        prop_assert_eq!(next, len, "pages cover every row");
        prop_assert_eq!(p.page_count(len), len.div_ceil(p.size()).max(1));
    }

    #[test]
    fn spread_items_are_a_consistent_window((spread, total, current) in spread_total()) {
        let items = spread.items(current, total);
        if total == 0 {
            prop_assert!(items.is_empty(), "no pages, no items");
            return Ok(());
        }
        let near = current.clamp(1, total);
        prop_assert_eq!(&items, &spread.items(near, total), "a stale page counts as the nearest");
        prop_assert!(items.contains(&PageItem::Page(near)), "the current page is shown");
        let slots = spread
            .boundaries
            .saturating_mul(2)
            .saturating_add(spread.siblings.saturating_mul(2))
            .saturating_add(3);
        prop_assert_eq!(items.len(), total.min(slots), "constant item count");
        let mut last: Option<usize> = None;
        let mut gap = false;
        for item in &items {
            match *item {
                PageItem::Page(n) => {
                    prop_assert!((1..=total).contains(&n), "page {n} of {total}");
                    match (last, gap) {
                        (None, _) => {}
                        (Some(l), false) => prop_assert_eq!(n, l + 1, "adjacent pages are consecutive"),
                        (Some(l), true) => prop_assert!(n >= l + 3, "a gap hides at least two pages: {l} .. {n}"),
                    }
                    last = Some(n);
                    gap = false;
                }
                PageItem::Gap => {
                    prop_assert!(!gap, "no two gaps in a row");
                    gap = true;
                }
                other => prop_assert!(false, "unknown item {other:?}"),
            }
        }
        if spread.boundaries >= 1 {
            prop_assert_eq!(items.first(), Some(&PageItem::Page(1)), "first page always shown");
            prop_assert_eq!(items.last(), Some(&PageItem::Page(total)), "last page always shown");
        }
    }
}

#[cfg(feature = "icu")]
fn locale(locale: &egui_mineral::i18n::Locale) -> Box<dyn LocaleFormat> {
    Box::new(egui_mineral::i18n::IcuFormat::new(locale).expect("locale data"))
}

#[cfg(not(feature = "icu"))]
fn locale(locale: &egui_mineral::i18n::Locale) -> Box<dyn LocaleFormat> {
    Box::new(egui_mineral::i18n::AsciiFormat::new(locale))
}

static EN_US: LazyLock<Box<dyn LocaleFormat>> = LazyLock::new(|| locale(&egui_mineral::i18n::locale!("en-US")));
static JA_JP: LazyLock<Box<dyn LocaleFormat>> = LazyLock::new(|| locale(&egui_mineral::i18n::locale!("ja-JP")));
static DE_DE: LazyLock<Box<dyn LocaleFormat>> = LazyLock::new(|| locale(&egui_mineral::i18n::locale!("de-DE")));

fn orders() -> impl Strategy<Value = [DatePart; 3]> {
    Just([DatePart::Year, DatePart::Month, DatePart::Day]).prop_shuffle()
}

#[derive(Clone, Debug)]
enum SegOp {
    Digit(usize, u8),
    Clear(usize),
    Bump(usize, i32, Date),
    Commit,
    Fill(Option<Date>),
}

fn seg_op() -> impl Strategy<Value = SegOp> {
    let seg = prop_oneof![8 => 0..3usize, 1 => any::<usize>()];
    let digit = prop_oneof![8 => 0..10u8, 1 => any::<u8>()];
    let delta = prop_oneof![8 => -40..40i32, 1 => Just(i32::MIN), 1 => Just(i32::MAX), 1 => any::<i32>()];
    prop_oneof![
        8 => (seg.clone(), digit).prop_map(|(s, d)| SegOp::Digit(s, d)),
        2 => seg.clone().prop_map(SegOp::Clear),
        2 => (seg, delta, dates()).prop_map(|(s, d, t)| SegOp::Bump(s, d, t)),
        1 => Just(SegOp::Commit),
        1 => prop::option::of(dates()).prop_map(SegOp::Fill),
    ]
}

fn check_segments(s: &DateSegments) -> Result<(), TestCaseError> {
    let order = s.order();
    for (i, part) in order.iter().enumerate() {
        prop_assert_eq!(s.value(i), s.get(*part), "value(seg) follows the display order");
        let hi = match part {
            DatePart::Year => 9999,
            DatePart::Month => 12,
            DatePart::Day => 31,
        };
        prop_assert!(
            s.value(i).is_none_or(|v| v <= hi),
            "{part:?} segment {:?} above {hi}",
            s.value(i)
        );
        match (s.value(i), s.text(i)) {
            (None, None) => {}
            (Some(v), Some(t)) => {
                prop_assert!(
                    !t.is_empty() && t.chars().all(|c| c.is_ascii_digit()),
                    "ASCII digits: {t:?}"
                );
                prop_assert_eq!(t.parse::<u16>().ok(), Some(v), "text round-trips the value");
            }
            (v, t) => prop_assert!(false, "value {v:?} and text {t:?} disagree"),
        }
    }
    for seg in [3, 4, usize::MAX] {
        prop_assert!(s.value(seg).is_none() && s.text(seg).is_none(), "no segment {seg}");
    }
    let parts = order.map(|p| s.get(p));
    prop_assert_eq!(s.is_empty(), parts.iter().all(Option::is_none));
    let (y, m, d) = (s.get(DatePart::Year), s.get(DatePart::Month), s.get(DatePart::Day));
    let expected = y.zip(m).zip(d).and_then(|((y, m), d)| {
        let year = i16::try_from(y).ok().filter(|y| (1..=9999).contains(y))?;
        Date::new(year, i8::try_from(m).ok()?, i8::try_from(d).ok()?).ok()
    });
    prop_assert_eq!(
        s.date(),
        expected,
        "date() is Some exactly for complete, valid dates in 1..=9999"
    );
    Ok(())
}

fn days_in(s: &DateSegments) -> Option<u16> {
    let year = i16::try_from(s.get(DatePart::Year)?).ok()?;
    let month = i8::try_from(s.get(DatePart::Month)?).ok()?;
    let dim = Date::new(year, month, 1).ok()?.days_in_month();
    u16::try_from(dim).ok()
}

proptest! {
    #![proptest_config(config())]

    #[test]
    fn date_segments_hold_valid_values(
        order in orders(),
        ops in prop::collection::vec(seg_op(), 0..48),
    ) {
        let mut s = DateSegments::new(order);
        check_segments(&s)?;
        for op in ops {
            match op {
                SegOp::Digit(seg, d) => {
                    let at = seg.min(2);
                    let next = s.digit(seg, d);
                    prop_assert!(next == at || next == (at + 1).min(2), "focus {next} after typing into {at}");
                    prop_assert!(s.value(at).is_some(), "typed segment has a value");
                }
                SegOp::Clear(seg) => {
                    let at = seg.min(2);
                    let next = s.clear(seg);
                    prop_assert!(next <= 2, "focus {next}");
                    prop_assert!(s.value(at).is_none(), "cleared segment is empty");
                }
                SegOp::Bump(seg, delta, today) => {
                    let at = seg.min(2);
                    s.bump(seg, delta, today);
                    let v = s.value(at);
                    let part = order.get(at).copied();
                    let (lo, hi) = match part {
                        Some(DatePart::Year) => (1, 9999),
                        Some(DatePart::Month) => (1, 12),
                        _ => (1, days_in(&s).unwrap_or(31)),
                    };
                    prop_assert!(v.is_some_and(|v| (lo..=hi).contains(&v)), "{part:?} bumped to {v:?}, want {lo}..={hi}");
                }
                SegOp::Commit => s.commit(),
                SegOp::Fill(d) => {
                    s = DateSegments::from_date(order, d);
                    if let Some(d) = d.filter(|d| d.year() >= 1) {
                        prop_assert_eq!(s.date(), Some(d), "from_date round-trips");
                    }
                    prop_assert_eq!(s.is_empty(), d.is_none());
                }
            }
            check_segments(&s)?;
        }
    }

    #[test]
    fn typed_digits_never_leave_the_field_range(
        order in orders(),
        digits in prop::collection::vec((0..3usize, 0..10u8), 0..24),
    ) {
        // Typing without arrow keys: every complete date is valid.
        let mut s = DateSegments::new(order);
        for (seg, d) in digits {
            s.digit(seg, d);
        }
        check_segments(&s)?;
    }

    #[test]
    fn parse_date_returns_valid_dates(text in texts()) {
        for locale in [&**EN_US, &**JA_JP, &**DE_DE] {
            if let Some(d) = locale.parse_date(&text) {
                prop_assert!((1..=9999).contains(&d.year()), "year {} outside 1..=9999", d.year());
            }
            std::hint::black_box(locale.parse_number(&text));
        }
    }

    #[test]
    fn month_grid_covers_the_month(date in dates(), first in weekdays()) {
        let g = MonthGrid::new(date, first);
        prop_assert_eq!((g.first.year(), g.first.month(), g.first.day()), (date.year(), date.month(), 1));
        prop_assert!(g.days.contains(&date), "grid holds the date");
        prop_assert!(g.in_month(date) && !g.in_month(g.first.saturating_sub(1.days())) || g.first == Date::MIN);
        prop_assert!(g.days.windows(2).all(|w| w[0] <= w[1]), "days ascend");
        prop_assert_eq!(g.weeks().count(), 6);
        prop_assert!(g.weeks().all(|w| w.len() == 7));
        let (lo, hi) = (g.days[0], g.days[41]);
        if lo != Date::MIN && hi != Date::MAX {
            prop_assert_eq!(lo.weekday(), first, "grid starts on the first weekday");
            prop_assert!(g.days.windows(2).all(|w| w[0].saturating_add(1.days()) == w[1]), "days are consecutive");
            prop_assert!(g.days.contains(&g.first.last_of_month()), "grid holds the whole month");
        }
    }

    #[test]
    fn weekday_rows_are_consecutive(first in weekdays()) {
        let days = date::weekdays_from(first);
        prop_assert_eq!(days[0], first);
        prop_assert!(days.windows(2).all(|w| w[0].next() == w[1]));
    }

    #[test]
    fn month_arithmetic_saturates(d in dates(), n in prop_oneof![-30..30i32, Just(i32::MIN), Just(i32::MAX), any::<i32>()]) {
        let moved = date::add_months(d, n);
        match n.cmp(&0) {
            Ordering::Greater => prop_assert!(moved >= d, "{d} + {n} months = {moved}"),
            Ordering::Less => prop_assert!(moved <= d, "{d} + {n} months = {moved}"),
            Ordering::Equal => prop_assert_eq!(moved, d),
        }
    }

    #[test]
    fn date_keys_move_within_the_expected_span(
        d in dates(),
        key in keys(),
        mods in modifiers(),
        dir in directions(),
        first in weekdays(),
    ) {
        let moved = date::on_key(d, key, mods, dir, first);
        let handled = matches!(
            key,
            Key::ArrowLeft | Key::ArrowRight | Key::ArrowUp | Key::ArrowDown | Key::PageUp | Key::PageDown | Key::Home | Key::End
        );
        prop_assert_eq!(moved.is_some(), handled, "{:?} handled", key);
        let Some(moved) = moved else { return Ok(()) };
        match key {
            Key::ArrowUp | Key::PageUp => prop_assert!(moved <= d, "{key:?} moves back"),
            Key::ArrowDown | Key::PageDown => prop_assert!(moved >= d, "{key:?} moves forward"),
            Key::Home => prop_assert!(moved <= d && d.since(moved).is_ok(), "Home moves back"),
            Key::End => prop_assert!(moved >= d, "End moves forward"),
            _ => {}
        }
        let mirrored = |k, dir| date::on_key(d, k, mods, dir, first);
        prop_assert_eq!(mirrored(Key::ArrowLeft, Direction::Rtl), mirrored(Key::ArrowRight, Direction::Ltr), "RTL mirrors");
        if (-9990..9990).contains(&d.year()) && matches!(key, Key::Home | Key::End) {
            prop_assert_eq!(moved.weekday() == first, key == Key::Home, "Home and End hit the week edges");
            prop_assert!(moved.saturating_add(7.days()) > d && d.saturating_add(7.days()) > moved, "within the week");
        }
    }

    #[test]
    fn bounds_clamp_into_themselves(min in prop::option::of(dates()), max in prop::option::of(dates()), d in dates()) {
        let b = DateBounds::new(min, max);
        if let (Some(lo), Some(hi)) = (b.min(), b.max()) {
            prop_assert!(lo <= hi, "bounds ordered");
        }
        prop_assert_eq!(b.min(), min, "min is kept");
        if let (Some(lo), Some(hi)) = (min, max) {
            prop_assert_eq!(b.max(), Some(hi.max(lo)), "a crossed max is raised to min");
        }
        let c = b.clamp(d);
        prop_assert!(b.contains(c), "clamp({d}) = {c} inside {b:?}");
        prop_assert_eq!(b.contains(d), c == d, "only outside dates move");
        prop_assert_eq!(b.clamp(c), c);
        prop_assert_eq!(b.contains(d), b.min().is_none_or(|m| d >= m) && b.max().is_none_or(|m| d <= m));
    }

    #[test]
    fn ranges_are_ordered_and_inclusive(a in dates(), b in dates(), d in dates()) {
        let r = DateRange::new(a, b);
        prop_assert_eq!(r.span().map(|s| (s.start, s.end)), Some((a.min(b), a.max(b))), "crossed ends swap");
        prop_assert!(r.is_complete() && r.start() <= r.end());
        prop_assert_eq!(r.contains(d), d >= a.min(b) && d <= a.max(b));
        let open = DateRange::starting(a);
        prop_assert!(!open.is_complete() && !open.contains(a) && open.span().is_none() && open.end().is_none());
        prop_assert!(!DateRange::default().contains(d));
    }

    #[test]
    fn twelve_hour_clock_round_trips(hour in any::<i8>(), hour12 in any::<i8>(), pm in any::<bool>()) {
        let (h12, period) = date::to_12h(hour);
        prop_assert!((1..=12).contains(&h12), "12-hour value {h12}");
        prop_assert_eq!(date::from_12h(h12, period), hour.rem_euclid(24));
        prop_assert_eq!(period == DayPeriod::Pm, hour.rem_euclid(24) >= 12);
        let h = date::from_12h(hour12, if pm { DayPeriod::Pm } else { DayPeriod::Am });
        prop_assert!((0..24).contains(&h), "24-hour value {h}");
    }
}

#[derive(Clone, Debug)]
enum ListOp {
    Move(usize, usize),
    Push(u8),
    Truncate(usize),
}

proptest! {
    #![proptest_config(config())]

    #[test]
    fn reorder_is_remove_then_insert(
        mut items in prop::collection::vec(any::<u8>(), 0..16),
        ops in prop::collection::vec(
            prop_oneof![
                4 => (index(), index()).prop_map(|(f, t)| ListOp::Move(f, t)),
                1 => any::<u8>().prop_map(ListOp::Push),
                1 => (0..16usize).prop_map(ListOp::Truncate),
            ],
            0..32,
        ),
    ) {
        for op in ops {
            match op {
                ListOp::Move(from, to) => {
                    let mut expect = items.clone();
                    if let Some(last) = expect.len().checked_sub(1) {
                        let moved = expect.remove(from.min(last));
                        expect.insert(to.min(last), moved);
                    }
                    Reorder { from, to }.apply(&mut items);
                    prop_assert_eq!(&items, &expect, "{} -> {}", from, to);
                }
                ListOp::Push(v) => items.push(v),
                ListOp::Truncate(n) => items.truncate(n),
            }
        }
    }
}

#[derive(Clone, Debug)]
enum RangeOp {
    Rebuild(RangeModel),
    Set(f64),
    Snap(f64),
    StepBy(i64),
    Key(Key, Direction, bool),
    FromFraction(f64),
    ValueAt(f32, f32, f32, Direction),
    Parse(String),
}

fn range_model() -> impl Strategy<Value = RangeModel> {
    (
        f64s(),
        f64s(),
        prop::option::of(f64s()),
        prop::option::of(f64s()),
        prop::option::of(select(vec![Scale::Linear, Scale::Log])),
        any::<bool>(),
        prop::option::of(0u8..20),
        prop::option::of(f64s()),
        prop::option::of(0u8..20),
    )
        .prop_map(
            |(min, max, step, jump, scale, scale_first, significant, increment, fraction_digits)| {
                let mut r = RangeModel::new(min, max);
                if let (true, Some(s)) = (scale_first, scale) {
                    r = r.scale(s);
                }
                if let Some(s) = step {
                    r = r.step(s);
                }
                if let Some(j) = jump {
                    r = r.jump(j);
                }
                if let Some(d) = significant {
                    r = r.significant(d);
                }
                if let Some(i) = increment {
                    r = r.increment(i);
                }
                if let Some(d) = fraction_digits {
                    r = r.fraction_digits(d);
                }
                match scale {
                    Some(s) if !scale_first => r.scale(s),
                    _ => r,
                }
            },
        )
}

fn range_op() -> impl Strategy<Value = RangeOp> {
    prop_oneof![
        1 => range_model().prop_map(RangeOp::Rebuild),
        2 => f64s().prop_map(RangeOp::Set),
        2 => f64s().prop_map(RangeOp::Snap),
        2 => prop_oneof![-5..5i64, Just(i64::MIN), Just(i64::MAX)].prop_map(RangeOp::StepBy),
        4 => (keys(), directions(), any::<bool>()).prop_map(|(k, d, v)| RangeOp::Key(k, d, v)),
        2 => f64s().prop_map(RangeOp::FromFraction),
        2 => (f32s(), f32s(), f32s(), directions()).prop_map(|(a, b, p, d)| RangeOp::ValueAt(a, b, p, d)),
        1 => texts().prop_map(RangeOp::Parse),
    ]
}

fn within(r: &RangeModel, v: f64) -> Result<(), TestCaseError> {
    prop_assert!(
        v.is_finite() && v >= r.min() && v <= r.max(),
        "{v} inside {}..={} ({r:?})",
        r.min(),
        r.max()
    );
    Ok(())
}

fn check_range(r: &RangeModel, probe: f64) -> Result<(), TestCaseError> {
    prop_assert!(
        r.min().is_finite() && r.max().is_finite() && r.min() <= r.max(),
        "bounds {r:?}"
    );
    prop_assert!(r.step_size().is_none_or(|s| s.is_finite() && s > 0.0), "step {r:?}");
    within(r, r.snap(probe))?;
    if r.step_size().is_none() {
        let once = r.snap(probe);
        prop_assert!(
            same(r.snap(once), once),
            "snap({probe}) = {once} is a fixed point of {r:?}"
        );
    }
    let f = r.fraction(probe);
    prop_assert!((0.0..=1.0).contains(&f), "fraction({probe}) = {f} for {r:?}");
    for t in [-1.0, 0.0, 0.3, 1.0, 2.0, f64::NAN, f64::INFINITY, probe] {
        within(r, r.from_fraction(t))?;
    }
    prop_assert!(r.ticks(64).len() <= 64, "ticks respect the cap");
    Ok(())
}

proptest! {
    #![proptest_config(config())]

    #[test]
    fn range_model_keeps_values_in_bounds(
        first in range_model(),
        ops in prop::collection::vec(range_op(), 0..32),
    ) {
        let (mut r, mut v) = (first, 0.0f64);
        check_range(&r, v)?;
        for op in ops {
            match op {
                RangeOp::Rebuild(m) => r = m,
                RangeOp::Set(x) => v = x,
                RangeOp::Snap(x) => {
                    v = r.snap(x);
                    within(&r, v)?;
                    if x.is_finite() && x >= r.min() && x <= r.max() && r.step_size().is_none() && r.significant_digits().is_none() && r.fraction_digit_count().is_none() {
                        prop_assert!(same(v, x), "an in-range continuous value stays: {x} became {v}");
                    }
                }
                RangeOp::StepBy(n) => {
                    v = r.step_by(v, n);
                    within(&r, v)?;
                }
                RangeOp::Key(key, dir, horizontal) => {
                    let track = if horizontal { Track::horizontal(dir) } else { Track::vertical() };
                    let handled = matches!(
                        key,
                        Key::ArrowLeft | Key::ArrowRight | Key::ArrowUp | Key::ArrowDown | Key::PageUp | Key::PageDown | Key::Home | Key::End
                    );
                    let next = r.on_key(v, key, track);
                    prop_assert_eq!(next.is_some(), handled, "{:?} handled", key);
                    if let Some(next) = next {
                        within(&r, next)?;
                        match key {
                            Key::Home => prop_assert!(same(next, r.snap(r.min())), "Home snaps to min"),
                            Key::End => prop_assert!(same(next, r.snap(r.max())), "End snaps to max"),
                            _ => {}
                        }
                        v = next;
                    }
                }
                RangeOp::FromFraction(f) => {
                    v = r.from_fraction(f);
                    within(&r, v)?;
                }
                RangeOp::ValueAt(a, b, pos, dir) => {
                    for track in [Track::horizontal(dir), Track::vertical(), Track::vertical_down()] {
                        within(&r, r.value_at(Rangef::new(a, b), pos, track))?;
                    }
                }
                RangeOp::Parse(text) => {
                    for locale in [&**EN_US, &**DE_DE] {
                        if let Some(p) = r.parse(&text, locale) {
                            within(&r, p)?;
                        }
                    }
                }
            }
            check_range(&r, v)?;
        }
    }

    #[test]
    fn increments_move_by_their_size_wherever_min_sits(
        min in -1e6..1e6f64,
        span in 1e-3..1e6f64,
        increment in 1e-4..1e3f64,
        at in 0.0..1.0f64,
    ) {
        let r = RangeModel::new(min, min + span).increment(increment);
        let from = r.snap(min + at * span);
        prop_assume!(from + increment <= r.max());
        let next = r.step_by(from, 1);
        let tolerance = 1e-9 * from.abs().max(increment).max(1.0);
        prop_assert!((next - from - increment).abs() <= tolerance, "{from} + {increment} gave {next}");
    }

    #[test]
    fn arrow_steps_move_towards_the_key(
        r in range_model(),
        at in 0.0..=1.0f64,
        key in select(vec![Key::ArrowUp, Key::ArrowDown, Key::ArrowLeft, Key::ArrowRight, Key::PageUp, Key::PageDown]),
        dir in directions(),
    ) {
        let v = r.from_fraction(at);
        let track = Track::horizontal(dir);
        let Some(next) = r.on_key(v, key, track) else { return Err(TestCaseError::fail("arrow keys are handled")) };
        // Up/Right-like keys on the track raise the value; the opposite lower it.
        let up = match key {
            Key::ArrowUp | Key::PageUp => true,
            Key::ArrowDown | Key::PageDown => false,
            k => (k == Key::ArrowRight) != track.max_at_start(),
        };
        let eps = r.span() * 1e-9;
        if up {
            prop_assert!(next >= v - eps, "{key:?} lowered {v} to {next} in {r:?}");
        } else {
            prop_assert!(next <= v + eps, "{key:?} raised {v} to {next} in {r:?}");
        }
    }

    #[test]
    fn track_positions_stay_on_the_track(
        r in range_model(),
        v in f64s(),
        lo in -1e6f32..1e6,
        hi in -1e6f32..1e6,
        dir in directions(),
    ) {
        for track in [Track::horizontal(dir), Track::vertical()] {
            let p = r.position_of(Rangef::new(lo, hi), v, track);
            prop_assert!(p.is_finite(), "position {p} of {v} in {r:?}");
        }
    }

    #[test]
    fn nearest_thumb_is_the_closest(values in prop::collection::vec(-1e6f64..1e6, 0..8), v in -1e6f64..1e6) {
        match nearest_thumb(&values, v) {
            None => prop_assert!(values.is_empty()),
            Some(i) => {
                let best = (values[i] - v).abs();
                prop_assert!(values.iter().all(|x| (x - v).abs() >= best), "thumb {i} of {values:?} for {v}");
            }
        }
    }
}

#[derive(Clone, Debug)]
enum SelectionOp {
    Select(u8),
    Deselect(u8),
    Toggle(u8),
    Clear,
    Click(u8, bool, bool),
}

fn selection_op() -> impl Strategy<Value = SelectionOp> {
    prop_oneof![
        (0..6u8).prop_map(SelectionOp::Select),
        (0..6u8).prop_map(SelectionOp::Deselect),
        (0..6u8).prop_map(SelectionOp::Toggle),
        Just(SelectionOp::Clear),
        (0..6u8, any::<bool>(), any::<bool>()).prop_map(|(k, shift, command)| SelectionOp::Click(k, shift, command)),
    ]
}

proptest! {
    #![proptest_config(config())]

    #[test]
    fn selection_mutators_report_exactly_the_changes(
        multiple in any::<bool>(),
        required in any::<bool>(),
        ops in prop::collection::vec(selection_op(), 0..40),
    ) {
        let order: Vec<u8> = (0..6).collect();
        let mode = if multiple { Selection::multiple() } else { Selection::single() };
        let mut s = mode.required(required);
        for op in ops {
            let before: Vec<u8> = s.selected().copied().collect();
            let changed = match op {
                SelectionOp::Select(k) => s.select(k),
                SelectionOp::Deselect(k) => s.deselect(&k),
                SelectionOp::Toggle(k) => s.toggle(k),
                SelectionOp::Clear => s.clear(),
                SelectionOp::Click(k, shift, command) => {
                    s.click(k, Modifiers { shift, command, ..Modifiers::NONE }, &order)
                }
            };
            let after: Vec<u8> = s.selected().copied().collect();
            prop_assert_eq!(changed, before != after, "{:?}", op);
            prop_assert!(multiple || after.len() <= 1, "single mode holds at most one key");
        }
    }
}
