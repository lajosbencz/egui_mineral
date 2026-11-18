use egui_mineral::shortcut::ActionId;

use crate::MaterialIcon;

/// M3 Expressive size scale (Compose `*XSmall*`..`*XLarge*` tokens) of buttons, icon buttons,
/// split buttons, button groups, sliders and ratings.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Size {
    /// `*XSmall*` tokens.
    XSmall,
    /// `*Small*` tokens.
    Small,
    /// `*Medium*` tokens.
    Medium,
    /// `*Large*` tokens.
    Large,
    /// `*XLarge*` tokens.
    XLarge,
}

/// Icon action item of a [`TopAppBar`](crate::navigation::TopAppBar), toolbar
/// ([`Toolbar`](crate::navigation::Toolbar)) or [`TreeView`](crate::extras::TreeView) row.
#[derive(Clone, Debug)]
#[must_use]
pub struct IconAction {
    pub(crate) icon: MaterialIcon,
    pub(crate) label: String,
    pub(crate) command: Option<ActionId>,
    pub(crate) selected: Option<bool>,
    pub(crate) disabled: bool,
}

impl IconAction {
    /// `label` is the accessible name.
    pub fn new(icon: MaterialIcon, label: impl Into<String>) -> Self {
        Self {
            icon,
            label: label.into(),
            command: None,
            selected: None,
            disabled: false,
        }
    }

    /// Activation fires the app action `id` ([`shortcut::fire`](egui_mineral::shortcut::fire)) instead
    /// of reporting the item as clicked.
    pub fn command(mut self, id: impl Into<ActionId>) -> Self {
        self.command = Some(id.into());
        self
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

/// Outcome of a single-selection group: radio group, tabs, navigation bar/rail/drawer, stepper,
/// button group.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct SelectionResponse {
    /// Group response; `mark_changed` when `changed`.
    pub response: egui::Response,
    /// `selected` changed this frame.
    pub changed: bool,
    /// Index picked this frame.
    pub selected: Option<usize>,
}

impl SelectionResponse {
    pub(crate) fn new(mut response: egui::Response, selected: Option<usize>, changed: bool) -> Self {
        if changed {
            response.mark_changed();
        }
        Self {
            response,
            changed,
            selected,
        }
    }

    pub(crate) fn pick(response: egui::Response, bound: &mut usize, picked: Option<usize>) -> Self {
        let changed = picked.is_some_and(|i| i != *bound);
        if let Some(i) = picked {
            *bound = i;
        }
        Self::new(response, picked, changed)
    }
}

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
    crate::extras::TableColumn,
    crate::containment::DialogAction,
    crate::actions::ButtonGroupItem,
    crate::actions::Segment,
    crate::selection::MenuItem,
    crate::navigation::Tab,
    crate::extras::PaletteCommand,
    crate::extras::StepLabel,
    crate::extras::TimelineItem,
    crate::communication::SnackbarAction
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
