//! Widget state kept in egui temp memory between passes.

use std::{
    collections::HashMap,
    hash::{DefaultHasher, Hash, Hasher},
    marker::PhantomData,
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

/// Passes a widget state may go untouched before it is evicted; egui never frees temp data itself.
const EVICT_AFTER: u64 = 3600;
const SWEEP_EVERY: u64 = 600;

// Pass each state of type `S` was last stored, per context.
struct Touched<S> {
    last: HashMap<Id, u64>,
    swept: u64,
    _state: PhantomData<fn() -> S>,
}

impl<S> Clone for Touched<S> {
    fn clone(&self) -> Self {
        Self {
            last: self.last.clone(),
            swept: self.swept,
            _state: PhantomData,
        }
    }
}

impl<S> Default for Touched<S> {
    fn default() -> Self {
        Self {
            last: HashMap::new(),
            swept: 0,
            _state: PhantomData,
        }
    }
}

/// Stores widget `state` under `id` and evicts states of type `S` untouched for [`EVICT_AFTER`] passes, so rows
/// that vanished do not accumulate. Zero-sized states are not stored, they come back as `Default`.
pub(crate) fn store_evicting<S: Clone + Send + Sync + 'static>(ctx: &Context, pass: u64, id: Id, state: S) {
    if size_of::<S>() == 0 {
        return;
    }
    let log_id = Id::unique("mineral::touched-states");
    ctx.data_mut(|d| {
        d.insert_temp(id, state);
        let log = d.get_temp_mut_or_default::<Touched<S>>(log_id);
        log.last.insert(id, pass);
        if pass.saturating_sub(log.swept) < SWEEP_EVERY {
            return;
        }
        log.swept = pass;
        let mut expired = Vec::new();
        log.last.retain(|&id, &mut at| {
            let keep = pass.saturating_sub(at) < EVICT_AFTER;
            if !keep {
                expired.push(id);
            }
            keep
        });
        for id in expired {
            d.remove::<S>(id);
        }
    });
}

/// Value derived once per egui pass and key; a new pass or key recomputes it. Stylekits freeze setter-driven
/// state (tokens, styles) with it, so no frame mixes two states.
#[derive(Debug)]
pub struct PassCache<K, V> {
    slot: parking_lot::Mutex<Option<(u64, K, V)>>,
}

impl<K, V> Default for PassCache<K, V> {
    fn default() -> Self {
        Self {
            slot: parking_lot::Mutex::new(None),
        }
    }
}

impl<K: PartialEq, V: Clone> PassCache<K, V> {
    /// Value cached for `key` in egui pass `pass` (`Context::cumulative_pass_nr`); otherwise `compute()`.
    /// `compute` runs under the cache lock and must not re-enter it.
    pub fn get_or_insert(&self, pass: u64, key: K, compute: impl FnOnce() -> V) -> V {
        let mut slot = self.slot.lock();
        if let Some((p, k, v)) = &*slot
            && *p == pass
            && *k == key
        {
            return v.clone();
        }
        let v = compute();
        *slot = Some((pass, key, v.clone()));
        v
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn untouched_states_are_evicted_and_live_ones_kept() {
        let ctx = Context::default();
        let (old, live) = (Id::unique("old"), Id::unique("live"));
        store_evicting(&ctx, 1, old, 7_u32);
        store_evicting(&ctx, 1, live, 8_u32);
        for pass in (2..=EVICT_AFTER + SWEEP_EVERY).step_by(100) {
            store_evicting(&ctx, pass, live, 8_u32);
        }
        assert_eq!(ctx.data(|d| d.get_temp::<u32>(old)), None, "expired state is evicted");
        assert_eq!(ctx.data(|d| d.get_temp::<u32>(live)), Some(8), "touched state stays");
    }

    #[test]
    fn pass_cache_recomputes_per_pass_and_key() {
        let cache = PassCache::<u8, u32>::default();
        let mut calls = 0;
        let mut get = |pass, key| {
            cache.get_or_insert(pass, key, || {
                calls += 1;
                calls
            })
        };
        assert_eq!(get(1, 0), 1);
        assert_eq!(get(1, 0), 1, "same pass and key");
        assert_eq!(get(1, 1), 2, "new key");
        assert_eq!(get(2, 1), 3, "new pass");
    }

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
