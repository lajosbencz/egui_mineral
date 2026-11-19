//! Debug inspector for `egui_mineral` apps (semantic tree, layout, perf, input history, app state); not published.

use egui::{
    Align2, CentralPanel, Color32, Context, FontId, KeyboardShortcut, Stroke, StrokeKind, Ui, ViewportId, vec2,
};
use egui_mineral::{
    input::{Gesture, UnifiedInputEvent},
    runtime::{Runtime, Stylekit},
    semantic::{SemanticTree, serialize},
    shortcut::{self, ActionId, Scope},
    window::WindowSpec,
};
use serde::{Serialize, de::DeserializeOwned};
use serde_json::Value;
use std::time::Duration;

const INPUT_HISTORY: usize = 256;
const TOGGLE: ActionId = ActionId::from_static("mineral_inspector-toggle");

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
enum Tab {
    #[default]
    Agent,
    A11y,
    Layout,
    Perf,
    Input,
    State,
}

/// Failure converting app state to or from the State tab's JSON.
#[derive(Debug, thiserror::Error)]
pub enum StateError {
    /// serde (de)serialization failed.
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    /// NaN or infinity, which JSON cannot carry.
    #[error("{field}: {value} is not a finite JSON number")]
    NonFinite {
        /// JSON path of the offending field.
        field: String,
        /// NaN or +-infinity as read.
        value: f64,
    },
    /// Edited text that does not parse as the field's type.
    #[error("{field}: `{text}` is not a valid {expected}")]
    Parse {
        /// JSON path of the edited field.
        field: String,
        /// Text as entered.
        text: String,
        /// Type name the field needs.
        expected: &'static str,
    },
    /// Rejection by a custom [`InspectState`] impl.
    #[error("{0}")]
    Rejected(String),
}

/// State the inspector shows and edits as JSON.
pub trait InspectState {
    /// Snapshot shown in the State tab, taken each pass while it is open.
    ///
    /// # Errors
    /// When the state cannot be converted to JSON.
    fn to_json(&self) -> Result<Value, StateError>;
    /// Writes back the snapshot after a committed edit; an `Err` is shown in the tab.
    ///
    /// # Errors
    /// When `value` cannot be converted back into the state.
    fn apply_json(&mut self, value: Value) -> Result<(), StateError>;
}

/// [`InspectState`] over any serde type; a blanket impl would block custom impls on such types.
pub struct Serde<'a, T>(pub &'a mut T);

impl<T: Serialize + DeserializeOwned> InspectState for Serde<'_, T> {
    fn to_json(&self) -> Result<Value, StateError> {
        Ok(serde_json::to_value(&*self.0)?)
    }

    fn apply_json(&mut self, value: Value) -> Result<(), StateError> {
        *self.0 = serde_json::from_value(value)?;
        Ok(())
    }
}

/// Where the inspector UI lives.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum InspectorWindow {
    /// Own OS window; in-app when the backend embeds viewports.
    #[default]
    Detached,
    /// `egui::Window` inside the app viewport.
    Embedded,
}

/// Inspector panel state; keep one per app and call [`Self::show`] every pass. Starts closed.
#[derive(Default)]
pub struct Inspector {
    open: bool,
    window: InspectorWindow,
    toggle: Option<KeyboardShortcut>,
    tab: Tab,
    state_error: Option<StateError>,
    cpu: Option<Duration>,
}

const TITLE: &str = "Mineral Inspector";

impl Inspector {
    /// Visible now; the toggle shortcut and the window close button change it.
    pub fn is_open(&self) -> bool {
        self.open
    }

    /// Shows or hides the inspector from app code.
    pub fn set_open(&mut self, open: bool) {
        self.open = open;
    }

    /// Defaults to [`InspectorWindow::Detached`].
    #[must_use]
    pub fn window(mut self, window: InspectorWindow) -> Self {
        self.window = window;
        self
    }

    /// CPU time of the last frame, shown in the Perf tab; eframe reports it as
    /// `frame.info().cpu_usage`.
    pub fn set_cpu_time(&mut self, cpu: Option<Duration>) {
        self.cpu = cpu;
    }

    /// Call every pass; the shortcut is a global binding.
    pub fn toggle_on(&mut self, ctx: &Context, shortcut: KeyboardShortcut) {
        self.toggle = Some(shortcut);
        if toggled(ctx, shortcut) {
            self.open = !self.open;
        }
    }

