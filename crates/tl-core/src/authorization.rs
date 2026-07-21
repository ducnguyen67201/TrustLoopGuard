//! Canonical authorization contracts shared by every runtime domain.
//!
//! Policies and domain adapters emit findings. The authorization coordinator
//! composes those findings, resolves explicit authority requirements against
//! grants, and issues a one-attempt lease. Approval state is deliberately
//! separate from grant use and execution state.

use serde::{Deserialize, Serialize};

use crate::{
    Channel, EventKind, FinancialAction, FinancialActionKind, FinancialActionPrecondition,
    FinancialRail, Severity, SideEffectClass, ToolIdentity,
};

#[cfg(feature = "schema")]
use schemars::JsonSchema;
#[cfg(feature = "ts-export")]
use ts_rs::TS;
#[cfg(feature = "openapi")]
use utoipa::ToSchema;

macro_rules! wire_enum {
    ($(#[$meta:meta])* $name:ident { $($variant:ident),+ $(,)? }) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
        #[serde(rename_all = "snake_case")]
        #[cfg_attr(feature = "schema", derive(JsonSchema))]
        #[cfg_attr(feature = "openapi", derive(ToSchema))]
        #[cfg_attr(feature = "ts-export", derive(TS))]
        #[cfg_attr(feature = "ts-export", ts(export))]
        pub enum $name { $($variant),+ }
    };
}

wire_enum!(AuthorizationDomain {
    Content,
    Tool,
    Financial
});
wire_enum!(AuthorizationIntentStatus {
    Evaluating,
    PendingApproval,
    Authorized,
    Denied,
    Deferred,
    Canceled,
    Expired,
});
wire_enum!(ApprovalStatus {
    Pending,
    Approved,
    Denied,
    Canceled,
    Expired
});
wire_enum!(GrantStatus {
    Active,
    Revoked,
    Expired,
    Exhausted
});
wire_enum!(GrantMode { ExactOnce, Scoped });
wire_enum!(LeaseStatus {
    Claimed,
    Consumed,
    Canceled,
    Expired
});
wire_enum!(AuthorizationGrantSource {
    UserIntent,
    ReviewerApproval,
    WorkspaceAdmin
});
wire_enum!(FinancialExecutionStatus {
    NotStarted,
    Executing,
    Succeeded,
    Failed,
    Canceled,
    Reversed,
});

/// The only runtime decision vocabulary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub enum AuthorizationEffect {
    Permit,
    Deny,
    Transform,
    RequireApproval,
    Defer,
}

impl AuthorizationEffect {
    const fn precedence(self) -> u8 {
        match self {
            Self::Permit => 0,
            Self::Transform => 1,
            Self::RequireApproval => 2,
            Self::Defer => 3,
            Self::Deny => 4,
        }
    }

    /// Compose effects without ever weakening an existing result.
    pub fn worst_with(self, other: Self) -> Self {
        if other.precedence() > self.precedence() {
            other
        } else {
            self
        }
    }

