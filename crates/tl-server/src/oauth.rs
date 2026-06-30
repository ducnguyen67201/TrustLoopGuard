//! OAuth 2.1 authorization-server backend for MCP clients.
//!
//! The browser-facing login + consent + workspace picker live in `apps/web`
//! (reusing Auth.js); this module owns the machinery: discovery metadata,
//! dynamic client registration, PKCE-bound authorization codes, and the token
//! endpoint that mints workspace-scoped access tokens. The resource-server side
//! (validating those tokens) lives in `auth.rs`.
//!
//! Flow: client discovers `/.well-known/*` → `POST /oauth/register` → browser
//! to the web consent page → web app calls `POST /v1/oauth/authorize` (internal
//! auth + forwarded user/workspace) to mint a code → `POST /oauth/token`
//! exchanges code+PKCE for an access token bound to the chosen workspace.
//!
// ponytail: in-memory stores (clients/codes/refresh) — fine for a single node;
// a restart drops registrations + live codes. Durable tables are a follow-up.

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex;

use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Form, Json, Router};
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use chrono::Utc;
use serde::Deserialize;
use serde_json::json;
use uuid::Uuid;

use crate::jwt::ACCESS_TOKEN_TTL_MINUTES;
use crate::AppState;

const AUTH_CODE_TTL_SECONDS: i64 = 60;
const REFRESH_TTL_DAYS: i64 = 30;

/// External base URL the AS advertises in discovery metadata.
fn issuer() -> String {
    std::env::var("TL_PUBLIC_URL").unwrap_or_else(|_| "http://localhost:8080".to_string())
}

fn random_token() -> String {
    use rand::Rng;
    let mut bytes = [0u8; 32];
    rand::rng().fill_bytes(&mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)
}

#[derive(Clone)]
struct OAuthClient {
    redirect_uris: Vec<String>,
}

struct AuthCode {
    client_id: String,
    redirect_uri: String,
    user_id: Uuid,
    username: String,
    workspace_id: String,
    code_challenge: String,
    expires_at: i64,
}

struct RefreshEntry {
    user_id: Uuid,
    username: String,
    workspace_id: String,
    expires_at: i64,
}

/// In-memory OAuth state. Single instance per server, shared across routes.
#[derive(Default)]
pub struct OAuthStore {
    clients: Mutex<HashMap<String, OAuthClient>>,
    codes: Mutex<HashMap<String, AuthCode>>,
    refresh: Mutex<HashMap<String, RefreshEntry>>,
}

impl OAuthStore {
    fn register_client(&self, redirect_uris: Vec<String>) -> String {
        let client_id = format!("mcp_{}", random_token());
        self.clients
            .lock()
            .unwrap()
            .insert(client_id.clone(), OAuthClient { redirect_uris });
        client_id
    }

    fn redirect_ok(&self, client_id: &str, redirect_uri: &str) -> bool {
        self.clients
            .lock()
            .unwrap()
            .get(client_id)
            .is_some_and(|c| c.redirect_uris.iter().any(|u| u == redirect_uri))
    }

    #[allow(clippy::too_many_arguments)]
    fn issue_code(
        &self,
        client_id: &str,
        redirect_uri: &str,
        user_id: Uuid,
        username: &str,
        workspace_id: &str,
        code_challenge: &str,
    ) -> String {
        let code = random_token();
        self.codes.lock().unwrap().insert(
            code.clone(),
            AuthCode {
                client_id: client_id.to_string(),
                redirect_uri: redirect_uri.to_string(),
                user_id,
                username: username.to_string(),
                workspace_id: workspace_id.to_string(),
                code_challenge: code_challenge.to_string(),
                expires_at: Utc::now().timestamp() + AUTH_CODE_TTL_SECONDS,
            },
        );
        code
    }

    /// Single-use: removes the code so it can never be replayed.
    fn take_code(&self, code: &str) -> Option<AuthCode> {
        self.codes.lock().unwrap().remove(code)
    }

    fn issue_refresh(&self, user_id: Uuid, username: &str, workspace_id: &str) -> String {
        let token = random_token();
        self.refresh.lock().unwrap().insert(
            token.clone(),
            RefreshEntry {
                user_id,
                username: username.to_string(),
                workspace_id: workspace_id.to_string(),
                expires_at: Utc::now().timestamp() + REFRESH_TTL_DAYS * 86_400,
            },
        );
        token
    }

    /// Single-use + rotation: removes the presented refresh token.
    fn take_refresh(&self, token: &str) -> Option<RefreshEntry> {
        self.refresh.lock().unwrap().remove(token)
    }
}

#[derive(Clone)]
pub struct OAuthState {
    app: AppState,
    store: Arc<OAuthStore>,
}

