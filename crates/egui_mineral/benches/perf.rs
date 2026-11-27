//! Criterion benches for `perf` (`cargo bench -p egui_mineral`).

use std::hint::black_box;

use criterion::{Criterion, criterion_group, criterion_main};
use egui_mineral::{
    perf::HeightIndex,
    semantic::{KeyedSalt, StableId},
};

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

// Salt chains as widgets build them: static text, list indices, a node kind, read once.
fn keys(c: &mut Criterion) {
    c.bench_function("keys/static_chain", |b| {
        b.iter(|| {
            let salt = KeyedSalt::from(black_box("settings"))
                .child("general")
                .child(black_box(7_usize));
            StableId::of(&salt, "switch").key().len()
        });
    });
    c.bench_function("keys/owned_chain", |b| {
        b.iter(|| {
            let salt = KeyedSalt::from(black_box("settings".to_owned())).child(black_box("row".to_owned()));
            StableId::of(&salt, "field").key().len()
        });
    });
    c.bench_function("keys/static_build", |b| {
        b.iter(|| {
            let salt = KeyedSalt::from(black_box("settings"))
                .child("general")
                .child(black_box(7_usize));
            StableId::of(&salt, "switch")
        });
    });
    c.bench_function("keys/clone", |b| {
        let salt = KeyedSalt::from("settings").child("general").child(7_usize);
        b.iter(|| black_box(&salt).clone());
    });
}

criterion_group!(benches, height_index, keys);
criterion_main!(benches);
