use std::{
    borrow::Cow,
    fmt::{self, Write as _},
    hash::{Hash, Hasher},
    sync::{Arc, OnceLock},
};

use egui::{AsIdSalt, IdSalt};

// Inline parts; a longer key renders its prefix into `base`.
const CAP: usize = 5;

// 16 bytes: `Number` takes the null niche of the `&str` pointer.
#[derive(Clone, Copy, Debug)]
enum Part {
    Text(&'static str),
    Number(u64),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Join {
    /// Normalized salt part: `-` only between non-empty texts.
    Salt,
    /// Raw node kind: always `-`.
    Kind,
}

// Rendered prefix plus static and numeric parts, rendered on first read.
#[derive(Debug)]
struct Key {
    // Owned text and overflowed parts, joined as a salt part; `Arc<String>` is a thin pointer.
    base: Option<Arc<String>>,
    parts: [Part; CAP],
    joins: [Join; CAP],
    len: u8,
    // Holds text not known to be normalized: node kinds and verbatim overrides.
    verbatim: bool,
    text: OnceLock<Box<str>>,
}

impl Clone for Key {
    fn clone(&self) -> Self {
        Self {
            base: self.base.clone(),
            parts: self.parts,
            joins: self.joins,
            len: self.len,
            verbatim: self.verbatim,
            text: OnceLock::new(),
        }
    }
}

impl Key {
    const fn new() -> Self {
        Self {
            base: None,
            parts: [Part::Text(""); CAP],
            joins: [Join::Salt; CAP],
            len: 0,
            verbatim: false,
            text: OnceLock::new(),
        }
    }

    fn of_text(text: Cow<'static, str>) -> Self {
        let mut key = Self::new();
        key.verbatim = !is_normalized(&text);
        match text {
            Cow::Borrowed(s) => key.push(Part::Text(s), Join::Salt),
            Cow::Owned(s) => key.base = Some(Arc::new(s)),
        }
        key
    }

    fn parts(&self) -> impl Iterator<Item = (Part, Join)> {
        self.parts.into_iter().zip(self.joins).take(usize::from(self.len))
    }

    fn push(&mut self, part: Part, join: Join) {
        if usize::from(self.len) == CAP {
            self.base = Some(Arc::new(self.to_string()));
            self.len = 0;
        }
        if let (Some(p), Some(j)) = (
            self.parts.get_mut(usize::from(self.len)),
            self.joins.get_mut(usize::from(self.len)),
        ) {
            (*p, *j) = (part, join);
            self.len += 1;
        }
        self.verbatim |= join == Join::Kind;
        self.text = OnceLock::new();
    }

    // Renders the key and `part` into `base`; an empty key shares `part`.
    fn push_text(&mut self, part: Arc<String>, join: Join) {
        if join == Join::Salt && part.is_empty() {
            return;
        }
        if join == Join::Salt && self.len == 0 && self.base.is_none() {
            self.base = Some(part);
            return;
        }
        let mut text = self.to_string();
        if join == Join::Kind || !text.is_empty() {
            text.push('-');
        }
        text.push_str(&part);
        self.base = Some(Arc::new(text));
        self.len = 0;
        self.verbatim |= join == Join::Kind;
        self.text = OnceLock::new();
    }

    fn push_signed(&mut self, part: i64, join: Join) {
        // Normalization drops the sign like any other non-alphanumeric.
        if join == Join::Kind && part < 0 {
            self.push_text(Arc::new(part.to_string()), join);
        } else {
            self.push(Part::Number(part.unsigned_abs()), join);
        }
    }

    fn text(&self) -> &str {
        match (self.len, &self.base, self.parts.first(), self.joins.first()) {
            (0, None, ..) => "",
            (0, Some(base), ..) => base.as_str(),
            (1, None, Some(Part::Text(s)), Some(Join::Salt)) => s,
            _ => self.text.get_or_init(|| self.to_string().into()),
        }
    }

    fn appended(&self, part: &(impl KeySalt + ?Sized), join: Join) -> Self {
        let mut parts = KeyParts {
            key: self.clone(),
            join,
        };
        if part.key_parts(&mut parts) {
            return parts.key;
        }
        let mut raw = String::new();
        part.write_key(&mut raw);
        let text = match join {
            Join::Salt => normalize(&raw),
            Join::Kind => raw,
        };
        let mut key = self.clone();
        key.push_text(Arc::new(text), join);
        key
    }

    #[cfg(test)]
    fn is_unrendered(&self) -> bool {
        self.text.get().is_none() && self.base.is_none()
    }
}

impl fmt::Display for Key {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut started = false;
        if let Some(base) = self.base.as_deref().filter(|b| !b.is_empty()) {
            f.write_str(base)?;
            started = true;
        }
        for (part, join) in self.parts() {
            match (part, join) {
                (Part::Text(""), Join::Salt) => continue,
                (_, Join::Salt) if !started => {}
                _ => f.write_char('-')?,
            }
            match part {
                Part::Text(s) => f.write_str(s)?,
                Part::Number(n) => write!(f, "{n}")?,
            }
            started = true;
        }
        Ok(())
    }
}

/// Agent-facing node identity: the key alone. Keys encode structure (id salt, item index),
/// never displayed text. Building one from static text and numbers does not allocate; the text renders on first read.
#[derive(Clone)]
pub struct StableId {
    key: Key,
}

impl StableId {
    /// `key` must derive from structure (see [`KeyedSalt::key`]); the `stable-id-display-text` lint rejects label text.
    pub fn new(key: impl Into<Cow<'static, str>>) -> Self {
        Self {
            key: Key::of_text(key.into()),
        }
    }

