use egui::{Rect, UiBuilder, Vec2};
use egui_mineral_material::{extras::SettingRow, selection::Switch};

use crate::support::{MineralHarness, WIDE, harness};

#[test]
fn setting_row_stacks_below_first_breakpoint() {
    let h = harness(WIDE, Vec::new(), |ui, stacked: &mut Vec<bool>| {
        stacked.clear();
        let rt = ui.rt();
        let breakpoint = rt.core().policies().breakpoints[0];
        for (i, w) in [breakpoint / 2.0, breakpoint + 1.0].into_iter().enumerate() {
            let rect = Rect::from_min_size(ui.cursor().min, Vec2::new(w, WIDE.y / 4.0));
            ui.scope_builder(UiBuilder::new().max_rect(rect), |ui| {
                let mut on = false;
                let r = SettingRow::new(("row", i), "Wi-Fi").show(&mut rt.ui(ui), |ui, label| {
                    ui.mineral(Switch::new(("switch", i), label, &mut on).label_visible(false))
                });
                stacked.push(r.stacked);
            });
        }
    });
    assert_eq!(h.state().value, [true, false]);
    let names = h.tree().nodes.iter().filter(|n| n.name == "Wi-Fi").count();
    assert_eq!(names, 2, "one named node (the switch) per row");
}
