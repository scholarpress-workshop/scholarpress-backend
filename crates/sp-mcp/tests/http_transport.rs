use axum::Router;
use reqwest::Client;
use sp_mcp::{config::Config, http_router, tools::ScholarPressService};
use std::net::SocketAddr;
use tokio::net::TcpListener;

async fn spawn_http_server() -> (String, tokio::task::JoinHandle<()>) {
    let root = tempfile::tempdir().unwrap();
    let catalog = root.path().join("catalog");
    let workspaces = root.path().join("workspaces");
    std::fs::create_dir_all(&catalog).unwrap();
    std::fs::create_dir_all(&workspaces).unwrap();
    let config = Config::new(catalog, workspaces);
    let router: Router = http_router(ScholarPressService::new(config));
    let listener = TcpListener::bind(SocketAddr::from(([127, 0, 0, 1], 0)))
        .await
        .unwrap();
    let address = listener.local_addr().unwrap();
    let task = tokio::spawn(async move {
        let _root = root;
        axum::serve(listener, router).await.unwrap();
    });
    (format!("http://{address}"), task)
}

#[tokio::test]
async fn http_transport_accepts_mcp_initialize() {
    let (base, task) = spawn_http_server().await;
    let response = Client::new()
        .post(format!("{base}/mcp"))
        .header("content-type", "application/json")
        .header("accept", "application/json, text/event-stream")
        .header("MCP-Protocol-Version", "2025-11-25")
        .body(r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"test","version":"0"}}}"#)
        .send()
        .await
        .unwrap();
    assert!(response.status().is_success());
    let session_id = response
        .headers()
        .get("Mcp-Session-Id")
        .unwrap()
        .to_str()
        .unwrap()
        .to_owned();

    let tools = Client::new()
        .post(format!("{base}/mcp"))
        .header("content-type", "application/json")
        .header("accept", "application/json, text/event-stream")
        .header("MCP-Protocol-Version", "2025-11-25")
        .header("Mcp-Session-Id", session_id)
        .body(r#"{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}"#)
        .send()
        .await
        .unwrap();
    assert!(tools.status().is_success());
    task.abort();
}

#[tokio::test]
async fn health_endpoint_reports_ready() {
    let (base, task) = spawn_http_server().await;
    let response = Client::new()
        .get(format!("{base}/health"))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), reqwest::StatusCode::OK);
    assert_eq!(response.text().await.unwrap(), r#"{"status":"ok"}"#);
    task.abort();
}
