use chrono::{DateTime, Utc};
use diesel::{Insertable, Queryable, Selectable};
use serde_json::Value;
use uuid::Uuid;

use crate::schema::{
    agents, authorization_approvals, authorization_grants, authorization_intents,
    authorization_leases, authorization_receipts, budget_alert_configs, budget_alert_firings,
    entity_versions, escalations, financial_action_events, financial_action_outcomes,
    financial_actions, financial_budget_principal_locks, financial_ledger_entries,
    financial_payment_reservations, financial_payment_sessions, financial_receipts,
    gateway_provider_connections, gateway_routes, github_installation_states, github_installations,
    github_integration_jobs, github_repository_connections, human_review_events,
    llm_budget_principal_locks, llm_budget_reservations, llm_model_prices, llm_usage_events,
    oauth_identities, policies, policy_environment_deployments, redteam_attack_sessions,
    redteam_jobs, redteam_plans, redteam_report_shares, redteam_session_events, run_events, runs,
    tool_metadata, traces, users, workspace_environments,
};

#[derive(Debug, Insertable)]
#[diesel(table_name = agents)]
pub struct NewAgent {
    pub workspace_id: String,
    pub id: String,
    pub profile_yaml: String,
    pub parsed_profile: Value,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = tool_metadata)]
pub struct NewToolMetadata {
    pub workspace_id: String,
    pub tool: String,
    pub side_effect: String,
    pub reversible: bool,
    pub spec: Value,
    pub enabled: bool,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = policies)]
pub struct NewPolicy {
    pub workspace_id: String,
    pub id: String,
    pub policy_yaml: String,
    pub parsed_policy: Value,
    /// Agent that owns this policy. NULL for global policies authored
    /// directly via POST /v1/policies. FK to agents(id) ON DELETE RESTRICT.
    pub owner_agent_id: Option<String>,
    /// Policy family tag. NULL for content policies; e.g. `"financial"` for a
    /// financial-family policy whose `parsed_policy` holds a `FamilyPolicy`.
    pub family: Option<String>,
}

#[derive(Debug, Queryable, Selectable)]
#[diesel(table_name = policies)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct PolicyRecord {
    pub parsed_policy: Value,
    pub policy_yaml: String,
    pub enabled: bool,
    pub owner_agent_id: Option<String>,
    pub family: Option<String>,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = traces)]
pub struct NewTrace {
    pub workspace_id: String,
    pub trace_id: Uuid,
    pub run_id: Option<Uuid>,
    pub run_event_id: Option<Uuid>,
    pub session_id: Option<String>,
    pub environment_id: String,
    pub domain: String,
    pub decision: String,
    pub elapsed_ms: i32,
    pub payload: Value,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = runs)]
pub struct NewRun {
    pub workspace_id: String,
    pub id: Uuid,
    pub environment_id: String,
    pub agent_id: String,
    pub kind: String,
    pub status: String,
    pub external_id: Option<String>,
    pub metadata: Value,
}

#[derive(Debug, Queryable, Selectable)]
#[diesel(table_name = runs)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct RunRecord {
    pub workspace_id: String,
    pub id: Uuid,
    pub environment_id: String,
    pub agent_id: String,
    pub kind: String,
    pub status: String,
    pub external_id: Option<String>,
    pub metadata: Value,
    pub started_at: DateTime<Utc>,
    pub ended_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = run_events)]
pub struct NewRunEvent {
    pub workspace_id: String,
    pub id: Uuid,
    pub run_id: Uuid,
    pub sequence: i32,
    pub kind: String,
    pub label: Option<String>,
    pub input_summary: Option<String>,
    pub output_summary: Option<String>,
    pub metadata: Value,
    pub occurred_at: DateTime<Utc>,
}

#[derive(Debug, Queryable, Selectable)]
#[diesel(table_name = run_events)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct RunEventRecord {
    pub workspace_id: String,
    pub id: Uuid,
    pub run_id: Uuid,
    pub sequence: i32,
    pub kind: String,
    pub label: Option<String>,
    pub input_summary: Option<String>,
    pub output_summary: Option<String>,
    pub metadata: Value,
    pub occurred_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = human_review_events)]
