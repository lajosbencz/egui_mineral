# egui_mineral_macros

[docs.rs](https://docs.rs/egui_mineral_macros) | [crates.io](https://crates.io/crates/egui_mineral_macros) | [repository](https://github.com/lajosbencz/egui_mineral)

<!-- cargo-rdme start -->

Macros for [egui_mineral](https://crates.io/crates/egui_mineral). Do not depend on this crate:
`egui_mineral` re-exports it.

`#[derive(Route)]` implements `egui_mineral::nav::Route` for an enum whose variants carry
`#[route("/user/{id}")]` patterns (`{param?}` for a trailing optional parameter). Use it through
`egui_mineral::nav::Route`, which names both the trait and this derive. `#[route(crate = path)]` names the crate
that hosts `nav` when it is reached through a re-export, e.g. `egui_mineral_material::egui_mineral`.

`#[derive(KeySalt)]` implements `egui_mineral::semantic::KeySalt`: a unit variant or unit struct is keyed by its
kebab-case name (`ListItem` -> `list-item`), fields append their own keys joined by `-`, a struct with fields is
keyed by its fields alone. `#[key_salt(rename = "key")]` on a variant or the type fixes the name,
`#[key_salt(crate = path)]` on the type names the crate that hosts `semantic`.

`messages!(mod msg = "locales/en-US.ftl")` turns the Fluent catalog at that path (relative to the crate root)
into a module of typed message ids. Use it through `egui_mineral::i18n::messages`, which documents it.

<!-- cargo-rdme end -->

License: [GPL-3.0-or-later](https://github.com/lajosbencz/egui_mineral/blob/master/LICENSE)