fn oauth_error(status: StatusCode, code: &str, desc: &str) -> Response {
    (
        status,
        Json(json!({ "error": code, "error_description": desc })),
    )
        .into_response()
}

// ---- Discovery metadata (public) -------------------------------------------

async fn authorization_server_metadata() -> Response {
    let issuer = issuer();
    Json(json!({
        "issuer": issuer,
        "authorization_endpoint": format!("{issuer}/oauth/authorize"),
        "token_endpoint": format!("{issuer}/oauth/token"),
        "registration_endpoint": format!("{issuer}/oauth/register"),
        "response_types_supported": ["code"],
        "grant_types_supported": ["authorization_code", "refresh_token"],
        "code_challenge_methods_supported": ["S256"],
        "token_endpoint_auth_methods_supported": ["none"],
    }))
    .into_response()
}

async fn protected_resource_metadata() -> Response {
    let issuer = issuer();
    Json(json!({
        "resource": format!("{issuer}/mcp/pay"),
        "authorization_servers": [issuer],
    }))
    .into_response()
}

// ---- Dynamic client registration (public, RFC 7591) ------------------------

#[derive(Deserialize)]
struct RegisterRequest {
    #[serde(default)]
    redirect_uris: Vec<String>,
}

async fn register(State(state): State<OAuthState>, Json(req): Json<RegisterRequest>) -> Response {
    if req.redirect_uris.is_empty() {
        return oauth_error(
            StatusCode::BAD_REQUEST,
            "invalid_redirect_uri",
            "at least one redirect_uri is required",
        );
    }
    let client_id = state.store.register_client(req.redirect_uris.clone());
    (
        StatusCode::CREATED,
        Json(json!({
            "client_id": client_id,
            "redirect_uris": req.redirect_uris,
            "token_endpoint_auth_method": "none",
            "grant_types": ["authorization_code", "refresh_token"],
        })),
    )
        .into_response()
}

// ---- Authorization-code issuance (internal; called by the web consent page) -

#[derive(Deserialize)]
struct AuthorizeRequest {
    client_id: String,
    redirect_uri: String,
    code_challenge: String,
}

/// `POST /v1/oauth/authorize` — the web consent page calls this with the
/// internal API key + forwarded `x-tlg-user-id` / `x-tlg-workspace-id`. We
/// verify the user is a member of the workspace, then mint a PKCE-bound code.
async fn authorize(
    State(state): State<OAuthState>,
    headers: HeaderMap,
    Json(req): Json<AuthorizeRequest>,
) -> Response {
    let header = |name: &str| {
        headers
            .get(name)
            .and_then(|v| v.to_str().ok())
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
    };
    let (Some(user_id_raw), Some(workspace_id)) =
        (header("x-tlg-user-id"), header("x-tlg-workspace-id"))
    else {
        return oauth_error(
            StatusCode::UNAUTHORIZED,
            "access_denied",
            "missing forwarded user or workspace identity",
        );
    };
    let Ok(user_id) = Uuid::parse_str(&user_id_raw) else {
        return oauth_error(StatusCode::BAD_REQUEST, "access_denied", "bad user id");
    };

    if !state.store.redirect_ok(&req.client_id, &req.redirect_uri) {
        return oauth_error(
            StatusCode::BAD_REQUEST,
            "invalid_request",
            "unknown client_id or redirect_uri not registered",
        );
    }

    // Membership check: the signed-in user must belong to the chosen workspace.
    let members = match state.app.team_store.list_members(&workspace_id).await {
        Ok(members) => members,
        Err(e) => {
            tracing::error!(error = %e, "oauth authorize: membership lookup failed");
            return oauth_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "server_error",
                "membership lookup failed",
            );
        }
    };
    let member = members.iter().find(|m| m.user_id == user_id.to_string());
    let Some(member) = member else {
        return oauth_error(
            StatusCode::FORBIDDEN,
            "access_denied",
            "user is not a member of the selected workspace",
        );
    };

    let code = state.store.issue_code(
        &req.client_id,
        &req.redirect_uri,
        user_id,
        &member.username,
        &workspace_id,
        &req.code_challenge,
    );
    Json(json!({ "code": code })).into_response()
}

// ---- Token endpoint (public; PKCE exchange + refresh) ----------------------

#[derive(Deserialize)]
struct TokenForm {
    grant_type: String,
    code: Option<String>,
    code_verifier: Option<String>,
    redirect_uri: Option<String>,
    client_id: Option<String>,
    refresh_token: Option<String>,
}

fn verify_pkce_s256(verifier: &str, challenge: &str) -> bool {
    use sha2::{Digest, Sha256};
    let digest = Sha256::digest(verifier.as_bytes());
    URL_SAFE_NO_PAD.encode(digest) == challenge
}

