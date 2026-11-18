use egui::{InnerResponse, Layout, Rect, UiBuilder};

use crate::{semantic::KeyedSalt, widget::MineralUi};

/// Child `Ui` over `rect` inside space the caller already allocated (its own widget rect, a slot
/// of a bar, sheet or row). Unlike `scope_builder`, the parent cursor stays put, so later siblings
/// never move back over the caller.
pub fn child<K, R>(
    ui: &mut MineralUi<'_, K>,
    id_salt: impl Into<KeyedSalt>,
    rect: Rect,
    layout: Layout,
    add: impl FnOnce(&mut MineralUi<'_, K>) -> R,
) -> InnerResponse<R> {
    let mut child = ui.new_child(
        UiBuilder::new()
            .id_salt(id_salt.into().id_salt())
            .max_rect(rect)
            .layout(layout),
    );
    let inner = add(&mut ui.rt().ui(&mut child));
    let id = child.unique_id();
    // Dropping the child registers its final rect.
    drop(child);
    let response = ui.ctx().read_response(id).unwrap_or_else(|| {
        crate::kit::sanitize::report(format_args!("child widget {id:?} was not registered"));
        ui.interact(rect, id, egui::Sense::hover())
    });
    InnerResponse::new(inner, response)
}

#[cfg(test)]
mod tests {
    use egui::{IdSalt, Layout, Rect, Sense, vec2};

    use crate::{
        layout::{Constraints, Measure},
        semantic::{Role, SemanticNode, StableId},
        test_support::{TestKit, harness},
        widget::{MineralUi, MineralWidget, WidgetFrame, WidgetResponse},
    };

    struct Slotted;

    impl Slotted {
        const HEIGHT: f32 = 100.0;
    }

    impl MineralWidget for Slotted {
        type Kit = TestKit;
        type State = ();
        type Output = ();

        fn id_salt(&self) -> IdSalt {
            IdSalt::new("slotted")
        }

        fn layout(&self, _: &MineralUi<'_, TestKit>, c: Constraints, (): &Self::State) -> Measure {
            c.constrain(vec2(200.0, Self::HEIGHT)).into()
        }

        fn update(&mut self, ui: &mut MineralUi<'_, TestKit>, frame: &WidgetFrame<'_>, (): &mut ()) -> WidgetResponse {
            let slot = Rect::from_min_size(frame.zones.visual.min, vec2(40.0, 20.0));
            super::child(ui, "slot", slot, Layout::top_down(egui::Align::Min), |ui| {
                ui.allocate_exact_size(vec2(40.0, 20.0), Sense::hover())
            });
            WidgetResponse::new(frame.response.clone(), ())
        }

        fn semantics(&self, _: &crate::i18n::I18n, _: &WidgetResponse, (): &()) -> Option<SemanticNode> {
            Some(SemanticNode::new(StableId::new("slotted"), Role::Group, ""))
        }
    }

    #[test]
    fn child_slot_keeps_parent_cursor() {
        let h = harness(
            |ui, (top, sibling): &mut (f32, Rect)| {
                ui.spacing_mut().item_spacing = egui::Vec2::ZERO;
                *top = ui.cursor().min.y;
                ui.mineral(Slotted);
                *sibling = ui.allocate_exact_size(vec2(10.0, 10.0), Sense::hover()).0;
            },
            (0.0, Rect::NOTHING),
        );
        let (top, sibling) = h.state().value;
        assert_eq!(
            sibling.top(),
            top + Slotted::HEIGHT,
            "the sibling follows the widget's outer rect"
        );
    }
}
