use crate::Mui;
use egui::{Align, Layout, Sense, Ui, UiBuilder, Vec2, Widget, vec2};
use egui_mineral::kit::prelude::*;
use egui_mineral::widget::MineralUi;
use egui_mineral_material::Material;
use egui_mineral_material::prelude::*;

use super::{Shared, section};

pub(crate) const TOPICS: &[&str] = &["Raw egui widgets", "Stable key and touch target"];

pub(crate) struct State {
    checked: bool,
    fruit: usize,
    level: f32,
    clicks: u32,
}

impl Default for State {
    fn default() -> Self {
        Self {
            checked: true,
            fruit: 0,
            level: 0.6,
            clicks: 0,
        }
    }
}

struct Meter {
    fraction: f32,
}

impl Widget for Meter {
    fn ui(self, ui: &mut Ui) -> egui::Response {
        let (rect, response) = ui.allocate_exact_size(vec2(160.0, 12.0), Sense::hover());
        let v = ui.visuals();
        let r = rect.height() / 2.0;
        ui.painter().rect_filled(rect, r, v.extreme_bg_color);
        let mut fill = rect;
        fill.set_width(rect.width() * self.fraction.clamp(0.0, 1.0));
        ui.painter().rect_filled(fill, r, v.selection.bg_fill);
        response
    }
}

struct Interop<W> {
    key: &'static str,
    name: &'static str,
    size: Vec2,
    role: Role,
    actions: Vec<Action>,
    widget: Option<W>,
}

impl<W: Widget> Interop<W> {
    fn new(key: &'static str, name: &'static str, size: Vec2, role: Role, actions: Vec<Action>, widget: W) -> Self {
        Self {
            key,
            name,
            size,
            role,
            actions,
            widget: Some(widget),
        }
    }
}

impl<W: Widget> MineralWidget for Interop<W> {
    type Kit = Material;
    type State = ();
    type Output = ();

    fn id_salt(&self) -> egui::IdSalt {
        egui::IdSalt::new(self.key)
    }

    // The raw widget senses input itself.
    fn sense(&self) -> Sense {
        Sense::hover()
    }

    fn layout(&self, _ui: &MineralUi<'_, Material>, _constraints: Constraints, (): &Self::State) -> Measure {
        self.size.into()
    }

    fn update(&mut self, ui: &mut MineralUi<'_, Material>, frame: &WidgetFrame<'_>, (): &mut ()) -> WidgetResponse {
        let widget = self.widget.take().expect("update runs once per show");
        let mut child = ui.new_child(
            UiBuilder::new()
                .max_rect(frame.zones.hit)
                .layout(Layout::top_down(Align::Center).with_main_align(Align::Center)),
        );
        let response = child.add(widget);
        let changed = response.changed();
        WidgetResponse::new(response, ()).with_changed(changed)
    }

    fn semantics(&self, _: &egui_mineral::i18n::I18n, _: &WidgetResponse, (): &()) -> Option<SemanticNode> {
        Some(SemanticNode::new(StableId::new(self.key), self.role, self.name).actions(self.actions.iter().copied()))
    }
}

pub(crate) fn show_page(ui: &mut Mui<'_>, s: &mut State, shared: &mut Shared) {
    raw(ui, s);
    keyed(ui, s, shared);
}

fn raw(ui: &mut Mui<'_>, s: &mut State) {
    let rt = ui.rt();
    section(ui, TOPICS[0], None, |ui| {
        ui.label(
            egui::RichText::new("Raw egui and ecosystem widgets inherit the theme Visuals; they carry no stable agent key and ignore the touch-target minimum.")
                .font(TypeScale::BodySmall.font(rt.ctx())),
        );
        Card::new("interop-card", "Interop")
            .variant(CardVariant::Outlined)
            .content_padding(Card::CONTENT_PADDING)
            .show(ui, |ui| {
                super::row(ui, |row| {
                    row.add(|ui| ui.checkbox(&mut s.checked, "egui::Checkbox"));
                    // The drag value drives the custom meter.
                    row.add(|ui| ui.add(egui::DragValue::new(&mut s.level).range(0.0..=1.0).speed(0.01)));
                    row.add(|ui| {
                        egui::ComboBox::from_id_salt("interop-fruit")
                            .selected_text(["Apple", "Pear"][s.fruit])
                            .show_ui(ui, |ui| {
                                ui.selectable_value(&mut s.fruit, 0, "Apple");
                                ui.selectable_value(&mut s.fruit, 1, "Pear");
                            })
                    });
                    row.add(|ui| ui.add(Meter { fraction: s.level }));
                });
            });
    });
}

fn keyed(ui: &mut Mui<'_>, s: &mut State, shared: &mut Shared) {
    let rt = ui.rt();
    section(ui, TOPICS[1], None, |ui| {
        ui.label(
            egui::RichText::new(
                "Wrapped in a MineralWidget: agent key `interop-raw-button`, Button role, minimum hit target.",
            )
            .font(TypeScale::BodySmall.font(rt.ctx())),
        );
        let w = egui::Button::new(format!("Raw button ({})", s.clicks));
        if ui
            .mineral(Interop::new(
                "interop-raw-button",
                "Raw button",
                vec2(140.0, 24.0),
                Role::Button,
                vec![Action::Click],
                w,
            ))
            .clicked()
        {
            s.clicks += 1;
            shared.toast("Raw button clicked");
        }
    });
}
