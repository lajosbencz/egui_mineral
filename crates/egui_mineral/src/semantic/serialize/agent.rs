use crate::{select::Check, semantic::SemanticTree};

/// Agent-readable markdown snapshot of `tree`.
pub fn to_markdown(tree: &SemanticTree) -> String {
    let mut lines: Vec<String> = Vec::new();
    for n in &tree.nodes {
        let pad = "  ".repeat(n.depth);
        let s = &n.state;
        let mut notes: Vec<String> = Vec::new();
        notes.extend(n.value.iter().map(|v| format!("value: {v:?}")));
        notes.extend(
            n.shortcut
                .iter()
                .map(|shortcut| format!("shortcut: {}", shortcut.format(&egui::ModifierNames::NAMES, false))),
        );
        notes.extend(n.access_key.iter().map(|key| format!("access key: {}", key.char())));
        notes.extend(
            s.range
                .iter()
                .map(|r| format!("range: {} in {}..={}", r.value, r.min, r.max)),
        );
        notes.extend(s.level.iter().map(|l| format!("level: {l}")));
        notes.extend(s.live.iter().map(|l| format!("live: {l:?}").to_lowercase()));
        notes.extend(
            [
                (s.focused, "focused"),
                (s.disabled, "disabled"),
                (s.read_only, "read-only"),
                (s.required, "required"),
                (s.invalid, "invalid"),
                (s.checked == Some(Check::Checked), "checked"),
                (s.checked == Some(Check::Unchecked), "unchecked"),
                (s.checked == Some(Check::Mixed), "mixed"),
                (s.selected == Some(true), "selected"),
                (s.expanded == Some(true), "expanded"),
                (s.expanded == Some(false), "collapsed"),
            ]
            .into_iter()
            .filter(|&(on, _)| on)
            .map(|(_, label)| label.to_owned()),
        );
        notes.extend(
            n.active_descendant
                .and_then(|a| tree.nodes.iter().find(|d| d.widget_id == Some(a)))
                .map(|active| format!("active: {}", active.id.key())),
        );
        let head = format!("{pad}- [{}] {:?}: {}", n.id, n.role, n.name);
        let line: String = std::iter::once(head)
            .chain(notes.iter().map(|note| [" (", note, ")"].concat()))
            .collect();
        lines.push(line);
        if !n.actions.is_empty() {
            let actions: Vec<_> = n.actions.iter().map(|a| format!("{a:?}")).collect();
            lines.push(format!("{pad}  Actions: {}", actions.join(", ")));
        }
    }
    let mut out = String::from("# Current UI State\n\n");
    for line in lines {
        out.push_str(&line);
        out.push('\n');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::semantic::{Action, Role, SemanticNode, StableId};

    #[test]
    fn markdown_snapshot() {
        let tree = SemanticTree {
            nodes: vec![
                SemanticNode {
                    value: Some("john_doe".into()),
                    actions: vec![Action::Focus, Action::SetValue],
                    ..SemanticNode::new(StableId::new("username-field"), Role::TextInput, "Username")
                },
                SemanticNode {
                    actions: vec![Action::Click],
                    ..SemanticNode::new(StableId::new("submit-button"), Role::Button, "Sign In")
                },
            ],
        };
        assert_eq!(
            to_markdown(&tree),
            "# Current UI State\n\n\
             - [username-field] TextInput: Username (value: \"john_doe\")\n  Actions: Focus, SetValue\n\
             - [submit-button] Button: Sign In\n  Actions: Click\n"
        );
    }

    #[test]
    fn markdown_lists_shortcut_and_access_key() {
        let tree = SemanticTree {
            nodes: vec![SemanticNode {
                shortcut: Some(egui::KeyboardShortcut::new(egui::Modifiers::CTRL, egui::Key::S)),
                access_key: Some(crate::shortcut::Mnemonic::new('s')),
                ..SemanticNode::new(StableId::new("save-item"), Role::MenuItem, "Save")
            }],
        };
        assert!(
            to_markdown(&tree).contains("Save (shortcut: Ctrl+S) (access key: S)"),
            "{}",
            to_markdown(&tree)
        );
    }

    #[test]
    fn check_states_are_labelled() {
        let nodes = [Check::Unchecked, Check::Checked, Check::Mixed].map(|c| {
            let mut n = SemanticNode::new(StableId::new(format!("{c:?}")), Role::CheckBox, "C");
            n.state.checked = Some(c);
            n
        });
        let md = to_markdown(&SemanticTree { nodes: nodes.into() });
        for line in [
            "[Unchecked] CheckBox: C (unchecked)",
            "[Checked] CheckBox: C (checked)",
            "[Mixed] CheckBox: C (mixed)",
        ] {
            assert!(md.contains(line), "{line} in {md}");
        }
    }

    #[test]
    fn focus_level_and_live_are_labelled() {
        let mut n = SemanticNode::new(StableId::new("row"), Role::TreeItem, "Row");
        n.state.focused = true;
        n.state.level = Some(2);
        n.state.live = Some(egui::accesskit::Live::Polite);
        let md = to_markdown(&SemanticTree { nodes: vec![n] });
        assert!(md.contains("Row (level: 2) (live: polite) (focused)"), "{md}");
    }

    #[test]
    fn active_descendant_resolves_to_key() {
        let opt = egui::Id::unique("fruit-option");
        let mut combo = SemanticNode::new(StableId::new("fruit-select"), Role::ComboBox, "Fruit");
        combo.active_descendant = Some(opt);
        let mut option = SemanticNode::new(StableId::new("fruit-select-2-option"), Role::ListBoxOption, "Pear");
        option.widget_id = Some(opt);
        let md = to_markdown(&SemanticTree {
            nodes: vec![combo, option],
        });
        assert!(md.contains("Fruit (active: fruit-select-2-option)"), "{md}");
    }
}
