//! Transactional Postgres repository for the unified authorization kernel.

use chrono::{DateTime, Duration, Utc};
use diesel::prelude::*;
use diesel_async::{AsyncConnection, RunQueryDsl};
use tl_core::{
    ApprovalDecision, ApprovalEnvelope, AuthorizationApproval, AuthorizationDomain,
    AuthorizationGrant, AuthorizationLease, AuthorizationReceipt,
    CompleteAuthorizationLeaseRequest, CreateAuthorizationGrantRequest,
    DecideAuthorizationApprovalRequest, DecideAuthorizationApprovalResponse, GrantMode,
    LeaseStatus,
};
use uuid::Uuid;

use crate::models::{
    AuthorizationApprovalRecord, AuthorizationGrantRecord, AuthorizationLeaseRecord,
    AuthorizationReceiptRecord, NewAuthorizationApproval, NewAuthorizationGrant,
    NewAuthorizationIntent, NewAuthorizationLease, NewAuthorizationReceipt,
};
use crate::postgres::{DbConnection, DbPool};
use crate::schema::{
    authorization_approvals, authorization_grants, authorization_intents, authorization_leases,
    authorization_receipts,
};
use crate::StorageError;

#[derive(Debug, Clone)]
pub struct CreateAuthorizationApproval {
    pub workspace_id: String,
    pub environment_id: String,
    pub envelope: ApprovalEnvelope,
    pub envelope_hash: String,
    pub approver_roles: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct CreateAuthorizationIntent {
    pub workspace_id: String,
    pub environment_id: String,
    pub id: String,
    pub domain: AuthorizationDomain,
    pub subject_id: String,
    pub idempotency_key: String,
    pub principal_id: String,
    pub operation: String,
    pub fingerprint: String,
    pub fingerprint_version: i32,
    pub subject_snapshot: serde_json::Value,
    pub expires_at: Option<DateTime<Utc>>,
}

#[derive(Clone)]
pub struct AuthorizationRepo {
    pool: DbPool,
}

impl AuthorizationRepo {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    pub async fn create_or_get_intent(
        &self,
        input: CreateAuthorizationIntent,
    ) -> Result<String, StorageError> {
        let mut conn = self.connection().await?;
        let id = parse_uuid(&input.id)?;
        diesel::insert_into(authorization_intents::table)
            .values(NewAuthorizationIntent {
                workspace_id: input.workspace_id.clone(),
                environment_id: input.environment_id.clone(),
                id,
                domain: text(&input.domain)?,
                subject_id: input.subject_id.clone(),
                idempotency_key: input.idempotency_key,
                principal_id: input.principal_id,
                operation: input.operation,
                fingerprint: input.fingerprint.clone(),
                fingerprint_version: input.fingerprint_version,
                subject_snapshot: input.subject_snapshot,
                status: "evaluating".into(),
                current_effect: "permit".into(),
                reason: "authorization evaluation started".into(),
                trace_id: None,
                expires_at: input.expires_at,
            })
            .on_conflict((
                authorization_intents::workspace_id,
                authorization_intents::environment_id,
                authorization_intents::domain,
                authorization_intents::subject_id,
            ))
            .do_nothing()
            .execute(&mut conn)
            .await?;
        let (stored_id, stored_fingerprint) = authorization_intents::table
            .filter(authorization_intents::workspace_id.eq(input.workspace_id))
            .filter(authorization_intents::environment_id.eq(input.environment_id))
            .filter(authorization_intents::domain.eq(text(&input.domain)?))
            .filter(authorization_intents::subject_id.eq(input.subject_id))
            .select((
                authorization_intents::id,
                authorization_intents::fingerprint,
            ))
            .first::<(Uuid, String)>(&mut conn)
            .await?;
        if stored_fingerprint != input.fingerprint {
            return Err(StorageError::Conflict);
        }
        Ok(stored_id.to_string())
    }

    pub async fn record_decision(
        &self,
        workspace_id: &str,
        environment_id: &str,
        intent_id: &str,
        effect: tl_core::AuthorizationEffect,
        status: tl_core::AuthorizationIntentStatus,
        reason: &str,
        trace_id: &str,
    ) -> Result<(), StorageError> {
        let mut conn = self.connection().await?;
        let changed = diesel::update(
            authorization_intents::table
                .filter(authorization_intents::workspace_id.eq(workspace_id))
                .filter(authorization_intents::environment_id.eq(environment_id))
                .filter(authorization_intents::id.eq(parse_uuid(intent_id)?)),
        )
        .set((
            authorization_intents::current_effect.eq(text(&effect)?),
            authorization_intents::status.eq(text(&status)?),
            authorization_intents::reason.eq(reason),
            authorization_intents::trace_id.eq(Some(trace_id)),
            authorization_intents::updated_at.eq(Utc::now()),
        ))
        .execute(&mut conn)
        .await?;
        if changed == 1 {
            Ok(())
        } else {
            Err(StorageError::NotFound)
        }
    }

