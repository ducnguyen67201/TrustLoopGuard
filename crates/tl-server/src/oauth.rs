//! OAuth 2.1 authorization server for dashboard-approved MCP clients.

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Form, Json, Router};
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use chrono::Utc;
use serde::Deserialize;
use serde_json::{json, Map, Value};
use tl_core::{AuthorizeRequest, AuthorizeResponse};
use uuid::Uuid;

use crate::jwt::ACCESS_TOKEN_TTL_MINUTES;
use crate::oauth_store::{
    expires_after_seconds, hash_opaque_token, OAuthAuthorizationCodeRecord, OAuthClientRecord,
    OAuthRefreshTokenRecord, OAuthStoreError,
};
use crate::AppState;

const AUTH_CODE_TTL_SECONDS: i64 = 60;
const REFRESH_TTL_SECONDS: i64 = 30 * 86_400;
const MAX_CLIENTS: usize = 10_000;
const MAX_REDIRECT_URIS: usize = 10;
const MAX_REDIRECT_URI_LEN: usize = 2048;
const MAX_CLIENT_NAME_LEN: usize = 100;
const CLIENT_INACTIVE_TTL_DAYS: i64 = 30;
const REGISTRATION_BURST: u32 = 20;
const REGISTRATION_WINDOW: Duration = Duration::from_secs(60);
pub const MCP_SCOPE: &str = "mcp:tools";

#[derive(Clone)]
struct OAuthPublicState {
    app: AppState,
    registration_limiter: Arc<RegistrationLimiter>,
}

struct RegistrationWindow {
    started_at: Instant,
    used: u32,
}

struct RegistrationLimiter {
    limit: u32,
    window: Duration,
    current: Mutex<RegistrationWindow>,
}

impl RegistrationLimiter {
    fn new(limit: u32, window: Duration) -> Self {
        Self {
            limit,
            window,
            current: Mutex::new(RegistrationWindow {
                started_at: Instant::now(),
                used: 0,
            }),
        }
    }

    fn try_acquire(&self) -> bool {
        let now = Instant::now();
        let mut current = self
            .current
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        if now.duration_since(current.started_at) >= self.window {
            current.started_at = now;
            current.used = 0;
        }
        if current.used >= self.limit {
            return false;
        }
        current.used += 1;
        true
    }
}

fn redirect_uri_acceptable(uri: &str) -> bool {
    if uri.is_empty() || uri.len() > MAX_REDIRECT_URI_LEN {
        return false;
    }
    match uri.parse::<url::Url>() {
        Ok(parsed) => match parsed.scheme() {
            "https" => true,
            "http" => matches!(
                parsed.host_str(),
                Some("localhost") | Some("127.0.0.1") | Some("::1")
            ),
            scheme => !scheme.is_empty() && scheme != "javascript" && scheme != "data",
        },
        Err(_) => false,
    }
}

pub fn issuer() -> String {
    std::env::var("TL_PUBLIC_URL")
        .unwrap_or_else(|_| "http://localhost:8080".to_string())
        .trim_end_matches('/')
        .to_string()
}

pub fn mcp_resource_url() -> String {
    format!("{}/mcp", issuer())
}

fn authorization_endpoint_for(issuer: &str, dashboard_url: Option<&str>) -> String {
    let origin = dashboard_url
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(issuer)
        .trim_end_matches('/');
    format!("{origin}/oauth/authorize")
}

fn authorization_endpoint() -> String {
    let issuer = issuer();
    authorization_endpoint_for(&issuer, std::env::var("TL_DASHBOARD_URL").ok().as_deref())
}

