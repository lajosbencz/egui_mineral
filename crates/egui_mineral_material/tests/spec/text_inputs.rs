use egui_mineral_material::{
    icons::ICON_SEARCH,
    spec::{TypeScale, defaults::text_field, tokens::filled_text_field},
    text_inputs::{TextField, TextFieldVariant},
};

use crate::support::{Pass, min_interactive, render_zones};

fn render(mut field: impl FnMut(&mut String) -> TextField<'_>) -> (Pass, egui::Rect) {
    let mut value = "Input".to_owned();
    let p = render_zones(|ui| ui.mineral(field(&mut value)).zones());
    let visual = p.zones.expect("zones reported").visual;
    (p, visual)
}

#[test]
fn filled_label_and_input_lines() {
    let (p, visual) = render(|v| TextField::new("f", "Label", v));
    assert_eq!(visual.height(), text_field::MIN_HEIGHT);
    assert_eq!(p.rect("f-field"), p.outer, "nothing reserved above");
    let input = p.text("Input");
    let top = text_field::LABEL_VERTICAL_PADDING + text_field::MIN_FOCUSED_LABEL_LINE_HEIGHT;
    assert_eq!(input.top() - visual.top(), top);
    assert_eq!(input.height(), TypeScale::BodyLarge.line_height());
    assert_eq!(input.left() - visual.left(), text_field::PADDING);
    let indicator = p
        .shapes
        .iter()
        .find_map(|s| match &s.shape {
            egui::Shape::Rect(r) if r.rect.x_range() == visual.x_range() && r.rect.height() < visual.height() => {
                Some(r.rect)
            }
            _ => None,
        })
        .expect("indicator painted");
    assert_eq!(indicator.bottom(), visual.bottom());
    assert_eq!(indicator.height(), filled_text_field::ACTIVE_INDICATOR_HEIGHT);
}

#[test]
fn outlined_label_margin_and_supporting_row() {
    let (p, visual) = render(|v| {
        TextField::new("o", "Label", v)
            .variant(TextFieldVariant::Outlined)
            .supporting("Help")
    });
    // Half the minimized label line sits above the container, inside the reserved margin.
    let cutout = text_field::MIN_FOCUSED_LABEL_LINE_HEIGHT / 2.0;
    assert_eq!(visual.top() - p.outer.top(), cutout);
    let hit = p.rect("o-field");
    assert_eq!(hit.top() - p.outer.top(), cutout, "the cutout is not hit");
    let label = p.text("Label");
    assert!(p.outer.contains_rect(label), "label {label:?} overhangs {:?}", p.outer);
    let help = p.text("Help");
    assert_eq!(help.top() - visual.bottom(), text_field::SUPPORTING_TOP_PADDING);
    assert_eq!(help.left() - visual.left(), text_field::PADDING);
    assert_eq!(p.text("Input").center().y, visual.center().y);
}

#[test]
fn leading_icon_inset() {
    let (p, visual) = render(|v| TextField::new("i", "", v).icon(ICON_SEARCH));
    let slot = min_interactive();
    let icon_padding = (slot - filled_text_field::LEADING_ICON_SIZE) / 2.0;
    let inset = slot + text_field::PADDING - icon_padding;
    assert_eq!(p.text("Input").left() - visual.left(), inset);
}