    pub async fn create_or_get_approval(
        &self,
        input: CreateAuthorizationApproval,
    ) -> Result<AuthorizationApproval, StorageError> {
        let mut conn = self.connection().await?;
        conn.transaction::<_, StorageError, _>(async move |conn| {
            let intent_id = parse_uuid(&input.envelope.intent_id)?;
            let intent = NewAuthorizationIntent {
                workspace_id: input.workspace_id.clone(),
                environment_id: input.environment_id.clone(),
                id: intent_id,
                domain: text(&input.envelope.domain)?,
                subject_id: input.envelope.subject_id.clone(),
                idempotency_key: input.envelope.exact_fingerprint.clone(),
                principal_id: input.envelope.principal_id.clone(),
                operation: input.envelope.capability.to_string(),
                fingerprint: input.envelope.exact_fingerprint.clone(),
                fingerprint_version: input.envelope.fingerprint_version,
                subject_snapshot: json(&input.envelope)?,
                status: "pending_approval".into(),
                current_effect: "require_approval".into(),
                reason: "human authorization required".into(),
                trace_id: None,
                expires_at: Some(parse_time(&input.envelope.expires_at)?),
            };
            diesel::insert_into(authorization_intents::table)
                .values(&intent)
                .on_conflict((
                    authorization_intents::workspace_id,
                    authorization_intents::environment_id,
                    authorization_intents::domain,
                    authorization_intents::subject_id,
                ))
                .do_nothing()
                .execute(&mut *conn)
                .await?;

            let id = Uuid::now_v7();
            let approval = NewAuthorizationApproval {
                workspace_id: input.workspace_id.clone(),
                environment_id: input.environment_id.clone(),
                id,
                intent_id,
                fingerprint: input.envelope.exact_fingerprint.clone(),
                status: "pending".into(),
                envelope: json(&input.envelope)?,
                envelope_hash: input.envelope_hash,
                requirement_ids: json(&input.envelope.requirement_ids)?,
                approver_roles: json(&input.approver_roles)?,
                expires_at: parse_time(&input.envelope.expires_at)?,
            };
            diesel::insert_into(authorization_approvals::table)
                .values(&approval)
                .on_conflict_do_nothing()
                .execute(&mut *conn)
                .await?;
            let row = authorization_approvals::table
                .filter(authorization_approvals::workspace_id.eq(&input.workspace_id))
                .filter(authorization_approvals::environment_id.eq(&input.environment_id))
                .filter(authorization_approvals::intent_id.eq(intent_id))
                .filter(authorization_approvals::fingerprint.eq(&approval.fingerprint))
                .filter(authorization_approvals::status.eq("pending"))
                .select(AuthorizationApprovalRecord::as_select())
                .first(&mut *conn)
                .await?;
            approval_from_record(row, None)
        })
        .await
    }

    pub async fn get_approval(
        &self,
        workspace_id: &str,
        environment_id: &str,
        id: &str,
    ) -> Result<AuthorizationApproval, StorageError> {
        let id = parse_uuid(id)?;
        let mut conn = self.connection().await?;
        self.expire_approval(&mut conn, workspace_id, environment_id, id)
            .await?;
        let row = authorization_approvals::table
            .filter(authorization_approvals::workspace_id.eq(workspace_id))
            .filter(authorization_approvals::environment_id.eq(environment_id))
            .filter(authorization_approvals::id.eq(id))
            .select(AuthorizationApprovalRecord::as_select())
            .first(&mut conn)
            .await?;
        let grant_id = source_grant_id(&mut conn, workspace_id, environment_id, id).await?;
        approval_from_record(row, grant_id)
    }

    pub async fn list_approvals(
        &self,
        workspace_id: &str,
        environment_id: Option<&str>,
    ) -> Result<Vec<AuthorizationApproval>, StorageError> {
        let mut conn = self.connection().await?;
        let mut expiration = diesel::update(
            authorization_approvals::table
                .filter(authorization_approvals::workspace_id.eq(workspace_id))
                .filter(authorization_approvals::status.eq("pending"))
                .filter(authorization_approvals::expires_at.le(Utc::now())),
        )
        .into_boxed();
        if let Some(environment_id) = environment_id {
            expiration =
                expiration.filter(authorization_approvals::environment_id.eq(environment_id));
        }
        expiration
            .set((
                authorization_approvals::status.eq("expired"),
                authorization_approvals::updated_at.eq(Utc::now()),
            ))
            .execute(&mut conn)
            .await?;
        let mut query = authorization_approvals::table
            .filter(authorization_approvals::workspace_id.eq(workspace_id))
            .into_boxed();
        if let Some(environment_id) = environment_id {
            query = query.filter(authorization_approvals::environment_id.eq(environment_id));
        }
        let rows = query
            .order(authorization_approvals::created_at.desc())
            .limit(200)
            .select(AuthorizationApprovalRecord::as_select())
            .load(&mut conn)
            .await?;
        let mut approvals = Vec::with_capacity(rows.len());
        for row in rows {
            let grant_id =
                source_grant_id(&mut conn, &row.workspace_id, &row.environment_id, row.id).await?;
            approvals.push(approval_from_record(row, grant_id)?);
        }
        Ok(approvals)
    }