    /// `<salt key>-<kind>`, the key of a widget's own node; collection items pass `(index, "<kind>")`.
    pub fn of(salt: &KeyedSalt, kind: impl KeySalt) -> Self {
        Self {
            key: salt.key.appended(&kind, Join::Kind),
        }
    }

    /// `<key>-<kind>`, the key of a part of this node.
    #[must_use]
    pub fn sub(&self, kind: impl KeySalt) -> Self {
        Self {
            key: self.key.appended(&kind, Join::Kind),
        }
    }

    /// Key agents address the node by.
    pub fn key(&self) -> &str {
        self.key.text()
    }
}

impl PartialEq for StableId {
    fn eq(&self, other: &Self) -> bool {
        self.key() == other.key()
    }
}

impl Eq for StableId {}

impl Hash for StableId {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.key().hash(state);
    }
}

impl fmt::Debug for StableId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("StableId").field("key", &self.key()).finish()
    }
}

impl fmt::Display for StableId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.key())
    }
}

impl From<StableId> for String {
    fn from(id: StableId) -> Self {
        let mut key = id.key;
        key.text.take().map_or_else(|| key.to_string(), String::from)
    }
}

impl From<&'static str> for StableId {
    fn from(key: &'static str) -> Self {
        Self::new(key)
    }
}

impl From<String> for StableId {
    fn from(key: String) -> Self {
        Self::new(key)
    }
}

impl From<Cow<'static, str>> for StableId {
    fn from(key: Cow<'static, str>) -> Self {
        Self::new(key)
    }
}

impl From<&KeyedSalt> for StableId {
    fn from(salt: &KeyedSalt) -> Self {
        Self { key: salt.key.clone() }
    }
}

/// Kind suffix of a widget's [`AgentKey`]: lowercase words joined by single `-`, e.g. `date-field`.
///
/// Built with [`agent_kind!`](crate::agent_kind) or [`agent_key!`](crate::agent_key), which check the format at
/// compile time.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct KeyKind(&'static str);

impl KeyKind {
    /// `None` unless `kind` is lowercase ASCII words joined by single `-`.
    pub const fn new(kind: &'static str) -> Option<Self> {
        let b = kind.as_bytes();
        if b.is_empty() {
            return None;
        }
        let mut prev = b'-';
        let mut i = 0;
        while i < b.len() {
            let c = b[i];
            if !(c.is_ascii_lowercase() || c.is_ascii_digit() || (c == b'-' && prev != b'-')) {
                return None;
            }
            prev = c;
            i += 1;
        }
        if prev == b'-' { None } else { Some(Self(kind)) }
    }

