use serde::{Serialize, Serializer};

use crate::{
    select::Check,
    semantic::{NumericRange, SemanticNode, SemanticTree},
};

/// Agent-readable markdown snapshot of `tree`.
pub fn to_markdown(tree: &SemanticTree) -> String {
    let mut out = String::from("# Current UI State\n\n");
    for n in &tree.nodes {
        let pad = "  ".repeat(n.depth);
        let s = &n.state;
        let mut notes: Vec<String> = Vec::new();
        notes.extend(n.value.iter().map(|v| format!("value: {v:?}")));
        notes.extend(shortcut(n).map(|s| format!("shortcut: {s}")));
        notes.extend(n.access_key.iter().map(|key| format!("access key: {}", key.char())));
        notes.extend(
            s.range
                .iter()
                .map(|r| format!("range: {} in {}..={}", r.value, r.min, r.max)),
        );
        notes.extend(s.level.iter().map(|l| format!("level: {l}")));
        notes.extend(live(n).map(|l| format!("live: {l}")));
        notes.extend(flags(n).map(str::to_owned));
        notes.extend(active(tree, n).map(|key| format!("active: {key}")));
        let head = format!("{pad}- [{}] {:?}: {}", n.id, n.role, n.name);
        out.push_str(&head);
        for note in notes {
            out.push_str(&[" (", &note, ")"].concat());
        }
        out.push('\n');
        if !n.actions.is_empty() {
            let actions: Vec<_> = n.actions.iter().map(|a| format!("{a:?}")).collect();
            out.push_str(&[&pad, "  Actions: ", &actions.join(", "), "\n"].concat());
        }
    }
    out
}

fn flags(n: &SemanticNode) -> impl Iterator<Item = &'static str> {
    let s = &n.state;
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
        (s.multiselectable, "multiselectable"),
        (s.expanded == Some(true), "expanded"),
        (s.expanded == Some(false), "collapsed"),
    ]
    .into_iter()
    .filter(|&(on, _)| on)
    .map(|(_, label)| label)
}

fn shortcut(n: &SemanticNode) -> Option<String> {
    n.shortcut.map(|s| s.format(&egui::ModifierNames::NAMES, false))
}

fn live(n: &SemanticNode) -> Option<String> {
    n.state.live.map(|l| format!("{l:?}").to_lowercase())
}

fn active<'t>(tree: &'t SemanticTree, n: &SemanticNode) -> Option<&'t str> {
    let a = n.active_descendant?;
    tree.nodes.iter().find(|d| d.widget_id == Some(a)).map(|d| d.id.key())
}

/// Nested JSON with the markdown's names: `{"nodes": [{"key", "role", "name", "value"?, "states"?, "level"?,
/// "range"?, "live"?, "actions"?, "shortcut"?, "access_key"?, "active"?, "children"?}]}`.
impl Serialize for SemanticTree {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        #[derive(Serialize)]
        struct Tree<'a> {
            nodes: Vec<Node<'a>>,
        }
        Tree {
            nodes: nest(self, &self.nodes),
        }
        .serialize(serializer)
    }
}

#[derive(Serialize)]
struct Node<'a> {
    key: &'a str,
    role: String,
    name: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    value: Option<&'a str>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    states: Vec<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    level: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    range: Option<NumericRange>,
    #[serde(skip_serializing_if = "Option::is_none")]
    live: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    actions: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    shortcut: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    access_key: Option<char>,
    #[serde(skip_serializing_if = "Option::is_none")]
    active: Option<&'a str>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    children: Vec<Node<'a>>,
}

// `nodes` is pre-order with depths, as in `SemanticTree::nodes`.
fn nest<'a>(tree: &'a SemanticTree, nodes: &'a [SemanticNode]) -> Vec<Node<'a>> {
    let mut out = Vec::new();
    let mut rest = nodes;
    while let Some((n, tail)) = rest.split_first() {
        let len = tail.iter().position(|d| d.depth <= n.depth).unwrap_or(tail.len());
        let (kids, next) = tail.split_at(len);
        out.push(Node {
            key: n.id.key(),
            role: format!("{:?}", n.role),
            name: &n.name,
            value: n.value.as_deref(),
            states: flags(n).collect(),
            level: n.state.level,
            range: n.state.range,
            live: live(n),
            actions: n.actions.iter().map(|a| format!("{a:?}")).collect(),
            shortcut: shortcut(n),
            access_key: n.access_key.map(crate::shortcut::Mnemonic::char),
            active: active(tree, n),
            children: nest(tree, kids),
        });
        rest = next;
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
    fn json_nests_children_with_the_markdown_names() {
        let mut parent = SemanticNode::new(StableId::new("list"), Role::List, "Fruit");
        let mut child = SemanticNode::new(StableId::new("pear"), Role::ListItem, "Pear");
        child.state.selected = Some(true);
        child.depth = 1;
        parent.actions = vec![Action::Focus];
        let tree = SemanticTree {
            nodes: vec![parent, child],
        };
        let json = serde_json::to_value(&tree).expect("serializes");
        assert_eq!(
            json,
            serde_json::json!({"nodes": [{
                "key": "list", "role": "List", "name": "Fruit", "actions": ["Focus"],
                "children": [{"key": "pear", "role": "ListItem", "name": "Pear", "states": ["selected"]}],
            }]})
        );
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
