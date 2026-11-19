# Workspace details

Module map, config, commands and release. Overview: [WORKSPACE.md](../WORKSPACE.md).

## Module map

| Path | Owns |
|---|---|
| [`crates/egui_mineral/src/input`](../crates/egui_mineral/src/input) | IAL: egui events -> `UnifiedInputEvent`, pure `GestureRecognizer`, `Pull` gesture, hold repeat, `Debounce` |
| [`crates/egui_mineral/src/layout`](../crates/egui_mineral/src/layout) | `Constraints`, `Axis`, `Measure`/`Zones` box model, `Flow` rows and `flex` (egui_taffy), safe areas, breakpoints |
| [`crates/egui_mineral/src/perf`](../crates/egui_mineral/src/perf) | `VirtualList`, `HeightIndex`, `size_or_discard`, `ShapeCache` |
| [`crates/egui_mineral/src/widget.rs`](../crates/egui_mineral/src/widget.rs) | `MineralWidget` trait, `MineralUi` and its `mineral` driver, `SizeExt` size overrides |
| [`crates/egui_mineral/src/kit`](../crates/egui_mineral/src/kit) | Stylekit plumbing apps do not call: `sanitize`, `opaque_debug!`, `edit`, `focus`, `input`, `overlay`, `semantic` helpers, `state` (`TempState`, `Memo`), `scoped_id`, `kit::prelude` for widget authors |
| [`crates/egui_mineral/src/{focus,select,expand,pagination,range,number,edit,tree,date,filter,toast,splitter,anchor,table,steps,combobox,mask,tags}.rs`](../crates/egui_mineral/src) | Behavior primitives (models, no painting) |
| [`crates/egui_mineral/src/i18n`](../crates/egui_mineral/src/i18n) | Fluent catalogs, `Direction`, `LocaleFormat` (ICU4X/CLDR) |
| [`crates/egui_mineral/src/shortcut.rs`](../crates/egui_mineral/src/shortcut.rs) | Shortcut registry, `Mnemonic` |
| [`crates/egui_mineral/src/{form,nav,persist,undo,dnd,window,platform,motion,overlay}`](../crates/egui_mineral/src) | Forms, routing, storage, undo, drag-drop, windows, platform services, motion (springs, presence), overlays (`Dismiss`, anchored `Popup`) |
| [`crates/egui_mineral/src/semantic`](../crates/egui_mineral/src/semantic) | `SemanticTree`, serializers (agent markdown, AccessKit) |
| [`crates/egui_mineral/src/{agent,runtime,bus,policy}.rs`](../crates/egui_mineral/src) | Agent API, `Runtime` (one per native window, per-viewport pass state), executor-agnostic `Bus`, stylekit `Policies` |
| [`crates/egui_mineral_material/src`](../crates/egui_mineral_material/src) | `color` (`SchemeSpec`, `Contrast`), `Material` stylekit (`Rt`, `MaterialExt`: tokens, density), `theme` (`Tokens`, `ThemeMode`, feature `theme-file`: `load_toml`, `watch`), `adaptive` (`WindowClass`, `Density`, `MIN_INTERACTIVE`, the 4dp `grid`, wrapping rows), typography, `fonts` (icon font, `install_medium` for an app-supplied medium face), `motion` (MD3 spring tokens), `scroll` (scroll bar treatment), `catalog` (feature `catalog`: component provenance and category), `spec` (feature `spec`: `tokens` generated MD3 values and `defaults` Compose `*Defaults` values, role enums always public), `prelude` (stands alone: re-exports `egui_mineral::prelude`, plus components with option enums and item structs, `Material`, icons. Responses, outputs and `theme` stay in their modules) |
| [`crates/egui_mineral_material/src/{actions,communication,containment,navigation,selection,text_inputs}`](../crates/egui_mineral_material/src) | Components by m3.material.io category |
| [`crates/egui_mineral_material/src/extras`](../crates/egui_mineral_material/src/extras) | Non-MD3 components (`Origin::Other`) |
| [`crates/egui_mineral_material/src/widget`](../crates/egui_mineral_material/src/widget) | Private shared internals (painting parts, indication, row slots, shared vocabulary `Size`, `IconAction`, `SelectionResponse`) |

## Config

| File | Governs |
|---|---|
| [`Cargo.toml`](../Cargo.toml) `[workspace.lints]` | rustc, rustdoc and clippy lint levels for every member |
| [`clippy.toml`](../clippy.toml) (root and per crate) | Disallowed methods/types. Per-crate files replace the root one |
| [`deny.toml`](../deny.toml) | License allow list, banned crates, advisories, allowed sources |
| [`sgconfig.yml`](../sgconfig.yml), [`lints/rules`](../lints/rules) | ast-grep rules and their tests |
| [`_typos.toml`](../_typos.toml), [`rustfmt.toml`](../rustfmt.toml), [`.editorconfig`](../.editorconfig) | Spelling, formatting |
| [`.config/nextest.toml`](../.config/nextest.toml) | Test profiles (`ci`) |
| [`release-plz.toml`](../release-plz.toml) | Release grouping |
| [`.github/workflows`](../.github/workflows) | `ci`, `hygiene`, `pages`, `release-plz`, `dependabot-automerge` |

## Commands

The [`justfile`](../justfile) is the single source of the checks; [`ci.yml`](../.github/workflows/ci.yml) and [`hygiene.yml`](../.github/workflows/hygiene.yml) call its recipes. `just tools` installs the tools, `just ci` runs every blocking check, `just --list` shows the parts (`check`: fmt, clippy matrix, nextest, ICU4X trimmed data, docs, package; `hygiene`: typos, deny, ast-grep, machete, workflow lints; non-blocking: `spec`, `api-diff`). Tool pins live in the justfile, outside Dependabot. CI verifies pull requests; on master it only builds, when manifests, lockfile, toolchain or workflow change, to refresh the caches pull requests restore.

Not in CI: `cargo bench -p egui_mineral`, `cargo run -p showcase` (F12: inspector).

## Release

Paused: `release = false` in [`release-plz.toml`](../release-plz.toml) until trusted publishing is set up.

Automated by [`release-plz.toml`](../release-plz.toml) and [`release-plz.yml`](../.github/workflows/release-plz.yml): merging the release pull request publishes the published crates in lockstep and tags `vX.Y.Z`. `CHANGELOG.md` is generated, never hand-edited.
[`pages.yml`](../.github/workflows/pages.yml) deploys, on each release, the static index ([`.github/pages`](../.github/pages)) with the showcase's wasm build under `material/`.
[`dependabot.yml`](../.github/dependabot.yml) and [`dependabot-automerge.yml`](../.github/workflows/dependabot-automerge.yml) keep dependencies and pinned actions current.
