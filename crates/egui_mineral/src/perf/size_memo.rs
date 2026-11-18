use egui::{Context, Id, Vec2};

use crate::kit::state::TempState;

/// Last size stored for `id`; on first appearance requests a discard so the measured pass is shown instead.
pub fn size_or_discard(ctx: &Context, id: Id) -> Option<Vec2> {
    let size = Vec2::try_load(ctx, id);
    if size.is_none() {
        ctx.request_discard("mineral: unsized container");
    }
    size
}

/// Remembers `size` for `id` so the next pass can lay out without a discard.
pub fn store_size(ctx: &Context, id: Id, size: Vec2) {
    size.store(ctx, id);
}

#[cfg(test)]
mod tests {
    use egui::{Context, Id, vec2};

    use crate::test_support::{input, run};

    #[test]
    fn first_show_discards_until_sized() {
        let ctx = Context::default();
        let id = Id::unique("sized");
        let size = vec2(30.0, 20.0);
        let mut seen = Vec::new();
        run(&ctx, input(vec![]), |ui| {
            seen.push((super::size_or_discard(ui.ctx(), id), ui.ctx().will_discard()));
            super::store_size(ui.ctx(), id, size);
        });
        assert_eq!(
            seen,
            [(None, true), (Some(size), false)],
            "the unsized pass is discarded and rerun with the stored size"
        );
    }
}
