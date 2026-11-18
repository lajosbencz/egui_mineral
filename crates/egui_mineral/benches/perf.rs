//! Criterion benches for `perf` (`cargo bench -p egui_mineral`).

use std::hint::black_box;

use criterion::{Criterion, criterion_group, criterion_main};
use egui_mineral::perf::HeightIndex;

fn height_index(c: &mut Criterion) {
    let mut idx = HeightIndex::new(1_000_000, 20.0);
    c.bench_function("height_index/set", |b| {
        let mut i = 0;
        b.iter(|| {
            i = (i + 7919) % 1_000_000;
            idx.set(i, black_box(25.0));
        });
    });
    c.bench_function("height_index/range", |b| {
        b.iter(|| idx.range(black_box(egui::Rangef::new(5_000_000.0, 5_001_000.0))));
    });
}

criterion_group!(benches, height_index);
criterion_main!(benches);