fn random_token() -> String {
    use rand::Rng;
    let mut bytes = [0u8; 32];
    rand::rng().fill_bytes(&mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct OAuthBinding {
    resource: String,
    scope: String,
    hosted_mcp: bool,
}

fn resolve_binding(
    resource: Option<&str>,
    scope: Option<&str>,
) -> Result<OAuthBinding, &'static str> {
    let requested_resource = resource.map(str::trim).filter(|value| !value.is_empty());
    let requested_scope = scope.map(str::trim).filter(|value| !value.is_empty());
    let mcp_resource = mcp_resource_url();
    match (requested_resource, requested_scope) {
        (None, None) => Ok(OAuthBinding {
            resource: issuer(),
            scope: String::new(),
            hosted_mcp: false,
        }),
        (Some(value), None) if value == mcp_resource => Ok(OAuthBinding {
            resource: mcp_resource,
            scope: MCP_SCOPE.to_string(),
            hosted_mcp: true,
        }),
        (None, Some(MCP_SCOPE)) => Ok(OAuthBinding {
            resource: mcp_resource,
            scope: MCP_SCOPE.to_string(),
            hosted_mcp: true,
        }),
        (Some(value), Some(MCP_SCOPE)) if value == mcp_resource => Ok(OAuthBinding {
            resource: mcp_resource,
            scope: MCP_SCOPE.to_string(),
            hosted_mcp: true,
        }),
        _ => Err("unsupported resource or scope"),
    }
}

fn oauth_error(status: StatusCode, code: &str, description: &str) -> Response {
    (
        status,
        Json(json!({ "error": code, "error_description": description })),
    )
        .into_response()
}

async fn authorization_server_metadata() -> Response {
    let issuer = issuer();
    Json(json!({
        "issuer": issuer,
        "authorization_endpoint": authorization_endpoint(),
        "token_endpoint": format!("{issuer}/oauth/token"),
        "registration_endpoint": format!("{issuer}/oauth/register"),
        "response_types_supported": ["code"],
        "grant_types_supported": ["authorization_code", "refresh_token"],
        "code_challenge_methods_supported": ["S256"],
        "token_endpoint_auth_methods_supported": ["none"],
        "scopes_supported": [MCP_SCOPE],
    }))
    .into_response()
}

async fn protected_resource_metadata() -> Response {
    let issuer = issuer();
    Json(json!({
        "resource": issuer,
        "authorization_servers": [issuer],
    }))
    .into_response()
}

async fn mcp_protected_resource_metadata() -> Response {
    let issuer = issuer();
    Json(json!({
        "resource": format!("{issuer}/mcp"),
        "authorization_servers": [issuer],
        "scopes_supported": [MCP_SCOPE],
    }))
    .into_response()
}

#[derive(Deserialize)]
struct RegisterRequest {
    #[serde(default)]
    redirect_uris: Vec<String>,
    #[serde(default)]
    client_name: Option<String>,
}

async fn register(
    State(state): State<OAuthPublicState>,
    Json(req): Json<RegisterRequest>,
) -> Response {
    if !state.registration_limiter.try_acquire() {
        return oauth_error(
            StatusCode::TOO_MANY_REQUESTS,
            "temporarily_unavailable",
            "client registration rate limit exceeded",
        );
    }
    let app = &state.app;
    if req.redirect_uris.is_empty() || req.redirect_uris.len() > MAX_REDIRECT_URIS {
        return oauth_error(
            StatusCode::BAD_REQUEST,
            "invalid_redirect_uri",
            "between 1 and 10 redirect_uris are required",
        );
    }
    if !req
        .redirect_uris
        .iter()
        .all(|uri| redirect_uri_acceptable(uri))
    {
        return oauth_error(
            StatusCode::BAD_REQUEST,
            "invalid_redirect_uri",
            "redirect_uris must be https, loopback http, or a custom scheme",
        );
    }
    let client_name = req
        .client_name
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    if client_name
        .as_ref()
        .is_some_and(|value| value.len() > MAX_CLIENT_NAME_LEN)
    {
        return oauth_error(
            StatusCode::BAD_REQUEST,
            "invalid_client_metadata",
            "client_name is too long",
        );
    }
    if let Err(error) = app
        .oauth_store
        .prune_inactive_clients(Utc::now() - chrono::Duration::days(CLIENT_INACTIVE_TTL_DAYS))
        .await
    {
        tracing::warn!(error = %error, "oauth inactive client pruning failed");
    }
    let client_id = format!("mcp_{}", random_token());
    let client = OAuthClientRecord {
        client_id: client_id.clone(),
        client_name: client_name.clone(),
        redirect_uris: req.redirect_uris.clone(),
    };
    if let Err(error) = app
        .oauth_store
        .create_client_bounded(client, MAX_CLIENTS)
        .await
    {
        return match error {
            OAuthStoreError::Capacity => oauth_error(
                StatusCode::SERVICE_UNAVAILABLE,
                "temporarily_unavailable",
                "client registration capacity reached",
            ),
            error => {
                tracing::error!(error = %error, "oauth client registration failed");
                oauth_error(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "server_error",
                    "client registration failed",
                )
            }
        };
    }
    (
        StatusCode::CREATED,
        Json(json!({
            "client_id": client_id,
            "client_name": client_name,
            "redirect_uris": req.redirect_uris,
            "token_endpoint_auth_method": "none",
            "grant_types": ["authorization_code", "refresh_token"],
        })),
    )
        .into_response()
}

async fn client_redirect_uris(
    State(state): State<OAuthPublicState>,
    axum::extract::Path(client_id): axum::extract::Path<String>,
) -> Response {
    let app = &state.app;
    match app.oauth_store.get_client(&client_id).await {
        Ok(client) => Json(json!({
            "client_name": client.client_name,
            "redirect_uris": client.redirect_uris,
        }))
        .into_response(),
        Err(OAuthStoreError::NotFound) => {
            oauth_error(StatusCode::NOT_FOUND, "invalid_client", "unknown client")
        }
        Err(error) => {
            tracing::error!(error = %error, "oauth client lookup failed");
            oauth_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "server_error",
                "client lookup failed",
            )
        }
    }
}

