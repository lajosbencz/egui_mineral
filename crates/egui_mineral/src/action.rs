//! Action keys: how collections of actions report which item ran.
//!
//! An item of an action collection (palette entries, bar actions, menu items) takes an app key with
//! `.key(..)`; the collection's response reports the run item as [`Clicked`], so apps match on their own keys
//! instead of positions:
//!
//! ```
//! use egui_mineral::{action::Clicked, shortcut::ActionId};
//!
//! const SEARCH: ActionId = ActionId::from_static("search");
//!
//! fn handle(clicked: Option<&Clicked>) -> &'static str {
//!     match clicked.and_then(Clicked::key_str) {
//!         Some("search") => "open the palette",
//!         Some(_) => "another keyed item",
//!         None => "nothing ran, or the item has no key",
//!     }
//! }
//!
//! assert_eq!(handle(Some(&Clicked::new(3, Some(SEARCH)))), "open the palette");
//! assert_eq!(handle(None), "nothing ran, or the item has no key");
//! ```
//!
//! A key only names the item in the response. An item bound to an app command
//! ([`PaletteCommand::command`](crate::palette::PaletteCommand::command)) fires it through
//! [`shortcut`](crate::shortcut) instead and is not reported.

use crate::shortcut::ActionId;

/// Item of an action collection that ran this pass.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Clicked {
    index: usize,
    key: Option<ActionId>,
}

impl Clicked {
    /// Item at `index` of the collection, with the `key` the app gave it.
    pub fn new(index: usize, key: Option<ActionId>) -> Self {
        Self { index, key }
    }

    /// Position in the collection, in insertion order.
    pub fn index(&self) -> usize {
        self.index
    }

    /// App key set with `.key(..)` on the item; `None` when it has none.
    pub fn key(&self) -> Option<&ActionId> {
        self.key.as_ref()
    }

    /// Text of [`Self::key`], for `match`.
    pub fn key_str(&self) -> Option<&str> {
        self.key.as_ref().and_then(ActionId::as_str)
    }

    /// `true` when the item carries `key`.
    pub fn is(&self, key: &ActionId) -> bool {
        self.key.as_ref() == Some(key)
    }
}
