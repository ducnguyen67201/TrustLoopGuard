//! Typed runtime configuration for `tl-server`.
//!
//! Loaded once at startup via figment with the `TL_SERVER_` env prefix.
//! Fail-fast: a missing required value or a malformed entry aborts the
//! process before the listener binds, with a clear error path that
//! names the offending key.
//!
//! Example:
//!     TL_SERVER_LISTEN_ADDR=127.0.0.1:9090 cargo run -p tl-server

use std::net::SocketAddr;
use std::str::FromStr;

use anyhow::{Context, Result};
use figment::providers::Env;
use figment::Figment;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Config {
    /// Address the HTTP listener binds to. Defaults to `0.0.0.0:8080`
    /// so a fresh checkout works without any env wiring.
    #[serde(default = "default_listen_addr")]
    pub listen_addr: String,

    /// Comma-separated list of policy file paths to load on startup.
    /// Empty by default; the engine just runs with no policies.
    #[serde(default)]
    pub policy_paths: Vec<String>,
}

fn default_listen_addr() -> String {
    "0.0.0.0:8080".to_string()
}

impl Config {
    /// Load from process env. Reads keys prefixed with `TL_SERVER_`.
    /// Lowercases and strips the prefix, so `TL_SERVER_LISTEN_ADDR`
    /// maps to the `listen_addr` field.
    pub fn from_env() -> Result<Self> {
        let figment = Figment::new().merge(Env::prefixed("TL_SERVER_").split(","));
        figment
            .extract::<Config>()
            .context("failed to load TL_SERVER_* configuration")
    }

    /// Parse `listen_addr` as a `SocketAddr`. Errors carry the field
    /// name so a typo ("locahost:8080") is obvious.
    pub fn socket_addr(&self) -> Result<SocketAddr> {
        SocketAddr::from_str(&self.listen_addr).with_context(|| {
            format!(
                "TL_SERVER_LISTEN_ADDR is not a valid socket address: {}",
                self.listen_addr
            )
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use figment::providers::Serialized;
    use figment::Figment;

    fn config_from(map: &[(&str, &str)]) -> Result<Config> {
        let serialized: serde_json::Value = serde_json::Value::Object(
            map.iter()
                .map(|(k, v)| {
                    (
                        (*k).to_string(),
                        serde_json::Value::String((*v).to_string()),
                    )
                })
                .collect(),
        );
        Figment::new()
            .merge(Serialized::defaults(serialized))
            .extract::<Config>()
            .context("test config extract")
    }

    #[test]
    fn defaults_to_local_listener() {
        let cfg: Config = Figment::new().extract().unwrap();
        assert_eq!(cfg.listen_addr, "0.0.0.0:8080");
        assert!(cfg.policy_paths.is_empty());
    }

    #[test]
    fn parses_listen_addr_override() {
        let cfg = config_from(&[("listen_addr", "127.0.0.1:9090")]).unwrap();
        assert_eq!(cfg.socket_addr().unwrap().to_string(), "127.0.0.1:9090");
    }

    #[test]
    fn rejects_invalid_socket_addr() {
        let cfg = config_from(&[("listen_addr", "locahost:8080")]).unwrap();
        let err = cfg.socket_addr().unwrap_err();
        assert!(err.to_string().contains("TL_SERVER_LISTEN_ADDR"));
    }
}
