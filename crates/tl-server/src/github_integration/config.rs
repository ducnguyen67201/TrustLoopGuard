use std::time::Duration;

use base64::Engine as _;

#[derive(Debug, Clone)]
pub struct GitHubAppConfig {
    pub app_id: i64,
    pub app_slug: String,
    pub client_id: String,
    pub client_secret: String,
    pub private_key_der: Vec<u8>,
    pub webhook_secret: String,
}

impl GitHubAppConfig {
    pub fn from_env() -> Result<Self, String> {
        let app_id = required("TL_GITHUB_APP_ID")?
            .parse::<i64>()
            .map_err(|e| format!("TL_GITHUB_APP_ID must be an integer: {e}"))?;
        let private_key_base64 = required("TL_GITHUB_PRIVATE_KEY_BASE64")?;
        let private_key = base64::engine::general_purpose::STANDARD
            .decode(private_key_base64.as_bytes())
            .map_err(|e| format!("TL_GITHUB_PRIVATE_KEY_BASE64 is not base64: {e}"))?;
        let private_key_der = pem_or_der(private_key)?;
        Ok(Self {
            app_id,
            app_slug: required("TL_GITHUB_APP_SLUG")?,
            client_id: required("TL_GITHUB_CLIENT_ID")?,
            client_secret: required("TL_GITHUB_CLIENT_SECRET")?,
            private_key_der,
            webhook_secret: required("TL_GITHUB_WEBHOOK_SECRET")?,
        })
    }

    pub fn install_url(&self, state: &str) -> String {
        let mut url = url::Url::parse(&format!(
            "https://github.com/apps/{}/installations/new",
            self.app_slug
        ))
        .expect("static GitHub install URL");
        url.query_pairs_mut()
            .append_pair("state", state)
            .append_pair("request_oauth_on_install", "true");
        url.to_string()
    }
}

fn pem_or_der(bytes: Vec<u8>) -> Result<Vec<u8>, String> {
    if !bytes.starts_with(b"-----BEGIN") {
        return Ok(bytes);
    }
    let pem = String::from_utf8(bytes)
        .map_err(|e| format!("TL_GITHUB_PRIVATE_KEY_BASE64 is not UTF-8 PEM: {e}"))?;
    let body = pem
        .lines()
        .filter(|line| !line.starts_with("-----"))
        .collect::<String>();
    base64::engine::general_purpose::STANDARD
        .decode(body.as_bytes())
        .map_err(|e| format!("TL_GITHUB_PRIVATE_KEY_BASE64 PEM body is not base64: {e}"))
}

pub const GITHUB_REQUEST_TIMEOUT: Duration = Duration::from_secs(20);

fn required(name: &str) -> Result<String, String> {
    let value = std::env::var(name).map_err(|_| format!("{name} is not set"))?;
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(format!("{name} is empty"));
    }
    Ok(trimmed.to_string())
}
