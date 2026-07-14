use axum::{
    body::Bytes,
    extract::State,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use chrono::{DateTime, Utc};
use ring::hmac;
use serde::Deserialize;
use tl_core::{ApiError, ApiErrorCode};

use super::config::GitHubAppConfig;
use super::{store_error_response, GitHubIntegrationState};

pub async fn github_webhook(
    State(state): State<GitHubIntegrationState>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    let config = match GitHubAppConfig::from_env() {
        Ok(config) => config,
        Err(error) => {
            tracing::info!(error = %error, "github webhook ignored because integration is not configured");
            return StatusCode::ACCEPTED.into_response();
        }
    };
    if !valid_signature(&headers, &body, config.webhook_secret.as_bytes()) {
        return (
            StatusCode::UNAUTHORIZED,
            Json(ApiError {
                code: ApiErrorCode::Unauthorized,
                message: "invalid GitHub webhook signature".into(),
                retriable: false,
                details: serde_json::Value::Null,
            }),
        )
            .into_response();
    }
    let event = headers
        .get("x-github-event")
        .and_then(|value| value.to_str().ok())
        .unwrap_or("");
    match event {
        "installation" => handle_installation(state, &body).await,
        "installation_repositories" => StatusCode::ACCEPTED.into_response(),
        "pull_request" => handle_pull_request(state, &body).await,
        _ => StatusCode::ACCEPTED.into_response(),
    }
}

async fn handle_installation(state: GitHubIntegrationState, body: &[u8]) -> Response {
    #[derive(Deserialize)]
    struct InstallationPayload {
        action: String,
        installation: Installation,
    }
    #[derive(Deserialize)]
    struct Installation {
        id: i64,
    }
    let payload = match serde_json::from_slice::<InstallationPayload>(body) {
        Ok(payload) => payload,
        Err(_) => return StatusCode::ACCEPTED.into_response(),
    };
    if matches!(payload.action.as_str(), "deleted" | "suspend") {
        if let Err(error) = state
            .store
            .mark_installation_removed(payload.installation.id)
            .await
        {
            return store_error_response(error);
        }
    }
    StatusCode::ACCEPTED.into_response()
}

async fn handle_pull_request(state: GitHubIntegrationState, body: &[u8]) -> Response {
    #[derive(Deserialize)]
    struct PullRequestPayload {
        action: String,
        repository: Repository,
        number: i64,
        pull_request: PullRequest,
    }
    #[derive(Deserialize)]
    struct Repository {
        id: i64,
    }
    #[derive(Deserialize)]
    struct PullRequest {
        merged: bool,
        merged_at: Option<DateTime<Utc>>,
        closed_at: Option<DateTime<Utc>>,
        head: PullRequestHead,
    }
    #[derive(Deserialize)]
    struct PullRequestHead {
        #[serde(rename = "ref")]
        branch_name: String,
    }

    let payload = match serde_json::from_slice::<PullRequestPayload>(body) {
        Ok(payload) => payload,
        Err(_) => return StatusCode::ACCEPTED.into_response(),
    };
    if payload.action != "closed" {
        return StatusCode::ACCEPTED.into_response();
    }
    let closed_at = payload
        .pull_request
        .merged_at
        .or(payload.pull_request.closed_at)
        .unwrap_or_else(Utc::now);
    if let Err(error) = state
        .store
        .mark_pull_request_closed(
            payload.repository.id,
            payload.number,
            &payload.pull_request.head.branch_name,
            payload.pull_request.merged,
            closed_at,
        )
        .await
    {
        return store_error_response(error);
    }
    StatusCode::ACCEPTED.into_response()
}

fn valid_signature(headers: &HeaderMap, body: &[u8], secret: &[u8]) -> bool {
    let Some(signature) = headers
        .get("x-hub-signature-256")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("sha256="))
    else {
        return false;
    };
    let expected = hmac::sign(&hmac::Key::new(hmac::HMAC_SHA256, secret), body);
    let Ok(provided) = hex_to_bytes(signature) else {
        return false;
    };
    hmac::verify(&hmac::Key::new(hmac::HMAC_SHA256, secret), body, &provided).is_ok()
        && provided.as_slice() == expected.as_ref()
}

fn hex_to_bytes(hex: &str) -> Result<Vec<u8>, ()> {
    if hex.len() % 2 != 0 {
        return Err(());
    }
    let mut out = Vec::with_capacity(hex.len() / 2);
    let bytes = hex.as_bytes();
    for idx in (0..bytes.len()).step_by(2) {
        let hi = from_hex(bytes[idx])?;
        let lo = from_hex(bytes[idx + 1])?;
        out.push((hi << 4) | lo);
    }
    Ok(out)
}

fn from_hex(byte: u8) -> Result<u8, ()> {
    match byte {
        b'0'..=b'9' => Ok(byte - b'0'),
        b'a'..=b'f' => Ok(byte - b'a' + 10),
        b'A'..=b'F' => Ok(byte - b'A' + 10),
        _ => Err(()),
    }
}