pub struct NewHumanReviewEvent {
    pub workspace_id: String,
    pub id: Uuid,
    pub trace_id: Uuid,
    pub run_id: Option<Uuid>,
    pub run_event_id: Option<Uuid>,
    pub outcome: String,
    pub reviewer_id: Option<String>,
    pub reason_codes: Value,
    pub note: Option<String>,
    pub metadata: Value,
}

#[derive(Debug, Clone, Queryable, Selectable)]
#[diesel(table_name = human_review_events)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct HumanReviewEventRecord {
    pub workspace_id: String,
    pub id: Uuid,
    pub trace_id: Uuid,
    pub run_id: Option<Uuid>,
    pub run_event_id: Option<Uuid>,
    pub outcome: String,
    pub reviewer_id: Option<String>,
    pub reason_codes: Value,
    pub note: Option<String>,
    pub metadata: Value,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = financial_actions)]
pub struct NewFinancialAction {
    pub workspace_id: String,
    pub environment_id: String,
    pub id: Uuid,
    pub idempotency_key: String,
    pub principal_id: String,
    pub action_kind: String,
    pub operation: String,
    pub amount_minor: i64,
    pub currency: String,
    pub counterparty: Option<Value>,
    pub rail: String,
    pub memo: Option<String>,
    pub metadata: Value,
    pub evidence: Value,
}

#[derive(Debug, Clone, Queryable, Selectable)]
#[diesel(table_name = financial_actions)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct FinancialActionRecord {
    pub workspace_id: String,
    pub environment_id: String,
    pub id: Uuid,
    pub idempotency_key: String,
    pub principal_id: String,
    pub action_kind: String,
    pub operation: String,
    pub amount_minor: i64,
    pub currency: String,
    pub counterparty: Option<Value>,
    pub rail: String,
    pub memo: Option<String>,
    pub metadata: Value,
    pub evidence: Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub authorization_intent_id: Option<Uuid>,
    pub execution_status: String,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = authorization_intents)]
pub struct NewAuthorizationIntent {
    pub workspace_id: String,
    pub environment_id: String,
    pub id: Uuid,
    pub domain: String,
    pub subject_id: String,
    pub idempotency_key: String,
    pub principal_id: String,
    pub operation: String,
    pub fingerprint: String,
    pub fingerprint_version: i32,
    pub subject_snapshot: Value,
    pub status: String,
    pub current_effect: String,
    pub reason: String,
    pub trace_id: Option<String>,
    pub expires_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Queryable, Selectable)]
#[diesel(table_name = authorization_intents)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct AuthorizationIntentRecord {
    pub workspace_id: String,
    pub environment_id: String,
    pub id: Uuid,
    pub domain: String,
    pub subject_id: String,
    pub idempotency_key: String,
    pub principal_id: String,
    pub operation: String,
    pub fingerprint: String,
    pub fingerprint_version: i32,
    pub subject_snapshot: Value,
    pub status: String,
    pub current_effect: String,
    pub reason: String,
    pub trace_id: Option<String>,
    pub expires_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = authorization_approvals)]
pub struct NewAuthorizationApproval {
    pub workspace_id: String,
    pub environment_id: String,
    pub id: Uuid,
    pub intent_id: Uuid,
    pub fingerprint: String,
    pub status: String,
    pub envelope: Value,
    pub envelope_hash: String,
    pub requirement_ids: Value,
    pub approver_roles: Value,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Queryable, Selectable)]
#[diesel(table_name = authorization_approvals)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct AuthorizationApprovalRecord {
    pub workspace_id: String,
    pub environment_id: String,
    pub id: Uuid,
    pub intent_id: Uuid,
    pub fingerprint: String,
    pub status: String,
    pub envelope: Value,
    pub envelope_hash: String,
    pub requirement_ids: Value,
    pub approver_roles: Value,
    pub decided_by: Option<String>,
    pub decided_at: Option<DateTime<Utc>>,
    pub decision_reason: Option<String>,
    pub expires_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = authorization_grants)]
