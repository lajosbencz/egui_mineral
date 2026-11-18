use std::{collections::HashMap, sync::Arc};

use egui::{Id, Painter, Shape, Vec2};

type Entry = (Arc<[Shape]>, u64);

/// Cache entry identity.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ShapeKey {
    /// Owner of the shapes.
    pub id: Id,
    /// Hash of every input of the build closure.
    pub content_hash: u64,
}

/// Age limit for [`ShapeCache::evict`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Eviction {
    /// Current frame number.
    pub frame: u64,
    /// Frames an entry may go unpainted.
    pub max_age: u64,
}

/// Saves regeneration from source data; tessellation still runs per frame in egui.
#[derive(Clone, Debug, Default)]
pub struct ShapeCache {
    entries: HashMap<ShapeKey, Entry>,
}

impl ShapeCache {
    /// `key.content_hash` must cover every input of `build`, including `pixels_per_point` if used.
    #[expect(
        clippy::disallowed_types,
        reason = "replays cached shapes into a caller-supplied painter, creates none"
    )]
    pub fn paint(
        &mut self,
        painter: &Painter,
        key: ShapeKey,
        offset: Vec2,
        frame: u64,
        build: impl FnOnce() -> Vec<Shape>,
    ) {
        let entry = self.entries.entry(key).or_insert_with(|| (build().into(), frame));
        entry.1 = frame;
        painter.extend(entry.0.iter().cloned().map(|mut s| {
            s.translate(offset);
            s
        }));
    }

    /// Drops entries not painted in the last `eviction.max_age` frames before `eviction.frame`.
    pub fn evict(&mut self, eviction: Eviction) {
        self.entries
            .retain(|_, (_, last)| eviction.frame.saturating_sub(*last) <= eviction.max_age);
    }
}

#[cfg(test)]
mod tests {
    use egui::{Id, Shape, Vec2};

    use super::{Eviction, ShapeCache, ShapeKey};
    use crate::test_support::{ctx, input, run};

    fn key(content_hash: u64) -> ShapeKey {
        ShapeKey {
            id: Id::NULL,
            content_hash,
        }
    }

    #[expect(clippy::disallowed_methods, reason = "test painter for the cache under test")]
    fn paint(cache: &mut ShapeCache, key: ShapeKey, frame: u64, builds: &mut u32) {
        run(&ctx(), input(vec![]), |ui| {
            cache.paint(ui.painter(), key, Vec2::ZERO, frame, || {
                *builds += 1;
                vec![Shape::Noop]
            });
        });
    }

    #[test]
    fn builds_once_per_key_and_rebuilds_on_changed_content() {
        let (mut cache, mut builds) = (ShapeCache::default(), 0);
        paint(&mut cache, key(1), 0, &mut builds);
        paint(&mut cache, key(1), 1, &mut builds);
        assert_eq!(builds, 1, "same key reuses the shapes");
        paint(&mut cache, key(2), 2, &mut builds);
        assert_eq!(builds, 2, "changed content_hash rebuilds");
        paint(&mut cache, key(1), 3, &mut builds);
        assert_eq!(builds, 2, "the earlier key is still cached");
    }

    #[test]
    fn evicts_after_max_age_frames_unpainted() {
        let (mut cache, mut builds) = (ShapeCache::default(), 0);
        paint(&mut cache, key(1), 10, &mut builds);
        cache.evict(Eviction { frame: 15, max_age: 5 });
        paint(&mut cache, key(1), 15, &mut builds);
        assert_eq!(builds, 1, "an age of max_age frames is kept");
        cache.evict(Eviction { frame: 21, max_age: 5 });
        paint(&mut cache, key(1), 21, &mut builds);
        assert_eq!(builds, 2, "older than max_age is dropped");
    }
}
