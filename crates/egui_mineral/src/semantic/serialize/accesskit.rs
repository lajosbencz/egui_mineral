use egui::{Context, Id, accesskit::Toggled};

use crate::{select::Check, semantic::SemanticNode};

/// Writes into egui's own AccessKit node for `id`; no-op when AccessKit is inactive.
pub fn write(ctx: &Context, id: Id, node: &SemanticNode) {
    let shortcut = node.shortcut.map(|s| crate::shortcut::label(ctx, &s));
    ctx.accesskit_node_builder(id, |b| {
        let s = &node.state;
        b.set_role(node.role);
        b.set_label(node.name.as_str());
        if let Some(v) = &node.value {
            b.set_value(v.as_str());
        }
        if s.disabled {
            b.set_disabled();
        }
        if s.read_only {
            b.set_read_only();
        }
        if s.required {
            b.set_required();
        }
        if s.invalid {
            b.set_invalid(egui::accesskit::Invalid::True);
        }
        if let Some(c) = s.checked {
            b.set_toggled(match c {
                Check::Unchecked => Toggled::False,
                Check::Checked => Toggled::True,
                Check::Mixed => Toggled::Mixed,
            });
        }
        if let Some(sel) = s.selected {
            b.set_selected(sel);
        }
        if let Some(e) = s.expanded {
            b.set_expanded(e);
        }
        if let Some(r) = s.range {
            b.set_numeric_value(r.value);
            b.set_min_numeric_value(r.min);
            b.set_max_numeric_value(r.max);
            if let Some(step) = r.step {
                b.set_numeric_value_step(step);
            }
        }
        if let Some(a) = node.active_descendant {
            b.set_active_descendant(a.accesskit_id());
        }
        if let Some(live) = s.live {
            b.set_live(live);
        }
        if let Some(level) = s.level {
            b.set_level(level);
        }
        if let Some(shortcut) = shortcut {
            b.set_keyboard_shortcut(shortcut);
        }
        if let Some(key) = node.access_key {
            b.set_access_key(key.char().to_string());
        }
        for a in &node.actions {
            b.add_action(*a);
        }
    });
}

#[cfg(test)]
mod tests {
    use egui::{Id, Rect, Sense, accesskit::Toggled};

    use crate::{
        select::Check,
        semantic::{Role, SemanticNode, StableId},
        test_support::{ctx, input, run},
    };

    #[test]
    fn check_states_map_to_toggled() {
        let ctx = ctx();
        ctx.enable_accesskit();
        let checks = [
            (Check::Unchecked, Toggled::False),
            (Check::Checked, Toggled::True),
            (Check::Mixed, Toggled::Mixed),
        ];
        let id = |i: usize| Id::unique("check").with(i);
        let out = run(&ctx, input(vec![]), |ui| {
            for (i, (check, _)) in checks.iter().enumerate() {
                ui.interact(Rect::ZERO, id(i), Sense::click());
                let mut node = SemanticNode::new(StableId::new("c"), Role::CheckBox, "C");
                node.state.checked = Some(*check);
                super::write(ui.ctx(), id(i), &node);
            }
        });
        let update = out.platform_output.accesskit_update.expect("AccessKit enabled");
        for (i, (_, toggled)) in checks.into_iter().enumerate() {
            let node = update
                .nodes
                .iter()
                .find_map(|(nid, n)| (*nid == id(i).accesskit_id()).then_some(n))
                .expect("node written");
            assert_eq!(node.toggled(), Some(toggled));
        }
    }
}
