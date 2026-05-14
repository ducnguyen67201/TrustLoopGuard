//! Policy authoring endpoints.

use std::sync::Arc;

use async_trait::async_trait;
use axum::{
    extract::{Path, State},
    http::{header, HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use tl_core::{
    ApiError, ApiErrorCode, PolicyDocument, PolicyDraft, PolicyDraftRequest, PolicyDraftResponse,
    PolicyListResponse, PolicySetEnabledRequest, PolicySummary, PolicyValidateResponse,
    PolicyValidationIssue,
};
use tl_llm::{JsonSchema, LlmClient};
use tl_policy::{Policy, ValidationIssue};
use tokio::sync::RwLock;

#[derive(Debug, thiserror::Error)]
pub enum PolicyStoreError {
    #[error("not found")]
    NotFound,
    #[error("internal: {0}")]
    Internal(String),
}

#[async_trait]
pub trait PolicyStore: Send + Sync {
    async fn upsert(
        &self,
        policy: &Policy,
        source_yaml: &str,
    ) -> Result<PolicyDocument, PolicyStoreError>;
    async fn get(&self, policy_id: &str) -> Result<PolicyDocument, PolicyStoreError>;
    async fn list(&self) -> Result<Vec<PolicySummary>, PolicyStoreError>;
    async fn list_enabled(&self) -> Result<Vec<Arc<Policy>>, PolicyStoreError>;
    async fn set_enabled(
        &self,
        policy_id: &str,
        enabled: bool,
    ) -> Result<PolicyDocument, PolicyStoreError>;
    async fn delete(&self, policy_id: &str) -> Result<(), PolicyStoreError>;
}

#[derive(Clone)]
pub struct PolicyState {
    pub store: Arc<dyn PolicyStore>,
    /// LLM used by `POST /v1/policies/draft`. `None` when no provider
    /// key is configured — the handler returns 503 in that case.
    pub draft_llm: Option<Arc<dyn LlmClient>>,
    /// Model name passed to the LLM client for drafts. Defaults to
    /// `gpt-4o-mini`; override with `TL_POLICY_DRAFT_MODEL`.
    pub draft_model: String,
}

#[derive(Debug, Clone)]
struct MemoryPolicyRecord {
    policy: Policy,
    source_yaml: String,
    enabled: bool,
}

#[derive(Debug, Default)]
pub struct MemoryPolicyStore {
    inner: RwLock<std::collections::HashMap<String, MemoryPolicyRecord>>,
}

impl MemoryPolicyStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_policies(policies: &[Policy]) -> Self {
        let records = policies
            .iter()
            .map(|policy| {
                (
                    policy.id.clone(),
                    MemoryPolicyRecord {
                        policy: policy.clone(),
                        source_yaml: serde_yaml::to_string(policy).unwrap_or_default(),
                        enabled: true,
                    },
                )
            })
            .collect();
        Self {
            inner: RwLock::new(records),
        }
    }
}

#[async_trait]
impl PolicyStore for MemoryPolicyStore {
    async fn upsert(
        &self,
        policy: &Policy,
        source_yaml: &str,
    ) -> Result<PolicyDocument, PolicyStoreError> {
        let record = MemoryPolicyRecord {
            policy: policy.clone(),
            source_yaml: source_yaml.to_string(),
            enabled: true,
        };
        self.inner
            .write()
            .await
            .insert(policy.id.clone(), record.clone());
        Ok(policy_document(
            &record.policy,
            &record.source_yaml,
            record.enabled,
        ))
    }

    async fn get(&self, policy_id: &str) -> Result<PolicyDocument, PolicyStoreError> {
        let guard = self.inner.read().await;
        let record = guard.get(policy_id).ok_or(PolicyStoreError::NotFound)?;
        Ok(policy_document(
            &record.policy,
            &record.source_yaml,
            record.enabled,
        ))
    }

    async fn list(&self) -> Result<Vec<PolicySummary>, PolicyStoreError> {
        let mut policies: Vec<_> = self
            .inner
            .read()
            .await
            .values()
            .map(|record| policy_summary(&record.policy, record.enabled))
            .collect();
        policies.sort_by(|a, b| a.id.cmp(&b.id));
        Ok(policies)
    }

    async fn list_enabled(&self) -> Result<Vec<Arc<Policy>>, PolicyStoreError> {
        let mut policies: Vec<_> = self
            .inner
            .read()
            .await
            .values()
            .filter(|record| record.enabled)
            .map(|record| Arc::new(record.policy.clone()))
            .collect();
        policies.sort_by(|a, b| a.id.cmp(&b.id));
        Ok(policies)
    }

    async fn set_enabled(
        &self,
        policy_id: &str,
        enabled: bool,
    ) -> Result<PolicyDocument, PolicyStoreError> {
        let mut guard = self.inner.write().await;
        let record = guard.get_mut(policy_id).ok_or(PolicyStoreError::NotFound)?;
        record.enabled = enabled;
        Ok(policy_document(
            &record.policy,
            &record.source_yaml,
            record.enabled,
        ))
    }

    async fn delete(&self, policy_id: &str) -> Result<(), PolicyStoreError> {
        if self.inner.write().await.remove(policy_id).is_none() {
            return Err(PolicyStoreError::NotFound);
        }
        Ok(())
    }
}

/// `POST /v1/policies` — create or update a policy from YAML or JSON.
#[utoipa::path(
    post,
    path = "/v1/policies",
    tag = "policies",
    request_body(
        description = "Policy document, YAML or JSON",
        content_type = "application/yaml",
        content = String,
    ),
    responses(
        (status = 201, description = "Policy created or updated", body = PolicyDocument),
        (status = 400, description = "Malformed request body", body = ApiError),
        (status = 401, description = "Missing or invalid API key", body = ApiError),
        (status = 422, description = "Policy failed validation", body = ApiError),
    ),
)]
pub async fn upsert_policy(
    State(state): State<PolicyState>,
    headers: HeaderMap,
    body: bytes::Bytes,
) -> Response {
    let parsed = match parse_policy_body(&headers, &body) {
        Ok(parsed) => parsed,
        Err(resp) => return *resp,
    };
    if let Err(issues) = tl_policy::validate_policy(&parsed.policy) {
        let details: Vec<_> = issues.iter().map(policy_validation_issue).collect();
        return api_error_response_with_details(
            StatusCode::UNPROCESSABLE_ENTITY,
            ApiErrorCode::Unprocessable,
            "policy failed validation".into(),
            json!(details),
        );
    }

    match state
        .store
        .upsert(&parsed.policy, &parsed.source_yaml)
        .await
    {
        Ok(document) => (StatusCode::CREATED, Json(document)).into_response(),
        Err(e) => policy_store_error_response(e),
    }
}