    /// Call each pass from the window to inspect, after its UI; requests semantics and input history only while
    /// open.
    pub fn show<K: Stylekit>(&mut self, rt: &Runtime<K>) {
        self.show_inner(rt, None);
    }

    /// [`Self::show`] plus a State tab over `state` (wrap serde types in [`Serde`]).
    pub fn show_with_state<K: Stylekit>(&mut self, rt: &Runtime<K>, state: &mut dyn InspectState) {
        self.show_inner(rt, Some(state));
    }

    fn show_inner<K: Stylekit>(&mut self, rt: &Runtime<K>, mut state: Option<&mut dyn InspectState>) {
        let ctx = rt.ctx();
        let target = ctx.viewport_id();
        if self.open {
            rt.request_semantics(target);
        }
        rt.set_input_history(if self.open && self.tab == Tab::Input {
            INPUT_HISTORY
        } else {
            0
        });
        if !self.open {
            return;
        }
        if self.window == InspectorWindow::Detached && !ctx.embed_viewports() {
            let Self {
                open,
                toggle,
                tab,
                state_error,
                cpu,
                ..
            } = self;
            let mut key = false;
            WindowSpec::new(TITLE, TITLE)
                .size(vec2(420.0, 640.0))
                .show(rt, open, |ui| {
                    key |= toggle.is_some_and(|k| toggled(ui.ctx(), k));
                    CentralPanel::default().show(ui, |ui| {
                        body(ui, tab, rt, target, *cpu, state.as_deref_mut(), state_error);
                    });
                });
            if key {
                *open = false;
            }
        } else {
            let mut open = self.open;
            let Self {
                tab, state_error, cpu, ..
            } = self;
            egui::Window::new(TITLE)
                .open(&mut open)
                .default_width(420.0)
                .show(ctx, |ui| body(ui, tab, rt, target, *cpu, state, state_error));
            self.open = open;
        }
        // Painted on the app's viewport, also when detached.
        if self.open
            && self.tab == Tab::Layout
            && let Some(tree) = rt.tree(target)
        {
            layout_overlay(ctx, &tree);
        }
    }
}

fn toggled(ctx: &Context, shortcut: KeyboardShortcut) -> bool {
    shortcut::bind(ctx, Scope::Global, shortcut, TOGGLE);
    shortcut::take_action(ctx, &TOGGLE)
}

