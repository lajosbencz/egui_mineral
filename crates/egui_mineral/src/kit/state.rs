//! Widget state kept in egui temp memory between passes.

use std::{
    hash::{DefaultHasher, Hash, Hasher},
    sync::Arc,
};

use egui::{Context, Id};

/// Per-widget state in egui temp memory, mirroring egui's `State::load`/`store`; opt in with
/// `impl TempState for S {}`. `load` needs `Default`; seeded states use `load_or`.
pub trait TempState: Clone + Send + Sync + 'static {
    /// Value stored under `id`, if any.
    fn try_load(ctx: &Context, id: Id) -> Option<Self> {
        ctx.data(|d| d.get_temp(id))
    }

    /// Value stored under `id`; the default when absent.
    fn load(ctx: &Context, id: Id) -> Self
    where
        Self: Default,
    {
        Self::try_load(ctx, id).unwrap_or_default()
    }

    /// Value stored under `id`; `init()` when absent.
    fn load_or(ctx: &Context, id: Id, init: impl FnOnce() -> Self) -> Self {
        Self::try_load(ctx, id).unwrap_or_else(init)
    }

    /// Replaces the value under `id`.
    fn store(self, ctx: &Context, id: Id) {
        ctx.data_mut(|d| d.insert_temp(id, self));
    }

    /// Drops the value under `id`.
    fn remove(ctx: &Context, id: Id) {
        ctx.data_mut(|d| d.remove::<Self>(id));
    }
}

impl TempState for bool {}
impl TempState for String {}
impl TempState for Id {}
impl TempState for egui::Vec2 {}
impl<T: Clone + Send + Sync + 'static> TempState for Option<T> {}
impl<T: Clone + Send + Sync + 'static> TempState for Vec<T> {}

impl<T: Send + Sync + 'static> TempState for Memo<T> {}

/// Last computed value of a pass-invariant derivation, kept in widget state; `Default` is empty.
#[derive(Debug)]
pub struct Memo<T> {
    key: u64,
    value: Option<Arc<T>>,
}

impl<T> Clone for Memo<T> {
    fn clone(&self) -> Self {
        Self {
            key: self.key,
            value: self.value.clone(),
        }
    }
}

impl<T> Default for Memo<T> {
    fn default() -> Self {
        Self { key: 0, value: None }
    }
}

impl<T> Memo<T> {
    /// Cached value while the hash of `key` is unchanged since the last call; otherwise computes it with `compute`.
    /// `key` must cover everything `compute` reads.
    pub fn get_or_compute(&mut self, key: &impl Hash, compute: impl FnOnce() -> T) -> Arc<T> {
        let mut h = DefaultHasher::new();
        key.hash(&mut h);
        let key = h.finish();
        match &self.value {
            Some(v) if self.key == key => Arc::clone(v),
            _ => {
                let v = Arc::new(compute());
                (self.key, self.value) = (key, Some(Arc::clone(&v)));
                v
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn memo_recomputes_only_when_inputs_change() {
        let mut memo = Memo::<usize>::default();
        let mut calls = 0;
        let mut get = |memo: &mut Memo<usize>, inputs: &[&str]| {
            *memo.get_or_compute(&inputs, || {
                calls += 1;
                calls
            })
        };
        assert_eq!(get(&mut memo, &["a", "b"]), 1);
        assert_eq!(get(&mut memo, &["a", "b"]), 1, "same inputs reuse");
        assert_eq!(get(&mut memo, &["a", "c"]), 2, "changed input");
        assert_eq!(get(&mut memo, &["a"]), 3, "changed count");
    }
}
