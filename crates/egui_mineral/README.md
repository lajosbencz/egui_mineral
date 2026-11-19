# egui_mineral

[docs.rs](https://docs.rs/egui_mineral) | [crates.io](https://crates.io/crates/egui_mineral) | [repository](https://github.com/lajosbencz/egui_mineral) | [showcase](https://lajosbencz.github.io/egui_mineral/material/)

<!-- cargo-rdme start -->

Headless behavior core of [egui_mineral](https://github.com/lajosbencz/egui_mineral) for
[egui](https://github.com/emilk/egui). It paints nothing; a stylekit supplies the visuals. Most apps use
[egui_mineral_material](https://crates.io/crates/egui_mineral_material). Depend on this crate to write a
stylekit or headless widgets.

- Unified touch and pointer input, flexbox layout
- Behavior models: focus, selection, ranges, tables, dates, forms, routing
- Fluent and ICU4X i18n, right-to-left layout
- An accessibility semantic tree and an agent API

Targets egui 0.37 on native and wasm. Pre-1.0: any minor release may break the API.

## Writing a stylekit

A `Stylekit` supplies policies, catalogs, setup and the style to a
`Runtime`, one per native window. Anything yielding a
`WidgetResponse` is a `MineralWidget`, directly or through
`Composite`, and is shown with `ui.mineral(..)` on the `MineralUi`
that `Runtime::pass` hands out. A minimal headless pass:

```rust
use egui_mineral::{i18n::Catalog, policy::Policies, runtime::{Runtime, Stylekit}};

struct Kit;

impl Stylekit for Kit {
    fn policies(&self, _: &egui::Context) -> Policies {
        Policies::baseline()
    }
    fn catalogs(&self) -> &'static [Catalog] {
        &[("en", "hello = Hello")]
    }
    fn setup(&self, _: &egui::Context) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        Ok(())
    }
    fn style(&self, ctx: &egui::Context) -> std::sync::Arc<egui::Style> {
        ctx.global_style()
    }
}

let ctx = egui::Context::default();
let rt = Runtime::new(&ctx, Kit)?;
let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
    rt.pass(ui, |ui| {
        let hello = ui.core().i18n().tr("hello", &[]);
        ui.label(hello);
    });
});
output.textures_delta.clear();
```

## Cargo features

| Feature | Default | Adds |
|---|---|---|
| `icu` | yes | ICU4X formatting, collation and direction; without it `AsciiFormat` |
| `persist-fs` | yes | File-backed `persist` stores |
| `platform-native` | | Native file dialogs, clipboard and notifications for `platform` |
| `web-save-stream` | | Chromium streams `FileWriter` to disk; other browsers buffer |
| `unstable-hotpatch` | | Native only: Dioxus `subsecond` hot-patching of `Runtime::pass`; the `dx` CLI must match `subsecond` 0.7.10; enable it from the command line only (`dx serve --hotpatch --features egui_mineral/unstable-hotpatch`), never in a manifest, since feature unification would ship it; touch the app's `main.rs` before each `dx serve` (see `docs/blockers.md`) |

## App state

App state is the app's own struct; there is no store. Widgets borrow fields (`&mut T`); discrete actions and
`bus` results fold through one `update`. The `bus` module shows the pattern, with `undo`.

<!-- cargo-rdme end -->

License: [GPL-3.0-or-later](https://github.com/lajosbencz/egui_mineral/blob/master/LICENSE)