fn body<K: Stylekit>(
    ui: &mut Ui,
    tab: &mut Tab,
    rt: &Runtime<K>,
    target: ViewportId,
    cpu: Option<Duration>,
    state: Option<&mut (dyn InspectState + '_)>,
    state_error: &mut Option<StateError>,
) {
    ui.horizontal_wrapped(|ui| {
        ui.selectable_value(tab, Tab::Agent, "Agent");
        ui.selectable_value(tab, Tab::A11y, "A11y tree");
        ui.selectable_value(tab, Tab::Layout, "Layout");
        ui.selectable_value(tab, Tab::Perf, "Perf");
        ui.selectable_value(tab, Tab::Input, "Input");
        ui.selectable_value(tab, Tab::State, "State");
    });
    ui.separator();
    match tab {
        Tab::Perf => perf(ui, cpu),
        Tab::Input => input_view(ui, rt, target),
        Tab::State => state_view(ui, state, state_error),
        Tab::Agent | Tab::A11y | Tab::Layout => {
            let Some(tree) = rt.tree(target) else {
                ui.label("Collecting semantics...");
                return;
            };
            match tab {
                Tab::Agent => agent_view(ui, &tree),
                Tab::A11y => a11y_tree(ui, &tree),
                _ => {
                    ui.label("Outlining semantic nodes in the app.");
                }
            }
        }
    }
}

fn agent_view(ui: &mut Ui, tree: &SemanticTree) {
    let mut md = serialize::agent::to_markdown(tree);
    if ui.button("Copy").clicked() {
        ui.ctx().copy_text(md.clone());
    }
    egui::ScrollArea::vertical().show(ui, |ui| {
        ui.add(
            egui::TextEdit::multiline(&mut md)
                .code_editor()
                .interactive(false)
                .desired_width(f32::INFINITY),
        );
    });
}

fn a11y_tree(ui: &mut Ui, tree: &SemanticTree) {
    egui::ScrollArea::vertical().show(ui, |ui| {
        for n in &tree.nodes {
            ui.monospace(format!("{}{:?} [{}] {}", "  ".repeat(n.depth), n.role, n.id, n.name));
        }
    });
}

fn layout_overlay(ctx: &Context, tree: &SemanticTree) {
    let painter = ctx.debug_painter();
    for n in &tree.nodes {
        painter.rect_stroke(
            n.rect,
            0,
            Stroke::new(1.0, Color32::from_rgb(255, 0, 255)),
            StrokeKind::Inside,
        );
    }
}

fn perf(ui: &mut Ui, cpu: Option<Duration>) {
    let ms = |d: Duration| d.as_secs_f64() * 1000.0;
    match cpu {
        Some(cpu) => ui.label(format!("CPU: {:.2} ms", ms(cpu))),
        None => ui.label("CPU: not reported"),
    };
    // Time since the previous frame; follows the display refresh while repainting.
    let dt = ui.ctx().input(|i| i.unstable_dt);
    ui.label(format!("Frame interval: {:.2} ms", f64::from(dt) * 1000.0));
    ui.label(format!("Frame: {}", ui.ctx().cumulative_frame_nr()));
}

fn input_view<K: Stylekit>(ui: &mut Ui, rt: &Runtime<K>, target: ViewportId) {
    let ctx = ui.ctx().clone();
    let input = rt.core().input();
    let recent = rt.recent_input(target);
    ui.label(format!("Modality: {:?}", input.modality));
    let painter = ctx.debug_painter();
    let mark = Color32::from_rgb(0, 200, 255);
    let font = FontId::monospace(11.0);
    for e in &recent {
        match e {
            UnifiedInputEvent::PointerDown(p) => {
                painter.circle_stroke(p.pos, 6.0, Stroke::new(1.5, mark));
            }
            UnifiedInputEvent::Gesture(g) => match *g {
                Gesture::Tap { pos } | Gesture::LongPress { pos } => {
                    painter.circle_filled(pos, 4.0, mark);
                    painter.text(
                        pos + egui::vec2(8.0, -8.0),
                        Align2::LEFT_BOTTOM,
                        gesture_name(g),
                        font.clone(),
                        mark,
                    );
                }
                Gesture::Swipe { start, end, .. } => {
                    painter.arrow(start, end - start, Stroke::new(2.0, mark));
                }
                Gesture::Pan { pos, delta } => {
                    painter.line_segment([pos - delta, pos], Stroke::new(1.0, mark));
                }
                Gesture::Pinch { center, .. } => {
                    painter.circle_stroke(center, 12.0, Stroke::new(1.0, mark));
                }
                _ => {}
            },
            _ => {}
        }
    }
    egui::ScrollArea::vertical().stick_to_bottom(true).show(ui, |ui| {
        for e in &recent {
            ui.monospace(describe(e));
        }
    });
}

fn gesture_name(g: &Gesture) -> &'static str {
    match g {
        Gesture::Tap { .. } => "tap",
        Gesture::LongPress { .. } => "long-press",
        Gesture::Swipe { .. } => "swipe",
        Gesture::Pinch { .. } => "pinch",
        Gesture::Pan { .. } => "pan",
        _ => "gesture",
    }
}

fn describe(e: &UnifiedInputEvent) -> String {
    match e {
        UnifiedInputEvent::PointerMoved(p) => format!("move  {:?} {:?} ({:.0}, {:.0})", p.kind, p.id, p.pos.x, p.pos.y),
        UnifiedInputEvent::PointerDown(p) => format!("down  {:?} {:?} ({:.0}, {:.0})", p.kind, p.id, p.pos.x, p.pos.y),
        UnifiedInputEvent::PointerUp(p) => format!("up    {:?} {:?} ({:.0}, {:.0})", p.kind, p.id, p.pos.x, p.pos.y),
        UnifiedInputEvent::Gesture(g) => format!("{:<5} {g:?}", gesture_name(g)),
        UnifiedInputEvent::KeyAction(k) => {
            format!(
                "key   {:?} {} {:?}",
                k.key,
                if k.pressed { "down" } else { "up" },
                k.modifiers
            )
        }
        _ => format!("{e:?}"),
    }
}

