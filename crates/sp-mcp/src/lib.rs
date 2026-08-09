pub mod config;
pub mod error;
pub mod tools;
pub mod workspace;

use crate::config::Config;
use crate::config::{ServerOptions, TransportMode};
use crate::tools::ScholarPressService;
use anyhow::Result;
use axum::{routing::get, Json, Router};
use rmcp::transport::streamable_http_server::{
    session::local::LocalSessionManager, StreamableHttpServerConfig, StreamableHttpService,
};
use rmcp::{transport::stdio, ServiceExt};
use serde_json::json;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;

pub async fn run(config: Config) -> Result<()> {
    run_with_options(config, ServerOptions::default()).await
}

pub async fn run_with_options(config: Config, options: ServerOptions) -> Result<()> {
    match options.transport {
        TransportMode::Stdio => {
            let service = ScholarPressService::new(config).serve(stdio()).await?;
            service.waiting().await?;
            Ok(())
        }
        TransportMode::Http => run_http(config, options.bind, options.port).await,
    }
}

pub fn http_router(service: ScholarPressService) -> Router {
    let http_config = StreamableHttpServerConfig::default().with_json_response(true);
    let mcp = StreamableHttpService::new(
        move || Ok(service.clone()),
        Arc::new(LocalSessionManager::default()),
        http_config,
    );
    Router::new()
        .route("/health", get(|| async { Json(json!({ "status": "ok" })) }))
        .nest_service("/mcp", mcp)
}

pub async fn run_http(config: Config, bind: std::net::IpAddr, port: u16) -> Result<()> {
    let listener = TcpListener::bind(SocketAddr::from((bind, port))).await?;
    let address = listener.local_addr()?;
    eprintln!("ScholarPress MCP HTTP endpoint: http://{address}/mcp");
    axum::serve(listener, http_router(ScholarPressService::new(config))).await?;
    Ok(())
}
