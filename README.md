# egui_mineral

[![CI](https://github.com/lajosbencz/egui_mineral/actions/workflows/ci.yml/badge.svg)](https://github.com/lajosbencz/egui_mineral/actions/workflows/ci.yml)
[![version](https://img.shields.io/crates/v/egui_mineral.svg)](https://crates.io/crates/egui_mineral)
[![docs](https://img.shields.io/docsrs/egui_mineral)](https://docs.rs/egui_mineral)
[![license](https://img.shields.io/crates/l/egui_mineral)](LICENSE)
[![msrv](https://img.shields.io/crates/msrv/egui_mineral)](rust-toolchain.toml)

A headless, unopinionated behavior core with different stylekits for customization,
all on top of [egui](https://github.com/emilk/egui).

## Portable

It's cross-platform:

- Open in the [browser](https://lajosbencz.github.io/egui_mineral/material/)
- Or after cloning the repo, run: `cargo run -p showcase-material` or `cargo run -p showcase-fluent`.

## Release size
WASM showcase: `8.7 MB` (gzipped: `3.3 MB`).

Savings over a default build, uncompressed WASM:

| | Feature | Default | Replace with | Saved |
|--:|---|---|---|--:|
| 1 | Icon fonts | Material Symbols | a subset of the icons | `3.0 MB` |
| 2 | Text fonts | egui's bundled fonts | your own fonts | `1.4 MB` |
| 3 | Locale data | ICU4X, every locale | en-US locale only | `0.8 MB` |
| 4 | Renderer | wgpu | glow | `2.0 MB` |

How to opt-out:

1. Disable `bundled-icons` and build with `Material::from_icon_fonts` (plus `Material::with_icons` for another icon font). The showcase's [`build.rs`](examples/showcase-material/build.rs) subsets the fonts to the icons it uses.
2. Disable eframe's `default_fonts` feature and install your own fonts with `egui::Context::set_fonts`.
3. Set `ICU4X_DATA_DIR` to data for your locales ([`i18n`](https://docs.rs/egui_mineral/latest/egui_mineral/i18n/)), or disable `icu`.
4. Build eframe without its default features and enable `glow`, as the [showcase](examples/showcase-material/) does.

## Use

```sh
cargo add egui_mineral_material
cargo add eframe@0.37
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

Cargo features: [docs.rs](https://docs.rs/crate/egui_mineral/latest/features).

## License

- [LICENSE](LICENSE) - GPL-3.0-or-later
- [LICENSE-APACHE](crates/egui_mineral_material/LICENSE-APACHE) - Apache-2.0 data derived from Jetpack Compose
- [NOTICE](crates/egui_mineral_material/NOTICE)