    /// Kind text.
    pub const fn as_str(self) -> &'static str {
        self.0
    }
}

impl KeySalt for KeyKind {
    fn write_key(&self, out: &mut String) {
        out.push_str(self.0);
    }

    fn key_parts(&self, parts: &mut KeyParts) -> bool {
        parts.push_str(self.0)
    }
}

/// [`KeyKind`] checked at compile time: `agent_kind!("date-field")`.
///
/// ```compile_fail
/// let kind = egui_mineral::agent_kind!("Date Field");
/// ```
#[macro_export]
macro_rules! agent_kind {
    ($kind:literal) => {{
        const KIND: $crate::semantic::KeyKind = match $crate::semantic::KeyKind::new($kind) {
            Some(kind) => kind,
            None => ::core::panic!(::core::concat!(
                "agent kind `",
                $kind,
                "` is not lowercase words joined by single `-`"
            )),
        };
        KIND
    }};
}

/// [`AgentKey::Kind`] of a [`KeyKind`] checked at compile time: `agent_key!("date-field")`.
#[macro_export]
macro_rules! agent_key {
    ($kind:literal) => {
        $crate::semantic::AgentKey::Kind($crate::agent_kind!($kind))
    };
}

/// Widget agent key: `<salt key>-<kind>`, formatted only when semantics are built, or an app override.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AgentKey {
    /// Derived from the widget's salt: `<salt key>-<kind>`.
    Kind(KeyKind),
    /// App override from `agent_key(..)`; used verbatim.
    Custom(StableId),
}

impl AgentKey {
    /// Node identity for a widget salted with `salt`.
    pub fn id(&self, salt: &KeyedSalt) -> StableId {
        match self {
            Self::Kind(kind) => StableId::of(salt, *kind),
            Self::Custom(id) => id.clone(),
        }
    }
}

/// Unrendered key parts a [`KeySalt`] pushes, joined with `-`.
#[derive(Debug)]
pub struct KeyParts {
    key: Key,
    join: Join,
}

impl KeyParts {
    /// Appends `part`; `false` when a salt key needs it normalized, which falls back to [`KeySalt::write_key`].
    pub fn push_str(&mut self, part: &'static str) -> bool {
        if self.join == Join::Salt && !is_normalized(part) {
            return false;
        }
        self.key.push(Part::Text(part), self.join);
        true
    }

    /// Appends a number.
    pub fn push_unsigned(&mut self, part: u64) {
        self.key.push(Part::Number(part), self.join);
    }

    /// Appends a number; salt keys drop the sign.
    pub fn push_signed(&mut self, part: i64) {
        self.key.push_signed(part, self.join);
    }

    fn push_key(&mut self, key: &Key) -> bool {
        if self.join == Join::Kind || key.verbatim {
            return false;
        }
        if let Some(base) = &key.base {
            self.key.push_text(Arc::clone(base), Join::Salt);
        }
        for (part, join) in key.parts() {
            self.key.push(part, join);
        }
        true
    }
}

/// Stable agent key text of an id salt, independent of `Debug` output and displayed text. Implemented for
/// `&'static str`, `String`, `Cow<'static, str>`, integers, `char`, `bool`, references and tuples up to 4 (parts
/// joined with `-`). App salt types implement it next to `Hash` and `Debug`, which makes them convert into
/// [`KeyedSalt`].
pub trait KeySalt {
    /// Appends the raw key text; [`KeyedSalt`] normalizes it.
    fn write_key(&self, out: &mut String);

    /// Pushes the key as static text and numbers, which defers rendering to the first read; `false` falls back to
    /// [`Self::write_key`].
    fn key_parts(&self, _parts: &mut KeyParts) -> bool {
        false
    }
}

impl KeySalt for &'static str {
    fn write_key(&self, out: &mut String) {
        out.push_str(self);
    }

    fn key_parts(&self, parts: &mut KeyParts) -> bool {
        parts.push_str(self)
    }
}

impl KeySalt for String {
    fn write_key(&self, out: &mut String) {
        out.push_str(self);
    }
}

