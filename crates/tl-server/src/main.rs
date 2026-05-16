use tl_server::{build_app_state, router, AuthConfig, BuildOptions};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init_tracing();

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

fn init_tracing() {
    let log_format = std::env::var("TL_LOG_FORMAT").unwrap_or_else(|_| "json".to_string());

    match log_format.as_str() {
        "pretty" | "text" | "compact" => tracing_subscriber::fmt()
            .with_env_filter(env_filter())
            .with_ansi(std::io::IsTerminal::is_terminal(&std::io::stderr()))
            .compact()
            .init(),
        _ => tracing_subscriber::fmt()
            .with_env_filter(env_filter())
            .json()
            .init(),
    }
}

fn env_filter() -> tracing_subscriber::EnvFilter {
    tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"))
}