async fn token(State(state): State<OAuthState>, Form(form): Form<TokenForm>) -> Response {
    let Some(signer) = state.app.jwt_signer.as_ref() else {
        return oauth_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "server_error",
            "token signing is not configured (TL_JWT_SECRET unset)",
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
            let Some(entry) = state.store.take_code(&code) else {
                return oauth_error(
                    StatusCode::BAD_REQUEST,
                    "invalid_grant",
                    "unknown or used code",
                );
            };
            if entry.expires_at < Utc::now().timestamp() {
                return oauth_error(StatusCode::BAD_REQUEST, "invalid_grant", "code expired");
            }
            if entry.client_id != client_id || entry.redirect_uri != redirect_uri {
                return oauth_error(
                    StatusCode::BAD_REQUEST,
                    "invalid_grant",
                    "client/redirect mismatch",
                );
            }
            if !verify_pkce_s256(&verifier, &entry.code_challenge) {
                return oauth_error(
                    StatusCode::BAD_REQUEST,
                    "invalid_grant",
                    "PKCE verification failed",
                );
            }
            issue_tokens(
                &state,
                signer,
                entry.user_id,
                &entry.username,
                &entry.workspace_id,
            )
        }
        "refresh_token" => {
            let Some(refresh) = form.refresh_token else {
                return oauth_error(
                    StatusCode::BAD_REQUEST,
                    "invalid_request",
                    "missing refresh_token",
                );
            };
            let Some(entry) = state.store.take_refresh(&refresh) else {
                return oauth_error(
                    StatusCode::BAD_REQUEST,
                    "invalid_grant",
                    "unknown or used refresh token",
                );
            };
            if entry.expires_at < Utc::now().timestamp() {
                return oauth_error(
                    StatusCode::BAD_REQUEST,
                    "invalid_grant",
                    "refresh token expired",
                );
            }
            issue_tokens(
                &state,
                signer,
                entry.user_id,
                &entry.username,
                &entry.workspace_id,
            )
        }
        other => oauth_error(
            StatusCode::BAD_REQUEST,
            "unsupported_grant_type",
            &format!("unsupported grant_type: {other}"),
        ),
    }
}

fn issue_tokens(
    state: &OAuthState,
    signer: &crate::jwt::JwtSigner,
    user_id: Uuid,
    username: &str,
    workspace_id: &str,
) -> Response {
    let access = match signer.mint_access_token(user_id, username, workspace_id) {
        Ok(token) => token,
        Err(e) => {
            tracing::error!(error = %e, "oauth: access token mint failed");
            return oauth_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "server_error",
                "token mint failed",
            );
        }
    };
    let refresh = state.store.issue_refresh(user_id, username, workspace_id);
    Json(json!({
        "access_token": access,
        "token_type": "Bearer",
        "expires_in": ACCESS_TOKEN_TTL_MINUTES * 60,
        "refresh_token": refresh,
    }))
    .into_response()
}

// ---- Routers ---------------------------------------------------------------

/// Public OAuth routes (no bearer): discovery, registration, token exchange.
pub fn oauth_public_routes(app: AppState, store: Arc<OAuthStore>) -> Router {
    let state = OAuthState { app, store };
    Router::new()
        .route(
            "/.well-known/oauth-authorization-server",
            get(authorization_server_metadata),
        )
        .route(
            "/.well-known/oauth-protected-resource",
            get(protected_resource_metadata),
        )
        .route("/oauth/register", post(register))
        .route("/oauth/token", post(token))
        .with_state(state)
}

/// Protected OAuth route (under bearer auth): code issuance, called by the web
/// consent page with the internal key + forwarded user/workspace.
pub fn oauth_protected_routes(app: AppState, store: Arc<OAuthStore>) -> Router {
    let state = OAuthState { app, store };
    Router::new()
        .route("/v1/oauth/authorize", post(authorize))
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pkce_s256_matches_known_vector() {
        // RFC 7636 appendix B test vector.
        let verifier = "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk";
        let challenge = "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM";
        assert!(verify_pkce_s256(verifier, challenge));
        assert!(!verify_pkce_s256("wrong-verifier", challenge));
    }

    #[test]
    fn code_is_single_use() {
        let store = OAuthStore::default();
        let id = Uuid::new_v4();
        let code = store.issue_code("c", "https://r", id, "u", "ws", "chal");
        assert!(store.take_code(&code).is_some());
        assert!(store.take_code(&code).is_none()); // replay rejected
    }

    #[test]
    fn redirect_uri_must_be_registered() {
        let store = OAuthStore::default();
        let cid = store.register_client(vec!["https://ok".into()]);
        assert!(store.redirect_ok(&cid, "https://ok"));
        assert!(!store.redirect_ok(&cid, "https://evil"));
        assert!(!store.redirect_ok("unknown", "https://ok"));
    }
}
