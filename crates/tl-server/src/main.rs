use std::sync::Arc;

use axum::{extract::State, http::StatusCode, routing::post, Json, Router};
use tl_core::{CheckRequest, Decision};
use tl_engine::Engine;
use tower_http::trace::TraceLayer;

#[derive(Clone)]
struct AppState {
    engine: Arc<Engine>,
}

async fn check_handler(
    State(state): State<AppState>,
    Json(req): Json<CheckRequest>,
) -> Result<Json<Decision>, StatusCode> {
    let decision = state.engine.check(&req);
    Ok(Json(decision))
}

async fn health() -> &'static str {
    "ok"
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .json()
        .init();

    let state = AppState {
        engine: Arc::new(Engine::empty()),
    };

    let app = Router::new()
        .route("/health", axum::routing::get(health))
        .route("/v1/check", post(check_handler))
        .with_state(state)
        .layer(TraceLayer::new_for_http());

    let addr = "0.0.0.0:8080";
    let listener = tokio::net::TcpListener::bind(addr).await?;
    tracing::info!(addr, "tl-server listening");
    axum::serve(listener, app).await?;
    Ok(())
}
