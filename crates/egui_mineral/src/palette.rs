//! Command palette data: entries and the result a stylekit's palette reports.

use std::any::Any;

use egui::KeyboardShortcut;

use crate::{
    action::{ActionKey, Clicked},
    icon::Icon,
    kit::palette::Entry,
    overlay::Dismiss,
    semantic::KeyedSalt,
    shortcut::ActionId,
};

/// Entry of a stylekit's command palette.
#[derive(Clone, Debug)]
#[must_use]
pub struct PaletteCommand {
    label: String,
    group: Option<String>,
    id: Option<ActionId>,
    key: Option<ActionKey>,
    shortcut: Option<KeyboardShortcut>,
    icon: Option<Icon>,
    disabled: bool,
    salt: Option<KeyedSalt>,
}

impl PaletteCommand {
    /// `label` is the visible text and accessible name; also matched by the query.
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            group: None,
            id: None,
            key: None,
            shortcut: None,
            icon: None,
            disabled: false,
            salt: None,
        }
    }

    /// Identity within the collection (egui id and agent key part), stable when items are
    /// inserted before it; the stylekit picks the default.
    pub fn salt(mut self, id_salt: impl Into<KeyedSalt>) -> Self {
        self.salt = Some(id_salt.into());
        self
    }

    /// Header the entry is listed under.
    pub fn group(mut self, group: impl Into<String>) -> Self {
        self.group = Some(group.into());
        self
    }

    /// App key of any type, reported as [`Clicked::key`] in [`CommandPaletteResponse::clicked`] when the entry
    /// runs; several entries may share one (a page and its topics). See [`action`](crate::action).
    pub fn key<K: Any + Clone + PartialEq + Send + Sync>(mut self, key: K) -> Self {
        self.key = Some(ActionKey::new(key));
        self
    }

    /// Makes the entry a view of the app command `id`: running it fires the command
    /// ([`shortcut::fire`](crate::shortcut::fire)) instead of reporting the entry as clicked. `shortcut` is bound
    /// to `id` while the palette is shown, open or not; other views of `id` may bind the same shortcut.
    pub fn command(mut self, id: impl Into<ActionId>, shortcut: impl Into<Option<KeyboardShortcut>>) -> Self {
        self.id = Some(id.into());
        self.shortcut = shortcut.into();
        self
    }

    /// Leading icon.
    pub fn icon(mut self, icon: impl Into<Icon>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    /// Greys out the entry; it cannot run.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Value of [`Self::icon`].
    pub fn leading_icon(&self) -> Option<Icon> {
        self.icon
    }

    /// Value of [`Self::salt`].
    pub fn item_salt(&self) -> Option<&KeyedSalt> {
        self.salt.as_ref()
    }
}

impl Entry for PaletteCommand {
    fn label(&self) -> &str {
        &self.label
    }

    fn heading(&self) -> Option<&str> {
        self.group.as_deref()
    }

    fn is_disabled(&self) -> bool {
        self.disabled
    }

    fn action(&self) -> Option<&ActionId> {
        self.id.as_ref()
    }

    fn accelerator(&self) -> Option<KeyboardShortcut> {
        self.shortcut
    }

    fn action_key(&self) -> Option<&ActionKey> {
        self.key.as_ref()
    }
}

impl From<&str> for PaletteCommand {
    fn from(label: &str) -> Self {
        Self::new(label)
    }
}

impl From<String> for PaletteCommand {
    fn from(label: String) -> Self {
        Self::new(label)
    }
}

/// Result of showing a stylekit's command palette.
#[derive(Clone, Debug, Default)]
#[non_exhaustive]
pub struct CommandPaletteResponse {
    /// Palette surface; `None` while closed.
    pub response: Option<egui::Response>,
    /// Run entry without a [`PaletteCommand::command`], with its [`PaletteCommand::key`]; command entries fire
    /// their command instead.
    pub clicked: Option<Clicked>,
    /// Why it closed without running an entry.
    pub dismissed: Option<Dismiss>,
}

impl CommandPaletteResponse {
    /// Shown surface `response` with the entry run and the dismissal of this pass.
    pub fn new(response: egui::Response, clicked: Option<Clicked>, dismissed: Option<Dismiss>) -> Self {
        Self {
            response: Some(response),
            clicked,
            dismissed,
        }
    }
}
