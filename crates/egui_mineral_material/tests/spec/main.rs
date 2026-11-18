//! Component geometry against the Compose Material3 tokens and `*Defaults`, one module per
//! catalog category.

#[path = "../support/mod.rs"]
#[expect(dead_code, reason = "shared by the test binaries, each uses a part")]
mod support;

mod actions;
mod communication;
mod containment;
mod extras;
mod navigation;
mod selection;
mod text_inputs;
