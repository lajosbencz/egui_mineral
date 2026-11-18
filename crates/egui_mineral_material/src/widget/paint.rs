/// `MineralWidget` for a hover-only painted widget from its inherent `extent(&self, &MineralUi, Constraints) -> Vec2`
/// (its size within the constraints),
/// `paint(&self, &mut MineralUi, Rect)` (called while visible) and `node(&self, &I18n)` returning a `SemanticNode` or `Option<SemanticNode>`.
macro_rules! paint_widget {
    ($ty:ty) => {
        impl egui_mineral::widget::MineralWidget for $ty {
            type Kit = $crate::Material;
            type State = ();
            type Output = ();

            fn id_salt(&self) -> egui::IdSalt {
                self.salt.id_salt()
            }

            fn sense(&self) -> egui::Sense {
                egui::Sense::hover()
            }

            fn min_target(&self) -> egui_mineral::layout::MinTarget {
                egui_mineral::layout::MinTarget::None
            }

            fn layout(
                &self,
                ui: &egui_mineral::widget::MineralUi<'_, $crate::Material>,
                c: egui_mineral::layout::Constraints,
                _: &Self::State,
            ) -> egui_mineral::layout::Measure {
                self.extent(ui, c).into()
            }

            fn update(
                &mut self,
                ui: &mut egui_mineral::widget::MineralUi<'_, $crate::Material>,
                frame: &egui_mineral::widget::WidgetFrame<'_>,
                (): &mut (),
            ) -> egui_mineral::widget::WidgetResponse {
                self.paint(ui, frame.zones.visual);
                egui_mineral::widget::WidgetResponse::new(frame.response.clone(), ())
            }

            fn semantics(
                &self,
                i18n: &egui_mineral::i18n::I18n,
                _: &egui_mineral::widget::WidgetResponse,
                (): &(),
            ) -> Option<egui_mineral::semantic::SemanticNode> {
                self.node(i18n).into()
            }
        }
    };
}

pub(crate) use paint_widget;