/// `POST /v1/oauth/authorize` — mint a PKCE-bound code for a trusted dashboard identity.
#[utoipa::path(
    post,
    path = "/v1/oauth/authorize",
    tag = "oauth",
    request_body = AuthorizeRequest,
    responses(
        (status = 200, description = "Authorization code issued", body = AuthorizeResponse),
        (status = 400, description = "Invalid OAuth request"),
        (status = 401, description = "Internal dashboard authentication required"),
        (status = 403, description = "User is not a workspace member"),
        (status = 500, description = "Authorization code issuance failed"),
    ),
)]
pub(crate) async fn authorize(
    State(app): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<AuthorizeRequest>,
) -> Response {
    if req.code_challenge_method.as_deref().unwrap_or("S256") != "S256" {
        return oauth_error(
            StatusCode::BAD_REQUEST,
            "invalid_request",
            "only the S256 code_challenge_method is supported",
        );
    }
    let binding = match resolve_binding(req.resource.as_deref(), req.scope.as_deref()) {
        Ok(binding) => binding,
        Err(description) => {
            return oauth_error(StatusCode::BAD_REQUEST, "invalid_target", description)
        }
    };
    let header = |name: &str| {
        headers
            .get(name)
            .and_then(|value| value.to_str().ok())
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string)
    };
    let (Some(user_id_raw), Some(workspace_id)) = (
        header("x-featherlane-ai-user-id"),
        header("x-featherlane-ai-workspace-id"),
    ) else {
        return oauth_error(
            StatusCode::UNAUTHORIZED,
            "access_denied",
            "missing forwarded user or workspace identity",
        );
    };
    let Ok(user_id) = Uuid::parse_str(&user_id_raw) else {
        return oauth_error(StatusCode::BAD_REQUEST, "access_denied", "bad user id");
    };
    let client = match app.oauth_store.get_client(&req.client_id).await {
        Ok(client) => client,
        Err(_) => {
            return oauth_error(
                StatusCode::BAD_REQUEST,
                "invalid_request",
                "unknown client_id or redirect_uri not registered",
            )
        }
    };
    if !client
        .redirect_uris
        .iter()
        .any(|uri| uri == &req.redirect_uri)
    {
        return oauth_error(
            StatusCode::BAD_REQUEST,
            "invalid_request",
            "unknown client_id or redirect_uri not registered",
        );
    }
    let members = match app.team_store.list_members(&workspace_id).await {
        Ok(members) => members,
        Err(error) => {
            tracing::error!(error = %error, "oauth membership lookup failed");
            return oauth_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "server_error",
                "membership lookup failed",
            );
        }
    };
    let Some(member) = members
        .into_iter()
        .find(|member| member.user_id == user_id.to_string())
    else {
        return oauth_error(
            StatusCode::FORBIDDEN,
            "access_denied",
            "user is not a member of the selected workspace",
        );
    };
    let agent_id = if binding.hosted_mcp {
        let Some(agent_id) = req
            .agent_id
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty() && value.len() <= 200)
        else {
            return oauth_error(
                StatusCode::BAD_REQUEST,
                "invalid_request",
                "agent_id is required for hosted MCP access",
            );
        };
        match app.agent_store.get(&workspace_id, agent_id).await {
            Ok(_) => Some(agent_id.to_string()),
            Err(crate::agents::AgentStoreError::NotFound) => {
                return oauth_error(
                    StatusCode::FORBIDDEN,
                    "access_denied",
                    "the selected agent is not available in this workspace",
                )
            }
            Err(error) => {
                tracing::error!(workspace_id, error = %error, "oauth agent lookup failed");
                return oauth_error(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "server_error",
                    "agent lookup failed",
                );
            }
        }
    } else {
        None
    };
    let code = random_token();
    let record = OAuthAuthorizationCodeRecord {
        client_id: req.client_id,
        redirect_uri: req.redirect_uri,
        user_id,
        username: member.username,
        workspace_id,
        agent_id,
        resource: binding.resource,
        scope: binding.scope,
        code_challenge: req.code_challenge,
        expires_at: expires_after_seconds(AUTH_CODE_TTL_SECONDS),
    };
    if let Err(error) = app
        .oauth_store
        .put_code(hash_opaque_token(&code), record)
        .await
    {
        tracing::error!(error = %error, "oauth code persistence failed");
        return oauth_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "server_error",
            "authorization code issuance failed",
        );
    }
    Json(AuthorizeResponse { code }).into_response()
}

