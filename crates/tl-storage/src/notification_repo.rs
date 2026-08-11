use chrono::{Duration, Utc};
use diesel::prelude::*;
use diesel_async::{AsyncConnection, AsyncPgConnection, RunQueryDsl};
use serde_json::Value;
use tl_core::{
    NotificationDeliveryStatus, NotificationDeliverySummary, NotificationEventKind,
    NotificationRule,
};
use uuid::Uuid;

use crate::models::{
    NewNotificationDelivery, NewNotificationRule, NotificationDeliveryRecord,
    NotificationRuleRecord,
};
use crate::postgres::{DbConnection, DbPool};
use crate::schema::{notification_deliveries, notification_rules};
use crate::StorageError;

#[derive(Debug, Clone)]
pub struct ClaimedNotificationDelivery {
    pub workspace_id: String,
    pub delivery: NotificationDeliverySummary,
    pub email: String,
    pub payload: Value,
    pub run_id: Option<String>,
}

#[derive(Clone)]
pub struct NotificationRepo {
    pool: DbPool,
}

impl NotificationRepo {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    async fn connection(&self) -> Result<DbConnection<'_>, StorageError> {
        self.pool
            .get()
            .await
            .map_err(|error| StorageError::Internal(format!("db pool: {error}")))
    }

    pub async fn create_rule(
        &self,
        workspace_id: &str,
        environment_id: &str,
        agent_id: Option<String>,
        email: String,
        event_kinds: Vec<NotificationEventKind>,
        enabled: bool,
    ) -> Result<NotificationRule, StorageError> {
        let mut conn = self.connection().await?;
        let row = diesel::insert_into(notification_rules::table)
            .values(NewNotificationRule {
                workspace_id: workspace_id.to_string(),
                id: Uuid::now_v7(),
                environment_id: environment_id.to_string(),
                agent_id,
                email,
                event_kinds: event_kinds
                    .into_iter()
                    .map(event_kind_text)
                    .map(str::to_string)
                    .collect(),
                enabled,
            })
            .returning(NotificationRuleRecord::as_returning())
            .get_result::<NotificationRuleRecord>(&mut conn)
            .await?;
        rule_to_wire(row)
    }

    pub async fn list_rules(
        &self,
        workspace_id: &str,
        environment_id: &str,
    ) -> Result<Vec<NotificationRule>, StorageError> {
        let mut conn = self.connection().await?;
        notification_rules::table
            .filter(notification_rules::workspace_id.eq(workspace_id))
            .filter(notification_rules::environment_id.eq(environment_id))
            .filter(notification_rules::deleted_at.is_null())
            .order(notification_rules::created_at.asc())
            .select(NotificationRuleRecord::as_select())
            .load::<NotificationRuleRecord>(&mut conn)
            .await?
            .into_iter()
            .map(rule_to_wire)
            .collect()
    }

    pub async fn update_rule(
        &self,
        workspace_id: &str,
        rule_id: &str,
        email: Option<String>,
        event_kinds: Option<Vec<NotificationEventKind>>,
        enabled: Option<bool>,
    ) -> Result<NotificationRule, StorageError> {
        let id = parse_uuid(rule_id)?;
        let mut conn = self.connection().await?;
        let mut row = notification_rules::table
            .filter(notification_rules::workspace_id.eq(workspace_id))
            .filter(notification_rules::id.eq(id))
            .filter(notification_rules::deleted_at.is_null())
            .select(NotificationRuleRecord::as_select())
            .first::<NotificationRuleRecord>(&mut conn)
            .await?;
        if let Some(value) = email {
            row.email = value;
        }
        if let Some(value) = event_kinds {
            row.event_kinds = value
                .into_iter()
                .map(event_kind_text)
                .map(str::to_string)
                .collect();
        }
        if let Some(value) = enabled {
            row.enabled = value;
        }
        let row = diesel::update(
            notification_rules::table
                .filter(notification_rules::workspace_id.eq(workspace_id))
                .filter(notification_rules::id.eq(id)),
        )
        .set((
            notification_rules::email.eq(row.email),
            notification_rules::event_kinds.eq(row.event_kinds),
            notification_rules::enabled.eq(row.enabled),
            notification_rules::updated_at.eq(Utc::now()),
        ))
        .returning(NotificationRuleRecord::as_returning())
        .get_result::<NotificationRuleRecord>(&mut conn)
        .await?;
        rule_to_wire(row)
    }

    pub async fn delete_rule(&self, workspace_id: &str, rule_id: &str) -> Result<(), StorageError> {
        let id = parse_uuid(rule_id)?;
        let mut conn = self.connection().await?;
        let count = diesel::update(
            notification_rules::table
                .filter(notification_rules::workspace_id.eq(workspace_id))
                .filter(notification_rules::id.eq(id))
                .filter(notification_rules::deleted_at.is_null()),
        )
        .set((
            notification_rules::deleted_at.eq(Utc::now()),
            notification_rules::updated_at.eq(Utc::now()),
        ))
        .execute(&mut conn)
        .await?;
        if count == 0 {
            return Err(StorageError::NotFound);
        }
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn enqueue_matching(
        &self,
        workspace_id: &str,
        environment_id: &str,
        agent_id: Option<&str>,
        rule_id: Option<&str>,
        event_kind: NotificationEventKind,
        subject_id: &str,
        subject_version: &str,
        run_id: Option<&str>,
        payload: Value,
    ) -> Result<usize, StorageError> {
        let run_id = run_id.map(parse_uuid).transpose()?;
        let mut conn = self.connection().await?;
        enqueue_matching_on_connection(
            &mut conn,
            workspace_id,
            environment_id,
            agent_id,
            rule_id.map(parse_uuid).transpose()?,
            event_kind,
            subject_id,
            subject_version,
            run_id,
            payload,
        )
        .await
    }

    pub async fn list_deliveries(
        &self,
        workspace_id: &str,
        environment_id: &str,
        limit: i64,
    ) -> Result<Vec<NotificationDeliverySummary>, StorageError> {
        let mut conn = self.connection().await?;
        notification_deliveries::table
            .filter(notification_deliveries::workspace_id.eq(workspace_id))
            .filter(notification_deliveries::environment_id.eq(environment_id))
            .order(notification_deliveries::created_at.desc())
            .limit(limit.clamp(1, 500))
            .select(NotificationDeliveryRecord::as_select())
            .load::<NotificationDeliveryRecord>(&mut conn)
            .await?
            .into_iter()
            .map(delivery_to_wire)
            .collect()
    }

    pub async fn claim_delivery(
        &self,
        worker_id: &str,
        lease_seconds: i64,
    ) -> Result<Option<ClaimedNotificationDelivery>, StorageError> {
        let mut conn = self.connection().await?;
        conn.transaction::<Option<ClaimedNotificationDelivery>, StorageError, _>(
            async move |conn| {
                let now = Utc::now();
                let candidate = notification_deliveries::table
                    .filter(
                        notification_deliveries::status
                            .eq("pending")
                            .and(notification_deliveries::next_attempt_at.le(now))
                            .or(notification_deliveries::status
                                .eq("sending")
                                .and(notification_deliveries::lease_expires_at.le(now))),
                    )
                    .order(notification_deliveries::created_at.asc())
                    .for_update()
                    .skip_locked()
                    .select(NotificationDeliveryRecord::as_select())
                    .first::<NotificationDeliveryRecord>(conn)
                    .await
                    .optional()?;
                let Some(candidate) = candidate else {
                    return Ok(None);
                };
                let claimed = diesel::update(
                    notification_deliveries::table
                        .filter(notification_deliveries::workspace_id.eq(&candidate.workspace_id))
                        .filter(notification_deliveries::id.eq(candidate.id)),
                )
                .set((
                    notification_deliveries::status.eq("sending"),
                    notification_deliveries::lease_owner.eq(Some(worker_id)),
                    notification_deliveries::lease_expires_at
                        .eq(Some(now + Duration::seconds(lease_seconds))),
                    notification_deliveries::attempt_count
                        .eq(notification_deliveries::attempt_count + 1),
                    notification_deliveries::updated_at.eq(now),
                ))
                .returning(NotificationDeliveryRecord::as_returning())
                .get_result::<NotificationDeliveryRecord>(conn)
                .await?;
                let email = notification_rules::table
                    .filter(notification_rules::workspace_id.eq(&claimed.workspace_id))
                    .filter(notification_rules::id.eq(claimed.rule_id))
                    .select(notification_rules::email)
                    .first::<String>(conn)
                    .await?;
                Ok(Some(ClaimedNotificationDelivery {
                    workspace_id: claimed.workspace_id.clone(),
                    email,
                    run_id: claimed.run_id.map(|id| id.to_string()),
                    payload: claimed.payload.clone(),
                    delivery: delivery_to_wire(claimed)?,
                }))
            },
        )
        .await
    }

    pub async fn mark_sent(
        &self,
        workspace_id: &str,
        delivery_id: &str,
    ) -> Result<(), StorageError> {
        self.transition_delivery(workspace_id, delivery_id, true, None, None)
            .await
    }

    pub async fn retry_or_fail(
        &self,
        workspace_id: &str,
        delivery_id: &str,
        max_attempts: i32,
        error_code: &str,
        error_message: &str,
    ) -> Result<(), StorageError> {
        self.transition_delivery(
            workspace_id,
            delivery_id,
            false,
            Some(max_attempts),
            Some((error_code, error_message)),
        )
        .await
    }

    async fn transition_delivery(
        &self,
        workspace_id: &str,
        delivery_id: &str,
        sent: bool,
        max_attempts: Option<i32>,
        error: Option<(&str, &str)>,
    ) -> Result<(), StorageError> {
        let id = parse_uuid(delivery_id)?;
        let mut conn = self.connection().await?;
        let current = notification_deliveries::table
            .filter(notification_deliveries::workspace_id.eq(workspace_id))
            .filter(notification_deliveries::id.eq(id))
            .select(NotificationDeliveryRecord::as_select())
            .first::<NotificationDeliveryRecord>(&mut conn)
            .await?;
        let now = Utc::now();
        let status = if sent {
            "sent"
        } else if current.attempt_count >= max_attempts.unwrap_or(5) {
            "failed"
        } else {
            "pending"
        };
        let delay = 1_i64 << current.attempt_count.clamp(0, 8);
        diesel::update(
            notification_deliveries::table
                .filter(notification_deliveries::workspace_id.eq(workspace_id))
                .filter(notification_deliveries::id.eq(id)),
        )
        .set((
            notification_deliveries::status.eq(status),
            notification_deliveries::next_attempt_at.eq(now + Duration::seconds(delay)),
            notification_deliveries::lease_owner.eq::<Option<String>>(None),
            notification_deliveries::lease_expires_at.eq::<Option<chrono::DateTime<Utc>>>(None),
            notification_deliveries::last_error_code.eq(error.map(|value| value.0)),
            notification_deliveries::last_error_message
                .eq(error.map(|value| truncate(value.1, 500))),
            notification_deliveries::sent_at.eq(if sent { Some(now) } else { None }),
            notification_deliveries::updated_at.eq(now),
        ))
        .execute(&mut conn)
        .await?;
        Ok(())
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn enqueue_matching_on_connection(
    conn: &mut AsyncPgConnection,
    workspace_id: &str,
    environment_id: &str,
    agent_id: Option<&str>,
    rule_id: Option<Uuid>,
    event_kind: NotificationEventKind,
    subject_id: &str,
    subject_version: &str,
    run_id: Option<Uuid>,
    payload: Value,
) -> Result<usize, StorageError> {
    let event_text = event_kind_text(event_kind);
    let mut query = notification_rules::table
        .filter(notification_rules::workspace_id.eq(workspace_id))
        .filter(notification_rules::environment_id.eq(environment_id))
        .filter(notification_rules::enabled.eq(true))
        .filter(notification_rules::deleted_at.is_null())
        .into_boxed();
    if let Some(rule_id) = rule_id {
        query = query.filter(notification_rules::id.eq(rule_id));
    } else {
        query =
            query.filter(notification_rules::event_kinds.contains(vec![event_text.to_string()]));
    }
    if let Some(agent_id) = agent_id {
        query = query.filter(
            notification_rules::agent_id
                .is_null()
                .or(notification_rules::agent_id.eq(agent_id)),
        );
    } else {
        query = query.filter(notification_rules::agent_id.is_null());
    }
    let rules = query
        .select(NotificationRuleRecord::as_select())
        .load::<NotificationRuleRecord>(conn)
        .await?;
    let mut inserted = 0;
    for rule in rules {
        inserted += diesel::insert_into(notification_deliveries::table)
            .values(NewNotificationDelivery {
                workspace_id: workspace_id.to_string(),
                id: Uuid::now_v7(),
                rule_id: rule.id,
                environment_id: environment_id.to_string(),
                run_id,
                event_kind: event_text.to_string(),
                subject_id: subject_id.to_string(),
                subject_version: subject_version.to_string(),
                payload: payload.clone(),
            })
            .on_conflict((
                notification_deliveries::workspace_id,
                notification_deliveries::rule_id,
                notification_deliveries::event_kind,
                notification_deliveries::subject_id,
                notification_deliveries::subject_version,
            ))
            .do_nothing()
            .execute(conn)
            .await?;
    }
    Ok(inserted)
}

fn parse_uuid(value: &str) -> Result<Uuid, StorageError> {
    Uuid::parse_str(value).map_err(|_| StorageError::Internal("invalid UUID".into()))
}

fn truncate(value: &str, max: usize) -> String {
    value.chars().take(max).collect()
}

fn event_kind_text(kind: NotificationEventKind) -> &'static str {
    match kind {
        NotificationEventKind::EvaluationFailed => "evaluation_failed",
        NotificationEventKind::EvaluationInconclusive => "evaluation_inconclusive",
        NotificationEventKind::EvaluationError => "evaluation_error",
        NotificationEventKind::ProviderTerminalFailure => "provider_terminal_failure",
        NotificationEventKind::Test => "test",
    }
}

fn parse_event_kind(value: &str) -> Result<NotificationEventKind, StorageError> {
    match value {
        "evaluation_failed" => Ok(NotificationEventKind::EvaluationFailed),
        "evaluation_inconclusive" => Ok(NotificationEventKind::EvaluationInconclusive),
        "evaluation_error" => Ok(NotificationEventKind::EvaluationError),
        "provider_terminal_failure" => Ok(NotificationEventKind::ProviderTerminalFailure),
        "test" => Ok(NotificationEventKind::Test),
        _ => Err(StorageError::Internal(
            "invalid notification event kind".into(),
        )),
    }
}

fn rule_to_wire(row: NotificationRuleRecord) -> Result<NotificationRule, StorageError> {
    Ok(NotificationRule {
        id: row.id.to_string(),
        workspace_id: row.workspace_id,
        environment_id: row.environment_id,
        agent_id: row.agent_id,
        email: row.email,
        event_kinds: row
            .event_kinds
            .iter()
            .map(|value| parse_event_kind(value))
            .collect::<Result<_, _>>()?,
        enabled: row.enabled,
        created_at: row.created_at.to_rfc3339(),
        updated_at: row.updated_at.to_rfc3339(),
    })
}

fn delivery_to_wire(
    row: NotificationDeliveryRecord,
) -> Result<NotificationDeliverySummary, StorageError> {
    Ok(NotificationDeliverySummary {
        id: row.id.to_string(),
        rule_id: row.rule_id.to_string(),
        event_kind: parse_event_kind(&row.event_kind)?,
        subject_id: row.subject_id,
        subject_version: row.subject_version,
        status: match row.status.as_str() {
            "pending" => NotificationDeliveryStatus::Pending,
            "sending" => NotificationDeliveryStatus::Sending,
            "sent" => NotificationDeliveryStatus::Sent,
            "failed" => NotificationDeliveryStatus::Failed,
            _ => {
                return Err(StorageError::Internal(
                    "invalid notification delivery status".into(),
                ))
            }
        },
        attempt_count: row.attempt_count,
        last_error_code: row.last_error_code,
        sent_at: row.sent_at.map(|value| value.to_rfc3339()),
        created_at: row.created_at.to_rfc3339(),
        updated_at: row.updated_at.to_rfc3339(),
    })
}