    pub async fn decide_approval(
        &self,
        workspace_id: &str,
        environment_id: &str,
        approval_id: &str,
        actor_id: &str,
        request: DecideAuthorizationApprovalRequest,
    ) -> Result<DecideAuthorizationApprovalResponse, StorageError> {
        let approval_id = parse_uuid(approval_id)?;
        let mut conn = self.connection().await?;
        conn.transaction::<_, StorageError, _>(async move |conn| {
            let row = authorization_approvals::table
                .filter(authorization_approvals::workspace_id.eq(workspace_id))
                .filter(authorization_approvals::environment_id.eq(environment_id))
                .filter(authorization_approvals::id.eq(approval_id))
                .for_update()
                .select(AuthorizationApprovalRecord::as_select())
                .first(&mut *conn)
                .await?;
            if row.status != "pending"
                || row.expires_at <= Utc::now()
                || row.envelope_hash != request.envelope_hash
            {
                return Err(StorageError::Conflict);
            }
            let envelope: ApprovalEnvelope = from_json(row.envelope.clone())?;
            let status = match request.decision {
                ApprovalDecision::Approve => "approved",
                ApprovalDecision::Deny => "denied",
            };
            let now = Utc::now();
            diesel::update(
                authorization_approvals::table
                    .filter(authorization_approvals::workspace_id.eq(workspace_id))
                    .filter(authorization_approvals::environment_id.eq(environment_id))
                    .filter(authorization_approvals::id.eq(approval_id))
                    .filter(authorization_approvals::status.eq("pending")),
            )
            .set((
                authorization_approvals::status.eq(status),
                authorization_approvals::decided_by.eq(Some(actor_id)),
                authorization_approvals::decided_at.eq(Some(now)),
                authorization_approvals::decision_reason.eq(request.reason.clone()),
                authorization_approvals::updated_at.eq(now),
            ))
            .execute(&mut *conn)
            .await?;

            let grant = if request.decision == ApprovalDecision::Approve {
                let starts_at = parse_optional_time(request.starts_at.as_deref())?;
                let expires_at =
                    parse_optional_time(request.expires_at.as_deref())?.or(Some(row.expires_at));
                if expires_at
                    .as_ref()
                    .is_some_and(|expires| *expires > row.expires_at)
                    || starts_at
                        .as_ref()
                        .zip(expires_at.as_ref())
                        .is_some_and(|(starts, expires)| starts >= expires)
                {
                    return Err(StorageError::Conflict);
                }
                let (scope, exact_fingerprint, max_uses) = match request.mode {
                    GrantMode::ExactOnce => {
                        (None, Some(envelope.exact_fingerprint.clone()), Some(1))
                    }
                    GrantMode::Scoped => {
                        let scope = request.scope.clone().ok_or(StorageError::Conflict)?;
                        if envelope.proposed_scope.as_ref() != Some(&scope) {
                            return Err(StorageError::Conflict);
                        }
                        (Some(json(&scope)?), None, None)
                    }
                };
                let grant_id = Uuid::now_v7();
                diesel::insert_into(authorization_grants::table)
                    .values(NewAuthorizationGrant {
                        workspace_id: workspace_id.into(),
                        environment_id: environment_id.into(),
                        id: grant_id,
                        principal_id: envelope.principal_id.clone(),
                        domain: text(&envelope.domain)?,
                        capability: envelope.capability.to_string(),
                        mode: text(&request.mode)?,
                        status: "active".into(),
                        source: "reviewer_approval".into(),
                        scope_schema: "authorization-grant-scope:v1".into(),
                        scope,
                        exact_fingerprint,
                        fingerprint_version: envelope.fingerprint_version,
                        source_approval_id: Some(approval_id),
                        requirement_ids: json(&envelope.requirement_ids)?,
                        max_uses,
                        starts_at,
                        expires_at,
                        created_by: actor_id.into(),
                    })
                    .execute(&mut *conn)
                    .await?;
                let grant_row = authorization_grants::table
                    .filter(authorization_grants::workspace_id.eq(workspace_id))
                    .filter(authorization_grants::environment_id.eq(environment_id))
                    .filter(authorization_grants::id.eq(grant_id))
                    .select(AuthorizationGrantRecord::as_select())
                    .first(&mut *conn)
                    .await?;
                Some(grant_from_record(grant_row)?)
            } else {
                None
            };
            let decided = authorization_approvals::table
                .filter(authorization_approvals::workspace_id.eq(workspace_id))
                .filter(authorization_approvals::environment_id.eq(environment_id))
                .filter(authorization_approvals::id.eq(approval_id))
                .select(AuthorizationApprovalRecord::as_select())
                .first(&mut *conn)
                .await?;
            Ok(DecideAuthorizationApprovalResponse {
                approval: approval_from_record(
                    decided,
                    grant.as_ref().map(|grant| grant.id.clone()),
                )?,
                grant,
            })
        })
        .await
    }

