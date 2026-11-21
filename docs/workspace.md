# Workspace details

Module map, config, commands and release. Overview: [WORKSPACE.md](../WORKSPACE.md).

## Module map

| Path | Owns |
|---|---|
| [`crates/egui_mineral/src/input`](../crates/egui_mineral/src/input) | IAL: egui events -> `UnifiedInputEvent`, pure `GestureRecognizer`, `Pull` gesture, hold repeat, `Debounce` |
| [`crates/egui_mineral/src/layout`](../crates/egui_mineral/src/layout) | `Constraints`, `Axis`, `Measure`/`Zones` box model, `Flow` rows and `flex` (egui_taffy), safe areas, breakpoints |
| [`crates/egui_mineral/src/perf`](../crates/egui_mineral/src/perf) | `VirtualList`, `HeightIndex`, `size_or_discard`, `ShapeCache` |
| [`crates/egui_mineral/src/widget.rs`](../crates/egui_mineral/src/widget.rs) | `MineralWidget` trait, `MineralUi` and its `mineral` driver, `SizeExt` size overrides |
| [`crates/egui_mineral/src/kit`](../crates/egui_mineral/src/kit) | Stylekit authoring API, public for third-party widgets and kits (stability contract in its rustdoc): `interact`, `layer`, `scroll`, `nodes`, `paint`, `state` (`TempState`, `Memo`), `choice`, `sanitize`, the shared component models `command_palette`, `menubar`, `strip`, `rating`, `tr`/`tr_or`, `scoped_id`, `opaque_debug!`, `kit::prelude` for widget authors; submodules never reuse a core module name |
| [`crates/egui_mineral/src/{focus,select,expand,pagination,range,number,edit,tree,date,filter,toast,splitter,anchor,table,steps,combobox,mask,tags,status,palette}.rs`](../crates/egui_mineral/src) | Behavior primitives (models, no painting) and data types shared by the stylekits (`SelectionResponse`, `StatusItem`, `PaletteCommand`) |
| [`crates/egui_mineral/src/i18n`](../crates/egui_mineral/src/i18n) | Fluent catalogs, typed message ids (`messages!`, `Message`; proc macro in `crates/egui_mineral_macros`), `Direction`, `LocaleFormat` (ICU4X/CLDR) |
| [`crates/egui_mineral/src/shortcut.rs`](../crates/egui_mineral/src/shortcut.rs) | Shortcut registry, `Mnemonic` |
| [`crates/egui_mineral/src/{form,nav,persist,undo,dnd,window,platform,motion,overlay}`](../crates/egui_mineral/src) | Forms, routing, storage, undo, drag-drop, windows, platform services, motion (springs, presence), overlays (`Dismiss`, anchored `Popup`) |
| [`crates/egui_mineral/src/semantic`](../crates/egui_mineral/src/semantic) | `SemanticTree`, serializers (agent markdown, AccessKit) |
| [`crates/egui_mineral/src/{agent,runtime,bus,policy}.rs`](../crates/egui_mineral/src) | Agent API, `Runtime` (one per native window, per-viewport pass state), executor-agnostic `Bus`, stylekit `Policies` |
| [`crates/egui_mineral_eframe/src`](../crates/egui_mineral_eframe/src) | `MineralApp` (app's UI inside the pass, `frame`/`after_pass`/`save` hooks), `EframeApp` (`eframe::App` owning the `Runtime`; `show` runs a frame headless) |
| [`crates/egui_mineral_mcp/src`](../crates/egui_mineral_mcp/src) | `Server` (MCP tools over `AgentHandle`, bearer token, loopback `Host`, `/mcp` router) |
| [`crates/egui_mineral_material/src`](../crates/egui_mineral_material/src) | `color` (`SchemeSpec`, `Contrast`), `Material` stylekit (`Rt`, `MaterialExt`: tokens, density), `theme` (`Tokens`, feature `theme-file`: `load_toml`, `watch`), `adaptive` (`WindowClass`, `Density`, `MIN_INTERACTIVE`, the 4dp `grid`, wrapping rows), typography, `fonts` (icon font, `install_medium` for an app-supplied medium face), `motion` (MD3 spring tokens), `scroll` (scroll bar treatment), `catalog` (feature `unstable-catalog`: component provenance and category), `spec` (feature `unstable-spec`: `tokens` generated MD3 values and `defaults` Compose `*Defaults` values, role enums always public), `prelude` (stands alone: re-exports `egui_mineral::prelude`, plus components with option enums and item structs, `Material`, icons. Responses, outputs and `theme` stay in their modules) |
| [`crates/egui_mineral_material/src/{actions,communication,containment,navigation,selection,text_inputs}`](../crates/egui_mineral_material/src) | Components by m3.material.io category |
| [`crates/egui_mineral_material/src/extras`](../crates/egui_mineral_material/src/extras) | Non-MD3 components (`Origin::Other`), behind the default feature `extras` |
| [`crates/egui_mineral_material/src/widget`](../crates/egui_mineral_material/src/widget) | Private shared internals (painting parts, indication, row slots, shared vocabulary `Size`, `IconAction`) |
| [`crates/egui_mineral_fluent/src`](../crates/egui_mineral_fluent/src) | `Fluent` stylekit (`Rt`, `FluentExt`: tokens, light/dark/system preference, high contrast, brand ramp), `theme` (`Tokens`, `Scheme`, `Elevation`, `Radius`), `typography` (`TypeRamp`, Fluent typographyStyles), `fonts` (icon font, `install_semibold`), `icon`/`icons` (generated Fluent UI System Icons consts), `catalog` (feature `unstable-catalog`), `spec` (feature `unstable-spec`: generated v9 `tokens` and hand-ported `defaults` from the `use*Styles.styles.ts` files), `prelude` |
| [`crates/egui_mineral_fluent/src/{actions,inputs,feedback,surfaces,navigation,data}`](../crates/egui_mineral_fluent/src) | Components by Fluent 2 category |
| [`crates/egui_mineral_fluent/src/widget`](../crates/egui_mineral_fluent/src/widget) | Private shared internals (focus indicator, shadows, state ramps, vocabulary `Size`, `IconPosition`, `ButtonAppearance`) |
| [`tools/fluent_tokens`](../tools/fluent_tokens) | Fluent token and icon generator: `fetch.sh` fetches fluentui and fluentui-system-icons at the SHAs in `PIN` (gitignored), the binary writes `spec/tokens` and `icons.rs` |

## Config

| File | Governs |
|---|---|
| [`Cargo.toml`](../Cargo.toml) `[workspace.lints]` | rustc, rustdoc and clippy lint levels for every member |
| [`clippy.toml`](../clippy.toml) (root and per crate) | Disallowed methods/types. Per-crate files replace the root one |
| [`deny.toml`](../deny.toml) | License allow list, banned crates, advisories, allowed sources |
| [`sgconfig.yml`](../sgconfig.yml), [`lints/rules`](../lints/rules) | ast-grep rules and their tests |
| [`_typos.toml`](../_typos.toml), [`rustfmt.toml`](../rustfmt.toml), [`.editorconfig`](../.editorconfig) | Spelling, formatting |
| [`.config/nextest.toml`](../.config/nextest.toml) | Test profiles (`ci`) |
| [`release-plz.toml`](../release-plz.toml) | Release config |
| [`.github/workflows`](../.github/workflows) | `ci`, `hygiene`, `showcases`, `pages`, `pages-build`, `release-plz`, `dependabot-automerge` |

## Commands

The [`justfile`](../justfile) is the single source of the checks; [`ci.yml`](../.github/workflows/ci.yml) and [`hygiene.yml`](../.github/workflows/hygiene.yml) call its recipes. `just tools` installs the tools at the pinned versions, `just ci` runs every blocking check, `just --list` shows the parts.

| Recipe | Runs |
|---|---|
| `quick` | fmt check, default-feature clippy of the libraries, first test leg (`just fix`: fmt and `clippy --fix`) |
| `check` | `check-clippy`, `check-test` and `check-docs`, the three parallel CI jobs |
| `check-clippy` | `clippy`: feature matrix (cargo-hack, native and wasm32) |
| `check-test` | `test` (nextest legs) and `icu-trimmed` |
| `check-docs` | `doc` (doctests, rustdoc, docs.rs build from each crate's `[package.metadata.docs.rs]`), `readme-check`, `package`, `versions-check` (shared breaking version, toolchain channel equals `rust-version`), `fmt`, `unstable-guard` (no published crate enables an `unstable-*` feature), `minimal-versions` |
| `hygiene` | typos, deny, ast-grep, machete, workflow lints (`just workflows`: actionlint and zizmor; actionlint is a sha-checked binary pinned in the justfile, not on crates.io) |
| `spec` | non-blocking: regenerated spec must not change |
| `api-diff <base> [head] [release]` | PRs only: `api-diff-one` per `api-targets` line (published crate x docs.rs target, a CI matrix cell each); fails on removed or changed public API unless the crate's breaking version rises; `release=true` (release-plz PRs) takes each crate's base from its last `<crate>-v*` tag |
| `published` | published crates (no `publish = false`), the list `readme`, `package`, `versions-check` and `bump-breaking` use |
| `showcases [public_url]` | release trunk build of both showcases, as Pages deploys them |

Pins live in the justfile (`*_version`, `nightly_version`), outside Dependabot. The nightly is the one the docs.rs build, `minimal-versions` and `api-diff` use: at least a week old, bump it by hand after `just doc` and `just api-diff` pass. Crate READMEs (`crates/*/README.md`) are generated from their `lib.rs` docs: edit the docs, then `just readme`.

CI verifies pull requests; ci.yml on master only builds (clippy, test, docs, the spec generators, rustdoc JSON), when manifests, lockfile, toolchain, `justfile` or the workflow change, to save the per-job caches pull requests restore. `showcases.yml` does the same for the trunk builds when `crates/` or `examples/` change. Hygiene runs on pull requests and weekly, the one scheduled job. A merged release PR publishes (`release-plz.yml`), builds Pages alongside and deploys it after a successful publish, then refreshes the release PR; its align job raises the other crates' breaking versions on it and rewrites its description to match. The release app's private key (`RELEASE_APP_PRIVATE_KEY`) is an environment secret, never a repo one: `release` (master only) for the release PR and align jobs, `crates-io` for publishing.

Not in CI: `cargo bench -p egui_mineral`, `cargo run -p showcase-material`, `cargo run -p showcase-fluent` (F12: inspector), `just hotpatch` (showcase-material under `dx` with `egui_mineral/unstable-hotpatch`: edits inside UI functions apply live, struct layout changes need a restart; any app can pass that feature to `dx serve --hotpatch` on the command line).

## Release

Automated by [`release-plz.toml`](../release-plz.toml) and [`release-plz.yml`](../.github/workflows/release-plz.yml): merging the release pull request publishes the changed crates and tags `<crate>-vX.Y.Z` with a git release each. Each published crate has its own `crates/<crate>/CHANGELOG.md` (generated, never hand-edited).
Versions are per crate, but the published crates share the breaking component (0.MINOR, MAJOR from 1.0): release-plz has no shared-minor option (`version_group` forces identical full versions), and bumps a dependent only by a patch. After release-plz updates its release PR, the align job of `.github/workflows/release-plz.yml` runs `just align-breaking` on that branch: when one crate's breaking version rose, the others and the workspace requirements move to it; otherwise nothing changes. `just versions-check` fails a PR when they diverge. Patch releases are plain release-plz runs: workspace requirements are carets, so a patch of one crate does not release the others.
`just minimal-versions` (nightly `-Z direct-minimal-versions`) keeps the requirement lower bounds honest; raise a bound by hand when a crate starts using a newer sibling API.
[`pages-build.yml`](../.github/workflows/pages-build.yml) builds the site: the static index ([`.github/pages`](../.github/pages)) with the showcases' wasm builds under `material/` and `fluent/`. The release workflow builds it during a publish and deploys it after a successful one; [`pages.yml`](../.github/workflows/pages.yml) builds and deploys by hand.
[`dependabot.yml`](../.github/dependabot.yml) and [`dependabot-automerge.yml`](../.github/workflows/dependabot-automerge.yml) keep dependencies and pinned actions current. Auto-merge uses `GITHUB_TOKEN` (no Dependabot secrets; its merges fire no push workflows, the next push catches up) and takes patch updates, and minor ones only from major >= 1, since 0.MINOR is breaking.
