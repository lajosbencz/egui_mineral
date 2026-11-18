use crate::Mui;
use egui_mineral_material::MaterialExt;
use std::collections::BTreeSet;

use egui::{Align, Layout, RichText, Sense, UiBuilder, vec2};
use egui_mineral::{i18n, perf::VirtualList, tree};
use egui_mineral_material::{catalog::Component, extras::TreeAction, prelude::*};

use super::{Cmd, Shared, caption, section};

pub(crate) const TOPICS: &[&str] = &[
    "Data table",
    "Tree view",
    "Avatar",
    "Timeline",
    "Description list",
    "Empty state",
    "Statistic",
    "Keyboard shortcut",
    "Link",
    "Pull to refresh",
    "Virtual list",
];

const ROWS: usize = 1_000;
const NAMES: [&str; 8] = ["Ada", "Grace", "Linus", "Margaret", "Dennis", "Barbara", "Ken", "Radia"];
const TEAMS: [&str; 4] = ["Platform", "Design", "Runtime", "Docs"];
const JOB_BASE: u32 = 1_000_000;

struct Node {
    key: u32,
    label: String,
    icon: MaterialIcon,
    lazy: bool,
    dir: bool,
    children: Vec<Node>,
}

impl Node {
    fn leaf(key: u32, label: &str) -> Self {
        Self {
            key,
            label: label.to_owned(),
            icon: icons::ICON_DESCRIPTION,
            lazy: false,
            dir: false,
            children: Vec::new(),
        }
    }

    fn dir(key: u32, label: &str, children: Vec<Self>) -> Self {
        Self {
            icon: icons::ICON_FOLDER,
            dir: true,
            children,
            ..Self::leaf(key, label)
        }
    }

    fn find_mut(&mut self, key: u32) -> Option<&mut Self> {
        if self.key == key {
            return Some(self);
        }
        self.children.iter_mut().find_map(|c| c.find_mut(key))
    }
}

fn remove_node(nodes: &mut Vec<Node>, key: u32) {
    nodes.retain(|n| n.key != key);
    for n in nodes {
        remove_node(&mut n.children, key);
    }
}

impl TreeData for Node {
    type Key = u32;

    fn key(&self) -> &u32 {
        &self.key
    }

    fn label(&self) -> &str {
        &self.label
    }

    fn children(&self) -> &[Self] {
        &self.children
    }

    fn lazy(&self) -> bool {
        self.lazy
    }

    fn accepts_children(&self) -> bool {
        self.dir
    }
}

impl TreeDataMut for Node {
    fn children_mut(&mut self) -> &mut Vec<Self> {
        &mut self.children
    }
}

fn demo_tree() -> Vec<Node> {
    let mut downloads = Node::dir(100, "Downloads (lazy)", Vec::new());
    downloads.lazy = true;
    vec![
        Node::dir(
            1,
            "src",
            vec![
                Node::leaf(2, "main.rs"),
                Node::leaf(3, "lib.rs"),
                Node::dir(4, "pages", vec![Node::leaf(5, "data.rs"), Node::leaf(6, "system.rs")]),
            ],
        ),
        Node::dir(
            10,
            "docs",
            vec![Node::leaf(11, "AGENTS.md"), Node::leaf(12, "README.md")],
        ),
        downloads,
        Node::leaf(20, "Cargo.toml"),
    ]
}

pub(crate) struct State {
    table_sel: Selection<usize>,
    table_note: String,
    tree: Vec<Node>,
    tree_exp: Expansion<u32>,
    outline_exp: Expansion<u32>,
    tree_sel: Selection<u32>,
    tree_checked: BTreeSet<u32>,
    dl_layout: usize,
    dl_columns: usize,
    empty_clicks: u32,
    next_job: u32,
    refreshing: Option<u32>,
    refreshed: u32,
    vl_variable: bool,
    vl_target: usize,
    vl_scroll: Option<usize>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            table_sel: Selection::multiple(),
            table_note: String::new(),
            tree: demo_tree(),
            tree_exp: Expansion::default(),
            outline_exp: Expansion::default(),
            tree_sel: Selection::single(),
            tree_checked: BTreeSet::new(),
            dl_layout: 0,
            dl_columns: 2,
            empty_clicks: 0,
            next_job: JOB_BASE,
            refreshing: None,
            refreshed: 0,
            vl_variable: false,
            vl_target: 5_000,
            vl_scroll: None,
        }
    }
}