    pub async fn create_grant(
        &self,
        workspace_id: &str,
        environment_id: &str,
        actor_id: &str,
        request: CreateAuthorizationGrantRequest,
    ) -> Result<AuthorizationGrant, StorageError> {
        let mut conn = self.connection().await?;
        let id = Uuid::now_v7();
        diesel::insert_into(authorization_grants::table)
            .values(NewAuthorizationGrant {
                workspace_id: workspace_id.into(),
                environment_id: environment_id.into(),
                id,
                principal_id: request.principal_id,
                domain: text(&request.domain)?,
                capability: request.capability.to_string(),
                mode: "scoped".into(),
                status: "active".into(),
                source: "user_intent".into(),
                scope_schema: "authorization-grant-scope:v1".into(),
                scope: Some(json(&request.scope)?),
                exact_fingerprint: None,
                fingerprint_version: 1,
                source_approval_id: None,
                requirement_ids: json(&request.requirement_ids)?,
                max_uses: request.max_uses.map(|value| value as i32),
                starts_at: parse_optional_time(request.starts_at.as_deref())?,
                expires_at: parse_optional_time(request.expires_at.as_deref())?,
                created_by: actor_id.into(),
            })
            .execute(&mut conn)
            .await?;
        self.get_grant_with_conn(&mut conn, workspace_id, environment_id, id)
            .await
    }

    pub async fn get_grant(
        &self,
        workspace_id: &str,
        environment_id: &str,
        id: &str,
    ) -> Result<AuthorizationGrant, StorageError> {
        let mut conn = self.connection().await?;
        let id = parse_uuid(id)?;
        self.expire_grant(&mut conn, workspace_id, environment_id, id)
            .await?;
        self.get_grant_with_conn(&mut conn, workspace_id, environment_id, id)
            .await
    }

    pub async fn list_grants(
        &self,
        workspace_id: &str,
        environment_id: Option<&str>,
    ) -> Result<Vec<AuthorizationGrant>, StorageError> {
        let mut conn = self.connection().await?;
        let mut expiration = diesel::update(
            authorization_grants::table
                .filter(authorization_grants::workspace_id.eq(workspace_id))
                .filter(authorization_grants::status.eq("active"))
                .filter(authorization_grants::expires_at.le(Utc::now())),
        )
        .into_boxed();
        if let Some(environment_id) = environment_id {
            expiration = expiration.filter(authorization_grants::environment_id.eq(environment_id));
        }
        expiration
            .set((
                authorization_grants::status.eq("expired"),
                authorization_grants::updated_at.eq(Utc::now()),
            ))
            .execute(&mut conn)
            .await?;
        let mut query = authorization_grants::table
            .filter(authorization_grants::workspace_id.eq(workspace_id))
            .into_boxed();
        if let Some(environment_id) = environment_id {
            query = query.filter(authorization_grants::environment_id.eq(environment_id));
        }
        query
            .order(authorization_grants::created_at.desc())
            .limit(200)
            .select(AuthorizationGrantRecord::as_select())
            .load(&mut conn)
            .await?
            .into_iter()
            .map(grant_from_record)
            .collect()
    }

    pub async fn revoke_grant(
        &self,
        workspace_id: &str,
        environment_id: &str,
        id: &str,
        actor_id: &str,
    ) -> Result<AuthorizationGrant, StorageError> {
        let id = parse_uuid(id)?;
        let mut conn = self.connection().await?;
        let now = Utc::now();
        let changed = diesel::update(
            authorization_grants::table
                .filter(authorization_grants::workspace_id.eq(workspace_id))
                .filter(authorization_grants::environment_id.eq(environment_id))
                .filter(authorization_grants::id.eq(id))
                .filter(authorization_grants::status.eq("active")),
        )
        .set((
            authorization_grants::status.eq("revoked"),
            authorization_grants::revoked_at.eq(Some(now)),
            authorization_grants::revoked_by.eq(Some(actor_id)),
            authorization_grants::updated_at.eq(now),
        ))
        .execute(&mut conn)
        .await?;
        if changed != 1 {
            return Err(StorageError::Conflict);
        }
        self.get_grant_with_conn(&mut conn, workspace_id, environment_id, id)
            .await
    }

