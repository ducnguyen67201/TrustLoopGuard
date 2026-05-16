use chrono::{DateTime, Utc};
use diesel::{Insertable, Queryable, Selectable};
use serde_json::Value;
use uuid::Uuid;

use crate::schema::{agents, escalations, policies, traces, users};

#[derive(Debug, Insertable)]
#[diesel(table_name = agents)]
pub struct NewAgent {
    pub workspace_id: String,
    pub id: String,
    pub profile_yaml: String,
    pub parsed_profile: Value,
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
    pub domain: String,
    pub decision: String,
    pub elapsed_ms: i32,
    pub payload: Value,
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
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
