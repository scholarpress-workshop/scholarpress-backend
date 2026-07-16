pub mod check;
pub mod compile;
pub mod extract;
pub mod institutions;
pub mod spec;
pub mod template;

use crate::institutions::Registry;
use axum::{
    extract::DefaultBodyLimit,
    routing::{get, post},
    Router,
};

pub fn router(registry: Registry) -> Router {
    Router::new()
        .route("/extract", post(extract::handler))
        .route("/compile", post(compile::handler))
        .route(
            "/check",
            post(check::handler).layer(DefaultBodyLimit::max(50 * 1024 * 1024)),
        )
        .route("/health", get(|| async { "ok" }))
        .route("/institutions", get(institutions::handler))
        .route("/institutions/:id/spec", get(spec::handler))
        .route("/institutions/:id/template", get(template::handler))
        .with_state(registry)
}