    pub async fn claim_lease(
        &self,
        workspace_id: &str,
        environment_id: &str,
        intent_id: &str,
        grant_id: Option<&str>,
        attempt_id: &str,
        fingerprint: &str,
    ) -> Result<AuthorizationLease, StorageError> {
        let intent_id = parse_uuid(intent_id)?;
        let grant_id = grant_id.map(parse_uuid).transpose()?;
        let mut conn = self.connection().await?;
        conn.transaction::<_, StorageError, _>(async move |conn| {
            let existing = authorization_leases::table
                .filter(authorization_leases::workspace_id.eq(workspace_id))
                .filter(authorization_leases::environment_id.eq(environment_id))
                .filter(authorization_leases::intent_id.eq(intent_id))
                .filter(authorization_leases::attempt_id.eq(attempt_id))
                .select(AuthorizationLeaseRecord::as_select())
                .first::<AuthorizationLeaseRecord>(&mut *conn)
                .await
                .optional()?;
            if let Some(existing) = existing {
                if existing.fingerprint != fingerprint
                    || existing.grant_id != grant_id
                    || !matches!(existing.status.as_str(), "claimed" | "consumed")
                {
                    return Err(StorageError::Conflict);
                }
                return lease_from_record(existing);
            }
            let intent_fingerprint = authorization_intents::table
                .filter(authorization_intents::workspace_id.eq(workspace_id))
                .filter(authorization_intents::environment_id.eq(environment_id))
                .filter(authorization_intents::id.eq(intent_id))
                .select(authorization_intents::fingerprint)
                .first::<String>(&mut *conn)
                .await?;
            if intent_fingerprint != fingerprint {
                return Err(StorageError::Conflict);
            }
            if let Some(grant_id) = grant_id {
                let grant = authorization_grants::table
                    .filter(authorization_grants::workspace_id.eq(workspace_id))
                    .filter(authorization_grants::environment_id.eq(environment_id))
                    .filter(authorization_grants::id.eq(grant_id))
                    .for_update()
                    .select(AuthorizationGrantRecord::as_select())
                    .first(&mut *conn)
                    .await?;
                if grant.status != "active"
                    || grant.starts_at.is_some_and(|value| value > Utc::now())
                    || grant.expires_at.is_some_and(|value| value <= Utc::now())
                    || grant.max_uses.is_some_and(|max| grant.use_count >= max)
                {
                    return Err(StorageError::Conflict);
                }
                let next_count = grant.use_count + 1;
                let next_status = if grant.max_uses.is_some_and(|max| next_count >= max) {
                    "exhausted"
                } else {
                    "active"
                };
                diesel::update(
                    authorization_grants::table
                        .filter(authorization_grants::workspace_id.eq(workspace_id))
                        .filter(authorization_grants::environment_id.eq(environment_id))
                        .filter(authorization_grants::id.eq(grant_id)),
                )
                .set((
                    authorization_grants::use_count.eq(next_count),
                    authorization_grants::status.eq(next_status),
                    authorization_grants::updated_at.eq(Utc::now()),
                ))
                .execute(&mut *conn)
                .await?;
            }
            let id = Uuid::now_v7();
            diesel::insert_into(authorization_leases::table)
                .values(NewAuthorizationLease {
                    workspace_id: workspace_id.into(),
                    environment_id: environment_id.into(),
                    id,
                    intent_id,
                    grant_id,
                    attempt_id: attempt_id.into(),
                    fingerprint: fingerprint.into(),
                    status: "claimed".into(),
                    expires_at: Utc::now() + Duration::minutes(5),
                })
                .execute(&mut *conn)
                .await?;
            let row = authorization_leases::table
                .filter(authorization_leases::workspace_id.eq(workspace_id))
                .filter(authorization_leases::environment_id.eq(environment_id))
                .filter(authorization_leases::id.eq(id))
                .select(AuthorizationLeaseRecord::as_select())
                .first(&mut *conn)
                .await?;
            lease_from_record(row)
        })
        .await
    }

    pub async fn get_lease_by_attempt(
        &self,
        workspace_id: &str,
        environment_id: &str,
        intent_id: &str,
        attempt_id: &str,
    ) -> Result<Option<AuthorizationLease>, StorageError> {
        let mut conn = self.connection().await?;
        authorization_leases::table
            .filter(authorization_leases::workspace_id.eq(workspace_id))
            .filter(authorization_leases::environment_id.eq(environment_id))
            .filter(authorization_leases::intent_id.eq(parse_uuid(intent_id)?))
            .filter(authorization_leases::attempt_id.eq(attempt_id))
            .select(AuthorizationLeaseRecord::as_select())
            .first::<AuthorizationLeaseRecord>(&mut conn)
            .await
            .optional()?
            .map(lease_from_record)
            .transpose()
    }

