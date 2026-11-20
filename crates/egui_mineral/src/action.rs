//! Action keys: how collections of actions report which item ran.
//!
//! An item of an action collection (palette entries, bar actions, menu items) takes an app key of any type
//! with `.key(..)`; the collection's response reports the run item as [`Clicked`], so apps match on their own
//! keys instead of positions or strings:
//!
//! ```
//! use egui_mineral::action::{ActionKey, Clicked};
//!
//! #[derive(Clone, Copy, PartialEq)]
//! enum Cmd {
//!     Search,
//!     Share,
//! }
//!
//! fn handle(clicked: Option<&Clicked>) -> &'static str {
//!     match clicked.and_then(Clicked::key::<Cmd>) {
//!         Some(Cmd::Search) => "open the palette",
//!         Some(Cmd::Share) => "share",
//!         None => "nothing ran, or the item has no `Cmd` key",
//!     }
//! }
//!
//! let clicked = Clicked::new(3, Some(ActionKey::new(Cmd::Search)));
//! assert_eq!(handle(Some(&clicked)), "open the palette");
//! assert!(clicked.is(&Cmd::Search));
//! assert_eq!(handle(None), "nothing ran, or the item has no `Cmd` key");
//! ```
//!
//! A key only names the item in the response. An item bound to an app command
//! ([`PaletteCommand::command`](crate::palette::PaletteCommand::command)) fires it through
//! [`shortcut`](crate::shortcut) instead and is not reported.
//!
//! Per-item values of a collection (the button of a bar action, a rendered row) are an [`ItemResponses`]
//! field of its response.

use std::{any::Any, fmt, sync::Arc};

type Erased = dyn Any + Send + Sync;

/// App key of an action item, type-erased; cheap to clone. Keys of different types never compare equal.
#[derive(Clone)]
pub struct ActionKey {
    value: Arc<Erased>,
    eq: fn(&Erased, &Erased) -> bool,
    type_name: &'static str,
}

impl ActionKey {
    /// Wraps the app key `key`.
    pub fn new<K: Any + Clone + PartialEq + Send + Sync>(key: K) -> Self {
        Self {
            value: Arc::new(key),
            eq: |a, b| matches!((a.downcast_ref::<K>(), b.downcast_ref::<K>()), (Some(a), Some(b)) if a == b),
            type_name: std::any::type_name::<K>(),
        }
    }

    /// `None` when the key is of another type than `K`.
    pub fn get<K: Any>(&self) -> Option<&K> {
        self.value.downcast_ref()
    }

    /// `true` when the key is a `K` equal to `key`.
    pub fn is<K: Any + PartialEq>(&self, key: &K) -> bool {
        self.get::<K>() == Some(key)
    }
}

impl PartialEq for ActionKey {
    fn eq(&self, other: &Self) -> bool {
        (self.eq)(&*self.value, &*other.value)
    }
}

impl fmt::Debug for ActionKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("ActionKey").field(&self.type_name).finish()
    }
}

/// Item of an action collection that ran this pass.
#[derive(Clone, Debug, PartialEq)]
pub struct Clicked {
    index: usize,
    key: Option<ActionKey>,
}

impl Clicked {
    /// Item at `index` of the collection, with the `key` the app gave it.
    pub fn new(index: usize, key: Option<ActionKey>) -> Self {
        Self { index, key }
    }

    /// Position in the collection, in insertion order.
    pub fn index(&self) -> usize {
        self.index
    }

    /// App key set with `.key(..)` on the item; `None` when it has none or it is not a `K`.
    pub fn key<K: Any>(&self) -> Option<&K> {
        self.key.as_ref().and_then(ActionKey::get)
    }

    /// `true` when the item carries a `K` key equal to `key`.
    pub fn is<K: Any + PartialEq>(&self, key: &K) -> bool {
        self.key.as_ref().is_some_and(|k| k.is(key))
    }
}

