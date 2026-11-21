//! The server over HTTP against a runtime stepped on the test thread.

use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use egui::{Context, RawInput, Rect, pos2};
use egui_mineral::{
    i18n::Catalog,
    policy::Policies,
    runtime::{Runtime, Stylekit},
    semantic::{Action, Role, SemanticNode, StableId, emit_in},
};
use egui_mineral_mcp::Server;
use tokio::io::{AsyncReadExt as _, AsyncWriteExt as _};

const TOKEN: &str = "test-token";

struct Kit;

impl Stylekit for Kit {
    fn policies(&self, _: &Context) -> Policies {
        Policies::baseline()
    }
    fn catalogs(&self) -> &'static [Catalog] {
        &[]
    }
    fn setup(&self, _: &Context) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        Ok(())
    }
    fn style(&self, ctx: &Context, theme: egui::Theme) -> Arc<egui::Style> {
        ctx.style_of(theme)
    }
}

async fn post(addr: std::net::SocketAddr, token: Option<&str>, body: &str) -> String {
    let mut s = tokio::net::TcpStream::connect(addr).await.expect("connects");
    let auth = token.map_or_default(|t| format!("Authorization: Bearer {t}\r\n"));
    let req = format!(
        "POST /mcp HTTP/1.1\r\nHost: 127.0.0.1\r\n{auth}Content-Type: application/json\r\n\
         Accept: application/json, text/event-stream\r\nConnection: close\r\nContent-Length: {}\r\n\r\n{body}",
        body.len()
    );
    s.write_all(req.as_bytes()).await.expect("writes");
    let mut out = String::new();
    s.read_to_string(&mut out).await.expect("reads");
    out
}

fn call(id: u32, tool: &str, args: &str) -> String {
    format!(r#"{{"jsonrpc":"2.0","id":{id},"method":"tools/call","params":{{"name":"{tool}","arguments":{args}}}}}"#)
}

#[test]
fn serves_tools_behind_the_token() {
    let ctx = Context::default();
    let rt = Runtime::new(&ctx, Kit).expect("runtime");
    let server = Server::new(Arc::clone(&rt), TOKEN).expect("token");
    let client = std::thread::spawn(move || {
        let exec = tokio::runtime::Builder::new_current_thread()
            .enable_io()
            .enable_time()
            .build()
            .expect("executor");
        exec.block_on(async move {
            let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.expect("binds");
            let addr = listener.local_addr().expect("address");
            tokio::spawn(server.serve(listener));
            let list = r#"{"jsonrpc":"2.0","id":1,"method":"tools/list"}"#;
            [
                post(addr, None, list).await,
                post(addr, Some("wrong"), list).await,
                post(addr, Some(TOKEN), list).await,
                post(addr, Some(TOKEN), &call(2, "snapshot", "{}")).await,
                post(addr, Some(TOKEN), &call(3, "click", r#"{"key":"hello"}"#)).await,
            ]
        })
    });
    let deadline = Instant::now() + Duration::from_secs(20);
    while !client.is_finished() && Instant::now() < deadline {
        let mut output = ctx.run_ui(RawInput::default(), |ui| {
            rt.pass(ui, |ui| {
                emit_in(
                    ui,
                    None,
                    Rect::from_min_size(pos2(0.0, 0.0), egui::vec2(10.0, 10.0)),
                    || {
                        let mut n = SemanticNode::new(StableId::new("hello"), Role::Label, "Hello");
                        n.actions = vec![Action::Click];
                        n
                    },
                );
            });
        });
        output.textures_delta.clear();
        std::thread::sleep(Duration::from_millis(2));
    }
    assert!(client.is_finished(), "client still running");
    let [none, wrong, tools, snapshot, click] = client.join().expect("client thread");
    assert!(none.starts_with("HTTP/1.1 401"), "{none}");
    assert!(wrong.starts_with("HTTP/1.1 401"), "{wrong}");
    assert!(
        tools.contains(r#""name":"snapshot""#) && tools.contains(r#""name":"press_key""#),
        "{tools}"
    );
    assert!(snapshot.contains("[hello] Label: Hello"), "{snapshot}");
    assert!(
        snapshot.contains(r#""structuredContent":{"nodes":[{"actions":["Click"],"key":"hello""#),
        "{snapshot}"
    );
    assert!(
        click.contains(r#""isError":true"#) && click.contains("no egui widget id"),
        "{click}"
    );
}
