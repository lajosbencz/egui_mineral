use egui_mineral::expand::Expansion;
use egui_mineral_fluent::{
    FluentExt, Size,
    spec::ColorToken,
    spec::defaults::{accordion, card},
    surfaces::{Accordion, AccordionItem, AccordionSize, Card, CardAppearance},
};

use crate::support::render;

#[test]
fn card_pads_its_body_by_the_size() {
    for (size, pad) in [
        (Size::Small, card::SIZE_SMALL),
        (Size::Medium, card::SIZE_MEDIUM),
        (Size::Large, card::SIZE_LARGE),
    ] {
        let p = render(|ui| {
            Card::new("c").size(size).show(ui, |c| {
                c.body(|ui| {
                    ui.label("Body");
                });
            });
        });
        let t = p.rt.tokens();
        let fill = p.one(t.color(ColorToken::ColorNeutralBackground1));
        let text = p.text("Body");
        assert!(
            (text.left() - fill.left() - pad).abs() < 1.5,
            "{size:?}: {text:?} {fill:?}"
        );
    }
}

#[test]
fn card_appearances_fill_with_their_background() {
    for (appearance, token) in [
        (CardAppearance::Filled, ColorToken::ColorNeutralBackground1),
        (CardAppearance::FilledAlternative, ColorToken::ColorNeutralBackground2),
    ] {
        let p = render(|ui| {
            Card::new("c").appearance(appearance).show(ui, |c| c.body(|_| ()));
        });
        let t = p.rt.tokens();
        assert_eq!(p.fills(t.color(token)).len(), 1, "{appearance:?}");
    }
}

#[test]
fn header_is_at_least_the_size_minimum() {
    for (size, min) in [
        (AccordionSize::Small, accordion::HEADER_MIN_HEIGHT_SMALL),
        (AccordionSize::Medium, accordion::HEADER_MIN_HEIGHT),
    ] {
        let mut open = Expansion::default();
        let p = render(|ui| {
            Accordion::new("a", &mut open).size(size).show(ui, |a| {
                a.item(AccordionItem::new("One"), |_| ());
            });
        });
        let tree = p.rt.tree(egui::ViewportId::ROOT).expect("semantics collected");
        let node = tree.find_key("a-item-0-header").expect("header node");
        assert!(node.rect.height() >= min, "{size:?}: {:?}", node.rect);
    }
}