pub struct NewAuthorizationGrant {
    pub workspace_id: String,
    pub environment_id: String,
    pub id: Uuid,
    pub principal_id: String,
    pub domain: String,
    pub capability: String,
    pub mode: String,
    pub status: String,
    pub source: String,
    pub scope_schema: String,
    pub scope: Option<Value>,
    pub exact_fingerprint: Option<String>,
    pub fingerprint_version: i32,
    pub source_approval_id: Option<Uuid>,
    pub requirement_ids: Value,
    pub max_uses: Option<i32>,
    pub starts_at: Option<DateTime<Utc>>,
    pub expires_at: Option<DateTime<Utc>>,
    pub created_by: String,
}

#[derive(Debug, Clone, Queryable, Selectable)]
#[diesel(table_name = authorization_grants)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct AuthorizationGrantRecord {
    pub workspace_id: String,
    pub environment_id: String,
    pub id: Uuid,
    pub principal_id: String,
    pub domain: String,
    pub capability: String,
    pub mode: String,
    pub status: String,
    pub source: String,
    pub scope_schema: String,
    pub scope: Option<Value>,
    pub exact_fingerprint: Option<String>,
    pub fingerprint_version: i32,
    pub source_approval_id: Option<Uuid>,
    pub requirement_ids: Value,
    pub max_uses: Option<i32>,
    pub use_count: i32,
    pub starts_at: Option<DateTime<Utc>>,
    pub expires_at: Option<DateTime<Utc>>,
    pub revoked_at: Option<DateTime<Utc>>,
    pub revoked_by: Option<String>,
    pub created_by: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = authorization_leases)]
pub struct NewAuthorizationLease {
    pub workspace_id: String,
    pub environment_id: String,
    pub id: Uuid,
    pub intent_id: Uuid,
    pub grant_id: Option<Uuid>,
    pub attempt_id: String,
    pub fingerprint: String,
    pub status: String,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Queryable, Selectable)]
#[diesel(table_name = authorization_leases)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct AuthorizationLeaseRecord {
    pub workspace_id: String,
    pub environment_id: String,
    pub id: Uuid,
    pub intent_id: Uuid,
    pub grant_id: Option<Uuid>,
    pub attempt_id: String,
    pub fingerprint: String,
    pub status: String,
    pub claimed_at: DateTime<Utc>,
    pub consumed_at: Option<DateTime<Utc>>,
    pub canceled_at: Option<DateTime<Utc>>,
    pub expires_at: DateTime<Utc>,
    pub outcome: Value,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = authorization_receipts)]
pub struct NewAuthorizationReceipt {
    pub workspace_id: String,
    pub environment_id: String,
    pub id: Uuid,
    pub intent_id: Option<Uuid>,
    pub trace_id: Option<String>,
    pub domain: String,
    pub effect: String,
    pub intent_status: Option<String>,
    pub subject_hash: String,
    pub reason: String,
    pub findings: Value,
    pub policy_versions: Value,
    pub approval_id: Option<Uuid>,
    pub grant_id: Option<Uuid>,
    pub lease_id: Option<Uuid>,
    pub domain_evidence: Value,
}

#[derive(Debug, Clone, Queryable, Selectable)]
#[diesel(table_name = authorization_receipts)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct AuthorizationReceiptRecord {
    pub workspace_id: String,
    pub environment_id: String,
    pub id: Uuid,
    pub intent_id: Option<Uuid>,
    pub trace_id: Option<String>,
    pub domain: String,
    pub effect: String,
    pub intent_status: Option<String>,
    pub subject_hash: String,
    pub reason: String,
    pub findings: Value,
    pub policy_versions: Value,
    pub approval_id: Option<Uuid>,
    pub grant_id: Option<Uuid>,
    pub lease_id: Option<Uuid>,
    pub domain_evidence: Value,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = financial_action_events)]
pub struct NewFinancialActionEvent {
    pub workspace_id: String,
    pub id: Uuid,
    pub action_id: Uuid,
    pub event_type: String,
    pub from_status: Option<String>,
    pub to_status: Option<String>,
    pub actor_id: Option<String>,
    pub reason: Option<String>,
    pub metadata: Value,
}

