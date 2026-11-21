//! Semantic tree plumbing: collection checks, popup layer owners, culled rows.

use egui::{Context, Id, LayerId, Rect, Sense, Ui, accesskit::Action as AccessAction};

use crate::semantic::{self, Action, Pass, SemanticNode, collecting};

/// True when this pass collects a tree or AccessKit has a node for `id`.
pub fn wanted(ctx: &Context, id: Option<Id>) -> bool {
    collecting(ctx) || id.is_some_and(|id| ctx.accesskit_node_builder(id, |_| ()).is_some())
}

/// Nodes of `layer` nest under the node of `owner` (a popup under its trigger), as ARIA `aria-owns`. Also
/// set `Area::accessibility_parent` for AccessKit.
pub fn own_layer(ctx: &Context, layer: LayerId, owner: Id) {
    Pass::with(ctx, |c| {
        if c.wanted {
            c.owners.insert(layer, owner);
        }
    });
}

/// Nodes emitted while the guard lives nest under the node of `owner` when their egui hierarchy reaches `ui`
/// (the `Ui` holding `owner`) first: parts a widget interacts beside itself, as [`MineralUi::mineral`] does
/// for every widget's `update`.
///
/// [`MineralUi::mineral`]: crate::widget::MineralUi::mineral
pub fn parts(ctx: &Context, owner: Id, ui: Id) -> Parts {
    Parts {
        scope: semantic::enter(ctx, owner, ui),
        ctx: ctx.clone(),
    }
}

/// Open [`parts`] scope; closes on drop.
#[must_use = "the scope closes when dropped"]
#[derive(Debug)]
pub struct Parts {
    ctx: Context,
    scope: Option<usize>,
}

impl Drop for Parts {
    fn drop(&mut self) {
        semantic::leave(&self.ctx, self.scope, None);
    }
}

/// Adds `node` to the pass's tree when one is wanted, without writing AccessKit.
pub fn push(ctx: &Context, node: SemanticNode) {
    push_from(ctx, node, None);
}

// `anchor` starts the parent walk when the node's widget id has no `WidgetRect` this pass.
pub(crate) fn push_from(ctx: &Context, node: SemanticNode, anchor: Option<Id>) {
    Pass::with(ctx, |c| {
        if c.wanted {
            c.tree.nodes.push(node);
            c.anchors.push(anchor);
            c.in_scope.push(c.open.last().copied());
        }
    });
}

/// Tree-only node for a row outside the viewport that paints nothing: agents reach it with `ScrollIntoView`.
/// `id` is the id the drawn row would have; a widget exists only for the pass that serves the scroll request.
/// Call inside the scrolled content, only when [`wanted`].
pub fn emit_culled(ui: &Ui, id: Id, rect: Rect, node: impl FnOnce() -> SemanticNode) {
    if ui.input(|i| i.has_accesskit_action_request(id, AccessAction::ScrollIntoView)) {
        ui.interact(rect, id, Sense::hover());
    }
    semantic::emit_in(ui, Some(id), rect, || {
        let mut n = node();
        n.actions.push(Action::ScrollIntoView);
        n
    });
}