    pub async fn complete_lease(
        &self,
        workspace_id: &str,
        environment_id: &str,
        id: &str,
        request: CompleteAuthorizationLeaseRequest,
    ) -> Result<AuthorizationLease, StorageError> {
        let id = parse_uuid(id)?;
        let mut conn = self.connection().await?;
        let status = match request.status {
            LeaseStatus::Consumed => "consumed",
            LeaseStatus::Canceled => "canceled",
            _ => return Err(StorageError::Conflict),
        };
        let now = Utc::now();
        let changed = diesel::update(
            authorization_leases::table
                .filter(authorization_leases::workspace_id.eq(workspace_id))
                .filter(authorization_leases::environment_id.eq(environment_id))
                .filter(authorization_leases::id.eq(id))
                .filter(authorization_leases::status.eq("claimed")),
        )
        .set((
            authorization_leases::status.eq(status),
            authorization_leases::consumed_at.eq(if status == "consumed" {
                Some(now)
            } else {
                None
            }),
            authorization_leases::canceled_at.eq(if status == "canceled" {
                Some(now)
            } else {
                None
            }),
            authorization_leases::outcome.eq(json(&request.outcome)?),
        ))
        .execute(&mut conn)
        .await?;
        let row = authorization_leases::table
            .filter(authorization_leases::workspace_id.eq(workspace_id))
            .filter(authorization_leases::environment_id.eq(environment_id))
            .filter(authorization_leases::id.eq(id))
            .select(AuthorizationLeaseRecord::as_select())
            .first(&mut conn)
            .await?;
        if changed == 0 && row.status != status {
            return Err(StorageError::Conflict);
        }
        lease_from_record(row)
    }

    pub async fn get_lease_principal(
        &self,
        workspace_id: &str,
        environment_id: &str,
        id: &str,
    ) -> Result<String, StorageError> {
        let mut conn = self.connection().await?;
        let intent_id = authorization_leases::table
            .filter(authorization_leases::workspace_id.eq(workspace_id))
            .filter(authorization_leases::environment_id.eq(environment_id))
            .filter(authorization_leases::id.eq(parse_uuid(id)?))
            .select(authorization_leases::intent_id)
            .first::<Uuid>(&mut conn)
            .await?;
        authorization_intents::table
            .filter(authorization_intents::workspace_id.eq(workspace_id))
            .filter(authorization_intents::environment_id.eq(environment_id))
            .filter(authorization_intents::id.eq(intent_id))
            .select(authorization_intents::principal_id)
            .first(&mut conn)
            .await
            .map_err(Into::into)
    }

    pub async fn write_receipt(
        &self,
        workspace_id: &str,
        environment_id: &str,
        receipt: AuthorizationReceipt,
    ) -> Result<(), StorageError> {
        let mut conn = self.connection().await?;
        diesel::insert_into(authorization_receipts::table)
            .values(NewAuthorizationReceipt {
                workspace_id: workspace_id.into(),
                environment_id: environment_id.into(),
                id: parse_uuid(&receipt.id)?,
                intent_id: receipt.intent_id.as_deref().map(parse_uuid).transpose()?,
                trace_id: receipt.trace_id,
                principal_id: receipt.principal_id,
                operation: receipt.operation,
                run_id: receipt.run_id.as_deref().map(parse_uuid).transpose()?,
                domain: text(&receipt.domain)?,
                effect: text(&receipt.effect)?,
                intent_status: receipt.intent_status.as_ref().map(text).transpose()?,
                subject_hash: receipt.subject_hash,
                reason: receipt.reason,
                findings: json(&receipt.findings)?,
                policy_versions: json(&receipt.policy_versions)?,
                approval_id: receipt.approval_id.as_deref().map(parse_uuid).transpose()?,
                grant_id: receipt.grant_id.as_deref().map(parse_uuid).transpose()?,
                lease_id: receipt.lease_id.as_deref().map(parse_uuid).transpose()?,
                domain_evidence: json(&receipt.domain_evidence)?,
            })
            .on_conflict_do_nothing()
            .execute(&mut conn)
            .await?;
        Ok(())
    }

    pub async fn get_receipt(
        &self,
        workspace_id: &str,
        environment_id: &str,
        id: &str,
    ) -> Result<AuthorizationReceipt, StorageError> {
        let mut conn = self.connection().await?;
        let row = authorization_receipts::table
            .filter(authorization_receipts::workspace_id.eq(workspace_id))
            .filter(authorization_receipts::environment_id.eq(environment_id))
            .filter(authorization_receipts::id.eq(parse_uuid(id)?))
            .select(AuthorizationReceiptRecord::as_select())
            .first(&mut conn)
            .await?;
        receipt_from_record(row)
    }

    pub async fn get_receipt_principal(
        &self,
        workspace_id: &str,
        environment_id: &str,
        id: &str,
    ) -> Result<String, StorageError> {
        let mut conn = self.connection().await?;
        let (principal_id, intent_id) = authorization_receipts::table
            .filter(authorization_receipts::workspace_id.eq(workspace_id))
            .filter(authorization_receipts::environment_id.eq(environment_id))
            .filter(authorization_receipts::id.eq(parse_uuid(id)?))
            .select((
                authorization_receipts::principal_id,
                authorization_receipts::intent_id,
            ))
            .first::<(Option<String>, Option<Uuid>)>(&mut conn)
            .await?;
        if let Some(principal_id) = principal_id {
            return Ok(principal_id);
        }
        let intent_id = intent_id.ok_or(StorageError::NotFound)?;
        authorization_intents::table
            .filter(authorization_intents::workspace_id.eq(workspace_id))
            .filter(authorization_intents::environment_id.eq(environment_id))
            .filter(authorization_intents::id.eq(intent_id))
            .select(authorization_intents::principal_id)
            .first(&mut conn)
            .await
            .map_err(Into::into)
    }

