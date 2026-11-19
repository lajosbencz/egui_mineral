use egui::{Context, Event, Key, Modifiers, Pos2, RawInput, Rect, ViewportId, accesskit};
use futures_channel::{
    mpsc::{UnboundedReceiver, UnboundedSender, unbounded},
    oneshot,
};

use crate::{
    input::synth,
    semantic::{Action, SemanticNode, SemanticTree, serialize},
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
    #[error("no node with key `{0}`")]
    NotFound(String),
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

enum Request {
    Snapshot(oneshot::Sender<String>),
    Act(Act),
}

struct Act {
    key: String,
    action: AgentAction,
    reply: oneshot::Sender<Result<(), AgentError>>,
}

/// Async channel to the app's agent host: snapshot the semantic tree and act on nodes by key.
#[derive(Clone, Debug)]
pub struct AgentHandle {
    tx: UnboundedSender<Request>,
    ctx: Context,
    viewport: ViewportId,
}

impl AgentHandle {
    /// Agent markdown of the tree collected on the next pass (see [`serialize::agent`]).
    ///
    /// # Errors
    /// [`AgentError::Disconnected`] when the runtime is gone or drops the request.
    pub async fn snapshot(&self) -> Result<String, AgentError> {
        let (tx, rx) = oneshot::channel();
        self.request(Request::Snapshot(tx))?;
        rx.await.map_err(|_| AgentError::Disconnected)
    }

    /// Performs `action` on the node with `key` from the latest snapshot; resolves once the
    /// input is injected into the next pass.
    ///
    /// # Errors
    /// [`AgentError::NonFinite`] for a NaN or infinite number, [`AgentError::Disconnected`] when the runtime is gone, else the host's rejection (e.g. [`AgentError::NotFound`]).
    pub async fn act(&self, key: impl Into<String>, action: AgentAction) -> Result<(), AgentError> {
        if let AgentAction::SetValue(AgentValue::Number(n)) = action
            && !n.is_finite()
        {
            return Err(AgentError::NonFinite(n));
        }
        let (reply, rx) = oneshot::channel();
        self.request(Request::Act(Act {
            key: key.into(),
            action,
            reply,
        }))?;
        rx.await.map_err(|_| AgentError::Disconnected)?
    }

    fn request(&self, r: Request) -> Result<(), AgentError> {
        self.tx.unbounded_send(r).map_err(|_| AgentError::Disconnected)?;
        self.ctx.request_repaint_of(self.viewport);
        Ok(())
    }
}

pub(crate) struct AgentHost {
    rx: UnboundedReceiver<Request>,
    snapshots: Vec<oneshot::Sender<String>>,
    acts: Vec<Act>,
}

impl AgentHost {
    pub(crate) fn new(ctx: &Context, viewport: ViewportId) -> (Self, AgentHandle) {
        let (tx, rx) = unbounded();
        (
            Self {
                rx,
                snapshots: Vec::new(),
                acts: Vec::new(),
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
        !self.snapshots.is_empty() || !self.acts.is_empty()
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
            reply(act.reply, result);
        }
        pointer
    }

    pub(crate) fn begin_pass(&mut self) {
        self.poll();
    }

    pub(crate) fn respond(&mut self, tree: &SemanticTree) {
        if self.snapshots.is_empty() {
            return;
        }
        let md = serialize::agent::to_markdown(tree);
        for tx in self.snapshots.drain(..) {
            reply(tx, md.clone());
        }
    }
}

// Node and, for `Tap`, the click position.
fn resolve<'t>(
    tree: &'t SemanticTree,
    act: &Act,
    screen: Rect,
) -> Result<(&'t SemanticNode, Option<Pos2>), AgentError> {
    let node = tree
        .find_key(&act.key)
        .ok_or_else(|| AgentError::NotFound(act.key.clone()))?;
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
        tracing::debug!("agent requester gone; reply dropped");
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
        assert!(matches!((&mut act).now_or_never(), Some(Err(AgentError::NotFound(_)))));
    }
}
