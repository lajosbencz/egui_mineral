use egui::{Context, Id};

use crate::kit::state::TempState;

/// Label column of a two-column form: every row reads the widest label of the previous pass, and a change of
/// that width discards the pass, so no shown frame mixes widths.
#[derive(Clone, Copy, Debug)]
pub struct LabelColumn {
    id: Id,
    last: Option<f32>,
    widest: f32,
}

#[derive(Clone, Copy)]
struct Stored(f32);

impl TempState for Stored {}

impl LabelColumn {
    /// Column state under `id`.
    pub fn begin(ctx: &Context, id: Id) -> Self {
        Self {
            id,
            last: Stored::try_load(ctx, id).map(|s| s.0),
            widest: 0.0,
        }
    }

    /// Widest label of the previous pass; 0 on the first.
    pub fn width(&self) -> f32 {
        self.last.unwrap_or(0.0)
    }

    /// Counts a label of natural (unwrapped) width `width` into this pass.
    pub fn measure(&mut self, width: f32) {
        if width.is_finite() {
            self.widest = self.widest.max(width);
        }
    }

    /// Stores this pass's widest label; a change from the previous pass requests a discard.
    pub fn end(self, ctx: &Context) {
        if self.last.is_none_or(|w| (w - self.widest).abs() > 0.5) {
            Stored(self.widest).store(ctx, self.id);
            ctx.request_discard("mineral: label column width");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn width_follows_the_widest_label_of_the_previous_pass() {
        let ctx = Context::default();
        let id = Id::unique("c");
        let mut c = LabelColumn::begin(&ctx, id);
        assert_eq!(c.width(), 0.0);
        c.measure(40.0);
        c.measure(f32::NAN);
        c.measure(90.0);
        c.end(&ctx);
        let mut c = LabelColumn::begin(&ctx, id);
        assert_eq!(c.width(), 90.0);
        c.measure(30.0);
        c.end(&ctx);
        assert_eq!(LabelColumn::begin(&ctx, id).width(), 30.0, "shrinks with the labels");
    }
}
