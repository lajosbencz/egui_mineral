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

// hello = Hello
egui_mineral::i18n::messages!(mod msg = "tests/locales/en-US.ftl");

struct Kit;

impl Stylekit for Kit {
    fn policies(&self, _: &egui::Context) -> Policies {
        Policies::baseline()
    }
    fn catalogs(&self) -> &'static [Catalog] {
        static CATALOGS: [Catalog; 1] = [msg::CATALOG];
        &CATALOGS
    }
    fn setup(&self, _: &egui::Context) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        Ok(())
    }
    fn style(&self, ctx: &egui::Context, theme: egui::Theme) -> std::sync::Arc<egui::Style> {
        ctx.style_of(theme)
    }
}

let ctx = egui::Context::default();
let rt = Runtime::new(&ctx, Kit)?;
let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
    rt.pass(ui, |ui| {
        let hello = ui.core().i18n().tr(msg::HELLO);
        ui.label(hello);
    });
});
output.textures_delta.clear();
```

## Borrowing the runtime

`Runtime::new` returns an `Arc<Runtime<K>>`; a pass borrows it (`let rt = &self.rt;`)
beside disjoint borrows of the app's state fields, without cloning the `Arc` per frame. eframe apps skip this
with [egui_mineral_eframe](https://crates.io/crates/egui_mineral_eframe), which owns the runtime.

## Cargo features

| Feature | Default | Adds |
|---|---|---|
| `icu` | yes | ICU4X formatting, collation and direction; without it `AsciiFormat` |
| `persist-json` | | `persist::Json` codec |
| `persist-toml` | yes | `persist::Toml` codec |
| `persist-fs` | yes | Native-only `persist::FsStorage`; pick a codec feature to encode values |
| `platform-pickers` | | File dialogs (`rfd`): `NativePlatform` implements `PlatformPickers` |
| `platform-clipboard` | | OS clipboard (`arboard`): `PlatformClipboard` |
| `platform-notify` | | OS notifications (`notify-rust`, D-Bus on Linux): `PlatformNotifier` |
| `platform-native` | | `platform-pickers`, `platform-clipboard` and `platform-notify` |
| `spawn-tokio` | | Native only: `spawn` `TokioSpawner`, a `Spawner` and `bus::BlockingSpawner` over the app's Tokio runtime handle |
| `spawn-wasm` | | wasm32 only: `spawn` `WasmSpawner`, a `Spawner` over `wasm_bindgen_futures::spawn_local` |
| `web-save-stream` | | Chromium streams `FileWriter` to disk; other browsers buffer |
| `unstable-hotpatch` | | Native only: Dioxus `subsecond` hot-patching of `Runtime::pass`; the `dx` CLI must match `subsecond` 0.7.10; enable it from the command line only (`dx serve --hotpatch --features egui_mineral/unstable-hotpatch`), never in a manifest, since feature unification would ship it; touch the app's `main.rs` before each `dx serve` (see `docs/blockers.md`) |

## App state

App state is the app's own struct; there is no store. Widgets borrow fields (`&mut T`); discrete actions and
`bus` results fold through one `update`. The `bus` module shows the pattern, with `undo`.

<!-- cargo-rdme end -->

License: [GPL-3.0-or-later](https://github.com/lajosbencz/egui_mineral/blob/master/LICENSE)
