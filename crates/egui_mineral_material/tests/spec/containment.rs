use egui::{Sense, vec2};
use egui_mineral_material::{
    containment::{Card, CardVariant, ListItem},
    spec::tokens::{elevated_card, filled_card, list, outlined_card},
};

use crate::support::render;

#[test]
fn card_variants_share_geometry_and_wrap_their_content() {
    let content = vec2(100.0, 40.0);
    let rects = [CardVariant::Elevated, CardVariant::Filled, CardVariant::Outlined].map(|v| {
        let p = render(|ui| {
            Card::new("c", "Card").variant(v).show(ui, |ui| {
                ui.allocate_exact_size(content, Sense::hover());
            });
        });
        let fill = match v {
            CardVariant::Elevated => elevated_card::CONTAINER_COLOR,
            CardVariant::Filled => filled_card::CONTAINER_COLOR,
            CardVariant::Outlined => outlined_card::CONTAINER_COLOR,
        };
        let c = p.one(fill);
        if v == CardVariant::Outlined {
            let outline = p.stroked(outlined_card::OUTLINE_COLOR);
            assert_eq!(outline, [c], "outline inside the container");
        }
        c
    });
    assert_eq!(rects[0].size(), content, "no padding of its own");
    assert!(rects.iter().all(|r| *r == rects[0]), "same geometry: {rects:?}");
}

#[test]
fn sized_card_pads_its_content() {
    let (width, content) = (220.0, vec2(10.0, 10.0));
    let p = render(|ui| {
        Card::new("c", "Card")
            .variant(CardVariant::Filled)
            .width(width)
            .padding(Card::CONTENT_PADDING)
            .show(ui, |ui| {
                ui.allocate_exact_size(content, Sense::hover());
            });
    });
    assert_eq!(
        p.one(filled_card::CONTAINER_COLOR).size(),
        vec2(width, content.y + Card::CONTENT_PADDING.sum().y)
    );
}

#[test]
fn list_items_follow_line_heights() {
    let cases: [(fn() -> ListItem, f32); 3] = [
        (|| ListItem::new("l", "One"), list::ITEM_ONE_LINE_CONTAINER_HEIGHT),
        (
            || ListItem::new("l", "Two").supporting("Supporting"),
            list::ITEM_TWO_LINE_CONTAINER_HEIGHT,
        ),
        (
            || {
                ListItem::new("l", "Three")
                    .supporting("Supporting")
                    .overline("Overline")
            },
            list::ITEM_THREE_LINE_CONTAINER_HEIGHT,
        ),
    ];
    for (item, height) in cases {
        let p = render(|ui| {
            ui.mineral(item());
        });
        assert_eq!(p.one(list::ITEM_CONTAINER_COLOR).height(), height);
    }
}
