use chrono::{DateTime, Utc};
use diesel::{Insertable, Queryable, Selectable};
use serde_json::Value;
use uuid::Uuid;

use crate::schema::{
    agents, enforcement_profiles, entity_versions, escalations, gateway_provider_connections,
    gateway_routes, human_review_events, oauth_identities, policies,
    policy_environment_deployments, run_events, runs, source_label_policy, tool_metadata, traces,
    users, workspace_environments,
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
#[diesel(table_name = source_label_policy)]
pub struct NewSourceLabelPolicy {
    pub workspace_id: String,
    pub origin: String,
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
}

#[derive(Debug, Queryable, Selectable)]
#[diesel(table_name = policies)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct PolicyRecord {
    pub parsed_policy: Value,
    pub policy_yaml: String,
    pub enabled: bool,
    pub owner_agent_id: Option<String>,
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
#[diesel(table_name = enforcement_profiles)]
pub struct NewEnforcementProfile {
    pub workspace_id: String,
    pub id: String,
    pub display_name: String,
    pub input_action: String,
    pub output_action: String,
    pub fail_mode: String,
    pub retention_mode: String,
    pub response_mode: String,
    pub fallback_message: String,
    pub max_regenerations: i32,
}

#[derive(Debug, Queryable, Selectable)]
#[diesel(table_name = enforcement_profiles)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct EnforcementProfileRecord {
    pub workspace_id: String,
    pub id: String,
    pub display_name: String,
    pub input_action: String,
    pub output_action: String,
    pub fail_mode: String,
    pub retention_mode: String,
    pub response_mode: String,
    pub fallback_message: String,
    pub max_regenerations: i32,
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
    pub enforcement_profile_id: String,
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
    pub enforcement_profile_id: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
