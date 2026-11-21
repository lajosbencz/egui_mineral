//! [MCP](https://modelcontextprotocol.io) server for [egui_mineral](https://crates.io/crates/egui_mineral) apps.
//!
//! Agents read the semantic tree of a [`Runtime`] and act on its nodes by agent key, over Streamable HTTP at
//! `/mcp`. Requests need `Authorization: Bearer <token>`; `Host` must be loopback and browser `Origin`s are
//! rejected. The app runs [`Server::serve`] on its own Tokio executor, with I/O and timers enabled; each request collects the tree it
//! needs on the next pass.
//!
//! ```no_run
//! # async fn run(rt: std::sync::Arc<egui_mineral::runtime::Runtime<impl egui_mineral::runtime::Stylekit>>)
//! # -> Result<(), Box<dyn std::error::Error>> {
//! let token = std::env::var("MINERAL_MCP_TOKEN")?;
//! let listener = tokio::net::TcpListener::bind("127.0.0.1:7878").await?;
//! egui_mineral_mcp::Server::new(rt, token)?.serve(listener).await?;
//! # Ok(()) }
//! ```
//!
//! Tools: `list_viewports`, `snapshot`, `click`, `tap`, `focus`, `type_text`, `press_key`, `set_value`,
//! `step`, `scroll`. Every action returns the tree after the UI reacted, as markdown.

use std::{io, sync::Arc};

use axum::{
    extract::Request,
    http::{StatusCode, header},
    middleware::Next,
    response::{IntoResponse, Response},
};
use egui::{Key, Modifiers, ViewportId};
use egui_mineral::{
    agent::{AgentAction, AgentError, AgentHandle, AgentValue},
    runtime::{Runtime, Stylekit},
    semantic::{SemanticTree, serialize::agent::to_markdown},
};
use rmcp::{
    ServerHandler,
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{CallToolResult, ContentBlock, Implementation, ServerCapabilities, ServerConfig},
    schemars, tool, tool_handler, tool_router,
    transport::streamable_http_server::{
        StreamableHttpServerConfig, StreamableHttpService, session::local::LocalSessionManager,
    },
};
use serde::Deserialize;

/// [`Server::new`] got an empty token.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
#[error("MCP bearer token is empty")]
pub struct EmptyToken;

/// MCP endpoint over one [`Runtime`].
#[derive(Clone)]
pub struct Server {
    host: Arc<dyn Host>,
    token: Arc<str>,
}

impl std::fmt::Debug for Server {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Server").finish_non_exhaustive()
    }
}

impl Server {
    /// Serves `rt` to clients sending `token` as bearer.
    ///
    /// # Errors
    /// [`EmptyToken`] when `token` is empty.
    pub fn new<K: Stylekit>(rt: Arc<Runtime<K>>, token: impl Into<String>) -> Result<Self, EmptyToken> {
        let token: String = token.into();
        if token.is_empty() {
            return Err(EmptyToken);
        }
        Ok(Self {
            host: rt,
            token: token.into(),
        })
    }

    /// Router serving `/mcp`, for nesting into an app's own axum server.
    pub fn router(&self) -> axum::Router {
        let host = Arc::clone(&self.host);
        let config = StreamableHttpServerConfig::default()
            .with_legacy_session_mode(false)
            .with_json_response(true)
            .enforce_origin_validation();
        let service: StreamableHttpService<Tools, LocalSessionManager> =
            StreamableHttpService::new(move || Ok(Tools::new(Arc::clone(&host))), Arc::default(), config);
        let token = Arc::clone(&self.token);
        axum::Router::new()
            .nest_service("/mcp", service)
            .layer(axum::middleware::from_fn(move |req: Request, next: Next| {
                let token = Arc::clone(&token);
                async move { authorize(&token, req, next).await }
            }))
    }

    /// Serves [`Self::router`] on `listener` until the listener fails.
    ///
    /// # Errors
    /// I/O errors of the listener.
    pub async fn serve(self, listener: tokio::net::TcpListener) -> io::Result<()> {
        axum::serve(listener, self.router()).await
    }
}

async fn authorize(token: &str, req: Request, next: Next) -> Response {
    let given = req
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "));
    match given {
        Some(given) if same(given.as_bytes(), token.as_bytes()) => next.run(req).await,
        _ => StatusCode::UNAUTHORIZED.into_response(),
    }
}

// Constant time in the content; the length is not secret.
fn same(a: &[u8], b: &[u8]) -> bool {
    a.len() == b.len() && a.iter().zip(b).fold(0, |acc, (x, y)| acc | (x ^ y)) == 0
}

trait Host: Send + Sync + 'static {
    fn viewports(&self) -> Vec<ViewportId>;
    fn agent(&self, viewport: ViewportId) -> AgentHandle;
}

impl<K: Stylekit> Host for Runtime<K> {
    fn viewports(&self) -> Vec<ViewportId> {
        Runtime::viewports(self)
    }
    fn agent(&self, viewport: ViewportId) -> AgentHandle {
        Runtime::agent(self, viewport)
    }
}

