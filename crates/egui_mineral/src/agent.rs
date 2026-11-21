use egui::{Context, Event, Key, Modifiers, Pos2, RawInput, Rect, ViewportId, accesskit};
use std::sync::Arc;

use futures_channel::{
    mpsc::{UnboundedReceiver, UnboundedSender, unbounded},
    oneshot,
};

use crate::{
    input::synth,
    semantic::{Action, SemanticNode, SemanticTree},
};

/// Action an agent performs on a widget, injected as raw egui input.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum AgentAction {
    /// AccessKit click straight to the widget: works while occluded or scrolled out of view, skips hit testing and gestures.
    Click,
    /// Real pointer press/release at the center of the node's visible rect: exercises hit testing and gestures, reaches nodes without a widget id; needs the node on screen ([`AgentError::NotOnScreen`]).
    Tap,
    /// AccessKit `Focus` request.
    Focus,
    /// AccessKit `SetValue`; value widgets also take numbers via `RangeModel::accesskit`.
    SetValue(AgentValue),
    /// AccessKit `ReplaceSelectedText` on a text field.
    ReplaceSelectedText(String),
    /// AccessKit `Increment` by the node's step.
    Increment,
    /// AccessKit `Decrement` by the node's step.
    Decrement,
    /// AccessKit `ScrollIntoView` on the node's scroll area.
    ScrollIntoView,
    /// AccessKit `ScrollUp`.
    ScrollUp,
    /// AccessKit `ScrollDown`.
    ScrollDown,
    /// AccessKit `ScrollLeft`.
    ScrollLeft,
    /// AccessKit `ScrollRight`.
    ScrollRight,
    /// Focuses the node, then sends `Event::Text`; needs the node's `Focus` action.
    Type(String),
    /// Focuses the node, then presses and releases `Key` with the modifiers; needs the node's `Focus` action.
    Key(Key, Modifiers),
}

/// AccessKit `SetValue` payload.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum AgentValue {
    /// Sent as `ActionData::Value`.
    Text(String),
    /// Sent as `ActionData::NumericValue`; must be finite.
    Number(f64),
}

impl From<String> for AgentValue {
    fn from(s: String) -> Self {
        Self::Text(s)
    }
}

impl From<&str> for AgentValue {
    fn from(s: &str) -> Self {
        Self::Text(s.to_owned())
    }
}

impl From<f64> for AgentValue {
    fn from(n: f64) -> Self {
        Self::Number(n)
    }
}

impl AgentAction {
    fn requires(&self) -> Action {
        match self {
            Self::Click | Self::Tap => Action::Click,
            Self::Focus | Self::Type(_) | Self::Key(..) => Action::Focus,
            Self::SetValue(_) => Action::SetValue,
            Self::ReplaceSelectedText(_) => Action::ReplaceSelectedText,
            Self::Increment => Action::Increment,
            Self::Decrement => Action::Decrement,
            Self::ScrollIntoView => Action::ScrollIntoView,
            Self::ScrollUp => Action::ScrollUp,
            Self::ScrollDown => Action::ScrollDown,
            Self::ScrollLeft => Action::ScrollLeft,
            Self::ScrollRight => Action::ScrollRight,
        }
    }
}

/// Why an agent request failed.
#[derive(Clone, Debug, PartialEq, thiserror::Error)]
#[non_exhaustive]
pub enum AgentError {
    /// Runtime dropped, or the request was abandoned.
    #[error("UI runtime is gone")]
    Disconnected,
    /// No node in the latest tree has this key.
    #[error("no node with key `{key}`; similar: {similar:?}")]
    NotFound {
        /// Requested key.
        key: String,
        /// Keys of the tree sharing the most `-` segments with `key`, best first.
        similar: Vec<String>,
    },
    /// Node lacks the AccessKit action `AgentAction` maps to.
    #[error("node `{key}` does not offer {action:?}; offered: {offered:?}")]
    ActionNotAvailable {
        /// Agent key of the node.
        key: String,
        /// Action the request needs.
        action: Action,
        /// Actions the node offers.
        offered: Vec<Action>,
    },
    /// [`AgentValue::Number`] was NaN or infinite; rejected before queueing.
    #[error("SetValue number must be finite: {0}")]
    NonFinite(f64),
    /// Tree-only node without an egui id; only [`AgentAction::Tap`] reaches it.
    #[error("node `{0}` has no egui widget id")]
    NoWidget(String),
    /// [`AgentAction::Tap`] on a node whose rect is empty, not finite or outside the viewport.
    #[error("node `{0}` is not on screen")]
    NotOnScreen(String),
}

type TreeReply = oneshot::Sender<Result<Arc<SemanticTree>, AgentError>>;

enum Request {
    Snapshot(TreeReply),
    Act(Act),
}

struct Act {
    key: String,
    action: AgentAction,
    reply: ActReply,
}

enum ActReply {
    Done(oneshot::Sender<Result<(), AgentError>>),
    Then(TreeReply),
}