#[derive(Debug, Clone, Queryable, Selectable)]
#[diesel(table_name = financial_action_events)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct FinancialActionEventRecord {
    pub workspace_id: String,
    pub id: Uuid,
    pub action_id: Uuid,
    pub event_type: String,
    pub from_status: Option<String>,
    pub to_status: Option<String>,
    pub actor_id: Option<String>,
    pub reason: Option<String>,
    pub metadata: Value,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = financial_ledger_entries)]
pub struct NewFinancialLedgerEntry {
    pub workspace_id: String,
    pub id: Uuid,
    pub action_id: Uuid,
    pub entry_kind: String,
    pub amount_minor: i64,
    pub currency: String,
    pub idempotency_key: String,
    pub metadata: Value,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = financial_budget_principal_locks)]
pub struct NewFinancialBudgetPrincipalLock {
    pub workspace_id: String,
    pub principal_id: String,
    pub currency: String,
}

#[derive(Debug, Clone, Queryable, Selectable)]
#[diesel(table_name = financial_ledger_entries)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct FinancialLedgerEntryRecord {
    pub workspace_id: String,
    pub id: Uuid,
    pub action_id: Uuid,
    pub entry_kind: String,
    pub amount_minor: i64,
    pub currency: String,
    pub idempotency_key: String,
    pub metadata: Value,
    pub effective_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = financial_payment_sessions)]
pub struct NewFinancialPaymentSession {
    pub workspace_id: String,
    pub id: String,
    pub principal_id: String,
    pub currency: String,
    pub max_amount_minor: i64,
    pub expires_at: DateTime<Utc>,
    pub metadata: Value,
}

#[derive(Debug, Clone, Queryable, Selectable)]
#[diesel(table_name = financial_payment_sessions)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct FinancialPaymentSessionRecord {
    pub workspace_id: String,
    pub id: String,
    pub principal_id: String,
    pub currency: String,
    pub max_amount_minor: i64,
    pub reserved_minor: i64,
    pub committed_minor: i64,
    pub released_minor: i64,
    pub status: String,
    pub expires_at: DateTime<Utc>,
    pub metadata: Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = financial_payment_reservations)]
pub struct NewFinancialPaymentReservation {
    pub workspace_id: String,
    pub id: Uuid,
    pub action_id: Uuid,
    pub session_id: String,
    pub principal_id: String,
    pub payment_requirement_hash: String,
    pub amount_minor: i64,
    pub currency: String,
    pub expires_at: DateTime<Utc>,
    pub metadata: Value,
}

#[derive(Debug, Clone, Queryable, Selectable)]
#[diesel(table_name = financial_payment_reservations)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct FinancialPaymentReservationRecord {
    pub workspace_id: String,
    pub id: Uuid,
    pub action_id: Uuid,
    pub session_id: String,
    pub principal_id: String,
    pub payment_requirement_hash: String,
    pub amount_minor: i64,
    pub currency: String,
    pub status: String,
    pub expires_at: DateTime<Utc>,
    pub commit_proof: Option<Value>,
    pub metadata: Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub committed_at: Option<DateTime<Utc>>,
    pub released_at: Option<DateTime<Utc>>,
}

/// Upsert row for a workspace model price. `currency` is omitted — the
/// table default (`USD`) applies; v1 pricing is USD-only.
#[derive(Debug, Insertable)]
#[diesel(table_name = llm_model_prices)]
pub struct NewLlmModelPrice {
    pub workspace_id: String,
    pub model: String,
    pub input_per_million_minor: i64,
    pub output_per_million_minor: i64,
    pub input_per_million_nanos: i64,
    pub output_per_million_nanos: i64,
}

#[derive(Debug, Clone, Queryable, Selectable)]
#[diesel(table_name = llm_model_prices)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct LlmModelPriceRecord {
    pub workspace_id: String,
    pub model: String,
    pub input_per_million_minor: i64,
    pub output_per_million_minor: i64,
    pub input_per_million_nanos: i64,
    pub output_per_million_nanos: i64,
    pub currency: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = budget_alert_configs)]