    pub async fn list_receipts(
        &self,
        workspace_id: &str,
        environment_id: Option<&str>,
    ) -> Result<Vec<AuthorizationReceipt>, StorageError> {
        let mut conn = self.connection().await?;
        let mut query = authorization_receipts::table
            .filter(authorization_receipts::workspace_id.eq(workspace_id))
            .into_boxed();
        if let Some(environment_id) = environment_id {
            query = query.filter(authorization_receipts::environment_id.eq(environment_id));
        }
        query
            .order(authorization_receipts::created_at.desc())
            .limit(200)
            .select(AuthorizationReceiptRecord::as_select())
            .load(&mut conn)
            .await?
            .into_iter()
            .map(receipt_from_record)
            .collect()
    }

    async fn get_grant_with_conn(
        &self,
        conn: &mut DbConnection<'_>,
        workspace_id: &str,
        environment_id: &str,
        id: Uuid,
    ) -> Result<AuthorizationGrant, StorageError> {
        let row = authorization_grants::table
            .filter(authorization_grants::workspace_id.eq(workspace_id))
            .filter(authorization_grants::environment_id.eq(environment_id))
            .filter(authorization_grants::id.eq(id))
            .select(AuthorizationGrantRecord::as_select())
            .first(conn)
            .await?;
        grant_from_record(row)
    }

    async fn expire_approval(
        &self,
        conn: &mut DbConnection<'_>,
        workspace_id: &str,
        environment_id: &str,
        id: Uuid,
    ) -> Result<(), StorageError> {
        diesel::update(
            authorization_approvals::table
                .filter(authorization_approvals::workspace_id.eq(workspace_id))
                .filter(authorization_approvals::environment_id.eq(environment_id))
                .filter(authorization_approvals::id.eq(id))
                .filter(authorization_approvals::status.eq("pending"))
                .filter(authorization_approvals::expires_at.le(Utc::now())),
        )
        .set((
            authorization_approvals::status.eq("expired"),
            authorization_approvals::updated_at.eq(Utc::now()),
        ))
        .execute(conn)
        .await?;
        Ok(())
    }

    async fn expire_grant(
        &self,
        conn: &mut DbConnection<'_>,
        workspace_id: &str,
        environment_id: &str,
        id: Uuid,
    ) -> Result<(), StorageError> {
        diesel::update(
            authorization_grants::table
                .filter(authorization_grants::workspace_id.eq(workspace_id))
                .filter(authorization_grants::environment_id.eq(environment_id))
                .filter(authorization_grants::id.eq(id))
                .filter(authorization_grants::status.eq("active"))
                .filter(authorization_grants::expires_at.le(Utc::now())),
        )
        .set((
            authorization_grants::status.eq("expired"),
            authorization_grants::updated_at.eq(Utc::now()),
        ))
        .execute(conn)
        .await?;
        Ok(())
    }

    async fn connection(&self) -> Result<DbConnection<'_>, StorageError> {
        self.pool
            .get()
            .await
            .map_err(|error| StorageError::Internal(format!("authorization connection: {error}")))
    }
}

async fn source_grant_id(
    conn: &mut DbConnection<'_>,
    workspace_id: &str,
    environment_id: &str,
    approval_id: Uuid,
) -> Result<Option<String>, StorageError> {
    Ok(authorization_grants::table
        .filter(authorization_grants::workspace_id.eq(workspace_id))
        .filter(authorization_grants::environment_id.eq(environment_id))
        .filter(authorization_grants::source_approval_id.eq(approval_id))
        .select(authorization_grants::id)
        .first::<Uuid>(conn)
        .await
        .optional()?
        .map(|id| id.to_string()))
}

fn approval_from_record(
    row: AuthorizationApprovalRecord,
    grant_id: Option<String>,
) -> Result<AuthorizationApproval, StorageError> {
    Ok(AuthorizationApproval {
        id: row.id.to_string(),
        workspace_id: row.workspace_id,
        environment_id: row.environment_id,
        intent_id: row.intent_id.to_string(),
        status: enum_from_text(&row.status)?,
        envelope: from_json(row.envelope)?,
        envelope_hash: row.envelope_hash,
        approver_roles: from_json(row.approver_roles)?,
        decided_by: row.decided_by,
        decided_at: row.decided_at.map(|value| value.to_rfc3339()),
        decision_reason: row.decision_reason,
        grant_id,
        expires_at: row.expires_at.to_rfc3339(),
        created_at: row.created_at.to_rfc3339(),
        updated_at: row.updated_at.to_rfc3339(),
    })
}