impl KeySalt for Cow<'static, str> {
    fn write_key(&self, out: &mut String) {
        out.push_str(self);
    }

    fn key_parts(&self, parts: &mut KeyParts) -> bool {
        match self {
            Cow::Borrowed(s) => parts.push_str(s),
            Cow::Owned(_) => false,
        }
    }
}

impl<T: KeySalt + ?Sized> KeySalt for &T {
    fn write_key(&self, out: &mut String) {
        (**self).write_key(out);
    }

    fn key_parts(&self, parts: &mut KeyParts) -> bool {
        (**self).key_parts(parts)
    }
}

macro_rules! number_key_salt {
    ($push:ident, $wide:ty: $($t:ty),*) => {$(
        impl KeySalt for $t {
            fn write_key(&self, out: &mut String) {
                out.push_str(&self.to_string());
            }

            fn key_parts(&self, parts: &mut KeyParts) -> bool {
                <$wide>::try_from(*self).map(|n| parts.$push(n)).is_ok()
            }
        }
    )*};
}

number_key_salt!(push_unsigned, u64: u8, u16, u32, u64, u128, usize);
number_key_salt!(push_signed, i64: i8, i16, i32, i64, i128, isize);

impl KeySalt for char {
    fn write_key(&self, out: &mut String) {
        out.push(*self);
    }
}

impl KeySalt for bool {
    fn write_key(&self, out: &mut String) {
        out.push_str(if *self { "true" } else { "false" });
    }

    fn key_parts(&self, parts: &mut KeyParts) -> bool {
        parts.push_str(if *self { "true" } else { "false" })
    }
}

macro_rules! tuple_key_salt {
    ($first:ident $first_i:tt $(, $rest:ident $rest_i:tt)*) => {
        impl<$first: KeySalt, $($rest: KeySalt),*> KeySalt for ($first, $($rest,)*) {
            fn write_key(&self, out: &mut String) {
                self.$first_i.write_key(out);
                $(
                    out.push('-');
                    self.$rest_i.write_key(out);
                )*
            }

            fn key_parts(&self, parts: &mut KeyParts) -> bool {
                self.$first_i.key_parts(parts) $(&& self.$rest_i.key_parts(parts))*
            }
        }
    };
}

tuple_key_salt!(A 0);
tuple_key_salt!(A 0, B 1);
tuple_key_salt!(A 0, B 1, C 2);
tuple_key_salt!(A 0, B 1, C 2, D 3);

/// Id parameter of the whole API: an egui [`IdSalt`] plus its normalized agent key text. Converts from any
/// [`KeySalt`] + [`AsIdSalt`] value (`"save"`, `("row", 3)`, `42usize`), hashing as `IdSalt::new(value)`. The key
/// keeps alphanumerics (lowercased) and turns other runs into `-`: `"Save file"` -> `save-file`.
// No `Hash`: it would overlap the reflexive `From<KeyedSalt>`.
#[derive(Clone)]
pub struct KeyedSalt {
    salt: IdSalt,
    key: Key,
}

impl KeyedSalt {
    /// Salt of `(self, part)` keyed `<key>-<part key>`; an empty side drops the `-`.
    #[must_use]
    pub fn child(&self, part: impl KeySalt + AsIdSalt) -> Self {
        Self {
            salt: IdSalt::new((self.salt, &part)),
            key: self.key.appended(&part, Join::Salt),
        }
    }

    /// [`Self::child`] with a keyed `part`, such as an item's own salt: keyed `<key>-<part key>`.
    #[must_use]
    pub fn join(&self, part: &KeyedSalt) -> Self {
        Self {
            salt: IdSalt::new((self.salt, part.salt)),
            key: self.key.appended(part, Join::Salt),
        }
    }

    /// Same salt with `key` as the key, verbatim (agent key overrides).
    #[must_use]
    pub fn with_key(mut self, key: impl Into<StableId>) -> Self {
        self.key = key.into().key;
        self
    }

    /// Agent key text.
    pub fn key(&self) -> &str {
        self.key.text()
    }

    /// Hashed egui salt.
    pub fn id_salt(&self) -> IdSalt {
        self.salt
    }
}

impl PartialEq for KeyedSalt {
    fn eq(&self, other: &Self) -> bool {
        self.salt == other.salt && self.key() == other.key()
    }
}