/// Per-item values of a collection's response (the button of each shown action, the response of each
/// rendered row), by item index and by the item's key. Items that were not shown have no entry.
#[derive(Clone, Debug)]
pub struct ItemResponses<T> {
    items: Vec<Item<T>>,
}

#[derive(Clone, Debug)]
struct Item<T> {
    index: usize,
    key: Option<ActionKey>,
    value: T,
}

impl<T> Default for ItemResponses<T> {
    fn default() -> Self {
        Self { items: Vec::new() }
    }
}

impl<T> ItemResponses<T> {
    /// Records `value` for the item at `index` of the collection, with the item's `key`.
    pub fn push(&mut self, index: usize, key: Option<ActionKey>, value: T) {
        self.items.push(Item { index, key, value });
    }

    /// Value of the item at `index` of the collection; `None` while it is not shown.
    pub fn get(&self, index: usize) -> Option<&T> {
        self.items.iter().find(|i| i.index == index).map(|i| &i.value)
    }

    /// Value of the first shown item whose key is a `K` equal to `key`.
    pub fn get_by_key<K: Any + PartialEq>(&self, key: &K) -> Option<&T> {
        self.items
            .iter()
            .find(|i| i.key.as_ref().is_some_and(|k| k.is(key)))
            .map(|i| &i.value)
    }

    /// Shown items as `(index, value)`, in the order they were laid out.
    pub fn iter(&self) -> impl Iterator<Item = (usize, &T)> {
        self.items.iter().map(|i| (i.index, &i.value))
    }

    /// Shown items whose key is a `K`, as `(key, value)`, in the order they were laid out.
    pub fn iter_keyed<K: Any>(&self) -> impl Iterator<Item = (&K, &T)> {
        self.items
            .iter()
            .filter_map(|i| Some((i.key.as_ref()?.get::<K>()?, &i.value)))
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

impl<T> FromIterator<T> for ItemResponses<T> {
    /// Unkeyed items indexed by their position.
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        Self {
            items: iter
                .into_iter()
                .enumerate()
                .map(|(index, value)| Item {
                    index,
                    key: None,
                    value,
                })
                .collect(),
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
    fn keys_compare_by_type_and_value() {
        let open = ActionKey::new(Cmd::Open);
        assert_eq!(open, ActionKey::new(Cmd::Open));
        assert_ne!(open, ActionKey::new(Cmd::Save));
        assert_ne!(ActionKey::new(1_u8), ActionKey::new(1_u16));
        assert_eq!(open.get::<Cmd>(), Some(&Cmd::Open));
        assert_eq!(open.get::<u8>(), None);
    }

    #[test]
    fn clicked_reads_its_key_typed() {
        let clicked = Clicked::new(2, Some(ActionKey::new(Cmd::Save)));
        assert_eq!(clicked.index(), 2);
        assert_eq!(clicked.key::<Cmd>(), Some(&Cmd::Save));
        assert!(clicked.is(&Cmd::Save));
        assert!(!clicked.is(&Cmd::Open));
        assert!(!clicked.is(&"save"));
        assert_eq!(Clicked::new(0, None).key::<Cmd>(), None);
    }

    #[test]
    fn item_responses_look_up_by_index_and_key() {
        let mut items = ItemResponses::default();
        items.push(0, None, 'a');
        items.push(3, Some(ActionKey::new(Cmd::Save)), 'b');
        assert_eq!(items.get(3), Some(&'b'));
        assert_eq!(items.get(1), None);
        assert_eq!(items.get_by_key(&Cmd::Save), Some(&'b'));
        assert_eq!(items.get_by_key(&Cmd::Open), None);
        assert_eq!(items.iter().collect::<Vec<_>>(), [(0, &'a'), (3, &'b')]);
        assert_eq!(items.iter_keyed::<Cmd>().collect::<Vec<_>>(), [(&Cmd::Save, &'b')]);
        assert_eq!(items.len(), 2);
    }
}
