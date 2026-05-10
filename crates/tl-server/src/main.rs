use std::sync::Arc;

use tl_engine::Engine;
use tl_server::{router, AppState};

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

    let app = router(state);
    let addr = "0.0.0.0:8080";
    let listener = tokio::net::TcpListener::bind(addr).await?;
    tracing::info!(addr, "tl-server listening");
    axum::serve(listener, app).await?;
    Ok(())
}