fn viewport_name(id: ViewportId) -> String {
    if id == ViewportId::ROOT {
        "root".to_owned()
    } else {
        format!("{:016x}", id.0.value())
    }
}

#[derive(Clone)]
struct Tools {
    host: Arc<dyn Host>,
    tool_router: ToolRouter<Self>,
}

#[derive(Deserialize, schemars::JsonSchema)]
struct SnapshotArgs {
    #[schemars(description = "From `list_viewports`; default `root`.")]
    viewport: Option<String>,
    #[schemars(description = "Agent key of the node whose subtree to return; default the whole tree.")]
    root: Option<String>,
}

#[derive(Deserialize, schemars::JsonSchema)]
struct NodeArgs {
    #[schemars(description = "From `list_viewports`; default `root`.")]
    viewport: Option<String>,
    #[schemars(description = "Agent key from the snapshot.")]
    key: String,
}

#[derive(Deserialize, schemars::JsonSchema)]
struct TextArgs {
    #[schemars(description = "From `list_viewports`; default `root`.")]
    viewport: Option<String>,
    #[schemars(description = "Agent key from the snapshot.")]
    key: String,
    text: String,
}

#[derive(Deserialize, schemars::JsonSchema)]
struct KeyArgs {
    #[schemars(description = "From `list_viewports`; default `root`.")]
    viewport: Option<String>,
    #[schemars(description = "Agent key of the node receiving the key press.")]
    key: String,
    #[schemars(description = "egui key name, e.g. `Enter`, `Escape`, `ArrowDown`, `A`.")]
    name: String,
    #[serde(default)]
    modifiers: Vec<Modifier>,
}

#[derive(Clone, Copy, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
enum Modifier {
    Alt,
    Ctrl,
    Shift,
    #[schemars(description = "Ctrl, Cmd on macOS.")]
    Command,
}

#[derive(Deserialize, schemars::JsonSchema)]
#[serde(untagged)]
enum Value {
    Number(f64),
    Text(String),
}

#[derive(Deserialize, schemars::JsonSchema)]
struct ValueArgs {
    #[schemars(description = "From `list_viewports`; default `root`.")]
    viewport: Option<String>,
    #[schemars(description = "Agent key from the snapshot.")]
    key: String,
    #[schemars(description = "Number for ranged nodes, text for text inputs.")]
    value: Value,
}

#[derive(Clone, Copy, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
enum Step {
    Increment,
    Decrement,
}

#[derive(Deserialize, schemars::JsonSchema)]
struct StepArgs {
    #[schemars(description = "From `list_viewports`; default `root`.")]
    viewport: Option<String>,
    #[schemars(description = "Agent key of a ranged node.")]
    key: String,
    direction: Step,
}

#[derive(Clone, Copy, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
enum Scroll {
    Up,
    Down,
    Left,
    Right,
    #[schemars(description = "Scrolls the node itself into view.")]
    IntoView,
}

#[derive(Deserialize, schemars::JsonSchema)]
struct ScrollArgs {
    #[schemars(description = "From `list_viewports`; default `root`.")]
    viewport: Option<String>,
    #[schemars(description = "Agent key from the snapshot.")]
    key: String,
    direction: Scroll,
}

type ToolResult = Result<CallToolResult, rmcp::ErrorData>;

fn text(s: impl Into<String>) -> CallToolResult {
    CallToolResult::success(vec![ContentBlock::text(s)])
}

fn failure(s: impl Into<String>) -> CallToolResult {
    CallToolResult::error(vec![ContentBlock::text(s)])
}

fn tree_result(tree: &SemanticTree) -> ToolResult {
    let json =
        serde_json::to_value(tree).map_err(|e| rmcp::ErrorData::internal_error(format!("tree to JSON: {e}"), None))?;
    let mut r = text(to_markdown(tree));
    r.structured_content = Some(json);
    Ok(r)
}

#[tool_router]
impl Tools {
    fn new(host: Arc<dyn Host>) -> Self {
        Self {
            host,
            tool_router: Self::tool_router(),
        }
    }

    fn handle(&self, viewport: Option<&str>) -> Result<AgentHandle, CallToolResult> {
        let ids = self.host.viewports();
        let id = match viewport {
            None | Some("root") => Some(ViewportId::ROOT),
            Some(name) => ids.iter().copied().find(|&id| viewport_name(id) == name),
        };
        if let Some(id) = id {
            return Ok(self.host.agent(id));
        }
        let names: Vec<_> = ids.into_iter().map(viewport_name).collect();
        Err(failure(format!(
            "no viewport `{}`; open: {names:?}",
            viewport.unwrap_or_default()
        )))
    }

    async fn act(&self, viewport: Option<&str>, key: String, action: AgentAction) -> ToolResult {
        let handle = match self.handle(viewport) {
            Ok(h) => h,
            Err(e) => return Ok(e),
        };
        match handle.act_then_snapshot(key, action).await {
            Ok(tree) => Ok(text(to_markdown(&tree))),
            Err(e) => Ok(agent_failure(&e)),
        }
    }

    #[tool(description = "Lists open viewports, `root` first; pass one as `viewport` to the other tools.")]
    fn list_viewports(&self) -> CallToolResult {
        let names: Vec<_> = self.host.viewports().into_iter().map(viewport_name).collect();
        text(names.join("\n"))
    }

