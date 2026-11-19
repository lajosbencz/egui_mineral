# AGENTS.md

Instructions for LLM agents.
Read [CONTRIBUTING.md](CONTRIBUTING.md) first, it indexes the [docs](docs).

## Where to look

| Task | Read |
|---|---|
| Any change | [rules](docs/rules.md), [conventions](docs/conventions.md), [module map](docs/workspace.md#module-map), then the rustdoc of the touched module (`cargo doc --workspace --no-deps --all-features`) |
| New or changed widget | [`crates/egui_mineral/src/widget.rs`](crates/egui_mineral/src/widget.rs), [`crates/egui_mineral/src/kit`](crates/egui_mineral/src/kit) (stylekit plumbing, `kit::prelude`), a similar component in its category module under [`crates/egui_mineral_material/src`](crates/egui_mineral_material/src) (`actions`, `communication`, `containment`, `navigation`, `selection`, `text_inputs`, `extras`), its tests in [`crates/egui_mineral_material/tests/{spec,behavior}`](crates/egui_mineral_material/tests) |
| Behavior model (focus, range, table, date, ...) | the module's file in [`crates/egui_mineral/src`](crates/egui_mineral/src), its unit tests |
| Agent API, semantic tree | [`crates/egui_mineral/src/{agent,semantic}`](crates/egui_mineral/src), [`crates/egui_mineral_material/tests/agent`](crates/egui_mineral_material/tests/agent) |
| Tests | [testing](docs/testing.md), [`crates/egui_mineral/src/test_support.rs`](crates/egui_mineral/src/test_support.rs), [`crates/egui_mineral_material/tests/support`](crates/egui_mineral_material/tests/support). Stateful model changes extend [`crates/egui_mineral/tests/models.rs`](crates/egui_mineral/tests/models.rs), layout changes keep the showcase's `every_page_holds_up` green |
| Public API change | review `just api-diff <base>` |
| Lint failure | [`lints/rules`](lints/rules), [`clippy.toml`](clippy.toml) (per crate), [`deny.toml`](deny.toml) |
| MD3 spec values | [`crates/egui_mineral_material/src/spec`](crates/egui_mineral_material/src/spec), [`tools/md3_tokens`](tools/md3_tokens) (never edit `spec/tokens`) |
| Upstream limitation | [blockers](docs/blockers.md), unexercised core APIs: [todo](docs/todo.md) |
| Departure from MD3, Compose or conventions | record it in [divergences](docs/divergences.md) |
| CI parity | [commands](docs/workspace.md#commands), [`justfile`](justfile) |

## Agent rules

- Before establishing a convention, check whether existing tooling (rustc/clippy/rustdoc lints, `rustfmt`, `cargo-deny`, the `ast-grep` rules) covers it idiomatically. Enforce it there instead of by hand or by a new tool.
- Verify crate APIs against the version in `Cargo.lock` (docs.rs or that version's source in the cargo registry, which may hold several) before use. Never assume an API from memory.
- Before reporting work done, run `just ci` (or the recipes the change touches) and fix every failure. Missing tools: ask the user to run `just tools`.
- Do not commit unless asked.
- On an unexpected blocker, stop and ask instead of working around it.
- No panics, public docs, lints and tests: [rules](docs/rules.md), [testing](docs/testing.md).
- Private code is uncommented except a `//` line pointing to the ported spec symbol (`// Compose TimeSelector`) or stating a fact the code cannot express (a literal's source, units, invariants, quirks). No restating, prose or history.
- ASCII only in code, comments and docs.
- Headless egui tests must call `output.textures_delta.clear()` (egui 0.37 panics on dropped deltas).