/// Async channel to the app's agent host: snapshot the semantic tree and act on nodes by key.
#[derive(Clone, Debug)]
pub struct AgentHandle {
    tx: UnboundedSender<Request>,
    ctx: Context,
    viewport: ViewportId,
}

impl AgentHandle {
    /// Tree collected on the next pass; [`crate::semantic::serialize::agent`] renders it as markdown, `serde` as
    /// JSON.
    ///
    /// # Errors
    /// [`AgentError::Disconnected`] when the runtime is gone or drops the request.
    pub async fn snapshot(&self) -> Result<Arc<SemanticTree>, AgentError> {
        let (tx, rx) = oneshot::channel();
        self.request(Request::Snapshot(tx))?;
        rx.await.map_err(|_| AgentError::Disconnected)?
    }

    /// Performs `action` on the node with `key` from the latest snapshot; resolves once the
    /// input is injected into the next pass.
    ///
    /// # Errors
    /// [`AgentError::NonFinite`] for a NaN or infinite number, [`AgentError::Disconnected`] when the runtime is gone, else the host's rejection (e.g. [`AgentError::NotFound`]).
    pub async fn act(&self, key: impl Into<String>, action: AgentAction) -> Result<(), AgentError> {
        let (tx, rx) = oneshot::channel();
        self.send_act(key.into(), action, ActReply::Done(tx))?;
        rx.await.map_err(|_| AgentError::Disconnected)?
    }

    /// [`Self::act`], then the tree of the pass after the one that received the input, when the UI has
    /// reacted to it.
    ///
    /// # Errors
    /// As [`Self::act`].
    pub async fn act_then_snapshot(
        &self,
        key: impl Into<String>,
        action: AgentAction,
    ) -> Result<Arc<SemanticTree>, AgentError> {
        let (tx, rx) = oneshot::channel();
        self.send_act(key.into(), action, ActReply::Then(tx))?;
        rx.await.map_err(|_| AgentError::Disconnected)?
    }

    fn send_act(&self, key: String, action: AgentAction, reply: ActReply) -> Result<(), AgentError> {
        if let AgentAction::SetValue(AgentValue::Number(n)) = action
            && !n.is_finite()
        {
            return Err(AgentError::NonFinite(n));
        }
        self.request(Request::Act(Act { key, action, reply }))
    }

    fn request(&self, r: Request) -> Result<(), AgentError> {
        self.tx.unbounded_send(r).map_err(|_| AgentError::Disconnected)?;
        self.ctx.request_repaint_of(self.viewport);
        Ok(())
    }
}

pub(crate) struct AgentHost {
    rx: UnboundedReceiver<Request>,
    snapshots: Vec<TreeReply>,
    acts: Vec<Act>,
    // Injected this pass; answered with the next pass's tree.
    injected: Vec<TreeReply>,
}

impl AgentHost {
    pub(crate) fn new(ctx: &Context, viewport: ViewportId) -> (Self, AgentHandle) {
        let (tx, rx) = unbounded();
        (
            Self {
                rx,
                snapshots: Vec::new(),
                acts: Vec::new(),
                injected: Vec::new(),
            },
            AgentHandle {
                tx,
                ctx: ctx.clone(),
                viewport,
            },
        )
    }

    fn poll(&mut self) {
        while let Ok(req) = self.rx.try_recv() {
            match req {
                Request::Snapshot(tx) => self.snapshots.push(tx),
                Request::Act(a) => self.acts.push(a),
            }
        }
    }

    pub(crate) fn wants_tree(&self) -> bool {
        !self.snapshots.is_empty() || !self.acts.is_empty() || !self.injected.is_empty()
    }

    pub(crate) fn inject(&mut self, raw: &mut RawInput, tree: Option<&SemanticTree>, screen: Rect) -> bool {
        self.poll();
        let Some(tree) = tree else { return false };
        let mut pointer = false;
        for act in self.acts.drain(..) {
            let result = resolve(tree, &act, screen).map(|(node, tap)| {
                pointer |= tap.is_some();
                events(node, tap, &act.action, &mut raw.events);
            });
            match (act.reply, result) {
                (ActReply::Done(tx), result) => reply(tx, result),
                (ActReply::Then(tx), Ok(())) => self.injected.push(tx),
                (ActReply::Then(tx), Err(e)) => reply(tx, Err(e)),
            }
        }
        pointer
    }

    pub(crate) fn begin_pass(&mut self) {
        self.poll();
    }

    pub(crate) fn respond(&mut self, tree: &Arc<SemanticTree>) {
        for tx in self.snapshots.drain(..) {
            reply(tx, Ok(Arc::clone(tree)));
        }
        // Acts injected this pass answer with the next tree.
        self.snapshots.append(&mut self.injected);
    }
}

