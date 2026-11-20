pub(crate) mod choice;
pub(crate) mod focus;
pub(crate) mod paint;
pub(crate) mod state;
pub(crate) mod tween;
pub(crate) mod vocab;

/// Builder setters shared by widgets; doc comments pass through.
macro_rules! setters {
    ($(#[$m:meta])* disabled) => {
        $(#[$m])*
        pub fn disabled(mut self, disabled: bool) -> Self {
            self.disabled = disabled;
            self
        }
    };
    ($(#[$m:meta])* item_salt) => {
        /// Identity within the collection (egui id and agent key part), stable when items are
        /// inserted before it; default its index.
        $(#[$m])*
        pub fn salt(mut self, id_salt: impl Into<egui_mineral::semantic::KeyedSalt>) -> Self {
            self.salt = Some(id_salt.into());
            self
        }
    };
    ($(#[$m:meta])* action_key) => {
        /// App key of any type, reported as [`Clicked::key`](egui_mineral::action::Clicked::key) when the item runs;
        /// several items may share one.
        $(#[$m])*
        pub fn key<K: std::any::Any + Clone + PartialEq + Send + Sync>(mut self, key: K) -> Self {
            self.action_key = Some(egui_mineral::action::ActionKey::new(key));
            self
        }
    };
    ($(#[$m:meta])* agent_key(key)) => {
        $(#[$m])*
        pub fn agent_key(mut self, key: impl Into<egui_mineral::semantic::StableId>) -> Self {
            self.key = egui_mineral::semantic::AgentKey::Custom(key.into());
            self
        }
    };
}

pub(crate) use setters;

// Centered row `height` tall: a row layout otherwise spans, and centers its items in, all the height left.
pub(crate) fn row(ui: &egui::Ui, dir: egui_mineral::i18n::Direction, height: f32) -> egui::UiBuilder {
    let size = egui::vec2(ui.available_width(), height);
    egui::UiBuilder::new()
        .layout(dir.row())
        .max_rect(egui::Rect::from_min_size(ui.cursor().min, size))
}
