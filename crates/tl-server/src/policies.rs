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
    AiEditRequest, AiEditResponse, ApiError, ApiErrorCode, EntityVersionDetail,
    EntityVersionListResponse, GuardrailGenerateResponse, GuardrailListResponse, PolicyAction,
    PolicyBatchSetEnabledRequest, PolicyBatchSetEnabledResponse, PolicyDocument, PolicyDraft,
    PolicyDraftRequest, PolicyDraftResponse, PolicyListResponse, PolicyMatchType,
    PolicySetEnabledRequest, PolicySummary, PolicyValidateResponse, PolicyValidationIssue,
    DEFAULT_WORKSPACE_ID,
};
use tl_llm::{JsonSchema, LlmClient};
use tl_policy::policy_ast::WhenClause;
use tl_policy::{Action, MatchClause, Matcher, Policy, ValidationIssue};
use tokio::sync::RwLock;

use crate::agents::{AgentStore, AgentStoreError};

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
        workspace_id: &str,
        policy: &Policy,
        source_yaml: &str,
    ) -> Result<PolicyDocument, PolicyStoreError>;
    async fn get(
        &self,
        workspace_id: &str,
        policy_id: &str,
    ) -> Result<PolicyDocument, PolicyStoreError>;
    async fn list(&self, workspace_id: &str) -> Result<Vec<PolicySummary>, PolicyStoreError>;
    async fn list_enabled(&self, workspace_id: &str) -> Result<Vec<Arc<Policy>>, PolicyStoreError>;
    async fn set_enabled(
        &self,
        workspace_id: &str,
        policy_id: &str,
        enabled: bool,
    ) -> Result<PolicyDocument, PolicyStoreError>;
    async fn batch_set_enabled(
        &self,
        workspace_id: &str,
        policy_ids: &[String],
        enabled: bool,
    ) -> Result<Vec<PolicySummary>, PolicyStoreError>;
    async fn delete(&self, workspace_id: &str, policy_id: &str) -> Result<(), PolicyStoreError>;

    /// Active policies owned by `agent_id`. Backs
    /// `GET /v1/agents/{id}/guardrails`. Returns an empty vec when the
    /// agent has none; existence of the agent is the caller's concern.
    async fn list_for_agent(
        &self,
        workspace_id: &str,
        agent_id: &str,
    ) -> Result<Vec<PolicySummary>, PolicyStoreError>;

    /// Soft-delete every active policy owned by `agent_id`. Returns the
    /// ids that were deleted. Called from the cascade-delete path when
    /// an agent is soft-deleted.
    async fn delete_for_agent(
        &self,
        workspace_id: &str,
        agent_id: &str,
    ) -> Result<Vec<String>, PolicyStoreError>;

    /// All saved versions for a policy, newest first. Returns an empty
    /// list when no versions exist yet (pre-versioning policies).
    async fn list_versions(
        &self,
        workspace_id: &str,
        policy_id: &str,
    ) -> Result<EntityVersionListResponse, PolicyStoreError>;

    /// Fetch the YAML for a specific historical version.
    async fn get_version(
        &self,
        workspace_id: &str,
        policy_id: &str,
        version: i32,
    ) -> Result<EntityVersionDetail, PolicyStoreError>;
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
    inner: RwLock<std::collections::HashMap<(String, String), MemoryPolicyRecord>>,
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
                    (DEFAULT_WORKSPACE_ID.to_string(), policy.id.clone()),
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
        workspace_id: &str,
        policy: &Policy,
        source_yaml: &str,
    ) -> Result<PolicyDocument, PolicyStoreError> {
        let record = MemoryPolicyRecord {
            policy: policy.clone(),
            source_yaml: source_yaml.to_string(),
            enabled: true,
        };
        self.inner.write().await.insert(
            (workspace_id.to_string(), policy.id.clone()),
            record.clone(),
        );
        Ok(policy_document(
            &record.policy,
            &record.source_yaml,
            record.enabled,
        ))
    }

    async fn get(
        &self,
        workspace_id: &str,
        policy_id: &str,
    ) -> Result<PolicyDocument, PolicyStoreError> {
        let guard = self.inner.read().await;
        let record = guard
            .get(&(workspace_id.to_string(), policy_id.to_string()))
            .ok_or(PolicyStoreError::NotFound)?;
        Ok(policy_document(
            &record.policy,
            &record.source_yaml,
            record.enabled,
        ))
    }

    async fn list(&self, workspace_id: &str) -> Result<Vec<PolicySummary>, PolicyStoreError> {
        let mut policies: Vec<_> = self
            .inner
            .read()
            .await
            .iter()
            .filter(|((workspace, _), _)| workspace == workspace_id)
            .map(|(_, record)| record)
            .map(|record| policy_summary(&record.policy, record.enabled))
            .collect();
        policies.sort_by(|a, b| a.id.cmp(&b.id));
        Ok(policies)
    }

    async fn list_enabled(&self, workspace_id: &str) -> Result<Vec<Arc<Policy>>, PolicyStoreError> {
        let mut policies: Vec<_> = self
            .inner
            .read()
            .await
            .iter()
            .filter(|((workspace, _), _)| workspace == workspace_id)
            .map(|(_, record)| record)
            .filter(|record| record.enabled)
            .map(|record| Arc::new(record.policy.clone()))
            .collect();
        policies.sort_by(|a, b| a.id.cmp(&b.id));
        Ok(policies)
    }

    async fn set_enabled(
        &self,
        workspace_id: &str,
        policy_id: &str,
        enabled: bool,
    ) -> Result<PolicyDocument, PolicyStoreError> {
        let mut guard = self.inner.write().await;
        let record = guard
            .get_mut(&(workspace_id.to_string(), policy_id.to_string()))
            .ok_or(PolicyStoreError::NotFound)?;
        record.enabled = enabled;
        Ok(policy_document(
            &record.policy,
            &record.source_yaml,
            record.enabled,
        ))
    }

    async fn batch_set_enabled(
        &self,
        workspace_id: &str,
        policy_ids: &[String],
        enabled: bool,
    ) -> Result<Vec<PolicySummary>, PolicyStoreError> {
        let mut guard = self.inner.write().await;
        let workspace = workspace_id.to_string();
        if policy_ids
            .iter()
            .any(|id| !guard.contains_key(&(workspace.clone(), id.to_string())))
        {
            return Err(PolicyStoreError::NotFound);
        }

        let mut policies = Vec::with_capacity(policy_ids.len());
        for id in policy_ids {
            let record = guard
                .get_mut(&(workspace.clone(), id.to_string()))
                .ok_or(PolicyStoreError::NotFound)?;
            record.enabled = enabled;
            policies.push(policy_summary(&record.policy, record.enabled));
        }
        policies.sort_by(|a, b| a.id.cmp(&b.id));
        Ok(policies)
    }

    async fn delete(&self, workspace_id: &str, policy_id: &str) -> Result<(), PolicyStoreError> {
        if self
            .inner
            .write()
            .await
            .remove(&(workspace_id.to_string(), policy_id.to_string()))
            .is_none()
        {
            return Err(PolicyStoreError::NotFound);
        }
        Ok(())
    }

    async fn list_for_agent(
        &self,
        workspace_id: &str,
        agent_id: &str,
    ) -> Result<Vec<PolicySummary>, PolicyStoreError> {
        let mut owned: Vec<_> = self
            .inner
            .read()
            .await
            .iter()
            .filter(|((workspace, _), _)| workspace == workspace_id)
            .map(|(_, record)| record)
            .filter(|record| record.policy.owner_agent_id.as_deref() == Some(agent_id))
            .map(|record| policy_summary(&record.policy, record.enabled))
            .collect();
        owned.sort_by(|a, b| a.id.cmp(&b.id));
        Ok(owned)
    }

    async fn delete_for_agent(
        &self,
        workspace_id: &str,
        agent_id: &str,
    ) -> Result<Vec<String>, PolicyStoreError> {
        // Memory store has no soft-delete state, so cascade = remove.
        // Matches the Postgres-side semantics from the caller's PoV
        // (deleted rows no longer surface in list_for_agent).
        let mut guard = self.inner.write().await;
        let owned_keys: Vec<(String, String)> = guard
            .iter()
            .filter(|((workspace, _), record)| {
                workspace == workspace_id
                    && record.policy.owner_agent_id.as_deref() == Some(agent_id)
            })
            .map(|(key, _)| key.clone())
            .collect();
        for key in &owned_keys {
            guard.remove(key);
        }
        Ok(owned_keys.into_iter().map(|(_, id)| id).collect())
    }

    async fn list_versions(
        &self,
        _workspace_id: &str,
        _policy_id: &str,
    ) -> Result<EntityVersionListResponse, PolicyStoreError> {
        Ok(EntityVersionListResponse { versions: vec![] })
    }

    async fn get_version(
        &self,
        _workspace_id: &str,
        _policy_id: &str,
        _version: i32,
    ) -> Result<EntityVersionDetail, PolicyStoreError> {
        Err(PolicyStoreError::NotFound)
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
    let workspace_id = workspace_id_from_headers(&headers);
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
        .upsert(&workspace_id, &parsed.policy, &parsed.source_yaml)
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
pub async fn list_policies(State(state): State<PolicyState>, headers: HeaderMap) -> Response {
    let workspace_id = workspace_id_from_headers(&headers);
    match state.store.list(&workspace_id).await {
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
pub async fn get_policy(
    State(state): State<PolicyState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Response {
    let workspace_id = workspace_id_from_headers(&headers);
    match state.store.get(&workspace_id, &id).await {
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
    headers: HeaderMap,
    Path(id): Path<String>,
    body: bytes::Bytes,
) -> Response {
    let workspace_id = workspace_id_from_headers(&headers);
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
    match state
        .store
        .set_enabled(&workspace_id, &id, req.enabled)
        .await
    {
        Ok(document) => Json(document).into_response(),
        Err(e) => policy_store_error_response(e),
    }
}

/// `PATCH /v1/policies/batch/enabled` — enable or disable multiple policies.
#[utoipa::path(
    patch,
    path = "/v1/policies/batch/enabled",
    tag = "policies",
    request_body = PolicyBatchSetEnabledRequest,
    responses(
        (status = 200, description = "Updated policies", body = PolicyBatchSetEnabledResponse),
        (status = 400, description = "Malformed request body", body = ApiError),
        (status = 401, description = "Missing or invalid API key", body = ApiError),
        (status = 404, description = "One or more policies were not found", body = ApiError),
    ),
)]
pub async fn batch_set_policy_enabled(
    State(state): State<PolicyState>,
    headers: HeaderMap,
    body: bytes::Bytes,
) -> Response {
    let workspace_id = workspace_id_from_headers(&headers);
    let req: PolicyBatchSetEnabledRequest = match serde_json::from_slice(&body) {
        Ok(req) => req,
        Err(e) => {
            return api_error_response(
                StatusCode::BAD_REQUEST,
                ApiErrorCode::Invalid,
                format!("request body is not valid JSON: {e}"),
            );
        }
    };
    let policy_ids = match normalize_policy_ids(req.ids) {
        Ok(ids) => ids,
        Err(message) => {
            return api_error_response(StatusCode::BAD_REQUEST, ApiErrorCode::Invalid, message);
        }
    };
    match state
        .store
        .batch_set_enabled(&workspace_id, &policy_ids, req.enabled)
        .await
    {
        Ok(policies) => Json(PolicyBatchSetEnabledResponse { policies }).into_response(),
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
pub async fn delete_policy(
    State(state): State<PolicyState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Response {
    let workspace_id = workspace_id_from_headers(&headers);
    match state.store.delete(&workspace_id, &id).await {
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

/// System instructions for `POST /v1/agents/{id}/guardrails:generate`.
/// The model receives the customer's agent system prompt and must emit a
/// **set** of guardrail drafts tailored to that agent — not a single one.
/// PR B wires this constant into the endpoint; PR A only registers the
/// prompt + schema so the surface is reviewable in isolation.
#[allow(dead_code)]
pub(crate) const POLICY_SET_DRAFT_SYSTEM_PROMPT: &str = concat!(
    "You write TrustLoopGuard guardrail policy sets for a single agent.\n",
    "Given the customer's agent system prompt, derive 3–8 policies that protect ",
    "that specific agent from common failure modes. Return a JSON array matching ",
    "the response schema.\n\n",
    "Required coverage (at minimum, when applicable to the agent):\n",
    "- Customer-info / PII leakage (emails, phone numbers, addresses, payment data).\n",
    "- Scope discipline: refuse off-topic requests outside the agent's stated role.\n",
    "- Tone discipline: avoid forbidden tones implied by the prompt.\n",
    "- Hallucinated guarantees: no promises about refunds, SLAs, medical/legal ",
    "outcomes, prices, or anything not explicitly authorized by the prompt.\n",
    "- Role-specific risks the prompt implies (e.g. a baking agent: no medical/",
    "dietary safety claims; a finance agent: no investment advice; a support agent: ",
    "no unauthorized refund commitments).\n\n",
    "Rules for each policy in the array:\n",
    "- `id` is kebab-case (lowercase letters, digits, hyphens). Distinct across the array.\n",
    "- Prefer `match_type` = `literal` for specific phrases; use `regex` for patterns.\n",
    "- Default `action` is `block`. Use `rewrite` only when a clear safe replacement ",
    "exists; in that case set `rewrite` to the replacement text. Otherwise leave ",
    "`rewrite` null.\n",
    "- Use `escalate` for ambiguous high-stakes cases the operator should review.\n",
    "- Do not emit near-duplicates: every entry should cover a distinct risk.\n",
);

/// Reusable policy-draft item schema. Shared by the single-draft endpoint
/// and the multi-draft array schema below so the two surfaces can't drift.
fn policy_draft_item_schema() -> serde_json::Value {
    json!({
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
    })
}

/// Strict JSON schema for the multi-policy draft endpoint. OpenAI's
/// strict mode requires a top-level object, so we wrap the array in
/// `{ "policies": [...] }` rather than returning a bare array.
#[allow(dead_code)]
pub(crate) fn policy_set_draft_json_schema() -> JsonSchema {
    JsonSchema {
        name: "policy_set_draft".to_string(),
        schema: json!({
            "type": "object",
            "additionalProperties": false,
            "required": ["policies"],
            "properties": {
                "policies": {
                    "type": "array",
                    "minItems": 3,
                    "maxItems": 8,
                    "items": policy_draft_item_schema(),
                },
            },
        }),
    }
}

fn policy_draft_json_schema() -> JsonSchema {
    JsonSchema {
        name: "policy_draft".to_string(),
        schema: policy_draft_item_schema(),
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

/// State for the `/v1/agents/{id}/guardrails*` endpoints. Carries both
/// stores plus the LLM client used to derive the policy set.
#[derive(Clone)]
pub struct GuardrailState {
    pub agent_store: Arc<dyn AgentStore>,
    pub policy_store: Arc<dyn PolicyStore>,
    pub draft_llm: Option<Arc<dyn LlmClient>>,
    pub draft_model: String,
}

/// `POST /v1/agents/{id}/guardrails/generate` — derive a set of
/// guardrail policies tailored to an agent's stored `system_prompt`,
/// auto-persist them with `enabled=false`, and return what was saved.
///
/// Callers review the set and flip individual policies on via
/// `PATCH /v1/policies/{id}/enabled`. Runtime checks never see these
/// policies until that happens (because `/v1/check` filters by enabled).
#[utoipa::path(
    post,
    path = "/v1/agents/{id}/guardrails/generate",
    tag = "agents",
    params(("id" = String, Path, description = "Agent identifier")),
    responses(
        (status = 200, description = "Generated and persisted policies", body = GuardrailGenerateResponse),
        (status = 401, description = "Missing or invalid API key", body = ApiError),
        (status = 404, description = "Agent not registered", body = ApiError),
        (status = 422, description = "Agent has no system_prompt to derive guardrails from", body = ApiError),
        (status = 502, description = "LLM provider failed or returned invalid shape", body = ApiError),
        (status = 503, description = "LLM is not configured on this deployment", body = ApiError),
    ),
)]
pub async fn generate_guardrails(
    State(state): State<GuardrailState>,
    headers: HeaderMap,
    Path(agent_id): Path<String>,
) -> Response {
    let workspace_id = workspace_id_from_headers(&headers);
    let agent = match state.agent_store.get(&workspace_id, &agent_id).await {
        Ok(agent) => agent,
        Err(AgentStoreError::NotFound) => {
            return api_error_response(
                StatusCode::NOT_FOUND,
                ApiErrorCode::NotFound,
                format!("agent `{agent_id}` not found"),
            );
        }
        Err(e) => {
            return api_error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                ApiErrorCode::Internal,
                e.to_string(),
            );
        }
    };

    let prompt = match agent.system_prompt.as_deref().map(str::trim) {
        Some(p) if !p.is_empty() => p.to_string(),
        _ => {
            return api_error_response(
                StatusCode::UNPROCESSABLE_ENTITY,
                ApiErrorCode::Unprocessable,
                format!(
                    "agent `{agent_id}` has no system_prompt — set it via POST /v1/agents \
                     before generating guardrails."
                ),
            );
        }
    };

    let Some(client) = state.draft_llm.clone() else {
        return api_error_response(
            StatusCode::SERVICE_UNAVAILABLE,
            ApiErrorCode::Unavailable,
            "guardrail generation is not configured on this deployment (no LLM key)".into(),
        );
    };

    let composed = format!("{POLICY_SET_DRAFT_SYSTEM_PROMPT}\nAgent system prompt:\n{prompt}");
    let schema = policy_set_draft_json_schema();
    let out = match client
        .complete(
            &state.draft_model,
            &composed,
            &schema,
            std::time::Duration::from_secs(60),
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

    let drafts = match parse_policy_set(out.json) {
        Ok(drafts) => drafts,
        Err(message) => {
            return api_error_response(StatusCode::BAD_GATEWAY, ApiErrorCode::Internal, message);
        }
    };

    let mut persisted = Vec::with_capacity(drafts.len());
    let mut seen_ids = std::collections::HashSet::new();
    for draft in drafts {
        // De-dupe at the response boundary even if the model misbehaved.
        if !seen_ids.insert(draft.id.clone()) {
            continue;
        }
        let policy = policy_from_draft(&draft, &agent_id);
        if let Err(issues) = tl_policy::validate_policy(&policy) {
            // One bad draft shouldn't sink the whole batch — skip and
            // keep the rest. Logged so operators notice.
            tracing::warn!(
                draft_id = %policy.id,
                agent_id = %agent_id,
                issues = ?issues,
                "skipping LLM-drafted policy that failed validation"
            );
            continue;
        }
        let source_yaml = match serde_yaml::to_string(&policy) {
            Ok(yaml) => yaml,
            Err(e) => {
                return api_error_response(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    ApiErrorCode::Internal,
                    format!("policy yaml render: {e}"),
                );
            }
        };
        match state
            .policy_store
            .upsert(&workspace_id, &policy, &source_yaml)
            .await
        {
            Ok(document) => persisted.push(document),
            Err(e) => return policy_store_error_response(e),
        }
        // Auto-persist starts disabled — runtime /v1/check ignores it
        // until an operator opts in via PATCH /v1/policies/{id}/enabled.
        if let Err(e) = state
            .policy_store
            .set_enabled(&workspace_id, &policy.id, false)
            .await
        {
            return policy_store_error_response(e);
        }
    }

    // Re-fetch so the returned `enabled` reflects the disabled state.
    let mut response = Vec::with_capacity(persisted.len());
    for document in &persisted {
        match state.policy_store.get(&workspace_id, &document.id).await {
            Ok(refreshed) => response.push(refreshed),
            Err(e) => return policy_store_error_response(e),
        }
    }

    Json(GuardrailGenerateResponse {
        generated: response,
    })
    .into_response()
}

/// `GET /v1/agents/{id}/guardrails` — list active policies owned by an
/// agent. Does not require the agent to exist; missing agent → empty list.
#[utoipa::path(
    get,
    path = "/v1/agents/{id}/guardrails",
    tag = "agents",
    params(("id" = String, Path, description = "Agent identifier")),
    responses(
        (status = 200, description = "Policies owned by the agent", body = GuardrailListResponse),
        (status = 401, description = "Missing or invalid API key", body = ApiError),
    ),
)]
pub async fn list_guardrails(
    State(state): State<GuardrailState>,
    headers: HeaderMap,
    Path(agent_id): Path<String>,
) -> Response {
    let workspace_id = workspace_id_from_headers(&headers);
    match state
        .policy_store
        .list_for_agent(&workspace_id, &agent_id)
        .await
    {
        Ok(policies) => Json(GuardrailListResponse { policies }).into_response(),
        Err(e) => policy_store_error_response(e),
    }
}

/// `GET /v1/policies/:id/versions` — list saved YAML versions newest first.
pub async fn list_policy_versions(
    State(state): State<PolicyState>,
    headers: HeaderMap,
    Path(policy_id): Path<String>,
) -> Response {
    let workspace_id = workspace_id_from_headers(&headers);
    match state.store.list_versions(&workspace_id, &policy_id).await {
        Ok(resp) => Json(resp).into_response(),
        Err(e) => policy_store_error_response(e),
    }
}

/// `GET /v1/policies/:id/versions/:version` — fetch one historical version.
pub async fn get_policy_version(
    State(state): State<PolicyState>,
    headers: HeaderMap,
    Path((policy_id, version)): Path<(String, i32)>,
) -> Response {
    let workspace_id = workspace_id_from_headers(&headers);
    match state
        .store
        .get_version(&workspace_id, &policy_id, version)
        .await
    {
        Ok(detail) => Json(detail).into_response(),
        Err(e) => policy_store_error_response(e),
    }
}

const AI_EDIT_SYSTEM_PROMPT: &str = concat!(
    "You are a TrustLoopGuard policy YAML editor. ",
    "Given the current policy YAML and an instruction, apply the instruction and return ",
    "ONLY the modified YAML — no explanation, no markdown fences, no surrounding text. ",
    "Preserve all unmodified fields exactly. ",
    "Valid fields: id, description, match (literal or regex), action, severity, rewrite, when.",
);

/// `POST /v1/policies/ai-edit` — apply a natural-language instruction to existing
/// policy YAML via LLM and return the modified YAML. Stateless; the caller decides
/// whether to save the result via the normal upsert endpoint.
pub async fn ai_edit_policy(State(state): State<PolicyState>, body: bytes::Bytes) -> Response {
    let req: AiEditRequest = match serde_json::from_slice(&body) {
        Ok(r) => r,
        Err(e) => {
            return api_error_response(
                StatusCode::BAD_REQUEST,
                ApiErrorCode::Invalid,
                format!("request body is not valid JSON: {e}"),
            );
        }
    };
    if req.yaml.trim().is_empty() || req.instruction.trim().is_empty() {
        return api_error_response(
            StatusCode::BAD_REQUEST,
            ApiErrorCode::Invalid,
            "yaml and instruction are required".into(),
        );
    }

    let Some(client) = state.draft_llm.clone() else {
        return api_error_response(
            StatusCode::SERVICE_UNAVAILABLE,
            ApiErrorCode::Unavailable,
            "AI editing is not configured on this deployment (no LLM key)".into(),
        );
    };

    let user_prompt = format!(
        "Current YAML:\n{}\n\nInstruction: {}",
        req.yaml.trim(),
        req.instruction.trim(),
    );

    // Use a simple text-return schema so the model returns raw YAML.
    let schema = tl_llm::JsonSchema {
        name: "yaml_edit_result".to_string(),
        schema: serde_json::json!({
            "type": "object",
            "additionalProperties": false,
            "required": ["yaml"],
            "properties": {
                "yaml": { "type": "string" }
            }
        }),
    };

    let out = match client
        .complete(
            &state.draft_model,
            &format!("{AI_EDIT_SYSTEM_PROMPT}\n\n{user_prompt}"),
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
                format!("LLM provider error: {e}"),
            );
        }
    };

    let yaml = match out.json.get("yaml").and_then(|v| v.as_str()) {
        Some(s) => {
            // Strip markdown fences if the model ignored the strict-mode schema.
            let stripped = s
                .trim()
                .trim_start_matches("```yaml")
                .trim_start_matches("```")
                .trim_end_matches("```")
                .trim();
            stripped.to_string()
        }
        None => {
            return api_error_response(
                StatusCode::BAD_GATEWAY,
                ApiErrorCode::Internal,
                "model returned unexpected shape".into(),
            );
        }
    };

    Json(AiEditResponse { yaml }).into_response()
}

/// Pull the array out of `{ "policies": [...] }` (OpenAI strict-mode
/// requires the wrapper object) and decode each item into a typed draft.
fn parse_policy_set(mut raw: serde_json::Value) -> Result<Vec<PolicyDraft>, String> {
    let arr = raw
        .get_mut("policies")
        .and_then(|v| v.as_array_mut())
        .ok_or_else(|| "model response missing `policies` array".to_string())?;
    let mut drafts = Vec::with_capacity(arr.len());
    for (idx, mut item) in std::mem::take(arr).into_iter().enumerate() {
        // Strict-mode null → absent so it lands as Option::None.
        if item.get("rewrite") == Some(&serde_json::Value::Null) {
            if let Some(obj) = item.as_object_mut() {
                obj.remove("rewrite");
            }
        }
        let draft: PolicyDraft = serde_json::from_value(item)
            .map_err(|e| format!("policies[{idx}] is not a valid policy draft: {e}"))?;
        drafts.push(draft);
    }
    Ok(drafts)
}

/// Convert an LLM-emitted draft into a stored `Policy` scoped to a
/// specific agent. `owner_agent_id` drives cascade delete; the
/// `when.agents` list makes the engine evaluate the policy only for
/// requests targeting that agent.
fn policy_from_draft(draft: &PolicyDraft, agent_id: &str) -> Policy {
    let matcher = match draft.match_type {
        PolicyMatchType::Literal => Matcher::Literal(draft.match_value.clone()),
        PolicyMatchType::Regex => Matcher::Regex(draft.match_value.clone()),
    };
    let action = match draft.action {
        PolicyAction::Block => Action::Block,
        PolicyAction::Rewrite => Action::Rewrite,
        PolicyAction::Escalate => Action::Escalate,
    };
    Policy {
        id: draft.id.clone(),
        description: Some(draft.description.clone()),
        when: WhenClause {
            channels: vec![],
            domains: vec![],
            agents: vec![agent_id.to_string()],
        },
        r#match: MatchClause::Single(matcher),
        action,
        rewrite: draft.rewrite.clone(),
        severity: draft.severity,
        owner_agent_id: Some(agent_id.to_string()),
    }
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
        action: Some(policy_action(&policy.action)),
        enabled,
        owner_agent_id: policy.owner_agent_id.clone(),
    }
}

fn policy_action(action: &Action) -> String {
    match action {
        Action::Allow => "allow",
        Action::Block => "block",
        Action::Rewrite => "rewrite",
        Action::Escalate => "escalate",
    }
    .to_string()
}

fn normalize_policy_ids(ids: Vec<String>) -> Result<Vec<String>, String> {
    let mut normalized = Vec::new();
    for id in ids {
        let id = id.trim();
        if id.is_empty() {
            return Err("policy ids must not be empty".into());
        }
        if !normalized.iter().any(|existing: &String| existing == id) {
            normalized.push(id.to_string());
        }
    }
    if normalized.is_empty() {
        return Err("at least one policy id is required".into());
    }
    Ok(normalized)
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

pub(crate) fn workspace_id_from_headers(headers: &HeaderMap) -> String {
    headers
        .get("x-tlg-workspace-id")
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(DEFAULT_WORKSPACE_ID)
        .to_string()
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
    crate::log_api_error(status, code, &message);
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
    fn policy_set_draft_schema_wraps_array_with_bounds() {
        let schema = policy_set_draft_json_schema();
        assert_eq!(schema.name, "policy_set_draft");
        let policies = &schema.schema["properties"]["policies"];
        assert_eq!(policies["type"], "array");
        assert_eq!(policies["minItems"], 3);
        assert_eq!(policies["maxItems"], 8);
        // Items must match the single-draft schema exactly — same source of
        // truth for both endpoints.
        assert_eq!(policies["items"], policy_draft_item_schema());
    }

    #[test]
    fn policy_set_draft_system_prompt_mentions_required_coverage() {
        // Cheap regression guard: if someone deletes a coverage area from the
        // prompt, the test fails so it gets discussed.
        let p = POLICY_SET_DRAFT_SYSTEM_PROMPT;
        for needle in ["PII", "Scope", "Tone", "Hallucinated"] {
            assert!(p.contains(needle), "prompt should mention {needle}");
        }
    }

    #[test]
    fn policy_error_type_still_formats_for_cli() {
        let err = tl_policy::PolicyError::Validation("id: id is required".into()).to_string();
        assert!(err.contains("policy validation"));
    }
}