#[derive(Deserialize)]
struct TokenForm {
    grant_type: String,
    code: Option<String>,
    code_verifier: Option<String>,
    redirect_uri: Option<String>,
    client_id: Option<String>,
    refresh_token: Option<String>,
    resource: Option<String>,
    scope: Option<String>,
}

fn verify_pkce_s256(verifier: &str, challenge: &str) -> bool {
    use sha2::{Digest, Sha256};
    URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes())) == challenge
}

async fn token(State(state): State<OAuthPublicState>, Form(form): Form<TokenForm>) -> Response {
    let app = state.app;
    let Some(signer) = app.jwt_signer.as_ref() else {
        return oauth_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "server_error",
            "token signing is not configured",
        );
    };
    match form.grant_type.as_str() {
        "authorization_code" => {
            let (Some(code), Some(verifier), Some(redirect_uri), Some(client_id)) = (
                form.code,
                form.code_verifier,
                form.redirect_uri,
                form.client_id,
            ) else {
                return oauth_error(StatusCode::BAD_REQUEST, "invalid_request", "missing fields");
            };
            let invalid = || {
                oauth_error(
                    StatusCode::BAD_REQUEST,
                    "invalid_grant",
                    "invalid authorization code",
                )
            };
            let entry = match app.oauth_store.take_code(&hash_opaque_token(&code)).await {
                Ok(entry) => entry,
                Err(_) => return invalid(),
            };
            if entry.expires_at < Utc::now()
                || entry.client_id != client_id
                || entry.redirect_uri != redirect_uri
                || !verify_pkce_s256(&verifier, &entry.code_challenge)
                || form
                    .resource
                    .as_deref()
                    .is_some_and(|value| value != entry.resource)
                || form
                    .scope
                    .as_deref()
                    .is_some_and(|value| value != entry.scope)
            {
                return invalid();
            }
            let client = match app.oauth_store.get_client(&client_id).await {
                Ok(client) => client,
                Err(_) => return invalid(),
            };
            if !client.redirect_uris.iter().any(|uri| uri == &redirect_uri) {
                return invalid();
            }
            issue_tokens(
                &app,
                signer,
                &client_id,
                entry.user_id,
                &entry.username,
                &entry.workspace_id,
                entry.agent_id.as_deref(),
                &entry.resource,
                &entry.scope,
            )
            .await
        }
        "refresh_token" => {
            let (Some(refresh), Some(client_id)) = (form.refresh_token, form.client_id) else {
                return oauth_error(
                    StatusCode::BAD_REQUEST,
                    "invalid_request",
                    "missing refresh_token or client_id",
                );
            };
            let invalid = || {
                oauth_error(
                    StatusCode::BAD_REQUEST,
                    "invalid_grant",
                    "invalid refresh token",
                )
            };
            let entry = match app
                .oauth_store
                .take_refresh(&hash_opaque_token(&refresh))
                .await
            {
                Ok(entry) => entry,
                Err(_) => return invalid(),
            };
            if entry.expires_at < Utc::now()
                || entry.client_id != client_id
                || form
                    .resource
                    .as_deref()
                    .is_some_and(|value| value != entry.resource)
                || form
                    .scope
                    .as_deref()
                    .is_some_and(|value| value != entry.scope)
            {
                return invalid();
            }
            issue_tokens(
                &app,
                signer,
                &client_id,
                entry.user_id,
                &entry.username,
                &entry.workspace_id,
                entry.agent_id.as_deref(),
                &entry.resource,
                &entry.scope,
            )
            .await
        }
        other => oauth_error(
            StatusCode::BAD_REQUEST,
            "unsupported_grant_type",
            &format!("unsupported grant_type: {other}"),
        ),
    }
}

