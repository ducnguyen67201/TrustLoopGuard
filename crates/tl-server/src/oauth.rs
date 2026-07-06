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
/// In-memory DoS backstops for the unauthenticated registration endpoint.
const MAX_CLIENTS: usize = 10_000;
const MAX_REDIRECT_URIS: usize = 10;
const MAX_REDIRECT_URI_LEN: usize = 2048;

/// A redirect URI is acceptable for an MCP client when it is `https`, a
/// loopback `http` (localhost/127.0.0.1/::1), or a custom app scheme — never
/// `javascript:`/`data:`, never oversized. Plain `http` to a remote host is
/// rejected: it would expose the authorization code on the wire.
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
    client_id: String,
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
    /// `None` when the global client cap is hit (in-memory DoS backstop).
    fn register_client(&self, redirect_uris: Vec<String>) -> Option<String> {
        let mut clients = self.clients.lock().unwrap_or_else(|e| e.into_inner());
        if clients.len() >= MAX_CLIENTS {
            return None;
        }
        let client_id = format!("mcp_{}", random_token());
        clients.insert(client_id.clone(), OAuthClient { redirect_uris });
        Some(client_id)
    }

    fn redirect_ok(&self, client_id: &str, redirect_uri: &str) -> bool {
        self.clients
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .get(client_id)
            .is_some_and(|c| c.redirect_uris.iter().any(|u| u == redirect_uri))
    }

    /// Registered redirect URIs for a client — used by the consent page to
    /// validate `redirect_uri` server-side before rendering (open-redirect guard).
    fn redirect_uris(&self, client_id: &str) -> Option<Vec<String>> {
        self.clients
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .get(client_id)
            .map(|c| c.redirect_uris.clone())
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
        self.codes.lock().unwrap_or_else(|e| e.into_inner()).insert(
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
        self.codes
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .remove(code)
    }

    fn issue_refresh(
        &self,
        client_id: &str,
        user_id: Uuid,
        username: &str,
        workspace_id: &str,
    ) -> String {
        let token = random_token();
        self.refresh
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .insert(
                token.clone(),
                RefreshEntry {
                    client_id: client_id.to_string(),
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
        self.refresh
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .remove(token)
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
        "resource": issuer,
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
    if req.redirect_uris.is_empty() || req.redirect_uris.len() > MAX_REDIRECT_URIS {
        return oauth_error(
            StatusCode::BAD_REQUEST,
            "invalid_redirect_uri",
            "between 1 and 10 redirect_uris are required",
        );
    }
    if !req.redirect_uris.iter().all(|u| redirect_uri_acceptable(u)) {
        return oauth_error(
            StatusCode::BAD_REQUEST,
            "invalid_redirect_uri",
            "redirect_uris must be https, loopback http, or a custom scheme",
        );
    }
    let Some(client_id) = state.store.register_client(req.redirect_uris.clone()) else {
        return oauth_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "temporarily_unavailable",
            "client registration capacity reached",
        );
    };
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

/// `GET /oauth/clients/{client_id}/redirect-uris` — public lookup so the web
/// consent page can validate `redirect_uri` server-side before rendering
/// (open-redirect guard, CRITICAL-1).
async fn client_redirect_uris(
    State(state): State<OAuthState>,
    axum::extract::Path(client_id): axum::extract::Path<String>,
) -> Response {
    match state.store.redirect_uris(&client_id) {
        Some(redirect_uris) => Json(json!({ "redirect_uris": redirect_uris })).into_response(),
        None => oauth_error(StatusCode::NOT_FOUND, "invalid_client", "unknown client"),
    }
}

// ---- Authorization-code issuance (internal; called by the web consent page) -

#[derive(Deserialize)]
struct AuthorizeRequest {
    client_id: String,
    redirect_uri: String,
    code_challenge: String,
    #[serde(default)]
    code_challenge_method: Option<String>,
}

/// `POST /v1/oauth/authorize` — the web consent page calls this with the
/// internal API key + forwarded `x-tlg-user-id` / `x-tlg-workspace-id`. We
/// verify the user is a member of the workspace, then mint a PKCE-bound code.
async fn authorize(
    State(state): State<OAuthState>,
    headers: HeaderMap,
    Json(req): Json<AuthorizeRequest>,
) -> Response {
    // PKCE: only S256 is supported. Reject `plain` (and anything else)
    // explicitly rather than relying on a later silent mismatch.
    if req.code_challenge_method.as_deref().unwrap_or("S256") != "S256" {
        return oauth_error(
            StatusCode::BAD_REQUEST,
            "invalid_request",
            "only the S256 code_challenge_method is supported",
        );
    }
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
            // Single generic error for every invalid-code condition (unknown,
            // used, expired, mismatched, bad PKCE) so an observer can't tell
            // a code's lifecycle state apart (M-3).
            let invalid = || {
                oauth_error(
                    StatusCode::BAD_REQUEST,
                    "invalid_grant",
                    "invalid authorization code",
                )
            };
            let Some(entry) = state.store.take_code(&code) else {
                return invalid();
            };
            if entry.expires_at < Utc::now().timestamp()
                || entry.client_id != client_id
                || entry.redirect_uri != redirect_uri
                || !verify_pkce_s256(&verifier, &entry.code_challenge)
            {
                return invalid();
            }
            // Defense in depth: re-validate the redirect against the still-
            // registered client, not only the value stored on the code (H-2).
            if !state.store.redirect_ok(&client_id, &redirect_uri) {
                return invalid();
            }
            issue_tokens(
                &state,
                signer,
                &client_id,
                entry.user_id,
                &entry.username,
                &entry.workspace_id,
            )
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
            let Some(entry) = state.store.take_refresh(&refresh) else {
                return invalid();
            };
            // RFC 6749 §10.4: refresh tokens are bound to the issuing client
            // (C-2) — a different client cannot use a leaked refresh token.
            if entry.expires_at < Utc::now().timestamp() || entry.client_id != client_id {
                return invalid();
            }
            issue_tokens(
                &state,
                signer,
                &client_id,
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
    client_id: &str,
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
    let refresh = state
        .store
        .issue_refresh(client_id, user_id, username, workspace_id);
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
        // Body limits cap the unauthenticated attack surface (H-1).
        .route(
            "/oauth/register",
            post(register).layer(axum::extract::DefaultBodyLimit::max(16 * 1024)),
        )
        .route(
            "/oauth/token",
            post(token).layer(axum::extract::DefaultBodyLimit::max(16 * 1024)),
        )
        .route(
            "/oauth/clients/:client_id/redirect-uris",
            get(client_redirect_uris),
        )
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
        let cid = store.register_client(vec!["https://ok".into()]).unwrap();
        assert!(store.redirect_ok(&cid, "https://ok"));
        assert!(!store.redirect_ok(&cid, "https://evil"));
        assert!(!store.redirect_ok("unknown", "https://ok"));
    }

    #[test]
    fn redirect_uri_scheme_rules() {
        assert!(redirect_uri_acceptable("https://app.example.com/cb"));
        assert!(redirect_uri_acceptable("http://localhost:9999/cb"));
        assert!(redirect_uri_acceptable("http://127.0.0.1/cb"));
        assert!(redirect_uri_acceptable("myapp://callback"));
        assert!(!redirect_uri_acceptable("http://evil.com/cb")); // remote http leaks the code
        assert!(!redirect_uri_acceptable("javascript:alert(1)"));
        assert!(!redirect_uri_acceptable("data:text/html,x"));
        assert!(!redirect_uri_acceptable(""));
        assert!(!redirect_uri_acceptable(&format!(
            "https://x/{}",
            "a".repeat(3000)
        )));
    }
}
