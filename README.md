# egui_mineral

[![CI](https://github.com/lajosbencz/egui_mineral/actions/workflows/ci.yml/badge.svg)](https://github.com/lajosbencz/egui_mineral/actions/workflows/ci.yml)
[![version](https://img.shields.io/crates/v/egui_mineral.svg)](https://crates.io/crates/egui_mineral)
[![docs](https://img.shields.io/docsrs/egui_mineral)](https://docs.rs/egui_mineral)
[![license](https://img.shields.io/crates/l/egui_mineral)](https://github.com/lajosbencz/egui_mineral/blob/master/LICENSE)
[![msrv](https://img.shields.io/crates/msrv/egui_mineral)](https://github.com/lajosbencz/egui_mineral/blob/master/rust-toolchain.toml)

A headless, unopinionated behavior core with different stylekits for customization,
all on top of [egui](https://github.com/emilk/egui).

## Demo

It's cross-platform:

- Open in the [browser](https://lajosbencz.github.io/egui_mineral/material/)
- Or after cloning the repo, run: `cargo run -p showcase`.

## Use

```toml
[dependencies]
egui_mineral = "0.1"
egui_mineral_material = "0.1"
egui = "0.37"
eframe = "0.37"
```

```rust
use std::sync::Arc;

use egui_mineral_material::prelude::*;

struct App {
    rt: Arc<Rt>,
    clicks: u32,
}

impl App {
    /// Fails when a catalog does not parse.
    fn new(ctx: &egui::Context) -> Result<Self, egui_mineral::runtime::InitError> {
        let rt = Runtime::new(ctx, Material::new(egui::Theme::Light))?;
        Ok(Self { rt, clicks: 0 })
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let clicks = &mut self.clicks;
        self.rt.pass(ui, |ui| {
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

## Release size

The WASM showcase is 13.1 MB (5.1 MB gzipped), of which:

| Asset | Size | Opt out |
|---|---|---|
| Material Symbols fonts, filled and outlined | 3.0 MB | disable `bundled-icons`, pass `Material::icon_fonts` (e.g. subset fonts) |
| egui text fonts | 1.4 MB | eframe's `default_fonts` feature, then your own via `egui::Context::set_fonts` |
| ICU4X data, every CLDR locale | 0.8 MB over `en` only | `ICU4X_DATA_DIR` with your locales ([`i18n`](https://docs.rs/egui_mineral/latest/egui_mineral/i18n/)), or disable `icu` |

## License

- [LICENSE](https://github.com/lajosbencz/egui_mineral/blob/master/LICENSE)- GPL-3.0-or-later
- [LICENSE-APACHE](https://github.com/lajosbencz/egui_mineral/blob/master/crates/egui_mineral_material/LICENSE-APACHE) - Apache-2.0 data derived from Jetpack Compose
- [NOTICE](https://github.com/lajosbencz/egui_mineral/blob/master/crates/egui_mineral_material/NOTICE)