/// `GET /v1/policies` — list active policy summaries.
#[utoipa::path(
    get,
    path = "/v1/policies",
    tag = "policies",
    responses(
        (status = 200, description = "All active policies", body = PolicyListResponse),
        (status = 401, description = "Missing or invalid API key", body = ApiError),
    ),
)]
pub async fn list_policies(State(state): State<PolicyState>) -> Response {
    match state.store.list().await {
        Ok(policies) => Json(PolicyListResponse { policies }).into_response(),
        Err(e) => policy_store_error_response(e),
    }
}

/// `GET /v1/policies/:id` — fetch a policy document.
#[utoipa::path(
    get,
    path = "/v1/policies/{id}",
    tag = "policies",
    params(("id" = String, Path, description = "Policy identifier")),
    responses(
        (status = 200, description = "Policy found", body = PolicyDocument),
        (status = 401, description = "Missing or invalid API key", body = ApiError),
        (status = 404, description = "Policy not found", body = ApiError),
    ),
)]
pub async fn get_policy(State(state): State<PolicyState>, Path(id): Path<String>) -> Response {
    match state.store.get(&id).await {
        Ok(document) => Json(document).into_response(),
        Err(e) => policy_store_error_response(e),
    }
}

/// `PATCH /v1/policies/:id/enabled` — enable or disable a policy.
#[utoipa::path(
    patch,
    path = "/v1/policies/{id}/enabled",
    tag = "policies",
    request_body = PolicySetEnabledRequest,
    params(("id" = String, Path, description = "Policy identifier")),
    responses(
        (status = 200, description = "Updated policy", body = PolicyDocument),
        (status = 400, description = "Malformed request body", body = ApiError),
        (status = 401, description = "Missing or invalid API key", body = ApiError),
        (status = 404, description = "Policy not found", body = ApiError),
    ),
)]
pub async fn set_policy_enabled(
    State(state): State<PolicyState>,
    Path(id): Path<String>,
    body: bytes::Bytes,
) -> Response {
    let req: PolicySetEnabledRequest = match serde_json::from_slice(&body) {
        Ok(req) => req,
        Err(e) => {
            return api_error_response(
                StatusCode::BAD_REQUEST,
                ApiErrorCode::Invalid,
                format!("request body is not valid JSON: {e}"),
            );
        }
    };
    match state.store.set_enabled(&id, req.enabled).await {
        Ok(document) => Json(document).into_response(),
        Err(e) => policy_store_error_response(e),
    }
}

