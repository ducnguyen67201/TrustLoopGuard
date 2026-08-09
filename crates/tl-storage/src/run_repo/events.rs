use chrono::{DateTime, Utc};
use diesel::dsl::max;
use diesel::prelude::*;
use diesel_async::{AsyncConnection, RunQueryDsl};
use tl_core::{CreateRunEventRequest, RunEventSummary};
use uuid::Uuid;

use crate::models::{NewRunEvent, RunEventRecord};
use crate::schema::{run_events, run_participants, runs};
use crate::StorageError;

use super::summary::event_summary;
use super::text::event_kind_text;
use super::validation::{
    non_empty_string, normalize_metadata, parse_run_id, validate_create_run_event,
};
use super::RunRepo;

impl RunRepo {
    pub async fn create_event(
        &self,
        workspace_id: &str,
        run_id: &str,
        input: CreateRunEventRequest,
    ) -> Result<RunEventSummary, StorageError> {
        validate_create_run_event(&input)?;
        let run_uuid = parse_run_id(run_id)?;
        let mut conn = self.connection().await?;
        let id = conn
            .transaction::<Uuid, StorageError, _>(async |conn| {
                let locked_run = diesel::update(
                    runs::table
                        .filter(runs::workspace_id.eq(workspace_id))
                        .filter(runs::id.eq(run_uuid)),
                )
                .set(runs::updated_at.eq(runs::updated_at))
                .returning((runs::agent_id, runs::environment_id, runs::finalized_at))
                .get_result::<(String, String, Option<DateTime<Utc>>)>(conn)
                .await
                .optional()?
                .ok_or(StorageError::NotFound)?;
                if locked_run.2.is_some() {
                    return Err(StorageError::Conflict);
                }
                let agent_id = input
                    .agent_id
                    .as_deref()
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .unwrap_or(locked_run.0.as_str())
                    .to_string();
                let participant = run_participants::table
                    .filter(run_participants::workspace_id.eq(workspace_id))
                    .filter(run_participants::environment_id.eq(&locked_run.1))
                    .filter(run_participants::run_id.eq(run_uuid))
                    .filter(run_participants::agent_id.eq(&agent_id))
                    .select(run_participants::agent_id)
                    .first::<String>(conn)
                    .await
                    .optional()?;
                if participant.is_none() {
                    return Err(StorageError::NotFound);
                }

                let sequence = match input.sequence {
                    Some(sequence) => sequence,
                    None => {
                        let current = run_events::table
                            .filter(run_events::workspace_id.eq(workspace_id))
                            .filter(run_events::run_id.eq(run_uuid))
                            .select(max(run_events::sequence))
                            .first::<Option<i32>>(conn)
                            .await?;
                        current.unwrap_or(0) + 1
                    }
                };
                let occurred_at = match input.occurred_at {
                    Some(value) => DateTime::parse_from_rfc3339(&value)
                        .map_err(|e| StorageError::Internal(format!("occurred_at parse: {e}")))?
                        .with_timezone(&Utc),
                    None => Utc::now(),
                };
                let id = Uuid::now_v7();
                let event = NewRunEvent {
                    workspace_id: workspace_id.to_string(),
                    id,
                    run_id: run_uuid,
                    agent_id,
                    sequence,
                    kind: event_kind_text(input.kind).to_string(),
                    label: input.label.and_then(|value| non_empty_string(value.trim())),
                    input_summary: input
                        .input_summary
                        .and_then(|value| non_empty_string(value.trim())),
                    output_summary: input
                        .output_summary
                        .and_then(|value| non_empty_string(value.trim())),
                    metadata: normalize_metadata(input.metadata),
                    occurred_at,
                };
                diesel::insert_into(run_events::table)
                    .values(&event)
                    .execute(conn)
                    .await?;
                Ok(id)
            })
            .await?;

        drop(conn);
        self.event(workspace_id, &id.to_string()).await
    }

    pub async fn events(
        &self,
        workspace_id: &str,
        run_id: &str,
        limit: i64,
    ) -> Result<Vec<RunEventSummary>, StorageError> {
        let id = parse_run_id(run_id)?;
        let mut conn = self.connection().await?;
        let records = run_events::table
            .filter(run_events::workspace_id.eq(workspace_id))
            .filter(run_events::run_id.eq(id))
            .select(RunEventRecord::as_select())
            .order((run_events::sequence.asc(), run_events::occurred_at.asc()))
            .limit(limit.clamp(1, 200))
            .load::<RunEventRecord>(&mut conn)
            .await
            .map_err(|e| StorageError::Internal(format!("run events: {e}")))?;
        records.into_iter().map(event_summary).collect()
    }

    pub async fn event_belongs_to_run(
        &self,
        workspace_id: &str,
        run_id: &str,
        event_id: &str,
    ) -> Result<(), StorageError> {
        let run_id = parse_run_id(run_id)?;
        let event_id = parse_run_id(event_id)?;
        let mut conn = self.connection().await?;
        run_events::table
            .filter(run_events::workspace_id.eq(workspace_id))
            .filter(run_events::run_id.eq(run_id))
            .filter(run_events::id.eq(event_id))
            .select(run_events::id)
            .first::<Uuid>(&mut conn)
            .await
            .optional()
            .map_err(|e| StorageError::Internal(format!("run event ownership: {e}")))?
            .map(|_| ())
            .ok_or(StorageError::NotFound)
    }

    async fn event(
        &self,
        workspace_id: &str,
        event_id: &str,
    ) -> Result<RunEventSummary, StorageError> {
        let id = parse_run_id(event_id)?;
        let mut conn = self.connection().await?;
        let record = run_events::table
            .filter(run_events::workspace_id.eq(workspace_id))
            .filter(run_events::id.eq(id))
            .select(RunEventRecord::as_select())
            .first::<RunEventRecord>(&mut conn)
            .await
            .optional()
            .map_err(|e| StorageError::Internal(format!("run event get: {e}")))?
            .ok_or(StorageError::NotFound)?;
        event_summary(record)
    }
}
