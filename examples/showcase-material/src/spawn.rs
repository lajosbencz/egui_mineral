//! The showcase's `Spawner`: the core adapter over a Tokio runtime on native, over `spawn_local` on the web.

use std::time::Duration;

#[cfg(not(target_arch = "wasm32"))]
pub(crate) use egui_mineral::spawn::TokioSpawner as AppSpawner;
#[cfg(target_arch = "wasm32")]
pub(crate) use egui_mineral::spawn::WasmSpawner as AppSpawner;

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn sleep(d: Duration) -> impl Future<Output = ()> {
    tokio::time::sleep(d)
}

#[cfg(target_arch = "wasm32")]
pub(crate) fn sleep(d: Duration) -> impl Future<Output = ()> {
    gloo_timers::future::sleep(d)
}
