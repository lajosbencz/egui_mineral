//! Interaction contracts of the components: input, focus, response fields and dismissal.

#[path = "../support/mod.rs"]
#[expect(dead_code, reason = "shared by the test binaries, each uses a part")]
mod support;

mod actions;
mod composites;
mod containment;
mod coverage;
mod data_table;
mod edit_fields;
mod extras;
mod fields;
mod i18n;
mod inputs;
#[cfg(feature = "unstable-spec")]
mod layout;
mod navigation;
mod overlays;
mod prelude_glob;
mod response_contract;
mod selection;
mod shell;
mod tree_view;
mod windows;