// Node and, for `Tap`, the click position.
fn resolve<'t>(
    tree: &'t SemanticTree,
    act: &Act,
    screen: Rect,
) -> Result<(&'t SemanticNode, Option<Pos2>), AgentError> {
    let node = tree.find_key(&act.key).ok_or_else(|| AgentError::NotFound {
        key: act.key.clone(),
        similar: similar_keys(tree, &act.key),
    })?;
    let need = act.action.requires();
    if !node.actions.contains(&need) {
        return Err(AgentError::ActionNotAvailable {
            key: act.key.clone(),
            action: need,
            offered: node.actions.clone(),
        });
    }
    if node.widget_id.is_none() && !matches!(act.action, AgentAction::Tap) {
        return Err(AgentError::NoWidget(act.key.clone()));
    }
    if !matches!(act.action, AgentAction::Tap) {
        return Ok((node, None));
    }
    let visible = node.rect.is_finite() && screen.is_finite() && node.rect.is_positive();
    let visible = visible.then(|| node.rect.intersect(screen)).filter(Rect::is_positive);
    visible
        .map(|r| (node, Some(r.center())))
        .ok_or_else(|| AgentError::NotOnScreen(act.key.clone()))
}

const SIMILAR: usize = 5;

fn similar_keys(tree: &SemanticTree, key: &str) -> Vec<String> {
    let wanted: Vec<&str> = key.split('-').filter(|s| !s.is_empty()).collect();
    let mut scored: Vec<(usize, &str)> = tree
        .nodes
        .iter()
        .map(|n| n.id.key())
        .map(|k| (k.split('-').filter(|s| wanted.contains(s)).count(), k))
        .filter(|&(score, _)| score > 0)
        .collect();
    scored.sort_by_key(|&(score, _)| std::cmp::Reverse(score));
    scored.into_iter().take(SIMILAR).map(|(_, k)| k.to_owned()).collect()
}

fn events(node: &SemanticNode, tap: Option<Pos2>, action: &AgentAction, out: &mut Vec<Event>) {
    // `resolve` rejects nodes without a widget for every action but `Tap`.
    let request = |action: Action, data: Option<accesskit::ActionData>| {
        let id = node.widget_id?;
        Some(Event::AccessKitActionRequest(accesskit::ActionRequest {
            action,
            target_tree: accesskit::TreeId::ROOT,
            target_node: id.accesskit_id(),
            data,
        }))
    };
    let value = |s: &str| Some(accesskit::ActionData::Value(s.into()));
    match action {
        AgentAction::Tap => out.extend(tap.map(synth::click).into_iter().flatten()),
        AgentAction::SetValue(AgentValue::Text(s)) => out.extend(request(Action::SetValue, value(s))),
        AgentAction::SetValue(AgentValue::Number(n)) => {
            out.extend(request(Action::SetValue, Some(accesskit::ActionData::NumericValue(*n))));
        }
        AgentAction::ReplaceSelectedText(s) => out.extend(request(Action::ReplaceSelectedText, value(s))),
        AgentAction::Type(text) => {
            out.extend(request(Action::Focus, None));
            out.push(Event::Text(text.clone()));
        }
        AgentAction::Key(key, mods) => {
            out.extend(request(Action::Focus, None));
            out.extend(synth::key_tap(*key, *mods));
        }
        other => out.extend(request(other.requires(), None)),
    }
}

fn reply<T>(tx: futures_channel::oneshot::Sender<T>, value: T) {
    if tx.send(value).is_err() {
        tracing::debug!(target: "egui_mineral", "agent requester gone; reply dropped");
    }
}

#[cfg(test)]
mod tests {
    use egui::ViewportId;
    use futures_util::FutureExt as _;

    use crate::agent::{AgentAction, AgentError};
    use crate::test_support::{ctx, input, run, runtime};

    #[test]
    fn request_wakes_the_handles_viewport() {
        let ctx = ctx();
        let rt = runtime(&ctx);
        let other = ViewportId::from_hash_of("other");
        assert!(rt.agent(other).snapshot().now_or_never().is_none());
        assert!(ctx.has_requested_repaint_for(&other));
    }

    #[test]
    fn act_after_a_pass_without_a_tree_completes_on_requested_repaints() {
        let ctx = ctx();
        let rt = runtime(&ctx);
        let viewport = ViewportId::ROOT;
        let pass = || {
            let out = run(&ctx, input(vec![]), |ui| rt.pass(ui, |_| {}));
            out.viewport_output
                .get(&viewport)
                .is_some_and(|v| v.repaint_delay.is_zero())
        };
        rt.set_collect_semantics(true);
        pass();
        rt.set_collect_semantics(false);
        while pass() {}
        let handle = rt.agent(viewport);
        let mut act = Box::pin(handle.act("missing", AgentAction::Click));
        assert!((&mut act).now_or_never().is_none());
        while pass() {}
        assert!(matches!(
            (&mut act).now_or_never(),
            Some(Err(AgentError::NotFound { .. }))
        ));
    }
}
