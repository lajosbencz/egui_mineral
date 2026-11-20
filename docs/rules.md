# Rules

Hard rules, their enforcement and immediate-mode performance rules.
Other files cite them as `docs/rules.md rule N`.

## Hard rules

1. No C dependencies in the core. [`deny.toml`](../deny.toml) bans known offenders.
2. State agnostic: UI reads via refs and writes via `Bus` commands. No forced MVC/Redux.
3. No backend types (`winit`, `eframe`) in the core public API, egui/emath types are fine.
4. No heavy work in the render path, push it to async handlers via `Bus` + `Spawner`.
5. The core is headless: no painting of its own, no visual styling, no string catalogs, no design-system values.
   It may submit shapes a stylekit hook returned (`Stylekit::window_close`). Stylekits paint from core responses (`SplitterResponse::handle_rect`, `ReorderOutput::indicator`, `dnd::RowView`),
   ship the catalogs for core message ids (`i18n::messages!` with `inherit = egui_mineral::core_messages` checks them at compile time), and return their `policy::Policies` from `Stylekit::policies`.
   Configuration lives in the typed runtime (`Runtime::new(ctx, kit) -> Result<Arc<Runtime<K>>>`:
   `Core` with i18n, policies, reduced motion and per-viewport pass state, plus the kit), never in `Context` data:
   there is no unset state, no fallback and no lazy default.
   A broken catalog fails construction (locales without baked ICU4X data format as CLDR root, ICU4X's own fallback).
   Design values (targets, breakpoints, fling, repeat, gestures, typeahead) and behavior policies (`readline`, `mnemonics`, `cursors`) are required `Policies` fields.
   Core types carry no design defaults either (`ToastQueue::new` takes its values, `Spring` and `Fling` are literal-constructed).
   Every `set_x` has an `x` getter. Stylekits never do gesture math.
6. Touch targets: widgets go through `MineralUi::mineral`, which allocates the `Zones` (visual box, reserved margin, minimum hit target).
   The target is the widget's `MinTarget` resolved by `Policies::target` and adjusted by `Stylekit::min_target` (Material: 4dp less per density step);
   parts a widget hit-tests itself ask `MineralUi::target`, the only public resolution of a `MinTarget`.
7. Agent parity: agents act through raw egui input (AccessKit action requests, pointer, keys)
   injected by the runtime's egui input plugin (every viewport, no app wiring),
   the same path as screen readers and real input.
   Value widgets accept `SetValue`/`Increment`/`Decrement` via `RangeModel::accesskit`.
8. License is GPL-3.0-or-later. `egui_mineral_material` is `GPL-3.0-or-later AND Apache-2.0`
   for its Compose-derived token data (`LICENSE-APACHE`, `NOTICE`).
   Dependencies must be GPL-3.0 compatible (`deny.toml` allow list).
   No GPL-2.0-only, EPL, CDDL, SSPL.
9. The core never requires Tokio, because it lacks full wasm32-unknown-unknown support, and never starts or owns a runtime.
   Apps supply a `Spawner`: their own, or an opt-in core adapter over their executor (`spawn-tokio`, `spawn-wasm`).
10. Library code never panics, whatever the input:
    caller arguments, layout sizes, text, pointer, key and AccessKit input, agent requests, app data that changes between frames, persisted state, timing. 
    Fallible APIs return `Result`, invalid values are replaced with the nearest valid one through `kit::sanitize` (`at_least`, `within`, `index`),
    which reports each call site once to `tracing`. Errors without a caller (render path, background callbacks) go to `tracing`.
    No silent fallbacks, no `let _ =` on a `Result`, no `# Panics` sections.
    Indices that outlive the current scope (stored, persisted, app-provided, agent or AccessKit requests) go through `.get()` or `kit::sanitize::index` where they are used.
    Locally derived ones may index, iterators are preferred over both.
11. Every thread or task has an owner that joins or cancels it. The core spawns none, apps own them through their `Spawner` (futures) and `BlockingSpawner` (blocking OS calls).
    The opt-in adapters (`spawn::TokioSpawner`, `spawn::WasmSpawner`) forward to the app's executor and own the tasks they spawn: drop or `shutdown` cancels them. Their spawn calls are the only `#[expect(clippy::disallowed_methods, reason)]` of this rule, in `spawn.rs` only.

## Enforcement

Machine-enforced:
- rules 1, 3, 8, 9 by `deny.toml`.
- rules 5, 9, 11 and the repaint rule by per-crate [`clippy.toml`](../clippy.toml) (deny in workspace lints, per-crate files replace the root one, keep base options in sync, exceptions via `#[expect(.., reason)]` on the smallest item)
- rule 10 by `clippy.toml` (`Duration::from_secs_f*`)

The library roots' `deny(clippy::panic, unwrap_used, expect_used, unreachable)` with `missing_panics_doc`, `deny(clippy::indexing_slicing)` and `arithmetic_side_effects` in the stateful core models (`combobox`, `table`, `tree`, `pagination`, `date`, `dnd`, `range`, `mask`, `nav`) and in `egui_mineral_macros`.

The ast-grep rules in [`lints/rules`](../lints/rules):

- `no-panic`: no `assert!` family (`debug_` included) or `# Panics` sections in library code.
- `builder-unchecked-float`: public float setters pass their argument through `kit::sanitize`.
- `nested-ctx-lock`: no Context/Ui/i18n calls inside egui lock closures (`input`, `memory`, `data`, `options`).
- `data-mut-get-temp`: `get_temp` under `data(..)`, `data_mut` only to write or `remove_temp`.
- `restating-doc`, `private-doc`, `ui-literal`, `raw-egui-widget`: comment (doc comments on public items only), i18n-literal and raw-widget conventions.
- `raw-egui-area`: no `Area::new` or `Modal::default_area` outside `kit::overlay`, use `kit::overlay::area` and `modal_area`.
- `stable-id-display-text`: `StableId::new` keys never include label/name/title text.

Unused deps by `cargo machete`.

Public API docs by `[workspace.lints]`: rustc `missing_docs` (every public item, fields, variants and methods included, plus crate roots of binaries, tests and benches) and the rustdoc lints (`broken_intra_doc_links`, `private_intra_doc_links`, `invalid_html_tags`, `unescaped_backticks`, `bare_urls`), all `deny`.
Clippy `missing_panics_doc` and `missing_errors_doc` (deny) require `# Panics` / `# Errors` sections, one line each, `restating-doc` keeps them factual.
Generated `spec/tokens` items get their doc (the Compose token name) from [`tools/md3_tokens`](../tools/md3_tokens).

## Performance rules

- Long or unbounded scroll content uses `perf::VirtualList` (`uniform` for fixed rows, `variable` otherwise).
- Skip work for off-screen rects via `ui.is_rect_visible` (the `MineralUi::mineral` driver already culls).
- Never call `request_repaint()` unconditionally. Use events, `request_repaint_after` or bounded animations.
- Semantic trees are only built when a consumer wants them (an agent snapshot or the inspector). AccessKit writes go through `ctx.accesskit_node_builder`, which returns `None` when AccessKit is inactive.
- `request_discard` only for first-show sizing (`perf::size_or_discard`). egui caps passes at `Options::max_passes`.
- Taffy: keep the tree persistent. `set_style` marks dirty on its own, so only re-set changed styles.
- Benchmark hot paths in [`crates/egui_mineral/benches`](../crates/egui_mineral/benches).
