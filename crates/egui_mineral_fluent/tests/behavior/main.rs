//! Interaction contracts of the components: input, focus, response fields, agent keys and catalogs.

#[path = "../support/mod.rs"]
#[expect(dead_code, reason = "shared by the test binaries, each uses a part")]
mod support;

mod actions;
mod alignment;
mod collections;
mod feedback;
mod i18n;
mod inputs;
mod menu;
mod navigation;
mod pan;
mod semantics;
mod surfaces;
