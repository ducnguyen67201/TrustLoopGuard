use tl_server::{build_app_state, router, AuthConfig, BuildOptions};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .json()
        .init();

    let state = build_app_state(BuildOptions::default()).await?;

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

    let app = router(state, auth);
    let addr = "0.0.0.0:8080";
    let listener = tokio::net::TcpListener::bind(addr).await?;
    tracing::info!(addr, "tl-server listening");
    axum::serve(listener, app).await?;
    Ok(())
}
