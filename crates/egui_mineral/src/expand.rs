use std::collections::BTreeSet;

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(
    try_from = "RawExpansion<K>",
    bound(deserialize = "K: Ord + serde::Deserialize<'de>")
)]
/// Expanded keys; single mode keeps at most one open.
pub struct Expansion<K: Ord> {
    single: bool,
    open: BTreeSet<K>,
}

#[derive(serde::Deserialize)]
#[serde(bound(deserialize = "K: Ord + serde::Deserialize<'de>"))]
struct RawExpansion<K: Ord> {
    single: bool,
    open: BTreeSet<K>,
}

impl<K: Ord> TryFrom<RawExpansion<K>> for Expansion<K> {
    type Error = String;

    fn try_from(r: RawExpansion<K>) -> Result<Self, String> {
        if r.single && r.open.len() > 1 {
            return Err(format!(
                "Expansion: single mode holds at most 1 open key, got {}",
                r.open.len()
            ));
        }
        Ok(Self {
            single: r.single,
            open: r.open,
        })
    }
}

/// Empty expansion that lets several keys stay open.
impl<K: Ord> Default for Expansion<K> {
    fn default() -> Self {
        Self {
            single: false,
            open: BTreeSet::new(),
        }
    }
}

impl<K: Ord + Clone> Expansion<K> {
    /// Accordion mode: expanding one key collapses the others.
    pub fn single() -> Self {
        Self {
            single: true,
            open: BTreeSet::new(),
        }
    }

    /// Open keys in ascending order.
    pub fn expanded(&self) -> impl Iterator<Item = &K> {
        self.open.iter()
    }

    /// `true` when `key` is open.
    pub fn is_expanded(&self, key: &K) -> bool {
        self.open.contains(key)
    }

    /// Opens `key`; in single mode, closes the others.
    pub fn expand(&mut self, key: K) {
        if self.single {
            self.open.clear();
        }
        self.open.insert(key);
    }

    /// No-op when `key` is closed.
    pub fn collapse(&mut self, key: &K) {
        self.open.remove(key);
    }

    /// `true` when `key` ends up expanded.
    pub fn toggle(&mut self, key: K) -> bool {
        if self.open.remove(&key) {
            false
        } else {
            self.expand(key);
            true
        }
    }

    /// Closes every key.
    pub fn collapse_all(&mut self) {
        self.open.clear();
    }
}

/// Collects into an empty multi-open expansion.
impl<K: Ord + Clone> FromIterator<K> for Expansion<K> {
    fn from_iter<I: IntoIterator<Item = K>>(iter: I) -> Self {
        let mut e = Self::default();
        e.extend(iter);
        e
    }
}

/// Opens each key in order; single mode keeps the last.
impl<K: Ord + Clone> Extend<K> for Expansion<K> {
    fn extend<I: IntoIterator<Item = K>>(&mut self, iter: I) {
        iter.into_iter().for_each(|k| self.expand(k));
    }
}

/// Open keys in ascending order.
#[expect(clippy::into_iter_without_iter, reason = "`expanded` is the named iterator")]
impl<'a, K: Ord> IntoIterator for &'a Expansion<K> {
    type Item = &'a K;
    type IntoIter = std::collections::btree_set::Iter<'a, K>;

    fn into_iter(self) -> Self::IntoIter {
        self.open.iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collects_extends_and_iterates() {
        let e: Expansion<u8> = [2, 1].into_iter().collect();
        assert_eq!((&e).into_iter().copied().collect::<Vec<_>>(), [1, 2]);
        let mut single = Expansion::single();
        single.extend([1, 2]);
        assert_eq!(single.expanded().copied().collect::<Vec<_>>(), [2]);
    }

    #[test]
    fn restore_rejects_inconsistent_state() {
        let open = |single, keys: &[u8]| RawExpansion {
            single,
            open: keys.iter().copied().collect(),
        };
        assert!(Expansion::try_from(open(true, &[1, 2])).is_err());
    }

    #[test]
    fn single_mode_keeps_one_open() {
        let mut e = Expansion::single();
        e.expand(1);
        e.expand(2);
        assert!(!e.is_expanded(&1) && e.is_expanded(&2));
        assert!(!e.toggle(2));
        assert!(e.toggle(1));
        assert!(e.toggle(3) && !e.is_expanded(&1));
    }
}
