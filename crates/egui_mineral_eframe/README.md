# egui_mineral_eframe

[docs.rs](https://docs.rs/egui_mineral_eframe) | [crates.io](https://crates.io/crates/egui_mineral_eframe) | [repository](https://github.com/lajosbencz/egui_mineral)

<!-- cargo-rdme start -->

[eframe](https://crates.io/crates/eframe) adapter for [egui_mineral](https://crates.io/crates/egui_mineral).

`EframeApp` owns the `Runtime` and runs one `Runtime::pass` per frame; the app implements
`MineralApp` and draws through the `MineralUi` it is handed, reaching the runtime with `ui.rt()`.

```rust
use egui_mineral::runtime::{Runtime, Stylekit};
use egui_mineral::widget::MineralUi;
use egui_mineral_eframe::{EframeApp, MineralApp};

struct Counter(u32);

impl<K: Stylekit> MineralApp<K> for Counter {
    fn ui(&mut self, ui: &mut MineralUi<'_, K>) {
        if ui.button(format!("Clicked {} times", self.0)).clicked() {
            self.0 += 1;
        }
    }
}

fn run<K: Stylekit>(kit: K) -> eframe::Result {
    eframe::run_native(
        "counter",
        eframe::NativeOptions::default(),
        Box::new(|cc| Ok(Box::new(EframeApp::new(Runtime::new(&cc.egui_ctx, kit)?, Counter(0))))),
    )
}
```

Features forward to eframe: renderers `glow` (default) and `wgpu`, at least one required; Linux windowing
`wayland` and `x11` (both default), at least one required there. Other eframe features (accesskit,
persistence, ..) are enabled on the app's own `eframe` dependency.

<!-- cargo-rdme end -->

License: [GPL-3.0-or-later](https://github.com/lajosbencz/egui_mineral/blob/master/LICENSE)