fn state_view(ui: &mut Ui, state: Option<&mut (dyn InspectState + '_)>, error: &mut Option<StateError>) {
    let Some(state) = state else {
        ui.label("No state attached. Use Inspector::show_with_state.");
        return;
    };
    let mut value = match state.to_json() {
        Ok(v) => v,
        Err(e) => {
            ui.colored_label(ui.visuals().error_fg_color, e.to_string());
            return;
        }
    };
    let mut committed = false;
    let mut edit_error = None;
    egui::ScrollArea::vertical().show(ui, |ui| {
        committed = json_editor(ui, "state", &mut value, &mut edit_error);
    });
    if let Some(e) = edit_error {
        *error = Some(e);
    } else if committed {
        *error = state.apply_json(value).err();
    }
    if let Some(e) = error {
        ui.colored_label(ui.visuals().error_fg_color, e.to_string());
    }
}

fn json_editor(ui: &mut Ui, label: &str, value: &mut Value, error: &mut Option<StateError>) -> bool {
    match value {
        Value::Object(map) => {
            let mut changed = false;
            egui::CollapsingHeader::new(format!("{label} {{{}}}", map.len()))
                .id_salt(label)
                .show(ui, |ui| {
                    for (k, v) in map.iter_mut() {
                        changed |= json_editor(ui, k, v, error);
                    }
                });
            changed
        }
        Value::Array(items) => {
            let mut changed = false;
            egui::CollapsingHeader::new(format!("{label} [{}]", items.len()))
                .id_salt(label)
                .show(ui, |ui| {
                    for (i, v) in items.iter_mut().enumerate() {
                        ui.push_id(i, |ui| changed |= json_editor(ui, &i.to_string(), v, error));
                    }
                });
            changed
        }
        Value::Bool(b) => {
            ui.horizontal(|ui| {
                ui.label(label);
                ui.checkbox(b, "").changed()
            })
            .inner
        }
        Value::Null => {
            ui.horizontal(|ui| {
                ui.label(label);
                ui.weak("null");
            });
            false
        }
        Value::Number(_) | Value::String(_) => {
            ui.horizontal(|ui| {
                ui.label(label);
                leaf_editor(ui, label, value, error)
            })
            .inner
        }
    }
}

fn leaf_editor(ui: &mut Ui, label: &str, value: &mut Value, error: &mut Option<StateError>) -> bool {
    let id = ui.make_persistent_id(label);
    let shown = match &*value {
        Value::String(s) => s.clone(),
        other => other.to_string(),
    };
    let mut text = ui.data(|d| d.get_temp::<String>(id)).unwrap_or_else(|| shown.clone());
    let r = ui.add(egui::TextEdit::singleline(&mut text).id(id).desired_width(160.0));
    if !r.lost_focus() {
        if r.has_focus() {
            ui.data_mut(|d| d.insert_temp(id, text));
        }
        return false;
    }
    ui.data_mut(|d| d.remove::<String>(id));
    if text == shown {
        return false;
    }
    match parse_leaf(label, value, text) {
        Ok(v) => {
            *value = v;
            true
        }
        Err(e) => {
            *error = Some(e);
            false
        }
    }
}

fn parse_leaf(label: &str, old: &Value, text: String) -> Result<Value, StateError> {
    let err = |expected| StateError::Parse {
        field: label.to_owned(),
        text: text.clone(),
        expected,
    };
    let t = text.trim();
    match old {
        Value::String(_) => Ok(Value::String(text)),
        // serde_json stores non-negative integers as u64; the target type checks the sign.
        Value::Number(n) if n.is_i64() || n.is_u64() => t
            .parse::<i64>()
            .map(Value::from)
            .or_else(|_| t.parse::<u64>().map(Value::from))
            .map_err(|_| err("integer")),
        Value::Number(_) => {
            let f = t.parse::<f64>().map_err(|_| err("number"))?;
            serde_json::Number::from_f64(f)
                .map(Value::Number)
                .ok_or_else(|| StateError::NonFinite {
                    field: label.to_owned(),
                    value: f,
                })
        }
        Value::Null | Value::Bool(_) | Value::Array(_) | Value::Object(_) => {
            unreachable!("leaf_editor edits numbers and strings")
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::Value;

    use super::parse_leaf;

    #[test]
    fn integers_parse_exactly() {
        let big = Value::from(u64::MAX);
        assert_eq!(
            parse_leaf("n", &big, (u64::MAX - 1).to_string()).unwrap(),
            Value::from(u64::MAX - 1)
        );
        let neg = Value::from(-1_i64);
        assert_eq!(
            parse_leaf("n", &neg, "-9007199254740993".into()).unwrap(),
            Value::from(-9_007_199_254_740_993_i64)
        );
        assert_eq!(
            parse_leaf("n", &Value::from(5_u64), "-3".into()).unwrap(),
            Value::from(-3)
        );
        assert!(parse_leaf("n", &neg, "1.5".into()).is_err());
    }
}