impl Eq for KeyedSalt {}

impl fmt::Debug for KeyedSalt {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("KeyedSalt")
            .field("salt", &self.salt)
            .field("key", &self.key())
            .finish()
    }
}

impl<T: KeySalt + AsIdSalt> From<T> for KeyedSalt {
    fn from(value: T) -> Self {
        Self {
            salt: IdSalt::new(&value),
            key: Key::new().appended(&value, Join::Salt),
        }
    }
}

impl KeySalt for KeyedSalt {
    fn write_key(&self, out: &mut String) {
        out.push_str(self.key());
    }

    fn key_parts(&self, parts: &mut KeyParts) -> bool {
        parts.push_key(&self.key)
    }
}

fn is_normalized(s: &str) -> bool {
    !s.starts_with('-')
        && !s.ends_with('-')
        && !s.contains("--")
        && s.chars()
            .all(|c| c == '-' || (c.is_alphanumeric() && c.to_lowercase().eq(std::iter::once(c))))
}

fn normalize(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    for c in raw.chars() {
        if c.is_alphanumeric() {
            out.extend(c.to_lowercase());
        } else if !out.is_empty() && !out.ends_with('-') {
            out.push('-');
        }
    }
    if out.ends_with('-') {
        out.pop();
    }
    out
}

#[cfg(test)]
mod tests {
    use super::{KeySalt, KeyedSalt, StableId};

    fn debug_key(salt: &impl std::fmt::Debug) -> String {
        super::normalize(&format!("{salt:?}"))
    }

    #[test]
    fn keys_match_the_debug_derived_keys() {
        let cases: Vec<(KeyedSalt, String)> = vec![
            ("save".into(), debug_key(&"save")),
            ("Save file".into(), debug_key(&"Save file")),
            ("nav_item".into(), debug_key(&"nav_item")),
            ("--x--".into(), debug_key(&"--x--")),
            (String::from("Owned Key").into(), debug_key(&"Owned Key")),
            (42usize.into(), debug_key(&42usize)),
            ((-7i32).into(), debug_key(&-7i32)),
            ('Q'.into(), debug_key(&'Q')),
            (("row", 3).into(), debug_key(&("row", 3))),
            (("row", -3, 'c').into(), debug_key(&("row", -3, 'c'))),
            (("nb", false).into(), debug_key(&("nb", false))),
            (("", 3u8, "").into(), debug_key(&("", 3u8, ""))),
            (
                ("a", ("b", 2u8), 3u64, "d e").into(),
                debug_key(&("a", ("b", 2u8), 3u64, "d e")),
            ),
            (u128::MAX.into(), debug_key(&u128::MAX)),
        ];
        for (salt, before) in cases {
            assert_eq!(salt.key(), before);
        }
    }

    #[test]
    fn salts_hash_like_egui() {
        use egui::{Id, IdSalt};

        assert_eq!(KeyedSalt::from("save").id_salt(), IdSalt::new("save"));
        assert_eq!(KeyedSalt::from(("row", 3)).id_salt(), IdSalt::new(("row", 3)));
        assert_eq!(KeyedSalt::from(String::from("s")).id_salt(), IdSalt::new("s"));
        let parent = Id::unique("parent");
        assert_eq!(parent.with_salt(KeyedSalt::from("save").id_salt()), parent.with("save"));
    }

    #[test]
    fn static_and_numeric_keys_stay_unrendered_until_read() {
        let list = KeyedSalt::from("list");
        for i in 0..100usize {
            let row = list.child(("row", i));
            let id = StableId::of(&row, (i, "item"));
            assert!(row.key.is_unrendered() && id.key.is_unrendered(), "row {i}");
            assert_eq!(id.key(), format!("list-row-{i}-{i}-item"));
        }
        assert!(!KeyedSalt::from("Save file").key.is_unrendered());
    }

    #[test]
    #[cfg(target_pointer_width = "64")]
    fn keys_stay_compact() {
        assert_eq!(size_of::<super::Part>(), 16, "number shares the str niche");
        assert_eq!(size_of::<StableId>(), 120, "builders move keys by value");
    }

