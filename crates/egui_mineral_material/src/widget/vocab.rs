use egui_mineral::{
    action::{ActionKey, Clicked},
    semantic::KeyedSalt,
    shortcut::ActionId,
};

use crate::Icon;

/// M3 Expressive size scale (Compose `*XSmall*`..`*XLarge*` tokens) of buttons, icon buttons,
/// split buttons, button groups, sliders and ratings.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum Size {
    /// `*XSmall*` tokens.
    XSmall,
    /// `*Small*` tokens; the Compose default of buttons.
    #[default]
    Small,
    /// `*Medium*` tokens.
    Medium,
    /// `*Large*` tokens.
    Large,
    /// `*XLarge*` tokens.
    XLarge,
}

/// Icon action item of a [`TopAppBar`](crate::navigation::TopAppBar), toolbar
/// ([`DockedToolbar`](crate::navigation::DockedToolbar)) or `TreeView` row.
#[derive(Clone, Debug)]
#[must_use]
pub struct IconAction {
    pub(crate) icon: Icon,
    pub(crate) label: String,
    pub(crate) command: Option<ActionId>,
    pub(crate) action_key: Option<ActionKey>,
    pub(crate) selected: Option<bool>,
    pub(crate) disabled: bool,
    pub(crate) salt: Option<KeyedSalt>,
}

impl IconAction {
    /// `label` is the accessible name.
    pub fn new(icon: impl Into<Icon>, label: impl Into<String>) -> Self {
        Self {
            icon: icon.into(),
            label: label.into(),
            command: None,
            action_key: None,
            selected: None,
            disabled: false,
            salt: None,
        }
    }

    crate::widget::setters! {
        /// Default the [`Self::command`] action, else the index.
        item_salt
    }

    /// Activation fires the app action `id` ([`shortcut::fire`](egui_mineral::shortcut::fire)) instead
    /// of reporting the item as clicked.
    pub fn command(mut self, id: impl Into<ActionId>) -> Self {
        self.command = Some(id.into());
        self
    }

    crate::widget::setters! {
        action_key
    }

    /// Toggle action.
    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = Some(selected);
        self
    }
    crate::widget::setters! {
        /// Greys the item out and ignores activation.
        disabled
    }
}

/// Text button of a message surface: [`Snackbar`](crate::communication::Snackbar),
/// [`RichTooltip`](crate::communication::RichTooltip), `Alert`, `EmptyPlaceholder`. A label converts.
#[derive(Clone, Debug)]
#[must_use]
pub struct TextAction {
    pub(crate) label: String,
    pub(crate) command: Option<ActionId>,
    pub(crate) action_key: Option<ActionKey>,
}

impl TextAction {
    /// `label` is the button text and accessible name.
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            command: None,
            action_key: None,
        }
    }

    crate::widget::setters! {
        action_key
    }

    /// Pressing it also fires the app action `id` ([`shortcut::fire`](egui_mineral::shortcut::fire)).
    pub fn command(mut self, id: impl Into<ActionId>) -> Self {
        self.command = Some(id.into());
        self
    }

    pub(crate) fn fire(&self, ctx: &egui::Context) {
        if let Some(command) = &self.command {
            egui_mineral::shortcut::fire(ctx, command);
        }
    }
}

pub(crate) fn clicked<T>(
    items: &[T],
    index: Option<usize>,
    key: impl FnOnce(&T) -> Option<&ActionKey>,
) -> Option<Clicked> {
    index.map(|i| Clicked::new(i, items.get(i).and_then(key).cloned()))
}

pub(crate) fn item_salt(
    parent: &KeyedSalt,
    own: Option<&KeyedSalt>,
    command: Option<&ActionId>,
    index: usize,
) -> KeyedSalt {
    match (own, command) {
        (Some(own), _) => parent.join(own),
        (None, Some(command)) => parent.child(command),
        (None, None) => parent.child(index),
    }
}

pub use egui_mineral::select::SelectionResponse;

/// Result of an inline picker: [`DatePicker::show`](crate::selection::DatePicker::show),
/// [`DateRangePicker::show`](crate::selection::DateRangePicker::show),
/// [`TimePicker::show`](crate::selection::TimePicker::show).
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct PickerResponse {
    /// Picker surface; `mark_changed` when `changed`.
    pub response: egui::Response,
    /// Bound value changed this frame.
    pub changed: bool,
}

impl PickerResponse {
    pub(crate) fn new(mut response: egui::Response, changed: bool) -> Self {
        if changed {
            response.mark_changed();
        }
        Self { response, changed }
    }
}