    #[tool(
        description = "Semantic tree of the UI as markdown (structured content: nested JSON). Nodes show `[key] Role: name`, state notes and the actions they offer."
    )]
    async fn snapshot(&self, Parameters(args): Parameters<SnapshotArgs>) -> ToolResult {
        let handle = match self.handle(args.viewport.as_deref()) {
            Ok(h) => h,
            Err(e) => return Ok(e),
        };
        let tree = match handle.snapshot().await {
            Ok(t) => t,
            Err(e) => return Ok(agent_failure(&e)),
        };
        match args.root {
            None => tree_result(&tree),
            Some(key) => match tree.subtree(&key) {
                Some(sub) => tree_result(&sub),
                None => Ok(failure(format!("no node with key `{key}`"))),
            },
        }
    }

    #[tool(description = "Clicks the node through its AccessKit Click action.")]
    async fn click(&self, Parameters(a): Parameters<NodeArgs>) -> ToolResult {
        self.act(a.viewport.as_deref(), a.key, AgentAction::Click).await
    }

    #[tool(
        description = "Presses and releases the pointer at the node's center; reaches nodes without a Click action."
    )]
    async fn tap(&self, Parameters(a): Parameters<NodeArgs>) -> ToolResult {
        self.act(a.viewport.as_deref(), a.key, AgentAction::Tap).await
    }

    #[tool(description = "Moves keyboard focus to the node.")]
    async fn focus(&self, Parameters(a): Parameters<NodeArgs>) -> ToolResult {
        self.act(a.viewport.as_deref(), a.key, AgentAction::Focus).await
    }

    #[tool(description = "Focuses the node and types `text` as keyboard text input.")]
    async fn type_text(&self, Parameters(a): Parameters<TextArgs>) -> ToolResult {
        self.act(a.viewport.as_deref(), a.key, AgentAction::Type(a.text)).await
    }

    #[tool(description = "Focuses the node and presses the key `name` with `modifiers`.")]
    async fn press_key(&self, Parameters(a): Parameters<KeyArgs>) -> ToolResult {
        let Some(key) = Key::from_name(&a.name) else {
            return Ok(failure(format!("unknown key `{}`", a.name)));
        };
        let mods = a.modifiers.iter().fold(Modifiers::NONE, |m, &x| {
            m | match x {
                Modifier::Alt => Modifiers::ALT,
                Modifier::Ctrl => Modifiers::CTRL,
                Modifier::Shift => Modifiers::SHIFT,
                Modifier::Command => Modifiers::COMMAND,
            }
        });
        self.act(a.viewport.as_deref(), a.key, AgentAction::Key(key, mods))
            .await
    }

    #[tool(description = "Sets the node's value: a number for sliders and other ranged nodes, text for text inputs.")]
    async fn set_value(&self, Parameters(a): Parameters<ValueArgs>) -> ToolResult {
        let value = match a.value {
            Value::Number(n) => AgentValue::Number(n),
            Value::Text(s) => AgentValue::Text(s),
        };
        self.act(a.viewport.as_deref(), a.key, AgentAction::SetValue(value))
            .await
    }

    #[tool(description = "Increments or decrements a ranged node by its step.")]
    async fn step(&self, Parameters(a): Parameters<StepArgs>) -> ToolResult {
        let action = match a.direction {
            Step::Increment => AgentAction::Increment,
            Step::Decrement => AgentAction::Decrement,
        };
        self.act(a.viewport.as_deref(), a.key, action).await
    }

    #[tool(description = "Scrolls a scrollable node one page, or scrolls the node into view.")]
    async fn scroll(&self, Parameters(a): Parameters<ScrollArgs>) -> ToolResult {
        let action = match a.direction {
            Scroll::Up => AgentAction::ScrollUp,
            Scroll::Down => AgentAction::ScrollDown,
            Scroll::Left => AgentAction::ScrollLeft,
            Scroll::Right => AgentAction::ScrollRight,
            Scroll::IntoView => AgentAction::ScrollIntoView,
        };
        self.act(a.viewport.as_deref(), a.key, action).await
    }
}

fn agent_failure(e: &AgentError) -> CallToolResult {
    failure(e.to_string())
}

#[expect(clippy::unused_async_trait_impl, reason = "`tool_handler` expansion")]
#[tool_handler(router = self.tool_router)]
impl ServerHandler for Tools {
    fn get_info(&self) -> ServerConfig {
        ServerConfig::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(Implementation::new(env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION")))
            .with_instructions(
                "Drives an egui_mineral app. Call `snapshot`, then act on nodes by their `[key]`; each action \
                 returns the updated tree.",
            )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_compares_content_and_length() {
        assert!(same(b"abc", b"abc"));
        assert!(!same(b"abc", b"abx"));
        assert!(!same(b"abc", b"abcd"));
    }

    #[test]
    fn root_viewport_is_named_root() {
        assert_eq!(viewport_name(ViewportId::ROOT), "root");
        assert_ne!(viewport_name(ViewportId::from_hash_of("other")), "root");
    }
}
