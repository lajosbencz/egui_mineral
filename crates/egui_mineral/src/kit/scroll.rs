//! Scroll containers shared by stylekits.

use egui::{ScrollArea, Ui, UiBuilder, UiKind, Vec2, Vec2b, scroll_area::ScrollAreaOutput, style::ScrollStyle};

use crate::{input::claim::forward, semantic::KeyedSalt, widget::MineralUi};

/// Bar styles of a [`page`] container.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PageBars {
    /// Bar of the page itself.
    pub page: ScrollStyle,
    /// Bars of the scroll areas inside the page.
    pub nested: ScrollStyle,
}

/// Vertical page scroll container filling the available space with the `page` bar; `add` runs with the `nested`
/// bar style without rubber banding, so scroll areas inside stay distinct from the page bar and [`chain`] their
/// overscroll to it. `id_salt` also scopes every id inside, so
/// per-position state (auto ids) never leaks between pages.
pub fn page<K, R>(
    ui: &mut MineralUi<'_, K>,
    id_salt: impl Into<KeyedSalt>,
    bars: PageBars,
    add: impl FnOnce(&mut MineralUi<'_, K>) -> R,
) -> ScrollAreaOutput<R> {
    let PageBars { page, nested } = bars;
    let rt = ui.rt();
    let id_salt = id_salt.into().id_salt();
    ui.scope_builder(UiBuilder::new().id_salt(id_salt), |ui| {
        ui.spacing_mut().scroll = page;
        ScrollArea::vertical()
            .id_salt(id_salt)
            .auto_shrink([false; 2])
            .show(ui, |ui| {
                ui.spacing_mut().scroll = nested;
                ui.spacing_mut().scroll.kinetic.rubber_band = false;
                add(&mut rt.ui(ui))
            })
    })
    .inner
}

#[derive(Clone, Copy, Debug, Default)]
struct Chain {
    offset: Vec2,
    chained: Vec2b,
    coast: Vec2,
}

/// Hands the touch drag travel the scroll area `out` cannot use to the enclosing scroll area, then coasts it with
/// the release velocity on those axes. Call every pass with the `ui` the area was shown in; a no-op without an
/// enclosing area.
pub fn chain<R>(ui: &Ui, out: &ScrollAreaOutput<R>) {
    if !ui.stack().iter().any(|f| f.kind() == Some(UiKind::ScrollArea)) {
        return;
    }
    // egui ScrollArea: the content drag response id.
    let area = out.id.with("area");
    let ctx = ui.ctx();
    let key = out.id.with("mineral::chain");
    let mut st = ctx.data(|d| d.get_temp::<Chain>(key)).unwrap_or_default();
    let offset = out.state.clamped_offset();
    let (delta, velocity, down, dragging, dt) = ui.input(|i| {
        (
            i.pointer.delta(),
            i.pointer.velocity(),
            i.pointer.any_down(),
            i.pointer.is_decidedly_dragging(),
            i.stable_dt,
        )
    });
    if dragging && ctx.is_being_dragged(area) {
        // The offset moves against the finger.
        let rest = delta + (offset - st.offset);
        let rest = Vec2::new(unused(rest.x), unused(rest.y));
        forward(ui, rest);
        st.chained = Vec2b::new(st.chained.x || rest.x != 0.0, st.chained.y || rest.y != 0.0);
        st.coast = Vec2::ZERO;
    } else {
        if ctx.drag_stopped_id() == Some(area) && st.chained.any() {
            let pick = |on: bool, v: f32| if on { v } else { 0.0 };
            st.coast = Vec2::new(pick(st.chained.x, velocity.x), pick(st.chained.y, velocity.y));
        }
        st.chained = Vec2b::FALSE;
        st.coast = coast(ui, st.coast, dt, down);
    }
    st.offset = offset;
    ctx.data_mut(|d| d.insert_temp(key, st));
}

// egui KineticScrollStyle: exponential decay, stopping under `stop_distance` of remaining travel.
fn coast(ui: &Ui, v: Vec2, dt: f32, down: bool) -> Vec2 {
    let k = ui.spacing().scroll.kinetic;
    if down || k.decay_time <= 0.0 || v.length() * k.decay_time < k.stop_distance {
        return Vec2::ZERO;
    }
    forward(ui, v * dt);
    ui.ctx().request_repaint_after(std::time::Duration::from_millis(16));
    v * (-dt / k.decay_time).exp()
}

// Float error between the offset and the pointer delta.
fn unused(v: f32) -> f32 {
    if v.abs() < 0.01 { 0.0 } else { v }
}

#[cfg(test)]
mod tests {
    use egui::{ScrollArea, Sense, pos2, vec2};

    use super::*;
    use crate::test_support::{harness, touch_drag};

    #[derive(Default)]
    struct Host {
        page: f32,
        inner: Vec2,
    }

    // A nested area of 60 points range on `axis` at the top of a vertical page, chaining to it.
    fn host(horizontal: bool) -> egui_kittest::Harness<'static, crate::test_support::Run<Host>> {
        harness(
            move |ui, h: &mut Host| {
                let page = ScrollArea::vertical()
                    .id_salt("page")
                    .max_height(300.0)
                    .auto_shrink([false; 2])
                    .show(ui, |ui| {
                        ui.spacing_mut().scroll.kinetic.rubber_band = false;
                        let area = if horizontal {
                            ScrollArea::horizontal()
                        } else {
                            ScrollArea::vertical()
                        };
                        let inner = area.id_salt("inner").max_width(200.0).max_height(100.0).show(ui, |ui| {
                            let size = if horizontal {
                                vec2(260.0, 100.0)
                            } else {
                                vec2(200.0, 160.0)
                            };
                            ui.allocate_exact_size(size, Sense::hover());
                        });
                        chain(ui, &inner);
                        h.inner = inner.state.clamped_offset();
                        ui.allocate_space(vec2(1.0, 2000.0));
                    });
                h.page = page.state.clamped_offset().y;
            },
            Host::default(),
        )
    }

    // egui enables drag-to-scroll once it has seen a touch.
    fn warm_up(h: &mut egui_kittest::Harness<'static, crate::test_support::Run<Host>>) {
        h.step();
        touch_drag(h, pos2(100.0, 250.0), Vec2::ZERO, 0, 0);
        h.step();
    }

    #[test]
    fn nested_overscroll_moves_and_coasts_the_page() {
        let mut h = host(false);
        warm_up(&mut h);
        touch_drag(&mut h, pos2(100.0, 80.0), vec2(0.0, -20.0), 6, 0);
        let released = h.state().value.page;
        h.run_steps(300);
        let s = &h.state().value;
        assert!(
            (s.inner.y - 60.0).abs() < 0.01,
            "inner scrolled to its end: {}",
            s.inner.y
        );
        assert!(released > 30.0, "the rest moved the page: {released}");
        assert!(s.page > released + 20.0, "the page coasted: {released} -> {}", s.page);
    }

    #[test]
    fn cross_axis_drag_on_a_nested_area_scrolls_the_page() {
        let mut h = host(true);
        warm_up(&mut h);
        touch_drag(&mut h, pos2(100.0, 80.0), vec2(0.0, -20.0), 6, 0);
        let s = &h.state().value;
        assert!(s.inner.x.abs() < f32::EPSILON, "inner stayed: {}", s.inner.x);
        assert!(s.page > 80.0, "the page followed: {}", s.page);
    }

    #[test]
    fn nested_area_keeps_a_drag_it_can_use() {
        let mut h = host(false);
        warm_up(&mut h);
        touch_drag(&mut h, pos2(100.0, 80.0), vec2(0.0, -10.0), 4, 0);
        h.run_steps(60);
        let s = &h.state().value;
        assert!(s.inner.y > 20.0, "inner scrolled: {}", s.inner.y);
        assert!(s.page.abs() < f32::EPSILON, "page stayed: {}", s.page);
    }
}