    pub const fn is_executable(self) -> bool {
        matches!(self, Self::Permit | Self::Transform)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct AuthorizationCapabilityId(String);

impl AuthorizationCapabilityId {
    pub fn parse(value: impl Into<String>) -> Result<Self, &'static str> {
        let value = value.into();
        let valid = value.len() <= 160
            && value
                .split_once(':')
                .is_some_and(|(namespace, name)| !namespace.is_empty() && !name.is_empty())
            && value.bytes().all(|byte| {
                byte.is_ascii_lowercase()
                    || byte.is_ascii_digit()
                    || matches!(byte, b':' | b'/' | b'_' | b'-' | b'.')
            });
        valid.then_some(Self(value)).ok_or(
            "capability must be a lowercase namespaced identifier such as action:external_message",
        )
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for AuthorizationCapabilityId {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "domain", rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub enum AuthorizationSubject {
    Content {
        event_kind: EventKind,
        channel: Channel,
        input: String,
        output: String,
    },
    Tool {
        invocation_id: String,
        operation: String,
        tool_identity: ToolIdentity,
        #[cfg_attr(feature = "ts-export", ts(type = "Record<string, unknown> | null"))]
        parameters: serde_json::Value,
        side_effect: SideEffectClass,
    },
    Financial {
        action_id: String,
        action: FinancialAction,
    },
}

impl AuthorizationSubject {
    pub const fn domain(&self) -> AuthorizationDomain {
        match self {
            Self::Content { .. } => AuthorizationDomain::Content,
            Self::Tool { .. } => AuthorizationDomain::Tool,
            Self::Financial { .. } => AuthorizationDomain::Financial,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct ActionGrantScope {
    #[serde(default)]
    pub operations: Vec<String>,
    #[serde(default)]
    pub side_effects: Vec<SideEffectClass>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub server_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub tool_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub schema_hash: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(
        feature = "ts-export",
        ts(optional, type = "Record<string, unknown> | null")
    )]
    pub parameters: Option<serde_json::Value>,
    #[serde(default)]
    pub allowed_destinations: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub maximum_data_confidentiality: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub minimum_source_trust: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct FinancialGrantScope {
    #[serde(default)]
    pub action_kinds: Vec<FinancialActionKind>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub operation: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub rail: Option<FinancialRail>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub currency: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub maximum_amount_minor: Option<i64>,
    #[serde(default)]
    pub counterparties: Vec<String>,
    #[serde(default)]
    pub x402_hosts: Vec<String>,
    #[serde(default)]
    pub x402_resources: Vec<String>,
    #[serde(default)]
    pub x402_networks: Vec<String>,
    #[serde(default)]
    pub x402_assets: Vec<String>,
    #[serde(default)]
    pub x402_payees: Vec<String>,
    #[serde(default)]
    pub required_preconditions: Vec<FinancialActionPrecondition>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "scope_type", content = "scope", rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub enum AuthorizationGrantScope {
    Action(ActionGrantScope),
    Financial(FinancialGrantScope),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct AuthorityRequirement {
    pub id: String,
    pub capability: AuthorizationCapabilityId,
    pub required_scope: AuthorizationGrantScope,
    #[serde(default)]
    pub approver_roles: Vec<String>,
    pub reason: String,
    pub reusable_allowed: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub max_grant_ttl_seconds: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct AuthorizationFinding {
    pub id: String,
    pub source: String,
    pub effect: AuthorizationEffect,
    pub reason: String,
    pub severity: Severity,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub policy_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub requirement_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub remediation: Option<String>,
    #[serde(default)]
    #[cfg_attr(feature = "ts-export", ts(type = "Record<string, unknown> | null"))]
    pub evidence: serde_json::Value,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct AuthorizationClaim {
    pub grant_id: String,
    pub attempt_id: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct ApprovalEnvelope {
    pub schema: String,
    pub intent_id: String,
    pub domain: AuthorizationDomain,
    pub capability: AuthorizationCapabilityId,
    pub principal_id: String,
    pub subject_id: String,
    pub subject_hash: String,
    pub exact_fingerprint: String,
    pub fingerprint_version: i32,
    #[serde(default)]
    pub requirement_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub proposed_scope: Option<AuthorizationGrantScope>,
    #[serde(default)]
    pub policy_versions: Vec<String>,
    pub issued_at: String,
    pub expires_at: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct AuthorizationApproval {
    pub id: String,
    pub workspace_id: String,
    pub environment_id: String,
    pub intent_id: String,
    pub status: ApprovalStatus,
    pub envelope: ApprovalEnvelope,
    pub envelope_hash: String,
    #[serde(default)]
    pub approver_roles: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub decided_by: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub decided_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub decision_reason: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub grant_id: Option<String>,
    pub expires_at: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct AuthorizationApprovalSummary {
    pub id: String,
    pub status: ApprovalStatus,
    pub envelope_hash: String,
    pub expires_at: String,
    pub poll_after_ms: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub enum ApprovalDecision {
    Approve,
    Deny,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct DecideAuthorizationApprovalRequest {
    pub decision: ApprovalDecision,
    pub mode: GrantMode,
    pub envelope_hash: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub scope: Option<AuthorizationGrantScope>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub starts_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub expires_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct DecideAuthorizationApprovalResponse {
    pub approval: AuthorizationApproval,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub grant: Option<AuthorizationGrant>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct AuthorizationGrant {
    pub id: String,
    pub workspace_id: String,
    pub environment_id: String,
    pub principal_id: String,
    pub domain: AuthorizationDomain,
    pub capability: AuthorizationCapabilityId,
    pub mode: GrantMode,
    pub status: GrantStatus,
    pub source: AuthorizationGrantSource,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub scope: Option<AuthorizationGrantScope>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub exact_fingerprint: Option<String>,
    pub fingerprint_version: i32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub source_approval_id: Option<String>,
    #[serde(default)]
    pub requirement_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub max_uses: Option<u32>,
    pub use_count: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub starts_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub expires_at: Option<String>,
    pub created_by: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct CreateAuthorizationGrantRequest {
    pub principal_id: String,
    pub domain: AuthorizationDomain,
    pub capability: AuthorizationCapabilityId,
    pub scope: AuthorizationGrantScope,
    #[serde(default)]
    pub requirement_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub max_uses: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub starts_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub expires_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct AuthorizationGrantRef {
    pub id: String,
    pub capability: AuthorizationCapabilityId,
    pub mode: GrantMode,
    pub source: AuthorizationGrantSource,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct AuthorizationLease {
    pub id: String,
    pub intent_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub grant_id: Option<String>,
    pub attempt_id: String,
    pub fingerprint: String,
    pub status: LeaseStatus,
    pub claimed_at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub completed_at: Option<String>,
    pub expires_at: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct CompleteAuthorizationLeaseRequest {
    pub status: LeaseStatus,
    #[serde(default)]
    #[cfg_attr(feature = "ts-export", ts(type = "Record<string, unknown> | null"))]
    pub outcome: serde_json::Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "domain", content = "evidence", rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub enum AuthorizationDomainEvidence {
    Content(
        #[cfg_attr(feature = "ts-export", ts(type = "Record<string, unknown> | null"))]
        serde_json::Value,
    ),
    Tool(
        #[cfg_attr(feature = "ts-export", ts(type = "Record<string, unknown> | null"))]
        serde_json::Value,
    ),
    Financial(
        #[cfg_attr(feature = "ts-export", ts(type = "Record<string, unknown> | null"))]
        serde_json::Value,
    ),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct AuthorizationReceipt {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub intent_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub trace_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub principal_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub operation: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub run_id: Option<String>,
    pub domain: AuthorizationDomain,
    pub effect: AuthorizationEffect,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub intent_status: Option<AuthorizationIntentStatus>,
    pub subject_hash: String,
    pub reason: String,
    #[serde(default)]
    pub findings: Vec<AuthorizationFinding>,
    #[serde(default)]
    pub policy_versions: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub approval_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub grant_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub lease_id: Option<String>,
    pub domain_evidence: AuthorizationDomainEvidence,
    pub created_at: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct AuthorizationDecision {
    pub trace_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub intent_id: Option<String>,
    pub domain: AuthorizationDomain,
    pub effect: AuthorizationEffect,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub status: Option<AuthorizationIntentStatus>,
    pub reason: String,
    #[serde(default)]
    pub findings: Vec<AuthorizationFinding>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(
        feature = "ts-export",
        ts(optional, type = "Record<string, unknown> | null")
    )]
    pub transformed_value: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub approval: Option<AuthorizationApprovalSummary>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub applied_grant: Option<AuthorizationGrantRef>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub lease: Option<AuthorizationLease>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub receipt_id: Option<String>,
    pub latency_ms: u64,
}

impl AuthorizationDecision {
    pub fn permit(trace_id: impl Into<String>, domain: AuthorizationDomain) -> Self {
        Self {
            trace_id: trace_id.into(),
            intent_id: None,
            domain,
            effect: AuthorizationEffect::Permit,
            status: None,
            reason: "no authorization findings".into(),
            findings: Vec::new(),
            transformed_value: None,
            approval: None,
            applied_grant: None,
            lease: None,
            receipt_id: None,
            latency_ms: 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct AuthorizationApprovalListResponse {
    pub approvals: Vec<AuthorizationApproval>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct AuthorizationGrantListResponse {
    pub grants: Vec<AuthorizationGrant>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct AuthorizationReceiptListResponse {
    pub receipts: Vec<AuthorizationReceipt>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn effect_precedence_is_fail_closed() {
        let ordered = [
            AuthorizationEffect::Permit,
            AuthorizationEffect::Transform,
            AuthorizationEffect::RequireApproval,
            AuthorizationEffect::Defer,
            AuthorizationEffect::Deny,
        ];
        for (index, weaker) in ordered.iter().enumerate() {
            for stronger in &ordered[index..] {
                assert_eq!(weaker.worst_with(*stronger), *stronger);
                assert_eq!(stronger.worst_with(*weaker), *stronger);
            }
        }
    }

    #[test]
    fn capability_requires_namespace_and_safe_characters() {
        assert!(AuthorizationCapabilityId::parse("tool:mail/send").is_ok());
        assert!(AuthorizationCapabilityId::parse("external_message").is_err());
        assert!(AuthorizationCapabilityId::parse("tool:Mail Send").is_err());
    }
}