/// `DELETE /v1/policies/:id` — soft-delete a policy.
#[utoipa::path(
    delete,
    path = "/v1/policies/{id}",
    tag = "policies",
    params(("id" = String, Path, description = "Policy identifier")),
    responses(
        (status = 204, description = "Policy deleted"),
        (status = 401, description = "Missing or invalid API key", body = ApiError),
        (status = 404, description = "Policy not found", body = ApiError),
    ),
)]
pub async fn delete_policy(State(state): State<PolicyState>, Path(id): Path<String>) -> Response {
    match state.store.delete(&id).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => policy_store_error_response(e),
    }
}

/// `POST /v1/policies/validate` — validate policy YAML or JSON without saving it.
#[utoipa::path(
    post,
    path = "/v1/policies/validate",
    tag = "policies",
    request_body(
        description = "Policy document, YAML or JSON",
        content_type = "application/yaml",
        content = String,
    ),
    responses(
        (status = 200, description = "Validation result", body = PolicyValidateResponse),
        (status = 400, description = "Malformed request body", body = ApiError),
        (status = 401, description = "Missing or invalid API key", body = ApiError),
    ),
)]
pub async fn validate_policy(headers: HeaderMap, body: bytes::Bytes) -> Response {
    let raw = match std::str::from_utf8(&body) {
        Ok(raw) => raw,
        Err(e) => {
            return api_error_response(
                StatusCode::BAD_REQUEST,
                ApiErrorCode::Invalid,
                format!("body is not valid UTF-8: {e}"),
            );
        }
    };

    Json(validate_raw_policy(&headers, raw)).into_response()
}

/// System instructions prepended to every policy-draft prompt. Kept here
/// rather than in a file so the OpenAPI surface fully describes the
/// behavior the server exposes.
const POLICY_DRAFT_SYSTEM_PROMPT: &str = concat!(
    "You write TrustLoopGuard guardrail policies. Given a short natural-language ",
    "description, return a single policy draft as JSON matching the response schema.\n\n",
    "Rules:\n",
    "- `id` is kebab-case (lowercase letters, digits, hyphens only).\n",
    "- Prefer `match_type` = `literal` for specific phrases; use `regex` for patterns.\n",
    "- Default `action` is `block`. Use `rewrite` only when a clear safe replacement exists; ",
    "in that case set `rewrite` to the replacement text. Otherwise leave `rewrite` null.\n",
    "- Use `escalate` for ambiguous high-stakes cases the operator should review.\n",
);

fn policy_draft_json_schema() -> JsonSchema {
    JsonSchema {
        name: "policy_draft".to_string(),
        schema: json!({
            "type": "object",
            "additionalProperties": false,
            "required": [
                "id", "description", "match_type", "match_value",
                "action", "severity", "rewrite",
            ],
            "properties": {
                "id": { "type": "string", "description": "kebab-case identifier" },
                "description": { "type": "string" },
                "match_type": { "type": "string", "enum": ["literal", "regex"] },
                "match_value": { "type": "string" },
                "action": { "type": "string", "enum": ["block", "rewrite", "escalate"] },
                "severity": {
                    "type": "string",
                    "enum": ["low", "medium", "high", "critical"],
                },
                "rewrite": {
                    "type": ["string", "null"],
                    "description": "safe replacement when action is rewrite, else null",
                },
            },
        }),
    }
}

