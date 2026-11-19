use egui::ViewportId;
use egui_mineral_fluent::{
    spec::defaults::{card, divider},
    surfaces::{Card, CardAppearance, Divider, DividerAlign},
};

use crate::support::render;

#[test]
fn plain_divider_is_one_full_width_line() {
    let p = render(|ui| {
        ui.mineral(Divider::new("d"));
    });
    let lines = p.hlines();
    assert_eq!(lines.len(), 1, "{lines:?}");
    let (l, r, _) = lines[0];
    assert_eq!(r - l, crate::support::SCREEN.x);
}

#[test]
fn inset_divider_frees_both_ends() {
    let p = render(|ui| {
        ui.mineral(Divider::new("d").inset(true));
    });
    let (l, r, _) = p.hlines()[0];
    assert_eq!(l, divider::INSET_SPACING);
    assert_eq!(r, crate::support::SCREEN.x - divider::INSET_SPACING);
}

#[test]
fn start_content_has_the_short_leading_line() {
    let p = render(|ui| {
        ui.mineral(Divider::new("d").content("Text").align_content(DividerAlign::Start));
    });
    let mut lines = p.hlines();
    lines.sort_by(|a, b| a.0.total_cmp(&b.0));
    assert_eq!(lines.len(), 2);
    assert_eq!(lines[0].1 - lines[0].0, divider::START_END_LENGTH);
    let text = p.text("Text");
    assert_eq!(text.left() - lines[0].1, divider::CONTENT_SPACING);
    assert_eq!(lines[1].0 - text.right(), divider::CONTENT_SPACING);
}

#[test]
fn card_checkbox_floats_at_the_end_top_corner() {
    let p = render(|ui| {
        let mut selected = false;
        Card::new("c")
            .appearance(CardAppearance::FilledAlternative)
            .selectable(&mut selected)
            .checkbox(true)
            .show(ui, |c| c.body(|ui| _ = ui.label("Body")));
    });
    let tree = p.rt.tree(ViewportId::ROOT).expect("semantics collected");
    let card_rect = tree.find_key("c-card").expect("card").rect;
    let check = tree.find_key("c-select-checkbox").expect("checkbox").rect;
    assert!(
        check.right() <= card_rect.right() - card::SELECT_INSET + 0.5,
        "{check:?} {card_rect:?}"
    );
    assert!(check.left() > card_rect.center().x, "{check:?} {card_rect:?}");
    assert!(
        check.top() >= card_rect.top() + card::SELECT_INSET - 0.5,
        "{check:?} {card_rect:?}"
    );
}
