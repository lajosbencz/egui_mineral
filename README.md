# egui_mineral

[![CI](https://github.com/lajosbencz/egui_mineral/actions/workflows/ci.yml/badge.svg)](https://github.com/lajosbencz/egui_mineral/actions/workflows/ci.yml)
[![version](https://img.shields.io/crates/v/egui_mineral.svg)](https://crates.io/crates/egui_mineral)
[![docs](https://img.shields.io/docsrs/egui_mineral)](https://docs.rs/egui_mineral)
[![license](https://img.shields.io/crates/l/egui_mineral)](LICENSE)
[![msrv](https://img.shields.io/crates/msrv/egui_mineral)](rust-toolchain.toml)

A portable, headless, unopinionated behavior core with swappable stylekits, on top of [egui](https://github.com/emilk/egui).

## Why?

- **Inside and outside:** the core owns input, focus, layout, forms and semantics; stylekits only paint.
- **Multiple stylekits:** Material 3 (Jetpack Compose tokens), Fluent 2 (Fluent UI React v9).
- **Ready for agents:** every widget lands in a semantic tree that agents read and act on, in-process or over MCP.
- **Language and locale:** ICU4X number, date and list formatting.
- **Platform adapters:** file pickers, clipboard, notifications and persistence, native and in the browser.
- **Executor-agnostic:** async work is run by the app's spawner (Tokio and wasm adapters included).
- **Hot patching:** UI edits apply live under `dx serve --hotpatch` with [Dioxus Subsecond](https://docs.rs/subsecond/latest/subsecond/index.html)

## Demos

### Browser

- [Material](https://lajosbencz.github.io/egui_mineral/material/)
- [Fluent](https://lajosbencz.github.io/egui_mineral/fluent/)

### Native

- `cargo run -p showcase-material`
- `cargo run -p showcase-fluent`

## Release size

Material showcase as WASM: `~9 MB` (gzipped: `~4 MB`).

Possible savings over a default build, uncompressed WASM ([how-to](docs/size.md), stylekit [features]):

| | Feature | Default | Replace with | Saved |
|--:|---|---|---|--:|
| 1 | Icon fonts | Material Symbols | [a subset of the icons](docs/size.md#icon-fonts) | `3.0 MB` |
| 2 | Text fonts | egui's bundled fonts | [your own fonts](docs/size.md#text-fonts) | `1.4 MB` |
| 3 | Locale data | ICU4X, every locale | [en-US locale only](docs/size.md#locale-data) | `0.8 MB` |
| 4 | Renderer | wgpu | [glow](docs/size.md#renderer) | `2.0 MB` |

## Quickstart

```sh
cargo add eframe@0.37
cargo add egui_mineral_material
```

```rust
use std::sync::Arc;

use eframe::egui;
use egui_mineral_material::prelude::*;

struct App {
    rt: Arc<Rt>,
    clicks: u32,
}

impl App {
    /// Fails when a catalog does not parse.
    fn new(ctx: &egui::Context) -> Result<Self, InitError> {
        let rt = Runtime::new(ctx, Material::default())?;
        Ok(Self { rt, clicks: 0 })
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        // Disjoint field borrows; the `Arc` is not cloned per frame.
        let rt = &self.rt;
        let clicks = &mut self.clicks;
        rt.pass(ui, |ui| {
            let label = format!("Clicked {clicks} times");
            if ui.mineral(PushButton::new("counter", label).variant(ButtonVariant::Filled)).clicked() {
                *clicks += 1;
            }
        });
    }
}

fn main() -> eframe::Result {
    eframe::run_native(
        "app",
        eframe::NativeOptions::default(),
        Box::new(|cc| Ok(Box::new(App::new(&cc.egui_ctx)?))),
    )
}
```

Stylekits already forward the core's [features]. A minimal build turns the defaults off and lists what it needs:

```sh
cargo add egui_mineral_material --no-default-features --features platform-clipboard,spawn-wasm
```

## Crates

| Crate | | Purpose |
|---|---|---|
| [`egui_mineral`](crates/egui_mineral) | [![crates.io](https://img.shields.io/crates/v/egui_mineral.svg)](https://crates.io/crates/egui_mineral) | Behavior core: input, layout, semantics, agents, i18n, persistence, platform |
| [`egui_mineral_material`](crates/egui_mineral_material) | [![crates.io](https://img.shields.io/crates/v/egui_mineral_material.svg)](https://crates.io/crates/egui_mineral_material) | Material Design 3 stylekit |
| [`egui_mineral_fluent`](crates/egui_mineral_fluent) | [![crates.io](https://img.shields.io/crates/v/egui_mineral_fluent.svg)](https://crates.io/crates/egui_mineral_fluent) | Fluent 2 stylekit |
| [`egui_mineral_eframe`](crates/egui_mineral_eframe) | [![crates.io](https://img.shields.io/crates/v/egui_mineral_eframe.svg)](https://crates.io/crates/egui_mineral_eframe) | eframe adapter: owns the runtime, the app implements its UI |
| [`egui_mineral_mcp`](crates/egui_mineral_mcp) | [![crates.io](https://img.shields.io/crates/v/egui_mineral_mcp.svg)](https://crates.io/crates/egui_mineral_mcp) | MCP server over the agent API (Streamable HTTP, native) |
| [`egui_mineral_macros`](crates/egui_mineral_macros) | [![crates.io](https://img.shields.io/crates/v/egui_mineral_macros.svg)](https://crates.io/crates/egui_mineral_macros) | Derive and catalog macros |

## Docs

- [agents](docs/agents.md): agent API, snapshot formats, MCP server
- [divergences](docs/divergences.md): deliberate departures from Material 3 and Fluent 2
- [blockers](docs/blockers.md): gaps blocked upstream
- [size](docs/size.md): release size opt-outs
- [workspace](docs/workspace.md): layers, module map, commands
- [conventions](docs/conventions.md), [rules](docs/rules.md), [testing](docs/testing.md): for contributors

## Contributing

[CONTRIBUTING.md](CONTRIBUTING.md)

LLM agents start at [AGENTS.md](AGENTS.md)

## License

### `egui_mineral`

- [LICENSE](LICENSE) - GPL-3.0-or-later

### `egui_mineral_material`

Data derived from Jetpack Compose

- [LICENSE-APACHE](crates/egui_mineral_material/LICENSE-APACHE)
- [NOTICE](crates/egui_mineral_material/NOTICE)

### `egui_mineral_fluent`

Data and icon font derived from Fluent UI

- [LICENSE-MIT](crates/egui_mineral_fluent/LICENSE-MIT)
- [NOTICE](crates/egui_mineral_fluent/NOTICE)

[features]: https://docs.rs/crate/egui_mineral_material/latest/features