#[allow(clippy::too_many_arguments)]
async fn issue_tokens(
    app: &AppState,
    signer: &crate::jwt::JwtSigner,
    client_id: &str,
    user_id: Uuid,
    username: &str,
    workspace_id: &str,
    agent_id: Option<&str>,
    resource: &str,
    scope: &str,
) -> Response {
    let access = if resource == mcp_resource_url() && scope == MCP_SCOPE {
        let Some(agent_id) = agent_id else {
            return oauth_error(
                StatusCode::BAD_REQUEST,
                "invalid_grant",
                "hosted MCP authorization must be renewed",
            );
        };
        if let Err(error) = app.agent_store.get(workspace_id, agent_id).await {
            if !matches!(error, crate::agents::AgentStoreError::NotFound) {
                tracing::error!(workspace_id, agent_id, error = %error, "oauth agent revalidation failed");
            }
            return oauth_error(
                StatusCode::BAD_REQUEST,
                "invalid_grant",
                "hosted MCP authorization must be renewed",
            );
        }
        signer.mint_mcp_access_token(
            user_id,
            username,
            workspace_id,
            agent_id,
            &issuer(),
            resource,
            client_id,
            scope,
        )
    } else {
        signer.mint_access_token(user_id, username, workspace_id)
    };
    let access = match access {
        Ok(token) => token,
        Err(error) => {
            tracing::error!(error = %error, "oauth access token mint failed");
            return oauth_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "server_error",
                "token mint failed",
            );
        }
    };
    let refresh = random_token();
    let record = OAuthRefreshTokenRecord {
        client_id: client_id.to_string(),
        user_id,
        username: username.to_string(),
        workspace_id: workspace_id.to_string(),
        agent_id: agent_id.map(str::to_string),
        resource: resource.to_string(),
        scope: scope.to_string(),
        expires_at: expires_after_seconds(REFRESH_TTL_SECONDS),
    };
    if let Err(error) = app
        .oauth_store
        .put_refresh(hash_opaque_token(&refresh), record)
        .await
    {
        tracing::error!(error = %error, "oauth refresh persistence failed");
        return oauth_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "server_error",
            "token mint failed",
        );
    }
    let mut body = Map::from_iter([
        ("access_token".to_string(), Value::String(access)),
        (
            "token_type".to_string(),
            Value::String("Bearer".to_string()),
        ),
        (
            "expires_in".to_string(),
            Value::Number((ACCESS_TOKEN_TTL_MINUTES * 60).into()),
        ),
        ("refresh_token".to_string(), Value::String(refresh)),
        ("resource".to_string(), Value::String(resource.to_string())),
    ]);
    if !scope.is_empty() {
        body.insert("scope".to_string(), Value::String(scope.to_string()));
    }
    Json(Value::Object(body)).into_response()
}

