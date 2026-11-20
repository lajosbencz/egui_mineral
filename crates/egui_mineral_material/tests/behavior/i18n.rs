use std::{collections::BTreeSet, fs, path::Path};

fn catalog_ids(ftl: &str) -> BTreeSet<String> {
    ftl.lines()
        .filter_map(|l| l.split_once('=').map(|(id, _)| id))
        .filter(|id| !id.starts_with([' ', '#', '-']) && !id.trim().is_empty())
        .map(|id| id.trim().to_owned())
        .collect()
}

#[test]
fn translations_define_no_id_the_source_catalog_lacks() {
    let locales = Path::new(env!("CARGO_MANIFEST_DIR")).join("locales");
    let read = |path: &Path| fs::read_to_string(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let source = catalog_ids(&read(&locales.join("en-US.ftl")));
    assert!(!source.is_empty(), "en-US.ftl has messages");
    for entry in fs::read_dir(&locales).expect("locales") {
        let path = entry.expect("dir entry").path();
        let ids = catalog_ids(&read(&path));
        let stale: Vec<_> = ids.difference(&source).collect();
        assert!(stale.is_empty(), "{}: ids en-US.ftl lacks: {stale:?}", path.display());
    }
}

mod mirror {
    use egui::{Rect, Shape, Vec2, vec2};
    use egui_mineral::{date::Date, expand::Expansion, i18n::Direction, select::Selection, steps::Steps};
    use egui_mineral_material::{
        IconAction,
        actions::{ButtonIcon, IconButton, PushButton, SplitButton},
        communication::Badge,
        communication::LinearProgressIndicator,
        containment::{Card, Dialog, SideSheet},
        extras::{
            Accordion, Avatar, AvatarGroup, Breadcrumbs, DateField, DescriptionItem, DescriptionList, InlineEdit, Kbd,
            Pagination, Rating, Skeleton, SkeletonShape, Statistic, TagsField, Timeline, TimelineItem,
        },
        extras::{
            CommandPalette, DataTable, MenuStrip, PaletteCommand, SettingRow, Stepper, TableCell, TransferList,
            TreeView,
        },
        icons::{ICON_ADD, ICON_ARROW_DROP_DOWN, ICON_EDIT, ICON_HOME, ICON_MAIL, ICON_SHARE},
        navigation::{FloatingToolbar, NavDestination, NavigationBar, NavigationDrawer, TopAppBar},
        navigation::{Tab, Tabs},
        selection::{CheckButton, Chip, Switch},
        selection::{DatePicker, Menu, MenuItem, Select, SelectOption},
        spec::TypeScale,
        text_inputs::{TextField, TextFieldVariant},
    };

    use egui_mineral::widget::SizeExt;

    use crate::support::{Mui, Pass, TreeNode, render_directed};

    const SIZE: Vec2 = vec2(400.0, 2400.0);
    const WIDE: Vec2 = vec2(800.0, 1600.0);

    fn bounds(pass: &Pass, flip: bool, width: f32) -> Vec<Rect> {
        fn walk(s: &Shape, out: &mut Vec<Rect>) {
            match s {
                Shape::Vec(v) => v.iter().for_each(|s| walk(s, out)),
                // Galley bounds: ink may stop short of the advance that start alignment uses.
                Shape::Text(t) if t.angle == 0.0 => out.push(t.galley.rect.translate(t.pos.to_vec2())),
                s => out.push(s.visual_bounding_rect()),
            }
        }
        let mut rects = Vec::new();
        pass.shapes.iter().for_each(|c| walk(&c.shape, &mut rects));
        rects.retain(|r| r.is_finite() && r.is_positive());
        if flip {
            for r in &mut rects {
                *r = Rect::from_x_y_ranges(width - r.right()..=width - r.left(), r.y_range());
            }
        }
        rects
    }

    fn unmatched(a: &[Rect], b: &[Rect]) -> Vec<Rect> {
        let mut free = b.to_vec();
        a.iter()
            .filter(|r| {
                let near = |o: &Rect| (r.min - o.min).length() <= 1.0 && (r.max - o.max).length() <= 1.0;
                free.iter().position(near).map(|i| free.swap_remove(i)).is_none()
            })
            .copied()
            .collect()
    }

    fn widgets(ui: &mut Mui<'_>) {
        let (mut text, mut on, mut rating, mut tab, mut page) = (String::from("text"), true, 2.5, 1, 3);
        let mut date = Some(Date::constant(2025, 6, 10));
        let mut tags = vec![String::from("one"), String::from("two")];
        ui.mineral(
            TextField::new("tf", "Label", &mut text)
                .icon(ICON_MAIL)
                .prefix("$")
                .suffix("kg")
                .supporting("Help"),
        );
        ui.mineral(
            TextField::new("tfo", "Label", &mut text)
                .variant(TextFieldVariant::Outlined)
                .icon(ICON_MAIL)
                .supporting("Help"),
        );
        ui.mineral(PushButton::new("b", "Button").icon(ICON_ADD));
        ui.mineral(PushButton::new("bt", "Menu").icon(ButtonIcon::Trailing(ICON_ARROW_DROP_DOWN.into())));
        ui.mineral(Chip::input("chip", "Chip"));
        ui.mineral(CheckButton::new("cb", "Check", &mut on));
        ui.mineral(Switch::new("sw", "Switch", &mut on));
        ui.mineral(Rating::new("r", "Rate", &mut rating));
        ui.mineral(LinearProgressIndicator::determinate("lp", 0.4));
        Tabs::new("tabs", "Tabs", &mut tab)
            .item(Tab::new("One"))
            .item(Tab::new("Two").icon(ICON_MAIL))
            .show(ui);
        ui.mineral(Statistic::new("st", "Label", 42.0).supporting("Up"));
        ui.mineral(
            DescriptionList::new("dl")
                .items([
                    DescriptionItem::term("Term").detail("Detail"),
                    DescriptionItem::term("Longer term").detail("x"),
                ])
                .columns(2),
        );
        ui.mineral(
            Timeline::new("tl")
                .item(TimelineItem::new("First").supporting("Detail"))
                .item(TimelineItem::new("Second")),
        );
        ui.mineral(AvatarGroup::new("ag").avatars([Avatar::new("a", "Ann"), Avatar::new("b", "Bob")]));
        ui.mineral(Kbd::new("k", ["Ctrl", "K"]));
        ui.mineral(Skeleton::new("sk", SkeletonShape::text(3, TypeScale::BodyMedium)));
        ui.mineral(Pagination::new("pg", 5, &mut page));
        Breadcrumbs::new("bc").item("Home").item("Docs").show(ui);
        ui.mineral(DateField::new("df", "Date", &mut date));
        ui.mineral(TagsField::new("tg", "Tags", &mut tags));
        ui.mineral(InlineEdit::new("ie", "Label", &mut text));
    }

    fn composites(ui: &mut Mui<'_>) {
        let (mut tab, mut chosen, mut selected) = (0, vec![1], Some(0));
        let mut steps = Steps::new(3);
        let mut expansion = Expansion::default();
        expansion.expand(1);
        let mut selection = Selection::multiple();
        let roots = [TreeNode::branch(1, "root", vec![TreeNode::leaf(2, "child")])];
        let menu = Menu::new("more", "More").item(MenuItem::new("Draft"));
        SplitButton::new("send", "Send", menu).show(ui);
        TopAppBar::new("bar", "Title")
            .actions([IconAction::new(ICON_EDIT, "Edit"), IconAction::new(ICON_SHARE, "Share")])
            .show(ui);
        DataTable::new("t", "Table", ["Name", "Size"], 2)
            .selection(&mut selection)
            .show(ui, |r, c| match c {
                0 => TableCell::from(format!("Row {r}")),
                _ => TableCell::Number(r as f64),
            });
        TreeView::new("tree", "Tree", &roots, &mut expansion).show(ui);
        // Wider than its content: the content hugs the reading start.
        ui.mineral(TransferList::new("tl", ["Red", "Green", "Blue"], &mut chosen).preferred_width(600.0));
        SettingRow::new("row", "Label").supporting("Help").show_value(ui, "On");
        Stepper::new("flow", &mut steps)
            .step("Account")
            .step("Plan")
            .show(ui, |ui, i| {
                ui.label(format!("Step {i}"));
            });
        NavigationBar::new("nav", &mut tab)
            .item(NavDestination::new(ICON_HOME, "Home"))
            .item(NavDestination::new(ICON_MAIL, "Mail"))
            .show(ui);
        MenuStrip::new("strip")
            .menu(Menu::new("file", "File").item(MenuItem::new("Open")))
            .menu(Menu::new("edit", "Edit").item(MenuItem::new("Copy")))
            .show(ui);
        FloatingToolbar::new("tb", "Edit")
            .actions([IconAction::new(ICON_EDIT, "Edit"), IconAction::new(ICON_SHARE, "Share")])
            .show(ui);
        ui.mineral(IconButton::new("mail", ICON_MAIL, "Mail").badge(Badge::count(12)));
        let options = [SelectOption::new("Wav"), SelectOption::new("Flac")];
        ui.mineral(Select::new("format", "Format", options, &mut selected));
    }

    type Gallery = fn(&mut Mui<'_>);

    fn assert_mirrored(what: &str, size: Vec2, add: impl Fn(&mut Mui<'_>) + Copy) {
        let gallery = |ui: &mut Mui<'_>| {
            let rt = ui.rt();
            let layout = ui.i18n().direction().column();
            ui.with_layout(layout, |ui| add(&mut rt.ui(ui)));
        };
        let ltr = bounds(&render_directed(Direction::Ltr, size, gallery), false, size.x);
        let rtl = bounds(&render_directed(Direction::Rtl, size, gallery), true, size.x);
        assert_ne!(ltr, [], "{what} paints");
        assert_eq!(
            (unmatched(&ltr, &rtl), unmatched(&rtl, &ltr)),
            (vec![], vec![]),
            "{what}: (LTR only, mirrored RTL only) shape bounds"
        );
    }

    #[test]
    fn rtl_paints_widgets_mirrored() {
        assert_mirrored("widgets", SIZE, widgets);
    }

    #[test]
    fn rtl_paints_composites_mirrored() {
        assert_mirrored("composites", WIDE, composites);
    }

    #[test]
    fn rtl_containers_lay_content_from_the_reading_start() {
        let containers: [(&str, Gallery); 3] = [
            ("card", |ui| {
                Card::new("card", "Card")
                    .width(300.0..=300.0)
                    .show(ui, |ui| ui.mineral(PushButton::new("body", "Body")));
            }),
            ("accordion", |ui| {
                let mut open = Expansion::single();
                open.expand(0);
                Accordion::new("acc", &mut open).show(ui, |a| {
                    a.panel("Panel", |ui| ui.mineral(PushButton::new("body", "Body")));
                });
            }),
            ("dialog", |ui| {
                Dialog::new("dlg", "Title")
                    .show_modal(ui.rt(), &mut true, |ui| ui.mineral(PushButton::new("body", "Body")));
            }),
        ];
        for (what, add) in containers {
            assert_mirrored(what, WIDE, add);
        }
    }

    #[test]
    fn rtl_paints_overlays_mirrored() {
        // Time pickers are left out: the dial and HH:MM keep their orientation in RTL.
        let overlays: [(&str, Gallery); 5] = [
            ("full-screen dialog", |ui| {
                Dialog::full_screen("fs", "Edit")
                    .confirm("Save")
                    .show_modal(ui.rt(), &mut true, |_| ());
            }),
            ("drawer", |ui| {
                NavigationDrawer::new("nav", &mut 0)
                    .item(NavDestination::new(ICON_HOME, "Home"))
                    .show_modal(ui.rt(), &mut true);
            }),
            ("side sheet", |ui| {
                SideSheet::new("sheet")
                    .headline("Title")
                    .show_modal(ui.rt(), &mut true, |_| ());
            }),
            ("date picker", |ui| {
                DatePicker::new("dp", "Date", &mut Some(Date::constant(2025, 6, 10))).show_modal(ui.rt(), &mut true);
            }),
            ("palette", |ui| {
                CommandPalette::new("pal", [PaletteCommand::new("Save")], &mut String::new())
                    .show_modal(ui.rt(), &mut true);
            }),
        ];
        for (what, add) in overlays {
            assert_mirrored(what, WIDE, add);
        }
    }
}