fn boxed<R>(ui: &mut Mui<'_>, height: f32, add: impl FnOnce(&mut Mui<'_>) -> R) -> R {
    let rt = ui.rt();
    let width = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(vec2(width, height), Sense::hover());
    add(&mut rt.ui(&mut ui.new_child(
        UiBuilder::new()
            .id_salt("boxed")
            .max_rect(rect)
            .layout(Layout::top_down(Align::Min)),
    )))
}

pub(crate) fn show(ui: &mut Mui<'_>, s: &mut State, shared: &mut Shared) {
    section(ui, TOPICS[0], Some(<DataTable<'static>>::META), |ui| table(ui, s));
    section(ui, TOPICS[1], Some(<TreeView<'static, Node>>::META), |ui| {
        tree(ui, s, shared);
        outline(ui, s);
    });
    section(ui, TOPICS[2], Some(Avatar::META), avatars);
    section(ui, TOPICS[3], Some(Timeline::META), timeline);
    section(ui, TOPICS[4], Some(DescriptionList::META), |ui| {
        description(ui, s);
    });
    section(ui, TOPICS[5], Some(EmptyPlaceholder::META), |ui| empty(ui, s));
    section(ui, TOPICS[6], Some(Statistic::META), statistics);
    section(ui, TOPICS[7], Some(Kbd::META), kbd);
    section(ui, TOPICS[8], Some(TextLink::META), link);
    section(ui, TOPICS[9], Some(PullToRefresh::META), |ui| {
        pull(ui, s, shared);
    });
    section(ui, TOPICS[10], None, |ui| virtual_list(ui, s));
}

fn table(ui: &mut Mui<'_>, s: &mut State) {
    let cols = vec![
        TableColumn::new("Name").width(180.0),
        TableColumn::new("Team").width(140.0),
        TableColumn::new("Score").numeric(true).fraction_digits(1).width(120.0),
        TableColumn::new("Commits")
            .numeric(true)
            .fraction_digits(0)
            .width(120.0),
        TableColumn::new("Notes").width(200.0).sortable(false),
    ];
    let r = DataTable::new("demo-table", "People", cols, ROWS)
        .selection(&mut s.table_sel)
        .paginate([10, 25, 50])
        .max_height(420.0)
        .show(ui, |row, col| match col {
            0 => TableCell::from(format!("{} {}", NAMES[row % NAMES.len()], row / NAMES.len() + 1)),
            1 => TableCell::from(TEAMS[(row * 7) % TEAMS.len()]),
            2 => TableCell::Number(((row * 37) % 1000) as f64 / 10.0),
            3 => TableCell::Number(((row * 101) % 5000) as f64),
            _ if row % 3 == 0 => TableCell::from("review pending"),
            _ => TableCell::Empty,
        });
    if let Some(row) = r.activated {
        s.table_note = format!("Activated row {row}");
    }
    if r.sort_changed {
        s.table_note = format!("Sort: {:?}", r.sort.column);
    }
    if r.page_changed {
        "Page changed".clone_into(&mut s.table_note);
    }
    caption(
        ui,
        TypeScale::BodySmall,
        &format!(
            "{ROWS} rows. Click headers to sort (none, ascending, descending); Shift extends selection. {} selected. {}",
            s.table_sel.len(),
            s.table_note
        ),
    );
}

// A desktop outline: the densest step for this subtree only, 40dp rows, 24dp indent, LabelLarge.
fn outline(ui: &mut Mui<'_>, s: &mut State) {
    caption(
        ui,
        TypeScale::LabelMedium,
        "Dense outline (scoped density, row height, indent, label style)",
    );
    boxed(ui, 240.0, |ui| {
        ui.with_density(Density::MIN, |ui| {
            TreeView::new("demo-outline", "Outline", &s.tree, &mut s.outline_exp)
                .row_height(40.0)
                .indent(24.0)
                .label_style(TypeScale::LabelLarge)
                .leading(|n| ListSlot::new().icon(n.icon))
                .show(ui)
        })
    });
}

fn tree(ui: &mut Mui<'_>, s: &mut State, shared: &mut Shared) {
    let rt = ui.rt();
    let checked = &s.tree_checked;
    let r = boxed(ui, 400.0, |ui| {
        TreeView::new("demo-tree", "Project files", &s.tree, &mut s.tree_exp)
            .selection(&mut s.tree_sel)
            .leading(|n| ListSlot::new().checkbox(tree::check_state(n, checked)).icon(n.icon))
            .actions(|n| {
                if n.dir {
                    Vec::new()
                } else {
                    vec![IconAction::new(icons::ICON_DELETE, "Delete")]
                }
            })
            .reorderable(true)
            .show(ui)
    });
    for (key, row) in &r.rows {
        if Menu::context_requested(rt, row) {
            let menu = Menu::new(("tree-menu", *key), "Node actions")
                .item(MenuItem::new("Details"))
                .item(MenuItem::new("Delete"));
            match menu.show_context(rt, row).clicked {
                Some(0) => shared.toast(format!("Node {key}")),
                Some(1) => remove_node(&mut s.tree, *key),
                _ => {}
            }
        }
    }
    if let Some(key) = r.leading_toggled
        && let Some(node) = tree::find(&s.tree, &key)
    {
        tree::toggle_check(node, &mut s.tree_checked);
    }
    if let Some(TreeAction { node: key, .. }) = r.clicked {
        remove_node(&mut s.tree, key);
    }
    if let Some(m) = r.moved {
        m.apply(&mut s.tree);
    }
    if let Some(key) = r.expanded
        && let Some(node) = s.tree.iter_mut().find_map(|n| n.find_mut(key))
        && node.lazy
        && node.children.is_empty()
    {
        node.children = vec![
            Node::leaf(101, "archive.zip"),
            Node::leaf(102, "notes.txt"),
            Node::leaf(103, "photo.png"),
        ];
    }
    let picked = s
        .tree_sel
        .first()
        .and_then(|k| s.tree.iter_mut().find_map(|n| n.find_mut(*k)))
        .map_or("nothing", |n| n.label.as_str());
    caption(
        ui,
        TypeScale::BodySmall,
        &format!(
            "Selected: {picked}. {} checked. Arrow keys, Home/End and typeahead work; drag rows or press \
             Alt+arrows to move them; right-click for a menu.",
            s.tree_checked.len()
        ),
    );
}

fn avatars(ui: &mut Mui<'_>) {
    super::row(ui, |row| {
        row.add(|ui| ui.label("Initials"));
        for (i, name) in ["Ada Lovelace", "Grace Hopper", "Linus"].into_iter().enumerate() {
            row.add(|ui| ui.mineral(Avatar::new(("av", i), name)));
        }
        row.add(|ui| ui.label("Icon fallback"));
        row.add(|ui| ui.mineral(Avatar::new("av-icon", "").icon(icons::ICON_PERSON)));
        row.add(|ui| ui.label("Sizes: chip, search bar, top app bar, list item, custom 64dp"));
        let sizes = [
            AvatarSize::Chip,
            AvatarSize::SearchBar,
            AvatarSize::TopAppBar,
            AvatarSize::ListItem,
        ];
        for (i, size) in sizes.into_iter().enumerate() {
            row.add(|ui| ui.mineral(Avatar::new(("av-size", i), "Radia Perlman").size(size)));
        }
        row.add(|ui| ui.mineral(Avatar::new("av-size-custom", "Radia Perlman").fixed_size(egui::Vec2::splat(64.0))));
    });
    ui.add_space(8.0);
    super::row(ui, |row| {
        for (max, size) in [
            (3, AvatarSize::ListItem),
            (5, AvatarSize::ListItem),
            (8, AvatarSize::TopAppBar),
        ] {
            let group = NAMES.iter().enumerate().fold(
                AvatarGroup::new(("av-group", max)).max_visible(max).size(size),
                |g, (i, n)| g.avatar(Avatar::new(("av-g", max, i), *n)),
            );
            row.add(|ui| ui.mineral(group));
            row.space(24.0);
        }
    });
}

fn timeline(ui: &mut Mui<'_>) {
    let rt = ui.rt();
    let f = rt.i18n().format();
    let day = |d: i8| {
        f.date(
            Date::constant(2026, 11, d),
            i18n::DateFields::YearMonthDay,
            i18n::DateLength::Medium,
        )
    };
    rt.ui(ui).mineral(
        Timeline::new("demo-timeline")
            .item(
                TimelineItem::new("Release 1.0 shipped")
                    .time(day(14))
                    .supporting("Tagged and published.")
                    .icon(icons::ICON_RUN_CIRCLE)
                    .active(true),
            )
            .item(
                TimelineItem::new("Beta feedback triaged")
                    .time(day(9))
                    .supporting("Twelve issues closed."),
            )
            .item(TimelineItem::new("Feature freeze").time(day(2)).icon(icons::ICON_LOCK))
            .item(TimelineItem::new("Kickoff").time(day(1))),
    );
}

fn description(ui: &mut Mui<'_>, s: &mut State) {
    let rt = ui.rt();
    super::row(ui, |row| {
        row.add(|ui| s.dl_layout = super::choice(ui, "dl-layout", "Layout", &["Horizontal", "Vertical"], s.dl_layout));
        row.add(|ui| {
            s.dl_columns = 1 + super::choice(
                ui,
                "dl-columns",
                "Columns",
                &["1 col", "2 col", "3 col"],
                s.dl_columns - 1,
            );
        });
    });
    let layout = if s.dl_layout == 0 {
        DescriptionLayout::Horizontal
    } else {
        DescriptionLayout::Vertical
    };
    rt.ui(ui).mineral(
        DescriptionList::new("demo-dl")
            .layout(layout)
            .columns(s.dl_columns)
            .dividers(true)
            .item(DescriptionItem::term("Name").detail("egui_mineral"))
            .item(DescriptionItem::term("License").detail("GPL-3.0-or-later"))
            .item(DescriptionItem::term("Toolkit").detail("egui 0.37"))
            .item(DescriptionItem::term("Layout").detail("Taffy"))
            .item(DescriptionItem::term("Locale data").detail("ICU4X / CLDR"))
            .item(DescriptionItem::term("Targets").detail("Linux, macOS, Windows, wasm32")),
    );
}

fn empty(ui: &mut Mui<'_>, s: &mut State) {
    if EmptyPlaceholder::new("demo-empty", "No messages")
        .icon(icons::ICON_INBOX)
        .supporting("New messages appear here once someone writes to you.")
        .action("Compose")
        .show(ui)
        .action_clicked
    {
        s.empty_clicks += 1;
    }
    if s.empty_clicks > 0 {
        caption(
            ui,
            TypeScale::BodySmall,
            &format!("Action activated {} time(s)", s.empty_clicks),
        );
    }
}

fn statistics(ui: &mut Mui<'_>) {
    let rt = ui.rt();
    super::top_row(ui, |row| {
        row.add(|ui| {
            ui.vertical(|ui| {
                rt.ui(ui).mineral(
                    Statistic::new("st-rev", "Revenue", 12_345.67)
                        .fraction_digits(2)
                        .unit("EUR")
                        .trend(Trend::Percent(12.5)),
                );
            })
        });
        row.space(32.0);
        row.add(|ui| {
            ui.vertical(|ui| {
                rt.ui(ui).mineral(
                    Statistic::new("st-lat", "Latency", 182.0)
                        .unit("ms")
                        .trend(Trend::Absolute(-14.0))
                        .inverted(true)
                        .supporting("Lower is better"),
                );
            })
        });
        row.space(32.0);
        row.add(|ui| {
            ui.vertical(|ui| {
                rt.ui(ui).mineral(
                    Statistic::new("st-err", "Errors", 31.0)
                        .trend(Trend::Percent(8.0))
                        .inverted(true),
                );
            })
        });
        row.space(32.0);
        row.add(|ui| {
            ui.vertical(|ui| {
                rt.ui(ui).mineral(Statistic::new("st-users", "Users", 1_204_000.0));
            })
        });
    });
}

fn kbd(ui: &mut Mui<'_>) {
    super::row(ui, |row| {
        row.add(|ui| ui.label("Command palette"));
        row.add(|ui| ui.mineral(Kbd::new("kbd-palette", ["Ctrl", "K"])));
        row.space(16.0);
        row.add(|ui| ui.label("Redo"));
        row.add(|ui| ui.mineral(Kbd::new("kbd-redo", ["Ctrl", "Shift", "Z"])));
        row.space(16.0);
        row.add(|ui| ui.label("Inspector"));
        row.add(|ui| ui.mineral(Kbd::new("kbd-inspector", ["F12"])));
    });
}

fn link(ui: &mut Mui<'_>) {
    super::row(ui, |row| {
        row.add(|ui| ui.label("See the"));
        row.mineral(
            TextLink::new("link-egui", "egui repository")
                .url("https://github.com/emilk/egui")
                .new_tab(true),
        );
        row.add(|ui| ui.label("and the"));
        row.mineral(
            TextLink::new("link-md3", "Material 3 guidelines")
                .url("https://m3.material.io/")
                .new_tab(true)
                .scale(TypeScale::TitleSmall),
        );
    });
}

fn pull(ui: &mut Mui<'_>, s: &mut State, shared: &mut Shared) {
    if let Some(id) = s.refreshing
        && shared.done.contains(&id)
    {
        s.refreshing = None;
        s.refreshed += 1;
    }
    let r = boxed(ui, 240.0, |ui| {
        PullToRefresh::new("demo-pull")
            .refreshing(s.refreshing.is_some())
            .show(ui, |ui| {
                ui.label(format!(
                    "Refreshed {} time(s). Pull down, scroll up with the wheel, or use the indicator.",
                    s.refreshed
                ));
                for i in 0..20 {
                    ui.label(format!("Item {i}"));
                }
            })
    });
    if r.refreshed && s.refreshing.is_none() {
        let id = s.next_job;
        match shared.bus.send(Cmd::Work { id, millis: 1500 }) {
            Ok(()) => {
                s.next_job += 1;
                s.refreshing = Some(id);
            }
            Err(_) => shared.toast("Worker is gone"),
        }
    }
}

fn virtual_list(ui: &mut Mui<'_>, s: &mut State) {
    let rt = ui.rt();
    super::row(ui, |row| {
        row.add(|ui| {
            s.vl_variable = super::choice(
                ui,
                "vl-kind",
                "Row heights",
                &["Uniform", "Variable"],
                usize::from(s.vl_variable),
            ) == 1;
        });
        row.add(|ui| super::count(ui, "vl-target", "Target row", &mut s.vl_target, 9_999));
        if row
            .mineral(PushButton::new("vl-scroll-top", "Scroll to top edge").variant(ButtonVariant::Tonal))
            .clicked()
        {
            s.vl_scroll = Some(s.vl_target);
        }
    });
    let scroll = s.vl_scroll.take().map(|r| (r, Some(Align::Min)));
    let tint = rt.tokens().color.on_surface_variant;
    boxed(ui, 260.0, |ui| {
        let list = if s.vl_variable {
            VirtualList::variable("vl-var", 40.0)
        } else {
            VirtualList::uniform("vl-uni", 24.0)
        };
        let list = match scroll {
            Some((row, a)) => list.scroll_to(row, a),
            None => list,
        };
        let variable = s.vl_variable;
        list.show(ui, 10_000, |ui, i| {
            if variable {
                ui.label(RichText::new(format!("Row {i}")).strong());
                for l in 0..=(i % 4) {
                    ui.colored_label(tint, format!("detail line {}", l + 1));
                }
            } else {
                ui.label(format!("Row {i}"));
            }
        });
    });
}