pub struct NewBudgetAlertConfig {
    pub id: Uuid,
    pub workspace_id: String,
    pub name: String,
    pub meter: String,
    pub window: String,
    pub principal_id: Option<String>,
    pub threshold_type: String,
    pub threshold_value: i64,
    pub webhook_url: Option<String>,
    pub enabled: bool,
}

#[derive(Debug, Clone, Queryable, Selectable)]
#[diesel(table_name = budget_alert_configs)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct BudgetAlertConfigRecord {
    pub id: Uuid,
    pub workspace_id: String,
    pub name: String,
    pub meter: String,
    pub window: String,
    pub principal_id: Option<String>,
    pub threshold_type: String,
    pub threshold_value: i64,
    pub webhook_url: Option<String>,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = budget_alert_firings)]
pub struct NewBudgetAlertFiring {
    pub id: Uuid,
    pub workspace_id: String,
    pub config_id: Uuid,
    pub meter: String,
    pub principal_id: String,
    pub window_start: DateTime<Utc>,
    pub cap_minor: i64,
    pub spent_minor: i64,
    pub currency: String,
    pub payload: Value,
}

#[derive(Debug, Clone, Queryable, Selectable)]
#[diesel(table_name = budget_alert_firings)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct BudgetAlertFiringRecord {
    pub id: Uuid,
    pub workspace_id: String,
    pub config_id: Uuid,
    pub meter: String,
    pub principal_id: String,
    pub window_start: DateTime<Utc>,
    pub cap_minor: i64,
    pub spent_minor: i64,
    pub currency: String,
    pub payload: Value,
    pub fired_at: DateTime<Utc>,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = llm_usage_events)]
pub struct NewLlmUsageEvent {
    pub workspace_id: String,
    pub id: Uuid,
    pub principal_id: String,
    pub api_key_id: String,
    pub usage_kind: String,
    pub model: String,
    pub prompt_tokens: i64,
    pub completion_tokens: i64,
    pub cost_minor: i64,
    pub cost_nanos: i64,
    pub currency: String,
    pub request_id: String,
    pub metadata: Value,
}

#[derive(Debug, Clone, Queryable, Selectable)]
#[diesel(table_name = llm_usage_events)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct LlmUsageEventRecord {
    pub workspace_id: String,
    pub id: Uuid,
    pub principal_id: String,
    pub api_key_id: String,
    pub usage_kind: String,
    pub model: String,
    pub prompt_tokens: i64,
    pub completion_tokens: i64,
    pub cost_minor: i64,
    pub cost_nanos: i64,
    pub currency: String,
    pub request_id: String,
    pub metadata: Value,
    pub effective_at: DateTime<Utc>,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = llm_budget_principal_locks)]
pub struct NewLlmBudgetPrincipalLock {
    pub workspace_id: String,
    pub principal_id: String,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = llm_budget_reservations)]
pub struct NewLlmBudgetReservation {
    pub workspace_id: String,
    pub request_id: String,
    pub principal_id: String,
    pub api_key_id: String,
    pub currency: String,
    pub reserved_nanos: i64,
    pub actual_nanos: Option<i64>,
    pub status: String,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = financial_receipts)]
pub struct NewFinancialReceipt {
    pub workspace_id: String,
    pub environment_id: String,
    pub id: Uuid,
    pub action_id: Uuid,
    pub authorization_receipt_id: Option<Uuid>,
    pub trace_id: Option<Uuid>,
    pub ledger_event_ids: Value,
    pub proof: Value,
}

#[derive(Debug, Clone, Queryable, Selectable)]
#[diesel(table_name = financial_receipts)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct FinancialReceiptRecord {
    pub workspace_id: String,
    pub environment_id: String,
    pub id: Uuid,
    pub action_id: Uuid,
    pub authorization_receipt_id: Option<Uuid>,
    pub trace_id: Option<Uuid>,
    pub ledger_event_ids: Value,
    pub proof: Value,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = financial_action_outcomes)]
