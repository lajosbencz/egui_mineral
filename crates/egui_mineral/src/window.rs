use egui::{Frame, Id, ViewportBuilder, ViewportId};

use crate::{runtime::Runtime, widget::MineralUi};

/// Native viewport window description; shows contents in a separate OS window.
#[derive(Clone, Debug)]
#[must_use = "call .show(..)"]
pub struct WindowSpec {
    id: ViewportId,
    builder: ViewportBuilder,
}

impl WindowSpec {
    /// `id_salt` hashes into the viewport id; `title` is the OS window title.
    pub fn new(id_salt: impl Into<crate::semantic::KeyedSalt>, title: impl Into<String>) -> Self {
        Self {
            id: ViewportId::from_hash_of(id_salt.into().id_salt()),
            builder: ViewportBuilder::default().with_title(title),
        }
    }

    /// Inner size in points.
    pub fn size(mut self, size: egui::Vec2) -> Self {
        self.builder = self.builder.with_inner_size(size);
        self
    }

    /// `add_contents` runs in a [`Runtime::pass`] of the new viewport on its root `Ui`, unframed (panels are
    /// the caller's); embedded (no native window support) it runs inside the current pass.
    pub fn show<K>(self, rt: &Runtime<K>, open: &mut bool, mut add_contents: impl FnMut(&mut MineralUi<'_, K>)) {
        let ctx = rt.ctx();
        if !*open {
            return;
        }
        if ctx.embed_viewports() {
            self.show_embedded(rt, open, add_contents);
            return;
        }
        let mut close = false;
        ctx.show_viewport_immediate(self.id, self.builder, |ui, _| {
            if ui.ctx().input(|i| i.viewport().close_requested()) {
                close = true;
            }
            rt.pass(ui, |ui| add_contents(ui));
        });
        if close {
            *open = false;
        }
    }
}

impl WindowSpec {
    // egui's own embedded viewport window has no close button, so build it here with one. The button's
    // hit box is the style's `icon_width`, raised to the pointer target for this pass.
    fn show_embedded<K>(self, rt: &Runtime<K>, open: &mut bool, mut add_contents: impl FnMut(&mut MineralUi<'_, K>)) {
        let ctx = rt.ctx();
        let target = crate::layout::MinTarget::Auto.points(rt.core());
        let outer = ctx.global_style();
        if outer.spacing.icon_width < target {
            let mut style = (*outer).clone();
            style.spacing.icon_width = target;
            ctx.set_global_style(style);
        }
        egui::Window::from_viewport(self.id, self.builder)
            .open(open)
            .collapsible(false)
            .show(ctx, |ui| rt.pass(ui, |ui| add_contents(ui)));
        ctx.set_global_style(outer);
    }
}

/// Modals render in push order; egui blocks input below the topmost.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ModalStack {
    stack: Vec<Id>,
}

impl ModalStack {
    /// Opens `id` on top; no-op when already open.
    pub fn push(&mut self, id: Id) {
        if !self.stack.contains(&id) {
            self.stack.push(id);
        }
    }

    /// Closes the last pushed modal, which renders on top.
    pub fn pop(&mut self) -> Option<Id> {
        self.stack.pop()
    }

    /// Closes `id` wherever it is in the stack.
    pub fn close(&mut self, id: Id) {
        self.stack.retain(|m| *m != id);
    }

    /// Entry that egui routes input to ([`crate::overlay::top_modal`]); `None` while a modal outside the stack is on
    /// top or before the first [`Self::show`].
    pub fn top(&self, ctx: &egui::Context) -> Option<Id> {
        crate::overlay::top_modal(ctx)
            .map(|l| l.id)
            .filter(|id| self.is_open(*id))
    }

    /// `id` anywhere in the stack.
    pub fn is_open(&self, id: Id) -> bool {
        self.stack.contains(&id)
    }

    /// `content` returns `true` to close its modal; backdrop click and Escape also close.
    pub fn show<K>(&mut self, rt: &Runtime<K>, mut content: impl FnMut(&mut MineralUi<'_, K>, Id) -> bool) {
        let mut closed = Vec::new();
        for &id in &self.stack {
            // The default frame reads the app-wide style.
            let r = egui::Modal::new(id)
                .area(crate::kit::overlay::modal_area(rt.core(), id))
                .frame(Frame::popup(&rt.style()))
                .show(rt.ctx(), |ui| content(&mut rt.ui(ui), id));
            if r.inner || r.should_close() {
                closed.push(id);
            }
        }
        self.stack.retain(|id| !closed.contains(id));
    }
}

#[cfg(test)]
mod tests {
    use egui::{LayerId, Order};

    use super::*;
    use crate::{
        shortcut::{Scope, scope_of},
        test_support::{ctx, input, run},
    };

    #[test]
    fn top_and_shortcut_scope_agree() {
        let ctx = ctx();
        let rt = crate::test_support::runtime(&ctx);
        let (a, b) = (Id::unique("modal-a"), Id::unique("modal-b"));
        let mut stack = ModalStack::default();
        stack.push(a);
        stack.push(b);
        run(&ctx, input(vec![]), |_| stack.show(&rt, |_, _| false));
        let layer = |id| LayerId::new(Order::Foreground, id);
        assert_eq!(stack.top(&ctx), Some(b));
        assert_eq!(scope_of(&ctx, layer(b)), Scope::Modal(b));
        assert_eq!(scope_of(&ctx, layer(a)), Scope::Global, "covered modal");
        stack.pop();
        run(&ctx, input(vec![]), |_| stack.show(&rt, |_, _| false));
        assert_eq!(stack.top(&ctx), Some(a));
        assert_eq!(scope_of(&ctx, layer(a)), Scope::Modal(a));
    }
}
