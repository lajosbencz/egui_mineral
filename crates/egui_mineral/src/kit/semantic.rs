//! Semantic tree plumbing: collection checks, parent rects, raw node pushes.

use egui::{Context, Id, Rect};

use crate::semantic::{Collector, SemanticNode, collecting, collector_id};

/// True when this pass collects a tree or AccessKit has a node for `id`.
pub fn wanted(ctx: &Context, id: Option<Id>) -> bool {
    collecting(ctx) || id.is_some_and(|id| ctx.accesskit_node_builder(id, |_| ()).is_some())
}

/// Stores `rect` for `id` for the semantic node of the same pass, when a tree is wanted.
pub fn remember_rect(ctx: &Context, id: Id, rect: Rect) {
    if wanted(ctx, Some(id)) {
        ctx.data_mut(|d| d.insert_temp(id.with(RECT_KEY), rect));
    }
}

pub(crate) const RECT_KEY: &str = "mineral::rect";

/// Adds `node` to the pass's tree when one is wanted.
pub fn push(ctx: &Context, node: SemanticNode) {
    let id = collector_id(ctx);
    ctx.data_mut(|d| {
        let c = d.get_temp_mut_or_default::<Collector>(id);
        if c.wanted {
            c.tree.nodes.push(node);
        }
    });
}