pub struct NewFinancialActionOutcome {
    pub workspace_id: String,
    pub id: Uuid,
    pub action_id: Uuid,
    pub status: String,
    pub reversal_capability: String,
    pub recovery_status: String,
    pub provider_status: Option<String>,
    pub provider_reference: Option<String>,
    pub final_loss_amount_minor: Option<i64>,
    pub final_loss_currency: Option<String>,
    pub occurred_at: DateTime<Utc>,
    pub metadata: Value,
}

#[derive(Debug, Clone, Queryable, Selectable)]
#[diesel(table_name = financial_action_outcomes)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct FinancialActionOutcomeRecord {
    pub workspace_id: String,
    pub id: Uuid,
    pub action_id: Uuid,
    pub status: String,
    pub reversal_capability: String,
    pub recovery_status: String,
    pub provider_status: Option<String>,
    pub provider_reference: Option<String>,
    pub final_loss_amount_minor: Option<i64>,
    pub final_loss_currency: Option<String>,
    pub occurred_at: DateTime<Utc>,
    pub metadata: Value,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = escalations)]
pub struct NewEscalation {
    pub id: Uuid,
    pub trace_id: Uuid,
    pub webhook_url: String,
    pub status: String,
    pub attempts: i32,
    pub payload: Value,
}

#[derive(Debug, Queryable, Selectable)]
#[diesel(table_name = escalations)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct EscalationRecord {
    pub id: Uuid,
    pub trace_id: Uuid,
    pub webhook_url: String,
    pub status: String,
    pub attempts: i32,
    pub payload: Value,
    pub created_at: DateTime<Utc>,
    pub sent_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = workspace_environments)]
pub struct NewWorkspaceEnvironment {
    pub workspace_id: String,
    pub id: String,
    pub slug: String,
    pub name: String,
    pub description: Option<String>,
    pub is_default: bool,
}

#[derive(Debug, Queryable, Selectable)]
#[diesel(table_name = workspace_environments)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct WorkspaceEnvironmentRecord {
    pub workspace_id: String,
    pub id: String,
    pub slug: String,
    pub name: String,
    pub description: Option<String>,
    pub is_default: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = policy_environment_deployments)]
pub struct NewPolicyEnvironmentDeployment {
    pub workspace_id: String,
    pub environment_id: String,
    pub policy_id: String,
    pub enabled: bool,
    pub deployed_version: Option<i32>,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = users)]
pub struct NewUser {
    pub id: Uuid,
    pub username: String,
    pub password_hash: String,
}

#[derive(Debug, Queryable, Selectable)]
#[diesel(table_name = users)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct UserRecord {
    pub id: Uuid,
    pub username: String,
    pub password_hash: String,
    pub is_approved: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = oauth_identities)]
pub struct NewOAuthIdentity {
    pub provider: String,
    pub provider_subject: String,
    pub user_id: Uuid,
    pub email: String,
}

#[derive(Debug, Queryable, Selectable)]
#[diesel(table_name = oauth_identities)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct OAuthIdentityRecord {
    pub provider: String,
    pub provider_subject: String,
    pub user_id: Uuid,
    pub email: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = gateway_provider_connections)]
pub struct NewGatewayProviderConnection {
    pub workspace_id: String,
    pub id: String,
    pub display_name: String,
    pub kind: String,
    pub base_url: Option<String>,
    pub default_model: String,
    pub encrypted_api_key: String,
}

#[derive(Debug, Queryable, Selectable)]
#[diesel(table_name = gateway_provider_connections)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct GatewayProviderConnectionRecord {
    pub workspace_id: String,
    pub id: String,
    pub display_name: String,
    pub kind: String,
    pub base_url: Option<String>,
    pub default_model: String,
    pub encrypted_api_key: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = entity_versions)]
pub struct NewEntityVersion {
    pub workspace_id: String,
    pub entity_type: String,
    pub entity_id: String,
    pub version: i32,
    pub content: String,
}

#[derive(Debug, Queryable, Selectable)]
#[diesel(table_name = entity_versions)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct EntityVersionRecord {
    pub workspace_id: String,
    pub entity_type: String,
    pub entity_id: String,
    pub version: i32,
    pub content: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = gateway_routes)]
