//! Typed actions: how collections of actions report which item ran.
//!
//! A collection pairs each action with an app value (`action(value, spec)`, `item(value, spec)`) and reports the
//! run one as `clicked: Option<A>`. The value is any [`ItemKey`]; it also salts the item's id and agent key. A collection starts as [`NoAction`] and takes
//! the type of its first value ([`Widen`]), so mixing value types does not compile:
//!
//! ```
//! use egui_mineral::action::{ItemResponses, NoAction, Widen};
//!
//! #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, egui_mineral::semantic::KeySalt)]
//! enum Cmd {
//!     Search,
//!     Share,
//! }
//!
//! fn widen<A: Widen<B>, B>(values: Vec<A>) -> Vec<B> {
//!     values.into_iter().map(Widen::widen).collect()
//! }
//!
//! let none: Vec<NoAction> = Vec::new();
//! let mut values: Vec<Cmd> = widen(none);
//! values.push(Cmd::Share);
//! let buttons: ItemResponses<Cmd, &str> = values.iter().map(|v| (*v, "button")).collect();
//! assert_eq!(buttons.get(&Cmd::Share), Some(&"button"));
//! assert_eq!(buttons.get(&Cmd::Search), None);
//! ```
//!
//! An item bound to an app command
//! ([`PaletteCommand::command`](crate::palette::PaletteCommand::command)) fires it through
//! [`shortcut`](crate::shortcut) instead and reports no value.

use crate::{select::ItemKey, semantic::KeyedSalt};

/// Action type of a collection before its first action; uninhabited, so its response never reports a click.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum NoAction {}

/// Value an action collection reports: any [`ItemKey`], or [`NoAction`] while it has no action. Sealed.
pub trait ActionValue: Clone + PartialEq + sealed::Sealed {
    /// Id salt and agent key part of the item carrying this value.
    fn salt(&self) -> KeyedSalt;
}

impl<A: ItemKey> sealed::Sealed for A {}

impl<A: ItemKey> ActionValue for A {
    fn salt(&self) -> KeyedSalt {
        KeyedSalt::from(self.clone())
    }
}

impl sealed::Sealed for NoAction {}

impl ActionValue for NoAction {
    fn salt(&self) -> KeyedSalt {
        match *self {}
    }
}

/// Action type `B` a collection of `Self` actions has after taking a `B` action: [`NoAction`] widens to any
/// action type, every other type only to itself, so one collection never mixes action types. Sealed.
pub trait Widen<B>: ActionValue {
    /// `self` as the wider type.
    fn widen(self) -> B;
}

impl<A: ItemKey> Widen<A> for A {
    fn widen(self) -> A {
        self
    }
}

impl<B: ActionValue> Widen<B> for NoAction {
    fn widen(self) -> B {
        match self {}
    }
}

mod sealed {
    pub trait Sealed {}
}

/// `items` of a collection with `A` actions, as a collection with `B` actions; see [`Widen`].
pub fn widen<A: Widen<B>, B, T>(items: Vec<(A, T)>) -> Vec<(B, T)> {
    items.into_iter().map(|(a, t)| (a.widen(), t)).collect()
}

/// Per-item values of a collection's response (the button of each shown action, the response of each rendered
/// row), by the item's value. Items that were not shown have no entry.
#[derive(Clone, Debug)]
pub struct ItemResponses<A, T> {
    items: Vec<(A, T)>,
}

impl<A, T> Default for ItemResponses<A, T> {
    fn default() -> Self {
        Self { items: Vec::new() }
    }
}

impl<A: PartialEq, T> ItemResponses<A, T> {
    /// Records `value` for the item carrying `key`.
    pub fn push(&mut self, key: A, value: T) {
        self.items.push((key, value));
    }

    /// Value of the first shown item carrying `key`.
    pub fn get(&self, key: &A) -> Option<&T> {
        self.items.iter().find(|(k, _)| k == key).map(|(_, v)| v)
    }

    /// Shown items as `(key, value)`, in the order they were laid out.
    pub fn iter(&self) -> impl Iterator<Item = (&A, &T)> {
        self.items.iter().map(|(k, v)| (k, v))
    }

    /// Number of shown items.
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// `true` when no item was shown.
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
}

impl<A, T> FromIterator<(A, T)> for ItemResponses<A, T> {
    fn from_iter<I: IntoIterator<Item = (A, T)>>(iter: I) -> Self {
        Self {
            items: iter.into_iter().collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Copy, Debug, PartialEq)]
    enum Cmd {
        Open,
        Save,
    }

    #[test]
    fn item_responses_look_up_by_key() {
        let mut items = ItemResponses::default();
        items.push(Cmd::Open, 'a');
        items.push(Cmd::Save, 'b');
        assert_eq!(items.get(&Cmd::Save), Some(&'b'));
        assert_eq!(
            items.iter().collect::<Vec<_>>(),
            [(&Cmd::Open, &'a'), (&Cmd::Save, &'b')]
        );
        assert_eq!(items.len(), 2);
    }

    #[test]
    fn no_action_widens_and_actions_stay() {
        fn widen<A: Widen<B>, B>(a: Vec<A>) -> Vec<B> {
            a.into_iter().map(Widen::widen).collect()
        }
        assert_eq!(widen::<NoAction, &str>(Vec::new()), Vec::<&str>::new());
        assert_eq!(widen::<_, &str>(vec!["open"]), ["open"]);
    }
}
