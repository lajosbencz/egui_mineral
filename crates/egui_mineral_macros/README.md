# egui_mineral_macros

Derive macros for [`egui_mineral`](https://crates.io/crates/egui_mineral). Use them through `egui_mineral`, which
re-exports them; this crate has no API of its own.

- `#[derive(Route)]`: typed routes for `egui_mineral::nav::Router`. `#[route(crate = path)]` names the
  `egui_mineral` crate when it is reached through a re-export (e.g. `egui_mineral_material::egui_mineral`).

License: GPL-3.0-or-later.
