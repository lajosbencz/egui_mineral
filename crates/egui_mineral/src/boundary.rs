#[cfg(panic = "unwind")]
use std::sync::Arc;

use egui::UiBuilder;

#[cfg(panic = "unwind")]
use crate::kit::state::TempState;
use crate::{semantic::KeyedSalt, widget::MineralUi};

#[cfg(panic = "unwind")]
#[derive(Clone)]
struct Tripped(Option<Arc<str>>);

#[cfg(panic = "unwind")]
impl TempState for Tripped {}

/// What a boundary does after its fallback ran.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Recovery {
    /// Rerun the content next pass.
    Retry,
    /// Keep showing the fallback.
    Keep,
}

/// Runs `add` in a scope keyed by `id_salt`; a panic trips the boundary until the `fallback` (given the
/// panic message) returns [`Recovery::Retry`]. Returns `Some` only when `add` ran to completion.
/// Only catches with `panic = "unwind"`; wasm32-unknown-unknown defaults to abort, where this is a passthrough.
pub fn error_boundary<K, R>(
    ui: &mut MineralUi<'_, K>,
    id_salt: impl Into<KeyedSalt>,
    add: impl FnOnce(&mut MineralUi<'_, K>) -> R,
    fallback: impl FnOnce(&mut MineralUi<'_, K>, Option<&str>) -> Recovery,
) -> Option<R> {
    let rt = ui.rt();
    let salt = id_salt.into().id_salt();
    let builder = UiBuilder::new().id_salt(salt);
    #[cfg(panic = "unwind")]
    {
        let id = ui.make_persistent_id(salt);
        let tripped = if let Some(t) = Tripped::try_load(ui.ctx(), id) {
            t
        } else {
            let layer = ui.layer_id();
            let start = ui.ctx().graphics(|g| g.get(layer).map_or(0, |l| l.next_idx().0));
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                ui.scope_builder(builder.clone(), |ui| add(&mut rt.ui(ui))).inner
            }));
            let payload = match result {
                Ok(r) => return Some(r),
                Err(payload) => payload,
            };
            let msg = payload
                .downcast_ref::<&str>()
                .map(|s| Arc::from(*s))
                .or_else(|| payload.downcast_ref::<String>().map(|s| Arc::from(s.as_str())));
            tracing::error!(msg = msg.as_deref(), "error boundary caught panic");
            // Shapes `add` put on other layers (areas, popups) are not erased.
            ui.ctx().graphics_mut(|g| {
                let list = g.entry(layer);
                for i in start..list.next_idx().0 {
                    list.reset_shape(egui::layers::ShapeIdx(i));
                }
            });
            let t = Tripped(msg);
            t.clone().store(ui.ctx(), id);
            t
        };
        let recovery = ui
            .scope_builder(builder, |ui| fallback(&mut rt.ui(ui), tripped.0.as_deref()))
            .inner;
        if recovery == Recovery::Retry {
            Tripped::remove(ui.ctx(), id);
            #[expect(
                clippy::disallowed_methods,
                reason = "one pass to rerun the content after the fallback requested a retry"
            )]
            ui.ctx().request_repaint();
        }
        None
    }
    #[cfg(not(panic = "unwind"))]
    {
        let _ = fallback;
        Some(ui.scope_builder(builder, |ui| add(&mut rt.ui(ui))).inner)
    }
}

#[cfg(all(test, panic = "unwind"))]
mod tests {
    use egui::{Color32, Context, Shape};

    use super::*;
    use crate::test_support::{TestKit, input, run, runtime};

    #[test]
    #[expect(clippy::disallowed_methods, reason = "test draws a shape to prove erasure")]
    fn trips_erases_and_retries() {
        let ctx = Context::default();
        let rt = runtime(&ctx);
        let mut runs = 0;
        let mut recovery = Recovery::Keep;
        for pass in 0..4 {
            let out = run(&ctx, input(vec![]), |ui| {
                error_boundary::<TestKit, _>(
                    &mut rt.ui(ui),
                    "b",
                    |ui| {
                        runs += 1;
                        ui.painter().circle_filled(egui::pos2(5.0, 5.0), 3.0, Color32::RED);
                        assert!(pass >= 2, "boom");
                    },
                    |_, msg| {
                        assert_eq!(msg, Some("boom"));
                        recovery
                    },
                );
            });
            assert!(
                out.shapes
                    .iter()
                    .all(|s| !matches!(s.shape, Shape::Circle(_)) || pass >= 2),
                "half-painted content erased on pass {pass}"
            );
            recovery = if pass == 0 { Recovery::Retry } else { Recovery::Keep };
        }
        // Pass 0 panics; pass 1 is tripped, skips `add` and retries; passes 2-3 run.
        assert_eq!(runs, 3);
    }

    #[test]
    fn keep_leaves_the_boundary_tripped() {
        let ctx = Context::default();
        let rt = runtime(&ctx);
        let (mut runs, mut fallbacks) = (0, 0);
        for _ in 0..4 {
            run(&ctx, input(vec![]), |ui| {
                error_boundary::<TestKit, _>(
                    &mut rt.ui(ui),
                    "b",
                    |_| {
                        runs += 1;
                        panic!("boom");
                    },
                    |_, _| {
                        fallbacks += 1;
                        Recovery::Keep
                    },
                );
            });
        }
        assert_eq!((runs, fallbacks), (1, 4));
    }
}
