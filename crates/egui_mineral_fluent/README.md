<!-- cargo-rdme start -->

[Fluent 2](https://fluent2.microsoft.design) stylekit for [egui_mineral](https://crates.io/crates/egui_mineral)
and [egui](https://github.com/emilk/egui), ported from Fluent UI React v9: alias color tokens per theme and
brand, the type ramp, geometry, shadows and motion, a theme engine and components built on the core behavior
models. See every component in the [live showcase](https://lajosbencz.github.io/egui_mineral/fluent/).

Components by category: `actions`, `inputs`, `feedback`, `surfaces`, `navigation`, `data`.

Targets egui 0.37 on native and wasm. Pre-1.0: any minor release may break the API.

# Use

```sh
cargo add egui_mineral_fluent
cargo add eframe@0.37
```

Pass a `Fluent` to `Runtime::new` once, import the `prelude` and
show widgets in each pass:

```rust
use std::sync::Arc;

use egui_mineral_fluent::{egui, prelude::*};

struct App {
    rt: Arc<Rt>,
    clicks: u32,
}

impl App {
    fn new(ctx: &egui::Context) -> Result<Self, InitError> {
        let rt = Runtime::new(ctx, Fluent::new(egui::Theme::Light))?;
        Ok(Self { rt, clicks: 0 })
    }

    /// Call from `eframe::App::ui`.
    fn ui(&mut self, ui: &mut egui::Ui) {
        // Disjoint field borrows; the `Arc` is not cloned per frame.
        let rt = &self.rt;
        let clicks = &mut self.clicks;
        rt.pass(ui, |ui| {
            let label = format!("Clicked {clicks} times");
            if ui.mineral(PushButton::new("counter", label).appearance(ButtonAppearance::Primary)).clicked() {
                *clicks += 1;
            }
        });
    }
}
```

Each native window can run its own runtime, with its own theme, brand and locale.

# Entry points

- Anything that yields a `WidgetResponse` is a
  `MineralWidget` and is shown with `ui.mineral(..)` on a
  `MineralUi`.
- Groups have `show(..)` returning their own `*Response` (`actions::SplitButton`).

# Cargo features

| Feature | Default | Adds |
|---|---|---|
| `icu` | yes | ICU4X locale formatting (`egui_mineral/icu`); without it `AsciiFormat` |
| `persist-fs` | yes | File-backed persistence (`egui_mineral/persist-fs`) |
| `bundled-icons` | yes | Fluent UI System Icons resizable font (1.6 MB); without it `Fluent::icon_font` is required |
| `unstable-catalog` | | `catalog`: each component's category and React v9 styles file (`catalog::CATALOG`) |
| `unstable-spec` | | Public `spec::tokens` and `spec::defaults`: the generated Fluent UI React v9 values, changed by every re-pin of the sources |

`unstable-*` features are outside the semver guarantee: any release may change them.

<!-- cargo-rdme end -->
