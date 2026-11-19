# Testing

Tests are light and lock in load-bearing contracts only: spec geometry, interaction and response contracts, behavior models, serializers, agent parity. A test that restates the implementation, or that no plausible change would break, is not added.

| Where | What | Harness |
|---|---|---|
| `crates/egui_mineral/src/**` `#[cfg(test)]` | Core unit tests | `test_support`: `policies()`, `ctx()`, `run` (one pass), `harness` (kittest, settles), event builders |
| [`crates/egui_mineral_material/tests/spec`](../crates/egui_mineral_material/tests/spec) | Component geometry against `spec::tokens` / `spec::defaults`, one module per catalog category, needs the `unstable-spec` and `icu` features | `support::render*` returns a `Pass`: `filled(ColorRole)`, `one`, `stroked`, `circles`, `text`, `node(key)`, `outer`, `zones` |
| [`crates/egui_mineral_material/tests/behavior`](../crates/egui_mineral_material/tests/behavior) | Input, focus, response fields, dismissal, layout (`layout` needs the `unstable-spec` feature), needs `icu`, as does `tests/agent` | `support::harness` / `still` (reduced motion), `MineralHarness`: `click(key)`, `press`, `node`, `rect`, `focused`, `outside()` (screen corner off every node), `step_until`. `i18n::mirror` asserts RTL paints the LTR shapes mirrored |
| [`crates/egui_mineral/tests/models.rs`](../crates/egui_mineral/tests/models.rs) | Property tests of the stateful models: random op sequences incl. shrinking collections and hostile values | `proptest` |
| [`crates/egui_mineral/tests/icu_trimmed.rs`](../crates/egui_mineral/tests/icu_trimmed.rs) | ICU4X with `ICU4X_DATA_DIR` baked for `en` and `ar`: native vs root formatting, direction, ignored unless that data is set (`just icu-trimmed`) | |
| [`examples/showcase`](../examples/showcase) `every_page_holds_up` | Every page at each window class's narrowest width and degenerate viewports, RTL, density, zoom: no panic, id clash, duplicate agent key or page wider than its window | `egui::Context::run_ui` |
| [`crates/egui_mineral_material/tests/agent`](../crates/egui_mineral_material/tests/agent) | Agent API end to end | `MineralHarness::drive_agent` |
| [`crates/egui_mineral_material/tests/support`](../crates/egui_mineral_material/tests/support) | Shared by the three binaries (`#[path]`) | `egui_kittest` harness running a Material `Runtime` in `en-US` |

Rules:

- A new test fails with its fix reverted (mutation-check it once by hand).
- Expected values come from their source: `spec::tokens`, `spec::defaults`, `adaptive::MIN_INTERACTIVE` (`support::min_interactive`), policies (`h.rt().core().policies()`), theme tokens. A literal without a source carries a pointer comment.
- No magic frame counts: settle with `Harness::run` (until no repaint is requested) or step until a condition (`MineralHarness::step_until`). `Harness::step` only where one pass is the contract.
- No pixel guesses: positions come from semantic node rects (`rect(key)`) or widget responses.
- Interact through queued input (`click`, `press`, kittest events) and the agent API, the same path as users.
