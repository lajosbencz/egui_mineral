# egui_mineral_mcp

[docs.rs](https://docs.rs/egui_mineral_mcp) | [crates.io](https://crates.io/crates/egui_mineral_mcp) | [repository](https://github.com/lajosbencz/egui_mineral)

<!-- cargo-rdme start -->

[MCP](https://modelcontextprotocol.io) server for [egui_mineral](https://crates.io/crates/egui_mineral) apps.

Agents read the semantic tree of a `Runtime` and act on its nodes by agent key, over Streamable HTTP at
`/mcp`. Requests need `Authorization: Bearer <token>`; `Host` must be loopback and browser `Origin`s are
rejected. The app runs `Server::serve` on its own Tokio executor, with I/O and timers enabled; each request collects the tree it
needs on the next pass.

```rust
let token = std::env::var("MINERAL_MCP_TOKEN")?;
let listener = tokio::net::TcpListener::bind("127.0.0.1:7878").await?;
egui_mineral_mcp::Server::new(rt, token)?.serve(listener).await?;
```

Tools: `list_viewports`, `snapshot`, `click`, `tap`, `focus`, `type_text`, `press_key`, `set_value`,
`step`, `scroll`. Every action returns the tree after the UI reacted, as markdown.

<!-- cargo-rdme end -->

License: [GPL-3.0-or-later](https://github.com/lajosbencz/egui_mineral/blob/master/LICENSE)