pub struct NewGatewayRoute {
    pub workspace_id: String,
    pub id: String,
    pub display_name: String,
    pub provider_connection_id: String,
    pub agent_id: String,
}

#[derive(Debug, Queryable, Selectable)]
#[diesel(table_name = gateway_routes)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct GatewayRouteRecord {
    pub workspace_id: String,
    pub id: String,
    pub display_name: String,
    pub provider_connection_id: String,
    pub agent_id: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = github_installation_states)]
pub struct NewGitHubInstallationState {
    pub state_hash: Vec<u8>,
    pub workspace_id: String,
    pub user_id: Uuid,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Queryable, Selectable)]
#[diesel(table_name = github_installation_states)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct GitHubInstallationStateRecord {
    pub state_hash: Vec<u8>,
    pub workspace_id: String,
    pub user_id: Uuid,
    pub expires_at: DateTime<Utc>,
    pub consumed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = github_installations)]
pub struct NewGitHubInstallation {
    pub workspace_id: String,
    pub id: Uuid,
    pub installation_id: i64,
    pub account_login: String,
    pub account_type: String,
    pub repository_selection: String,
    pub status: String,
    pub installed_by_user_id: Uuid,
}

#[derive(Debug, Queryable, Selectable)]
#[diesel(table_name = github_installations)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct GitHubInstallationRecord {
    pub workspace_id: String,
    pub id: Uuid,
    pub installation_id: i64,
    pub account_login: String,
    pub account_type: String,
    pub repository_selection: String,
    pub status: String,
    pub installed_by_user_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = github_repository_connections)]
pub struct NewGitHubRepositoryConnection {
    pub workspace_id: String,
    pub id: Uuid,
    pub installation_id: Uuid,
    pub repository_id: i64,
    pub owner: String,
    pub name: String,
    pub default_branch: String,
    pub root_path: String,
    pub agent_id: String,
    pub environment_id: String,
    pub status: String,
    pub recipe_version: String,
}

#[derive(Debug, Queryable, Selectable)]
#[diesel(table_name = github_repository_connections)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct GitHubRepositoryConnectionRecord {
    pub workspace_id: String,
    pub id: Uuid,
    pub installation_id: Uuid,
    pub repository_id: i64,
    pub owner: String,
    pub name: String,
    pub default_branch: String,
    pub root_path: String,
    pub agent_id: String,
    pub environment_id: String,
    pub status: String,
    pub recipe_version: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = github_integration_jobs)]
pub struct NewGitHubIntegrationJob {
    pub workspace_id: String,
    pub id: Uuid,
    pub connection_id: Uuid,
    pub status: String,
    pub risk_statement: String,
    pub base_branch: String,
    pub base_sha: Option<String>,
    pub recipe_version: String,
    pub proposed_changes: Value,
    pub manual_steps: Value,
    pub installation_connected_at: Option<DateTime<Utc>>,
    pub repository_connected_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Queryable, Selectable)]
#[diesel(table_name = github_integration_jobs)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct GitHubIntegrationJobRecord {
    pub workspace_id: String,
    pub id: Uuid,
    pub connection_id: Uuid,
    pub status: String,
    pub risk_statement: String,
    pub base_branch: String,
    pub base_sha: Option<String>,
    pub recipe_version: String,
    pub analysis_summary: Option<Value>,
    pub proposed_changes: Value,
    pub manual_steps: Value,
    pub branch_name: Option<String>,
    pub commit_sha: Option<String>,
    pub pull_request_number: Option<i64>,
    pub pull_request_url: Option<String>,
    pub error_code: Option<String>,
    pub error_message: Option<String>,
    pub attempt_count: i32,
    pub installation_connected_at: Option<DateTime<Utc>>,
    pub repository_connected_at: Option<DateTime<Utc>>,
    pub analysis_completed_at: Option<DateTime<Utc>>,
    pub pr_opened_at: Option<DateTime<Utc>>,
    pub pr_merged_at: Option<DateTime<Utc>>,
    pub first_verified_trace_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = redteam_jobs)]
pub struct NewRedteamJob {
    pub workspace_id: String,
    pub id: Uuid,
    pub environment_id: String,
    pub status: String,
    pub target: String,
    pub profile: String,
    pub generator: String,
    pub agent_id: Option<String>,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = redteam_plans)]
