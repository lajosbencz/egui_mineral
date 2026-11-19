use egui::Shape;
use egui_mineral::{expand::Expansion, select::Selection};
use egui_mineral_material::{
    extras::{Accordion, AccordionPanel, Avatar, DataTable, Pagination, Rating, SettingRow, TableCell, TreeView},
    spec::{
        defaults::list as list_defaults,
        tokens::{button_small, divider, filled_tonal_button, list, small_icon_button},
    },
};

use crate::support::{TreeNode, render};

#[test]
fn table_rows_are_one_line_list_items() {
    let rows = 3;
    let p = render(|ui| {
        DataTable::new("t", "Table", ["Name"], rows).show(ui, |r, _| TableCell::from(format!("Row {r}")));
    });
    let painted = p.filled(list::ITEM_CONTAINER_COLOR);
    assert_eq!(painted.len(), rows + 1, "header and rows: {painted:?}");
    assert!(
        painted
            .iter()
            .all(|r| r.height() == list::ITEM_ONE_LINE_CONTAINER_HEIGHT),
        "{painted:?}"
    );
    let text = p.text("Row 0");
    assert_eq!(text.left() - painted[0].left(), list_defaults::START_PADDING);
    assert_eq!(text.center().y, painted[1].center().y, "cell text centered");
}

#[test]
fn tree_rows_indent_by_one_leading_slot() {
    let roots = [TreeNode::branch(1, "root", vec![TreeNode::leaf(2, "child")])];
    let mut expansion = Expansion::default();
    expansion.expand(1);
    let mut selection = Selection::single();
    let p = render(|ui| {
        TreeView::new("tree", "Tree", &roots, &mut expansion)
            .selection(&mut selection)
            .show(ui);
    });
    let rows = p.filled(list::ITEM_CONTAINER_COLOR);
    assert_eq!(rows.len(), 2, "{rows:?}");
    assert_eq!(rows[0].height(), list::ITEM_ONE_LINE_CONTAINER_HEIGHT);
    let (root, child) = (p.text("root"), p.text("child"));
    let leading = list::ITEM_LEADING_ICON_SIZE + list_defaults::LEADING_CONTENT_END_PADDING;
    assert_eq!(
        root.left() - rows[0].left(),
        list_defaults::START_PADDING + leading,
        "label after the chevron slot"
    );
    assert_eq!(child.left() - root.left(), leading, "one level");
}

#[test]
fn avatar_is_the_list_leading_avatar() {
    let p = render(|ui| {
        ui.mineral(Avatar::new("a", "Ada Lovelace"));
    });
    let fill = p.color(list::ITEM_LEADING_AVATAR_COLOR);
    let radius = p
        .shapes
        .iter()
        .find_map(|s| match &s.shape {
            Shape::Circle(c) if c.fill == fill => Some(c.radius),
            _ => None,
        })
        .expect("avatar circle");
    assert_eq!(2.0 * radius, list::ITEM_LEADING_AVATAR_SIZE);
    assert_eq!(
        p.text("AL").height(),
        list::ITEM_LEADING_AVATAR_LABEL_FONT.line_height()
    );
}

#[test]
fn current_page_is_a_small_tonal_button() {
    let mut page = 1;
    let p = render(|ui| {
        ui.mineral(Pagination::new("p", 5, &mut page));
    });
    let current = p.one(filled_tonal_button::CONTAINER_COLOR);
    assert_eq!(current.height(), button_small::CONTAINER_HEIGHT);
    assert_eq!(p.text("2").center(), current.center(), "label centered");
}

#[test]
fn accordion_headers_are_list_items() {
    let mut expansion = Expansion::default();
    let p = render(|ui| {
        Accordion::new("acc", &mut expansion).show(ui, |a| {
            a.panel("One", |_| ());
            a.panel(AccordionPanel::new("Two").supporting("Sub"), |_| ());
        });
    });
    let one = list::ITEM_ONE_LINE_CONTAINER_HEIGHT;
    let two = list::ITEM_TWO_LINE_CONTAINER_HEIGHT;
    assert_eq!(p.outer.height(), one + divider::THICKNESS + two);
    assert_eq!(p.text("One").left(), p.outer.left() + list_defaults::START_PADDING);
    assert!(
        (p.text("One").center().y - (p.outer.top() + one / 2.0)).abs() < 1.0,
        "one-line headline centers in its row"
    );
}

#[test]
fn rating_stars_are_narrow_small_icon_buttons() {
    let mut value = 2.0;
    let p = render(|ui| {
        ui.mineral(Rating::new("rate", "Rating", &mut value));
    });
    let star = small_icon_button::ICON_SIZE
        + small_icon_button::NARROW_LEADING_SPACE
        + small_icon_button::NARROW_TRAILING_SPACE;
    let stars = p.node("rate-rating").state.range.expect("rating range").max;
    assert_eq!(p.outer.width(), stars as f32 * star, "{stars} stars");
    assert_eq!(p.outer.height(), small_icon_button::CONTAINER_HEIGHT);
}

#[test]
fn setting_row_is_a_list_item() {
    let mut stacked = false;
    let p = render(|ui| {
        stacked = SettingRow::new("row", "Label").show_value(ui, "On").stacked;
    });
    assert!(stacked, "the test screen is narrower than the first breakpoint");
    assert_eq!(p.outer.height(), list::ITEM_ONE_LINE_CONTAINER_HEIGHT);
    let (label, value) = (p.text("Label"), p.text("On"));
    assert_eq!(label.left(), p.outer.left() + list_defaults::START_PADDING);
    assert_eq!(value.left(), label.left());
    assert_eq!(value.top(), label.bottom());
}

#[test]
fn field_wrappers_forward_the_text_chrome() {
    use egui_mineral::range::RangeModel;
    use egui_mineral_material::{
        MaterialUiExt as _,
        adaptive::Density,
        extras::{Combobox, NumberField},
        spec::TypeScale,
    };
    let content = TypeScale::BodyLarge.line_height() + 8.0;
    let (mut x, mut text) = (1.5_f64, String::new());
    let p = crate::support::render_zones(|ui| {
        ui.with_density(Density::MIN, |ui| {
            let n = NumberField::new("x", "X", RangeModel::new(0.0, 9.0), &mut x)
                .label_visible(false)
                .step_buttons(false)
                .suffix("mm");
            let zones = ui.mineral(n).zones();
            ui.mineral(Combobox::new("c", "Unit", ["a"], &mut text).label_visible(false).prefix("#").placeholder("pick"));
            zones
        })
    });
    let field = p.zones.expect("zones reported").visual;
    assert_eq!(field.height(), content);
    assert_eq!(p.node("x-field").name, "X", "hidden label stays the name");
    assert!(field.contains_rect(p.text("mm")) && p.text("mm").left() > field.center().x);
    p.text("#");
    p.text("pick");
}
