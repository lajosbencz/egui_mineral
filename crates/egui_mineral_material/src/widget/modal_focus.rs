// WAI-ARIA APG dialog pattern

use egui::{Context, Id, Memory};
use egui_mineral::{kit::state::TempState, overlay};

#[derive(Clone, Copy, Default)]
pub(crate) struct ModalFocus {
    open: bool,
    enter: bool,
    since: u64,
    leave: bool,
    pub(crate) restore: Option<Id>,
}

impl TempState for ModalFocus {}

impl ModalFocus {
    pub(crate) fn scope<R>(
        ctx: &Context,
        id: Id,
        open: &mut bool,
        trigger: Option<Id>,
        show: impl FnOnce(&mut Self, &mut bool) -> R,
    ) -> R {
        let mut focus = Self::load(ctx, id);
        focus.sync(ctx, *open, trigger);
        let r = show(&mut focus, open);
        focus.sync(ctx, *open, trigger);
        focus.leave(ctx);
        focus.store(ctx, id);
        r
    }

    pub(crate) fn sync(&mut self, ctx: &Context, open: bool, trigger: Option<Id>) {
        if open && !self.open {
            *self = Self {
                open: true,
                enter: true,
                since: ctx.cumulative_pass_nr(),
                leave: false,
                restore: ctx.memory(Memory::focused).or(trigger),
            };
        } else if !open && self.open {
            self.open = false;
            self.enter = false;
            self.leave = true;
        }
    }

    pub(crate) fn enter(&mut self, ctx: &Context, target: Id) {
        if self.enter && ctx.cumulative_pass_nr() > self.since {
            self.enter = false;
            ctx.memory_mut(|m| m.request_focus(target));
        }
    }

    pub(crate) fn leave(&mut self, ctx: &Context) {
        if self.leave
            && overlay::top_modal(ctx).is_none()
            && let Some(r) = self.restore.take()
        {
            self.leave = false;
            ctx.memory_mut(|m| m.request_focus(r));
        }
    }
}
