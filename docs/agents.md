# Agents

Agents drive an app through the same input path as users and screen readers: they read the semantic tree and act on nodes by agent key.

## Agent API

- `Runtime::agent(viewport)` returns an `AgentHandle`, which is cloneable and `Send`. `Runtime::viewports()` lists the viewports, ROOT first.
- `snapshot()` returns the `SemanticTree` of the next pass. Trees are collected on demand, so no setup is needed.
- `act(key, action)` resolves once the input is injected. `act_then_snapshot(key, action)` returns the tree of the pass after that, when the UI has reacted.
- Keys come from the snapshot (`[key]` in the markdown, `key` in the JSON). Widgets default to `<id_salt>-<kind>`, for example `ok-button`.
- Errors: `NotFound` lists similar keys, and `ActionNotAvailable` lists the actions the node offers.

## Snapshot formats

- Markdown, from `semantic::serialize::agent::to_markdown`: one `- [key] Role: name (notes)` line per node, indented by depth, followed by an `Actions:` line.
- JSON, from `serde`: `{"nodes": [{"key", "role", "name", "value"?, "states"?, "level"?, "range"?, "live"?, "actions"?, "shortcut"?, "access_key"?, "active"?, "children"?}]}`. The states use the markdown labels.
- `SemanticTree::subtree(key)` returns a node with its descendants, as a tree rooted at that node.

## Nesting

Nodes nest by egui's widget hierarchy, widget parts and popup owners ([conventions](conventions.md)), so a popup sits under its trigger.

## MCP

`egui_mineral_mcp::Server` serves the agent API as MCP tools over Streamable HTTP at `/mcp`. It is native only, and the app runs it on a Tokio executor with I/O and timers enabled.

- Requests need `Authorization: Bearer <token>`. `Host` must be loopback, and requests with a browser `Origin` are rejected. Bind to `127.0.0.1`.
- Tools:
  - `list_viewports`
  - `snapshot`: takes `viewport` and `root`, and returns markdown plus JSON structured content.
  - `click`, `tap`, `focus`
  - `type_text`
  - `press_key`: an egui key name plus modifiers.
  - `set_value`: a number or text.
  - `step`: increment or decrement.
  - `scroll`: up, down, left, right or into view.
- Every action returns the tree after the UI reacted.

The showcase serves the endpoint when `MINERAL_MCP_TOKEN` is set. The address comes from `MINERAL_MCP_ADDR` and defaults to `127.0.0.1:7878`.

```sh
export MINERAL_MCP_TOKEN=$(head -c 24 /dev/urandom | base64)
cargo run -p showcase-material &
claude mcp add --transport http mineral http://127.0.0.1:7878/mcp --header "Authorization: Bearer $MINERAL_MCP_TOKEN"
```
