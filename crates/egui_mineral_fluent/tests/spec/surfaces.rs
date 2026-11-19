use egui_mineral_fluent::{
    spec::defaults::divider,
    surfaces::{Divider, DividerAlign},
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