pub struct NewRedteamPlan {
    pub workspace_id: String,
    pub id: Uuid,
    pub environment_id: String,
    pub agent_id: String,
    pub name: String,
    pub plan: Value,
}

#[derive(Debug, Queryable, Selectable)]
#[diesel(table_name = redteam_plans)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct RedteamPlanRecord {
    pub workspace_id: String,
    pub id: Uuid,
    pub environment_id: String,
    pub agent_id: String,
    pub name: String,
    pub plan: Value,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Queryable, Selectable)]
#[diesel(table_name = redteam_jobs)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct RedteamJobRecord {
    pub workspace_id: String,
    pub id: Uuid,
    pub environment_id: String,
    pub status: String,
    pub target: String,
    pub profile: String,
    pub generator: String,
    pub agent_id: Option<String>,
    pub attacks: i64,
    pub landed: i64,
    pub blocked: i64,
    pub error: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = redteam_attack_sessions)]
pub struct NewRedteamAttackSession {
    pub workspace_id: String,
    pub job_id: Uuid,
    pub session_id: String,
    pub runner_session_id: Option<String>,
    pub seq: i32,
    pub case_id: Option<String>,
    pub track: Option<String>,
    pub kind: Option<String>,
    pub trial_index: Option<i32>,
    pub attack: String,
    pub goal: String,
    pub status: String,
    pub outcome: String,
    pub landed: bool,
    pub trace_id: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Queryable, Selectable)]
#[diesel(table_name = redteam_attack_sessions)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct RedteamAttackSessionRecord {
    pub workspace_id: String,
    pub job_id: Uuid,
    pub session_id: String,
    pub runner_session_id: Option<String>,
    pub seq: i32,
    pub case_id: Option<String>,
    pub track: Option<String>,
    pub kind: Option<String>,
    pub trial_index: Option<i32>,
    pub attack: String,
    pub goal: String,
    pub status: String,
    pub outcome: String,
    pub landed: bool,
    pub trace_id: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = redteam_session_events)]
pub struct NewRedteamSessionEvent {
    pub workspace_id: String,
    pub job_id: Uuid,
    pub session_id: String,
    pub event_id: String,
    pub seq: i32,
    pub kind: String,
    pub actor: String,
    pub label: Option<String>,
    pub content_text: Option<String>,
    pub payload: Value,
    pub trace_id: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Queryable, Selectable)]
#[diesel(table_name = redteam_session_events)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct RedteamSessionEventRecord {
    pub workspace_id: String,
    pub job_id: Uuid,
    pub session_id: String,
    pub event_id: String,
    pub seq: i32,
    pub kind: String,
    pub actor: String,
    pub label: Option<String>,
    pub content_text: Option<String>,
    pub payload: Value,
    pub trace_id: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = redteam_report_shares)]
pub struct NewRedteamReportShare {
    pub token: String,
    pub workspace_id: String,
    pub job_id: Uuid,
    pub compare_job_id: Option<Uuid>,
    pub expires_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Queryable, Selectable)]
#[diesel(table_name = redteam_report_shares)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct RedteamReportShareRecord {
    pub token: String,
    pub workspace_id: String,
    pub job_id: Uuid,
    pub compare_job_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    pub revoked_at: Option<DateTime<Utc>>,
}

/// A `redteam_attack_sessions` row joined with its parent `redteam_jobs` context.
///
/// This spans two tables, so it is plain `Queryable` (no `table_name`/`Selectable`):
/// it is loaded positionally from an explicit `.select((...))` tuple. Field order
/// MUST match that tuple in `RedteamJobRepo::list_attack_records`.
#[derive(Debug, Queryable)]
pub struct RedteamAttackRecordRow {
    pub job_id: Uuid,
    pub target: String,
    pub profile: String,
    pub created_at: DateTime<Utc>,
    pub session_id: String,
    pub seq: i32,
    pub attack: String,
    pub goal: String,
    pub outcome: String,
    pub landed: bool,
    pub trace_id: Option<String>,
}
