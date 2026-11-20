//! Interaction contracts of the components: input, focus, response fields and dismissal.

#[path = "../support/mod.rs"]
#[expect(dead_code, reason = "shared by the test binaries, each uses a part")]
mod support;

mod actions;
mod agent;
mod composites;
mod containment;
mod coverage;
mod data_table;
mod density;
mod edit_fields;
mod extras;
mod fields;
mod i18n;
mod inputs;
mod navigation;
mod overlays;
mod pan;
mod prelude_glob;
mod response_contract;
mod route;
mod selection;
mod shell;
mod tree_view;
mod windows;