/// Result of a picker dialog: [`DatePicker::show_modal`](crate::selection::DatePicker::show_modal),
/// [`DateRangePicker::show_modal`](crate::selection::DateRangePicker::show_modal),
/// [`TimePicker::show_modal`](crate::selection::TimePicker::show_modal).
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct PickerModalResponse {
    /// Dialog surface; `None` while closed.
    pub response: Option<egui::Response>,
    /// Bound value confirmed this frame.
    pub changed: bool,
    /// Why it closed without confirming.
    pub dismissed: Option<egui_mineral::overlay::Dismiss>,
}

impl PickerModalResponse {
    pub(crate) fn closed() -> Self {
        Self {
            response: None,
            changed: false,
            dismissed: None,
        }
    }
}

/// `From<&str>` and `From<String>` through the item's `new(label)`, so collection setters take labels.
macro_rules! from_label {
    ($($ty:ty),+) => {$(
        impl From<&str> for $ty {
            fn from(label: &str) -> Self {
                Self::new(label)
            }
        }

        impl From<String> for $ty {
            fn from(label: String) -> Self {
                Self::new(label)
            }
        }
    )+};
}

from_label!(
    crate::containment::DialogAction,
    crate::actions::ButtonGroupItem,
    crate::actions::Segment,
    crate::selection::MenuItem,
    crate::navigation::Tab,
    TextAction
);

#[cfg(feature = "extras")]
from_label!(
    crate::extras::TableColumn,
    crate::extras::StepLabel,
    crate::extras::TimelineItem
);

/// Builder setters shared by widgets; doc comments pass through, `|s| place` names the target on an owned `s`.
macro_rules! setters {
    ($(#[$m:meta])* disabled) => {
        $crate::widget::setters!($(#[$m])* disabled |s| s.disabled);
    };
    ($(#[$m:meta])* disabled |$s:ident| $place:expr) => {
        $(#[$m])*
        pub fn disabled(self, disabled: bool) -> Self {
            let mut $s = self;
            $place = disabled;
            $s
        }
    };
    ($(#[$m:meta])* agent_key(salt)) => {
        $crate::widget::setters!($(#[$m])* agent_key(salt) |s| s.salt);
    };
    ($(#[$m:meta])* agent_key(salt) |$s:ident| $place:expr) => {
        $(#[$m])*
        pub fn agent_key(self, key: impl Into<egui_mineral::semantic::StableId>) -> Self {
            let mut $s = self;
            $place = $place.with_key(key.into());
            $s
        }
    };
    ($(#[$m:meta])* item_salt) => {
        /// Identity within the collection (egui id and agent key part), stable when items are
        /// inserted before it.
        $(#[$m])*
        pub fn salt(mut self, id_salt: impl Into<egui_mineral::semantic::KeyedSalt>) -> Self {
            self.salt = Some(id_salt.into());
            self
        }
    };
    ($(#[$m:meta])* action_key) => {
        /// App key of any type, reported as [`Clicked::key`](egui_mineral::action::Clicked::key) when the item is clicked;
        /// keys may repeat within a collection.
        $(#[$m])*
        pub fn key<K: std::any::Any + Clone + PartialEq + Send + Sync>(mut self, key: K) -> Self {
            self.action_key = Some(egui_mineral::action::ActionKey::new(key));
            self
        }
    };
    ($(#[$m:meta])* agent_key(key)) => {
        $crate::widget::setters!($(#[$m])* agent_key(key) |s| s.key);
    };
    ($(#[$m:meta])* agent_key(key) |$s:ident| $place:expr) => {
        $(#[$m])*
        pub fn agent_key(self, key: impl Into<egui_mineral::semantic::StableId>) -> Self {
            let mut $s = self;
            $place = egui_mineral::semantic::AgentKey::Custom(key.into());
            $s
        }
    };
}
pub(crate) use setters;

/// `error` and `maybe_error` builders over an `Option<String>` place; the doc comments go on `error`.
macro_rules! error_setters {
    ($(#[$m:meta])* |$s:ident| $place:expr) => {
        $(#[$m])*
        pub fn error(self, message: impl Into<String>) -> Self {
            let mut $s = self;
            $place = Some(message.into());
            $s
        }

        /// [`Self::error`] when `Some`.
        pub fn maybe_error(self, message: Option<String>) -> Self {
            let mut $s = self;
            $place = message;
            $s
        }
    };
}
pub(crate) use error_setters;
