use egui::Shape;
use egui_mineral::{expand::Expansion, select::Selection};
use egui_mineral_material::{
    extras::{Accordion, AccordionPanel, Avatar, DataTable, Pagination, Rating, SettingRow, TableCell, TreeView},
    spec::{
        defaults::list as list_defaults,
        tokens::{button_small, divider, expanded_list, filled_tonal_button, list, outlined_card, small_icon_button},
    },
};

use crate::support::{TreeNode, render};

#[test]
fn table_rows_are_one_line_list_items() {
    let rows = 3;
    let p = render(|ui| {
        DataTable::new("t", "Table", ["Name"]).show(ui, rows, |r, _| TableCell::from(format!("Row {r}")));
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
            a.panel(0, "One", |_| ());
            a.panel(1, AccordionPanel::new("Two").supporting("Sub"), |_| ());
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
fn flat_accordion_is_a_plain_section_header_without_a_container() {
    use egui_mineral_material::{MaterialUiExt as _, adaptive::Density, extras::AccordionVariant};
    let mut expansion = Expansion::default();
    let p = render(|ui| {
        ui.with_density(Density::MIN, |ui| {
            Accordion::new("acc", &mut expansion)
                .variant(AccordionVariant::Flat)
                .show(ui, |a| {
                    a.panel(0, "One", |_| ());
                });
        });
    });
    let one = list::ITEM_ONE_LINE_CONTAINER_HEIGHT;
    assert!(
        p.outer.height() < one + divider::THICKNESS,
        "dense header, divider below"
    );
    assert!(p.outer.height() >= divider::THICKNESS);
    assert_eq!(p.text("One").left(), p.outer.left(), "no start padding");
    assert_eq!(p.filled(expanded_list::EXPANDED_ITEM_CONTAINER_COLOR), []);
    assert_eq!(p.stroked(outlined_card::OUTLINE_COLOR), []);
    assert!(!p.filled(divider::COLOR).is_empty(), "divider under the header");
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
        extras::{ColorPicker, Combobox, NumberField},
        spec::TypeScale,
    };
    let content = TypeScale::BodyLarge.line_height() + 8.0;
    let (mut x, mut text) = (1.5_f64, String::new());
    let mut color = egui::Color32::RED;
    let p = crate::support::render_zones(|ui| {
        ui.with_density(Density::MIN, |ui| {
            let n = NumberField::new("x", "X", RangeModel::new(0.0, 9.0), &mut x)
                .label_visible(false)
                .step_buttons(false)
                .suffix("mm");
            let zones = ui.mineral(n).zones();
            ui.mineral(
                Combobox::optional("c", "Unit", &mut None::<usize>, &mut text)
                    .items(["a"].into_iter().enumerate())
                    .label_visible(false)
                    .prefix("#")
                    .placeholder("pick"),
            );
            ui.mineral(ColorPicker::new("cp", "Color", &mut color).label_visible(false));
            zones
        })
    });
    let field = p.zones.expect("zones reported").visual;
    assert_eq!(field.height(), content);
    assert_eq!(p.node("x-field").name, "X", "hidden label stays the name");
    assert!(field.contains_rect(p.text("mm")) && p.text("mm").left() > field.center().x);
    p.text("#");
    p.text("pick");
    assert_eq!(p.node("cp-hex-field").rect.height(), content, "color picker hex field");
}

#[test]
fn date_and_tag_fields_take_density_label_visibility_and_affixes() {
    use egui_mineral_material::{
        MaterialUiExt as _,
        adaptive::Density,
        extras::{DateField, TagsField},
        spec::TypeScale,
    };
    let content = TypeScale::BodyLarge.line_height() + 8.0;
    let mut date = None;
    let p = crate::support::render_zones(|ui| {
        ui.with_density(Density::MIN, |ui| {
            ui.mineral(
                DateField::new("d", "Date", &mut date)
                    .label_visible(false)
                    .suffix("UTC"),
            )
            .zones()
        })
    });
    let field = p.zones.expect("zones reported").visual;
    assert_eq!(field.height(), content);
    assert_eq!(p.node("d-date").name, "Date");
    assert!(field.contains_rect(p.text("UTC")) && p.text("UTC").left() > field.center().x);

    let mut tags = vec!["a".to_owned()];
    let p = crate::support::render_zones(|ui| {
        ui.with_density(Density::MIN, |ui| {
            ui.mineral(TagsField::new("t", "Tags", &mut tags).label_visible(false).prefix("#"))
                .zones()
        })
    });
    assert_eq!(p.node("t-tags").name, "Tags");
    p.text("#");
}

#[test]
fn otp_label_sits_above_the_cells_unless_hidden() {
    use egui_mineral_material::{extras::OtpField, spec::TypeScale};
    let mut code = String::new();
    let mut shown = |visible: bool| {
        crate::support::render_zones(|ui| {
            ui.mineral(OtpField::new("o", "Code", 4, &mut code).label_visible(visible))
                .zones()
        })
    };
    let (with, without) = (shown(true), shown(false));
    let (visual, bare) = (
        with.zones.expect("zones reported").visual,
        without.zones.expect("zones reported").visual,
    );
    let label_h = TypeScale::BodySmall.line_height() + 4.0;
    assert_eq!(visual.height() - bare.height(), label_h);
    assert!(with.text("Code").bottom() <= visual.top() + label_h);
    assert_eq!(without.node("o-otp").name, "Code");
    assert_eq!(with.node("o-otp").name, "Code");
}

#[test]
fn otp_cells_follow_density() {
    use egui_mineral_material::{MaterialUiExt as _, adaptive::Density, extras::OtpField};
    let mut code = String::new();
    let mut height = |step: i8| {
        crate::support::render_zones(|ui| {
            ui.with_density(Density::new(step), |ui| {
                ui.mineral(OtpField::new("o", "Code", 4, &mut code).label_visible(false))
                    .zones()
            })
        })
        .zones
        .expect("zones reported")
        .visual
        .height()
    };
    let (full, dense) = (height(0), height(-6));
    assert!(dense < full, "{dense} vs {full}");
}
