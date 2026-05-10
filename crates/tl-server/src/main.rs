use std::sync::Arc;

use tl_engine::Engine;
use tl_server::{router, AppState, AuthConfig, MemoryAgentStore};

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

    // TL_API_KEY is the production gate. Local dev can omit it; the
    // server logs a warning and serves /v1/* without auth. We do NOT
    // silently default to a constant — that would shadow forgotten
    // production configs.
    let auth = match AuthConfig::from_env() {
        Ok(cfg) => Some(cfg),
        Err(e) => {
            tracing::warn!(
                error = %e,
                "TL_API_KEY not configured — /v1/* endpoints are UNAUTHENTICATED"
            );
            None
        }
    };

    // Default to MemoryAgentStore for the v0 dev path. PR 15 swaps this
    // for an adapter over tl_storage::AgentRepo (Postgres-backed) once
    // the server boots a connection pool.
    let agents: std::sync::Arc<dyn tl_server::AgentStore> =
        std::sync::Arc::new(MemoryAgentStore::new());

    let app = router(state, auth, Some(agents));
    let addr = "0.0.0.0:8080";
    let listener = tokio::net::TcpListener::bind(addr).await?;
    tracing::info!(addr, "tl-server listening");
    axum::serve(listener, app).await?;
    Ok(())
}