/// `POST /v1/policies/draft` — LLM-draft a policy skeleton from a natural-
/// language prompt. The server holds the provider key; callers see a
/// strict, typed response. Returns 503 when no LLM is configured.
#[utoipa::path(
    post,
    path = "/v1/policies/draft",
    tag = "policies",
    request_body = PolicyDraftRequest,
    responses(
        (status = 200, description = "Drafted policy", body = PolicyDraftResponse),
        (status = 400, description = "Malformed request body", body = ApiError),
        (status = 401, description = "Missing or invalid API key", body = ApiError),
        (status = 502, description = "LLM provider failed or returned invalid shape", body = ApiError),
        (status = 503, description = "LLM is not configured on this deployment", body = ApiError),
    ),
)]
pub async fn draft_policy(State(state): State<PolicyState>, body: bytes::Bytes) -> Response {
    let req: PolicyDraftRequest = match serde_json::from_slice(&body) {
        Ok(req) => req,
        Err(e) => {
            return api_error_response(
                StatusCode::BAD_REQUEST,
                ApiErrorCode::Invalid,
                format!("request body is not valid JSON: {e}"),
            );
        }
    };
    let prompt = req.prompt.trim();
    if prompt.len() < 3 {
        return api_error_response(
            StatusCode::BAD_REQUEST,
            ApiErrorCode::Invalid,
            "prompt is too short".into(),
        );
    }

    let Some(client) = state.draft_llm.clone() else {
        return api_error_response(
            StatusCode::SERVICE_UNAVAILABLE,
            ApiErrorCode::Unavailable,
            "policy drafting is not configured on this deployment (no LLM key)".into(),
        );
    };

    let composed = format!("{POLICY_DRAFT_SYSTEM_PROMPT}\nUser request:\n{prompt}");
    let schema = policy_draft_json_schema();
    let out = match client
        .complete(
            &state.draft_model,
            &composed,
            &schema,
            std::time::Duration::from_secs(30),
        )
        .await
    {
        Ok(out) => out,
        Err(e) => {
            return api_error_response(
                StatusCode::BAD_GATEWAY,
                ApiErrorCode::Unavailable,
                format!("llm provider error: {e}"),
            );
        }
    };

    // Strict mode forces `rewrite` to be present; coerce null → absent so
    // it deserializes into Option<String>::None.
    let mut value = out.json;
    if value.get("rewrite") == Some(&serde_json::Value::Null) {
        if let Some(obj) = value.as_object_mut() {
            obj.remove("rewrite");
        }
    }

    let draft: PolicyDraft = match serde_json::from_value(value) {
        Ok(draft) => draft,
        Err(e) => {
            return api_error_response(
                StatusCode::BAD_GATEWAY,
                ApiErrorCode::Internal,
                format!("model returned invalid policy shape: {e}"),
            );
        }
    };

    Json(PolicyDraftResponse { draft }).into_response()
}

fn validate_raw_policy(headers: &HeaderMap, raw: &str) -> PolicyValidateResponse {
    let parsed = parse_policy(headers, raw);
    let policy = match parsed {
        Ok(policy) => policy,
        Err(issue) => {
            return PolicyValidateResponse {
                valid: false,
                policy_id: None,
                errors: vec![issue],
            };
        }
    };

    match tl_policy::validate_policy(&policy) {
        Ok(()) => PolicyValidateResponse {
            valid: true,
            policy_id: Some(policy.id),
            errors: vec![],
        },
        Err(issues) => PolicyValidateResponse {
            valid: false,
            policy_id: Some(policy.id),
            errors: issues.iter().map(policy_validation_issue).collect(),
        },
    }
}

struct ParsedPolicyBody {
    policy: Policy,
    source_yaml: String,
}

fn parse_policy_body(headers: &HeaderMap, body: &[u8]) -> Result<ParsedPolicyBody, Box<Response>> {
    let raw = std::str::from_utf8(body).map_err(|e| {
        Box::new(api_error_response(
            StatusCode::BAD_REQUEST,
            ApiErrorCode::Invalid,
            format!("body is not valid UTF-8: {e}"),
        ))
    })?;
    let policy = parse_policy(headers, raw).map_err(|issue| {
        Box::new(api_error_response(
            StatusCode::BAD_REQUEST,
            ApiErrorCode::Invalid,
            issue.message,
        ))
    })?;
    let source_yaml = if is_yaml_content_type(headers) {
        raw.to_string()
    } else {
        serde_yaml::to_string(&policy).map_err(|e| {
            Box::new(api_error_response(
                StatusCode::BAD_REQUEST,
                ApiErrorCode::Invalid,
                format!("policy yaml render: {e}"),
            ))
        })?
    };
    Ok(ParsedPolicyBody {
        policy,
        source_yaml,
    })
}

fn policy_validation_issue(issue: &ValidationIssue) -> PolicyValidationIssue {
    PolicyValidationIssue {
        path: issue.path.clone(),
        message: issue.message.clone(),
    }
}

fn parse_policy(headers: &HeaderMap, raw: &str) -> Result<Policy, PolicyValidationIssue> {
    if is_yaml_content_type(headers) {
        serde_yaml::from_str(raw).map_err(|e| PolicyValidationIssue {
            path: "$".into(),
            message: format!("yaml parse: {e}"),
        })
    } else {
        serde_json::from_str(raw).map_err(|e| PolicyValidationIssue {
            path: "$".into(),
            message: format!("json parse: {e}"),
        })
    }
}

