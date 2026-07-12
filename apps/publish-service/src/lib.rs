pub mod config;
pub mod error;
pub mod institutions;
pub mod routes;

use std::net::SocketAddr;
use tower_http::cors::CorsLayer;

pub async fn run() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "publish_service=info,tower_http=info".into()),
        )
        .init();

    let config = config::AppConfig::from_env();
    tracing::info!("Loading catalog from: {}", config.catalog_path.display());

    let registry = institutions::Registry::load(&config.catalog_path)?;
    tracing::info!("Loaded {} institutions", registry.institutions.len());

    let app = routes::router(registry)
        .layer(CorsLayer::permissive())
        .layer(axum::middleware::from_fn(request_id_middleware));

    let addr = SocketAddr::from(([0, 0, 0, 0], config.port as u16));
    tracing::info!("Listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

async fn request_id_middleware(
    req: axum::extract::Request,
    next: axum::middleware::Next,
) -> axum::response::Response {
    let request_id = uuid::Uuid::new_v4().to_string();
    let mut response = next.run(req).await;
    response.headers_mut().insert(
        "x-request-id",
        axum::http::HeaderValue::from_str(&request_id).unwrap(),
    );
    response
}