pub fn oauth_public_routes(app: AppState) -> Router {
    let state = OAuthPublicState {
        app,
        registration_limiter: Arc::new(RegistrationLimiter::new(
            REGISTRATION_BURST,
            REGISTRATION_WINDOW,
        )),
    };
    Router::new()
        .route(
            "/.well-known/oauth-authorization-server",
            get(authorization_server_metadata),
        )
        .route(
            "/.well-known/oauth-protected-resource",
            get(protected_resource_metadata),
        )
        .route(
            "/.well-known/oauth-protected-resource/mcp",
            get(mcp_protected_resource_metadata),
        )
        .route(
            "/oauth/register",
            post(register).layer(axum::extract::DefaultBodyLimit::max(16 * 1024)),
        )
        .route(
            "/oauth/token",
            post(token).layer(axum::extract::DefaultBodyLimit::max(16 * 1024)),
        )
        .route(
            "/oauth/clients/{client_id}/redirect-uris",
            get(client_redirect_uris),
        )
        .with_state(state)
}

pub fn oauth_protected_routes(app: AppState) -> Router {
    Router::new()
        .route("/v1/oauth/authorize", post(authorize))
        .with_state(app)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn pkce_s256_matches_known_vector() {
        let verifier = "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk";
        let challenge = "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM";
        assert!(verify_pkce_s256(verifier, challenge));
        assert!(!verify_pkce_s256("wrong-verifier", challenge));
    }

    #[test]
    fn legacy_and_hosted_bindings_are_separate() {
        let legacy = resolve_binding(None, None).expect("legacy");
        assert!(!legacy.hosted_mcp);
        let hosted = resolve_binding(None, Some(MCP_SCOPE)).expect("hosted");
        assert!(hosted.hosted_mcp);
        assert_eq!(hosted.resource, mcp_resource_url());
        assert!(resolve_binding(Some("https://wrong.example/mcp"), Some(MCP_SCOPE)).is_err());
    }

    #[test]
    fn redirects_reject_remote_plain_http() {
        assert!(!redirect_uri_acceptable("http://example.com/callback"));
        assert!(redirect_uri_acceptable("http://127.0.0.1:3000/callback"));
        assert!(redirect_uri_acceptable("https://example.com/callback"));
    }

    #[test]
    fn authorization_metadata_can_point_at_the_dashboard_origin() {
        assert_eq!(
            authorization_endpoint_for(
                "https://guard.example",
                Some("https://app.featherlane.ai/")
            ),
            "https://app.featherlane.ai/oauth/authorize"
        );
        assert_eq!(
            authorization_endpoint_for("https://guard.example/", None),
            "https://guard.example/oauth/authorize"
        );
    }

    #[test]
    fn dynamic_registration_limiter_rejects_a_burst() {
        let limiter = RegistrationLimiter::new(2, Duration::from_secs(60));
        assert!(limiter.try_acquire());
        assert!(limiter.try_acquire());
        assert!(!limiter.try_acquire());
    }
}
