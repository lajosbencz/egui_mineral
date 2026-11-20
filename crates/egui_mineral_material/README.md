# egui_mineral_material

[docs.rs](https://docs.rs/egui_mineral_material) | [crates.io](https://crates.io/crates/egui_mineral_material) | [repository](https://github.com/lajosbencz/egui_mineral) | [showcase](https://lajosbencz.github.io/egui_mineral/material/)

<!-- cargo-rdme start -->

[Material Design 3](https://m3.material.io) stylekit for [egui_mineral](https://crates.io/crates/egui_mineral)
and [egui](https://github.com/emilk/egui): color, typography and motion tokens, a theme engine and components
built on the core behavior models. See every component in the
[live showcase](https://lajosbencz.github.io/egui_mineral/material/).

Components by MD3 category: `actions`, `communication`, `containment`, `navigation`, `selection`,
`text_inputs`, plus `extras` (feature `extras`).

Targets egui 0.37 on native and wasm. Pre-1.0: any minor release may break the API.

## Use

```sh
cargo add egui_mineral_material
cargo add eframe@0.37
```

Pass a `Material` to `Runtime::new` once, import the `prelude` and
show widgets in each pass:

```rust
use std::sync::Arc;

use egui_mineral_material::{egui, prelude::*};

struct App {
    rt: Arc<Rt>,
    clicks: u32,
}

impl App {
    fn new(ctx: &egui::Context) -> Result<Self, InitError> {
        let rt = Runtime::new(ctx, Material::default())?;
        Ok(Self { rt, clicks: 0 })
    }

    /// Call from `eframe::App::ui`.
    fn ui(&mut self, ui: &mut egui::Ui) {
        // Disjoint field borrows; the `Arc` is not cloned per frame.
        let rt = &self.rt;
        let clicks = &mut self.clicks;
        rt.pass(ui, |ui| {
            if ui.mineral(PushButton::new("counter", format!("Clicked {clicks} times"))).clicked() {
                *clicks += 1;
            }
        });
    }
}
```

Each native window can run its own runtime, with its own theme, tokens, locale and density.

## Entry points

- Anything that yields a `WidgetResponse` is a
  `MineralWidget` (leaf or `Composite`)
  and is shown with `ui.mineral(..)` on a `MineralUi`.
- Containers, groups and overlays have `show(..)` returning their own `*Response`: in-layout ones take
  `&mut MineralUi`, floating and modal ones take the `Runtime`, anchored
  popups take the anchor `&Response`. Open state is passed at show time (`show(.., open: &mut bool, ..)`,
  `show_modal(.., open, ..)`).

## Cargo features

| Feature | Default | Adds |
|---|---|---|
| `extras` | yes | The `extras` module (components without an MD3 counterpart), `prelude` re-exports of them and `ValueSlider::value_field` |
| `icu` | yes | ICU4X locale formatting (`egui_mineral/icu`); without it `AsciiFormat` |
| `persist-json` | | JSON persistence codec (`egui_mineral/persist-json`) |
| `persist-toml` | yes | TOML persistence codec (`egui_mineral/persist-toml`) |
| `persist-fs` | yes | Native file-backed storage (`egui_mineral/persist-fs`) |
| `bundled-icons` | yes | Material Symbols Rounded fonts, filled and outlined ([3 MB](https://github.com/lajosbencz/egui_mineral#release-size)); without it `Material::new` and `Default` are absent: use `Material::from_icon_fonts` |
| `theme-file` | | TOML theme files and live reload (`theme::load_toml`, `theme::watch`); native only |
| `unstable-catalog` | | `catalog`: each component's category and spec origin (`catalog::CATALOG`) |
| `unstable-spec` | | Public `spec::tokens` and `spec::defaults`: the generated Compose Material3 values, changed by every re-pin of the Compose sources |

`unstable-*` features are outside the semver guarantee: any release may change them.

<!-- cargo-rdme end -->

License: [GPL-3.0-or-later](https://github.com/lajosbencz/egui_mineral/blob/master/LICENSE) AND [Apache-2.0](https://github.com/lajosbencz/egui_mineral/blob/master/crates/egui_mineral_material/LICENSE-APACHE) for data derived from Jetpack Compose ([NOTICE](https://github.com/lajosbencz/egui_mineral/blob/master/crates/egui_mineral_material/NOTICE))
