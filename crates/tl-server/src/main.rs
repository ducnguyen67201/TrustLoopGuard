use tl_server::{build_app_state, build_seal_key, router, AuthConfig, BuildOptions};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init_tracing();

    // Local development may omit TL_API_KEY only while listening on loopback.
    // A non-loopback listener without authentication would expose the entire
    // /v1 control and runtime surface to the surrounding network.
    let auth = match AuthConfig::from_env() {
        Ok(cfg) => Some(cfg),
        Err(e) => {
            tracing::warn!(
                error = %e,
                "TL_API_KEY not configured — only a loopback listener is permitted"
            );
            None
        }
    };

    let addr = std::env::var("TL_SERVER_ADDR").unwrap_or_else(|_| "127.0.0.1:8080".to_string());
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    validate_listener_auth(listener.local_addr()?, auth.is_some())?;

    let state = build_app_state(BuildOptions::default()).await?;
    let app = router(state, auth, build_seal_key());
    tracing::info!(addr, "tl-server listening");
    axum::serve(listener, app).await?;
    Ok(())
}

fn validate_listener_auth(
    addr: std::net::SocketAddr,
    authentication_enabled: bool,
) -> anyhow::Result<()> {
    if !authentication_enabled && !addr.ip().is_loopback() {
        anyhow::bail!(
            "refusing unauthenticated non-loopback listener {addr}; set TL_API_KEY or bind \
             TL_SERVER_ADDR to 127.0.0.1/[::1]"
        );
    }
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

#[cfg(test)]
mod tests {
    use super::validate_listener_auth;

    #[test]
    fn unauthenticated_listener_is_limited_to_loopback() {
        assert!(validate_listener_auth("127.0.0.1:8080".parse().unwrap(), false).is_ok());
        assert!(validate_listener_auth("[::1]:8080".parse().unwrap(), false).is_ok());
        assert!(validate_listener_auth("0.0.0.0:8080".parse().unwrap(), false).is_err());
        assert!(validate_listener_auth("[::]:8080".parse().unwrap(), false).is_err());
    }

    #[test]
    fn authenticated_listener_may_bind_non_loopback() {
        assert!(validate_listener_auth("0.0.0.0:8080".parse().unwrap(), true).is_ok());
    }
}
