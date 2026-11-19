//! Interaction contracts of the `extras` components.

#[path = "../support/mod.rs"]
#[expect(dead_code, reason = "shared by the test binaries, each uses a part")]
mod support;

mod agent;
mod composites;
mod containment;
mod coverage;
mod data_table;
mod edit_fields;
mod extras;
mod fields;
mod i18n;
mod inputs;
mod overlays;
mod prelude_glob;
mod selection;
mod shell;
mod tree_view;