fn grant_from_record(row: AuthorizationGrantRecord) -> Result<AuthorizationGrant, StorageError> {
    Ok(AuthorizationGrant {
        id: row.id.to_string(),
        workspace_id: row.workspace_id,
        environment_id: row.environment_id,
        principal_id: row.principal_id,
        domain: enum_from_text(&row.domain)?,
        capability: tl_core::AuthorizationCapabilityId::parse(row.capability)
            .map_err(|error| StorageError::Internal(error.into()))?,
        mode: enum_from_text(&row.mode)?,
        status: enum_from_text(&row.status)?,
        source: enum_from_text(&row.source)?,
        scope: row.scope.map(from_json).transpose()?,
        exact_fingerprint: row.exact_fingerprint,
        fingerprint_version: row.fingerprint_version,
        source_approval_id: row.source_approval_id.map(|id| id.to_string()),
        requirement_ids: from_json(row.requirement_ids)?,
        max_uses: row.max_uses.map(|value| value as u32),
        use_count: row.use_count as u32,
        starts_at: row.starts_at.map(|value| value.to_rfc3339()),
        expires_at: row.expires_at.map(|value| value.to_rfc3339()),
        created_by: row.created_by,
        created_at: row.created_at.to_rfc3339(),
        updated_at: row.updated_at.to_rfc3339(),
    })
}

fn lease_from_record(row: AuthorizationLeaseRecord) -> Result<AuthorizationLease, StorageError> {
    Ok(AuthorizationLease {
        id: row.id.to_string(),
        intent_id: row.intent_id.to_string(),
        grant_id: row.grant_id.map(|id| id.to_string()),
        attempt_id: row.attempt_id,
        fingerprint: row.fingerprint,
        status: enum_from_text(&row.status)?,
        claimed_at: row.claimed_at.to_rfc3339(),
        completed_at: row
            .consumed_at
            .or(row.canceled_at)
            .map(|value| value.to_rfc3339()),
        expires_at: row.expires_at.to_rfc3339(),
    })
}

fn receipt_from_record(
    row: AuthorizationReceiptRecord,
) -> Result<AuthorizationReceipt, StorageError> {
    Ok(AuthorizationReceipt {
        id: row.id.to_string(),
        intent_id: row.intent_id.map(|id| id.to_string()),
        trace_id: row.trace_id,
        principal_id: row.principal_id,
        operation: row.operation,
        run_id: row.run_id.map(|id| id.to_string()),
        domain: enum_from_text(&row.domain)?,
        effect: enum_from_text(&row.effect)?,
        intent_status: row
            .intent_status
            .as_deref()
            .map(enum_from_text)
            .transpose()?,
        subject_hash: row.subject_hash,
        reason: row.reason,
        findings: from_json(row.findings)?,
        policy_versions: from_json(row.policy_versions)?,
        approval_id: row.approval_id.map(|id| id.to_string()),
        grant_id: row.grant_id.map(|id| id.to_string()),
        lease_id: row.lease_id.map(|id| id.to_string()),
        domain_evidence: from_json(row.domain_evidence)?,
        created_at: row.created_at.to_rfc3339(),
    })
}

fn text<T: serde::Serialize>(value: &T) -> Result<String, StorageError> {
    serde_json::to_value(value)
        .ok()
        .and_then(|value| value.as_str().map(str::to_string))
        .ok_or_else(|| {
            StorageError::Internal("authorization enum did not serialize as text".into())
        })
}

fn enum_from_text<T: serde::de::DeserializeOwned>(value: &str) -> Result<T, StorageError> {
    from_json(serde_json::Value::String(value.into()))
}

fn json<T: serde::Serialize>(value: &T) -> Result<serde_json::Value, StorageError> {
    serde_json::to_value(value).map_err(|error| StorageError::Internal(error.to_string()))
}

fn from_json<T: serde::de::DeserializeOwned>(value: serde_json::Value) -> Result<T, StorageError> {
    serde_json::from_value(value).map_err(|error| StorageError::Internal(error.to_string()))
}

fn parse_uuid(value: &str) -> Result<Uuid, StorageError> {
    Uuid::parse_str(value).map_err(|_| StorageError::Conflict)
}

fn parse_time(value: &str) -> Result<DateTime<Utc>, StorageError> {
    DateTime::parse_from_rfc3339(value)
        .map(|value| value.with_timezone(&Utc))
        .map_err(|_| StorageError::Conflict)
}

fn parse_optional_time(value: Option<&str>) -> Result<Option<DateTime<Utc>>, StorageError> {
    value.map(parse_time).transpose()
}

impl std::fmt::Debug for AuthorizationRepo {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("AuthorizationRepo")
            .finish_non_exhaustive()
    }
}
