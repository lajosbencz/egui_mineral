//! Component geometry and colors against the Fluent UI React v9 tokens and the `spec::defaults` they hard-code,
//! one module per catalog category.

#[path = "../support/mod.rs"]
#[expect(dead_code, reason = "shared by the test binaries, each uses a part")]
mod support;

mod actions;
mod collections;
mod data;
mod feedback;
mod inputs;
mod navigation;
mod overlays;
mod surfaces;
mod theme;
