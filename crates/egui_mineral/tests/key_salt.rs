//! `#[derive(KeySalt)]` keys.

use egui_mineral::semantic::{KeySalt, KeyedSalt};

#[derive(Debug, Hash, KeySalt)]
enum Page {
    Home,
    ListItem,
    #[key_salt(rename = "inbox")]
    Mail,
    User(u32),
    Doc {
        id: u8,
        rev: u8,
    },
}

#[derive(Debug, Hash, KeySalt)]
struct Row(&'static str, usize);

#[derive(Debug, Hash, KeySalt)]
struct Pair<T> {
    left: T,
    right: T,
}

fn key(salt: impl Into<KeyedSalt>) -> String {
    salt.into().key().to_owned()
}

#[test]
fn derived_keys_follow_names_and_fields() {
    assert_eq!(key(Page::Home), "home");
    assert_eq!(key(Page::ListItem), "list-item");
    assert_eq!(key(Page::Mail), "inbox");
    assert_eq!(key(Page::User(7)), "user-7");
    assert_eq!(key(Page::Doc { id: 1, rev: 2 }), "doc-1-2");
    assert_eq!(key(Row("row", 3)), "row-3");
    assert_eq!(key(Pair { left: 'a', right: 'b' }), "a-b");
    assert_eq!(key(KeyedSalt::from("nav").child(Page::ListItem)), "nav-list-item");
}