    #[test]
    fn normalized_check_agrees_with_normalize() {
        for s in [
            "",
            "a",
            "a-b",
            "-a",
            "a-",
            "a--b",
            "A",
            "a b",
            "\u{e9}t\u{e9}",
            "\u{c9}",
            "\u{130}",
            "x_1",
            "9",
        ] {
            assert_eq!(super::is_normalized(s), super::normalize(s) == s, "{s:?}");
        }
    }

    #[test]
    fn child_keys_join_parent_and_part() {
        use egui::IdSalt;

        let parent = KeyedSalt::from("Dialog");
        assert_eq!(parent.child("confirm").key(), "dialog-confirm");
        assert_eq!(parent.child((2, "action")).key(), "dialog-2-action");
        assert_eq!(parent.child("").key(), "dialog");
        assert_eq!(KeyedSalt::from("").child("x").key(), "x");
        assert_eq!(parent.clone().with_key("custom").child(1).key(), "custom-1");
        assert_eq!(parent.clone().with_key("My Key").child(1).key(), "My Key-1");
        assert_eq!(
            parent.child("confirm").id_salt(),
            IdSalt::new((parent.id_salt(), "confirm"))
        );
        assert_ne!(parent.child("a").id_salt(), parent.child("b").id_salt());
    }

    #[test]
    fn node_kinds_join_raw() {
        let tabs = KeyedSalt::from("tabs");
        assert_eq!(StableId::of(&tabs, (3, "tab")).key(), "tabs-3-tab");
        assert_eq!(StableId::of(&KeyedSalt::from(""), "x").key(), "-x");
        assert_eq!(StableId::new("a").sub((-2, "b")).key(), "a--2-b");
        assert_eq!(StableId::of(&tabs, "x"), StableId::new("tabs-x"));
        assert_eq!(
            KeyedSalt::from("b").with_key(StableId::of(&tabs, "x")).child(1).key(),
            "tabs-x-1"
        );
    }

    #[test]
    fn long_keys_render_like_short_ones() {
        let mut salt = KeyedSalt::from("root");
        for i in 0..20u32 {
            salt = salt.child(("", i));
        }
        let expected = (0..20).fold(String::from("root"), |key, i| format!("{key}-{i}"));
        let id = StableId::of(&salt, (7, "kind")).sub("part");
        assert_eq!(salt.key(), expected);
        assert_eq!(id.key(), format!("{expected}-7-kind-part"));
    }

    #[test]
    fn app_salt_types_key_through_key_salt() {
        #[derive(Hash, Debug)]
        struct Account {
            id: u32,
        }

        impl KeySalt for Account {
            fn write_key(&self, out: &mut String) {
                out.push_str("account-");
                self.id.write_key(out);
            }
        }

        assert_eq!(KeyedSalt::from(Account { id: 7 }).key(), "account-7");
        assert_eq!(KeyedSalt::from(("row", Account { id: 7 })).key(), "row-account-7");
    }

    #[test]
    fn keys_never_come_from_debug() {
        #[derive(Hash)]
        struct Hashed(u64);

        impl std::fmt::Debug for Hashed {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                if cfg!(debug_assertions) {
                    write!(f, "IdSalt::new({})", self.0)
                } else {
                    write!(f, "salt_{:04X}", self.0)
                }
            }
        }

        impl KeySalt for Hashed {
            fn write_key(&self, out: &mut String) {
                out.push_str("stable");
            }
        }

        assert_eq!(KeyedSalt::from(Hashed(9)).key(), "stable");
        let parent = KeyedSalt::from(Hashed(9));
        assert_eq!(parent.child(Hashed(1)).key(), "stable-stable");
    }

    #[test]
    fn key_kinds_are_lowercase_words_joined_by_single_dashes() {
        for ok in ["date-field", "a", "h2", "x-1-y"] {
            assert_eq!(super::KeyKind::new(ok).map(super::KeyKind::as_str), Some(ok));
        }
        for bad in ["", "-a", "a-", "a--b", "Date", "a b", "a_b", "\u{e9}"] {
            assert_eq!(super::KeyKind::new(bad), None, "{bad:?}");
        }
    }
}