fn policy_document(policy: &Policy, source_yaml: &str, enabled: bool) -> PolicyDocument {
    PolicyDocument {
        id: policy.id.clone(),
        description: policy.description.clone(),
        severity: policy.severity,
        enabled,
        source_yaml: source_yaml.to_string(),
    }
}

fn policy_summary(policy: &Policy, enabled: bool) -> PolicySummary {
    PolicySummary {
        id: policy.id.clone(),
        description: policy.description.clone(),
        severity: policy.severity,
        enabled,
    }
}

fn is_yaml_content_type(headers: &HeaderMap) -> bool {
    let content_type = headers
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("application/json")
        .to_ascii_lowercase();

    content_type.starts_with("application/yaml")
        || content_type.starts_with("application/x-yaml")
        || content_type.starts_with("text/yaml")
        || content_type.starts_with("text/x-yaml")
}

fn api_error_response(status: StatusCode, code: ApiErrorCode, message: String) -> Response {
    api_error_response_with_details(status, code, message, json!(null))
}

fn api_error_response_with_details(
    status: StatusCode,
    code: ApiErrorCode,
    message: String,
    details: serde_json::Value,
) -> Response {
    let retriable = matches!(
        code,
        ApiErrorCode::RateLimited | ApiErrorCode::Internal | ApiErrorCode::Unavailable
    );
    let body = ApiError {
        code,
        message,
        retriable,
        details,
    };
    (status, Json(body)).into_response()
}

fn policy_store_error_response(err: PolicyStoreError) -> Response {
    match err {
        PolicyStoreError::NotFound => api_error_response(
            StatusCode::NOT_FOUND,
            ApiErrorCode::NotFound,
            "policy not found".into(),
        ),
        PolicyStoreError::Internal(e) => {
            api_error_response(StatusCode::INTERNAL_SERVER_ERROR, ApiErrorCode::Internal, e)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn malformed_yaml_returns_validation_issue() {
        let mut headers = HeaderMap::new();
        headers.insert(header::CONTENT_TYPE, "application/yaml".parse().unwrap());
        let out = validate_raw_policy(&headers, "not: valid: yaml: [");
        assert!(!out.valid);
        assert_eq!(out.errors[0].path, "$");
        assert!(out.errors[0].message.contains("yaml parse"));
    }

    #[test]
    fn valid_yaml_returns_policy_id() {
        let mut headers = HeaderMap::new();
        headers.insert(header::CONTENT_TYPE, "application/yaml".parse().unwrap());
        let out = validate_raw_policy(
            &headers,
            r#"
id: refund-guarantee
description: Prevents agents from guaranteeing refunds.
match:
  literal: "guaranteed refund"
action: block
"#,
        );
        assert!(out.valid);
        assert_eq!(out.policy_id.as_deref(), Some("refund-guarantee"));
        assert!(out.errors.is_empty());
    }

    #[test]
    fn validation_errors_are_structured() {
        let mut headers = HeaderMap::new();
        headers.insert(header::CONTENT_TYPE, "application/yaml".parse().unwrap());
        let out = validate_raw_policy(
            &headers,
            r#"
id: "Refund Guarantee"
match:
  regex: "["
action: rewrite
"#,
        );
        assert!(!out.valid);
        assert!(out.errors.iter().any(|e| e.path == "id"));
        assert!(out.errors.iter().any(|e| e.path == "match.regex"));
        assert!(out.errors.iter().any(|e| e.path == "rewrite"));
    }

    #[test]
    fn valid_json_policy_works() {
        let headers = HeaderMap::new();
        let out = validate_raw_policy(
            &headers,
            r#"{"id":"json-policy","match":{"literal":"refund"},"action":"block"}"#,
        );
        assert!(out.valid);
        assert_eq!(out.policy_id.as_deref(), Some("json-policy"));
    }

    #[test]
    fn load_str_and_validate_endpoint_agree_on_valid_yaml() {
        let mut headers = HeaderMap::new();
        headers.insert(header::CONTENT_TYPE, "application/yaml".parse().unwrap());
        let yaml = include_str!("../../../docs/policies/examples/refund-guarantee.yaml");
        let out = validate_raw_policy(&headers, yaml);
        assert!(out.valid);
        let parsed = tl_policy::load_str(yaml).expect("policy");
        assert_eq!(out.policy_id.as_deref(), Some(parsed.id.as_str()));
    }

    #[test]
    fn policy_error_type_still_formats_for_cli() {
        let err = tl_policy::PolicyError::Validation("id: id is required".into()).to_string();
        assert!(err.contains("policy validation"));
    }
}
