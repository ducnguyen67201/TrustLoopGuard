use chrono::{DateTime, Duration, Utc};
use diesel::dsl::{max, now};
use diesel::prelude::*;
use diesel_async::{AsyncConnection, RunQueryDsl};
use tl_core::{
    BoundaryConfidence, EvaluationJobStatus, FinalizeRunRequest, FinalizeRunResponse,
    RunBoundarySource, RunCaptureStatus, RunFinalizationSummary,
};

use crate::models::RunRecord;
use crate::schema::{run_events, runs};
use crate::StorageError;

use super::text::{parse_status, status_text};
use super::validation::parse_run_id;
use super::RunRepo;

impl RunRepo {
    pub async fn finalize(
        &self,
        workspace_id: &str,
        environment_id: &str,
        run_id: &str,
        input: FinalizeRunRequest,
        capture_wait_ms: u64,
    ) -> Result<FinalizeRunResponse, StorageError> {
        if !input.status.is_terminal() {
            return Err(StorageError::Internal(
                "final run status must be terminal".into(),
            ));
        }
        let run_uuid = parse_run_id(run_id)?;
        let requested_ended_at = input
            .ended_at
            .as_deref()
            .map(DateTime::parse_from_rfc3339)
            .transpose()
            .map_err(|error| StorageError::Internal(format!("ended_at parse: {error}")))?
            .map(|value| value.with_timezone(&Utc));
        let boundary_source = boundary_source_text(input.boundary_source);
        let boundary_confidence = input.boundary_source.confidence();
        let mut conn = self.connection().await?;
        conn.transaction::<(), StorageError, _>(async |conn| {
            let current = diesel::update(
                runs::table
                    .filter(runs::workspace_id.eq(workspace_id))
                    .filter(runs::environment_id.eq(environment_id))
                    .filter(runs::id.eq(run_uuid)),
            )
            .set(runs::updated_at.eq(runs::updated_at))
            .returning(RunRecord::as_returning())
            .get_result::<RunRecord>(conn)
            .await
            .optional()?
            .ok_or(StorageError::NotFound)?;

            if current.finalized_at.is_some() {
                let same_ended_at = requested_ended_at
                    .map(|value| current.ended_at == Some(value))
                    .unwrap_or(true);
                if parse_status(&current.status)? != input.status
                    || current.boundary_source.as_deref() != Some(boundary_source)
                    || current.expected_flush_id != input.expected_flush_id
                    || !same_ended_at
                {
                    return Err(StorageError::Conflict);
                }
                return Ok(());
            }

            if let Some(expected_sequence) = input.last_event_sequence {
                let actual_sequence = run_events::table
                    .filter(run_events::workspace_id.eq(workspace_id))
                    .filter(run_events::run_id.eq(run_uuid))
                    .select(max(run_events::sequence))
                    .first::<Option<i32>>(conn)
                    .await?
                    .unwrap_or(0);
                if actual_sequence != expected_sequence {
                    return Err(StorageError::Conflict);
                }
            }

            let finalized_at = Utc::now();
            let ended_at = requested_ended_at.unwrap_or(finalized_at);
            let capture_deadline = finalized_at
                + Duration::milliseconds(
                    i64::try_from(capture_wait_ms.min(3_600_000)).unwrap_or(3_600_000),
                );
            diesel::update(
                runs::table
                    .filter(runs::workspace_id.eq(workspace_id))
                    .filter(runs::id.eq(run_uuid)),
            )
            .set((
                runs::status.eq(status_text(input.status)),
                runs::ended_at.eq(ended_at),
                runs::boundary_source.eq(boundary_source),
                runs::boundary_confidence.eq(boundary_confidence_text(boundary_confidence)),
                runs::finalized_at.eq(finalized_at),
                runs::capture_status.eq("waiting"),
                runs::capture_deadline.eq(capture_deadline),
                runs::expected_flush_id.eq(input.expected_flush_id.as_deref()),
                runs::updated_at.eq(now),
            ))
            .execute(conn)
            .await?;
            Ok(())
        })
        .await?;
        drop(conn);

        let run = self.get(workspace_id, run_id).await?;
        let finalization = self
            .finalization(workspace_id, environment_id, run_id)
            .await?
            .ok_or_else(|| StorageError::Internal("finalized run has no finalization".into()))?;
        Ok(FinalizeRunResponse {
            run,
            finalization,
            evaluation_status: EvaluationJobStatus::WaitingCapture,
        })
    }

    pub async fn finalization(
        &self,
        workspace_id: &str,
        environment_id: &str,
        run_id: &str,
    ) -> Result<Option<RunFinalizationSummary>, StorageError> {
        let run_uuid = parse_run_id(run_id)?;
        let mut conn = self.connection().await?;
        let row = runs::table
            .filter(runs::workspace_id.eq(workspace_id))
            .filter(runs::environment_id.eq(environment_id))
            .filter(runs::id.eq(run_uuid))
            .select((
                runs::finalized_at,
                runs::boundary_source,
                runs::boundary_confidence,
                runs::capture_status,
                runs::capture_deadline,
                runs::expected_flush_id,
            ))
            .first::<(
                Option<DateTime<Utc>>,
                Option<String>,
                Option<String>,
                String,
                Option<DateTime<Utc>>,
                Option<String>,
            )>(&mut conn)
            .await
            .optional()?;
        let Some((
            Some(finalized_at),
            Some(source),
            Some(confidence),
            capture,
            Some(deadline),
            flush,
        )) = row
        else {
            return Ok(None);
        };
        Ok(Some(RunFinalizationSummary {
            finalized_at: finalized_at.to_rfc3339(),
            boundary_source: parse_boundary_source(&source)?,
            boundary_confidence: parse_boundary_confidence(&confidence)?,
            capture_status: parse_capture_status(&capture)?,
            capture_deadline: deadline.to_rfc3339(),
            expected_flush_id: flush,
        }))
    }
}

pub(crate) fn boundary_source_text(source: RunBoundarySource) -> &'static str {
    match source {
        RunBoundarySource::ExplicitSdk => "explicit_sdk",
        RunBoundarySource::FrameworkAdapter => "framework_adapter",
        RunBoundarySource::OtelSessionEnd => "otel_session_end",
        RunBoundarySource::RootSpanEnd => "root_span_end",
        RunBoundarySource::IdleTimeout => "idle_timeout",
        RunBoundarySource::MaxDuration => "max_duration",
        RunBoundarySource::Admin => "admin",
        RunBoundarySource::LegacySdk => "legacy_sdk",
    }
}

fn boundary_confidence_text(confidence: BoundaryConfidence) -> &'static str {
    match confidence {
        BoundaryConfidence::Authoritative => "authoritative",
        BoundaryConfidence::Strong => "strong",
        BoundaryConfidence::Inferred => "inferred",
    }
}

fn parse_boundary_source(value: &str) -> Result<RunBoundarySource, StorageError> {
    match value {
        "explicit_sdk" => Ok(RunBoundarySource::ExplicitSdk),
        "framework_adapter" => Ok(RunBoundarySource::FrameworkAdapter),
        "otel_session_end" => Ok(RunBoundarySource::OtelSessionEnd),
        "root_span_end" => Ok(RunBoundarySource::RootSpanEnd),
        "idle_timeout" => Ok(RunBoundarySource::IdleTimeout),
        "max_duration" => Ok(RunBoundarySource::MaxDuration),
        "admin" => Ok(RunBoundarySource::Admin),
        "legacy_sdk" => Ok(RunBoundarySource::LegacySdk),
        other => Err(StorageError::Internal(format!(
            "unknown boundary source `{other}`"
        ))),
    }
}

fn parse_boundary_confidence(value: &str) -> Result<BoundaryConfidence, StorageError> {
    match value {
        "authoritative" => Ok(BoundaryConfidence::Authoritative),
        "strong" => Ok(BoundaryConfidence::Strong),
        "inferred" => Ok(BoundaryConfidence::Inferred),
        other => Err(StorageError::Internal(format!(
            "unknown boundary confidence `{other}`"
        ))),
    }
}

fn parse_capture_status(value: &str) -> Result<RunCaptureStatus, StorageError> {
    match value {
        "open" => Ok(RunCaptureStatus::Open),
        "waiting" => Ok(RunCaptureStatus::Waiting),
        "complete" => Ok(RunCaptureStatus::Complete),
        "incomplete" => Ok(RunCaptureStatus::Incomplete),
        other => Err(StorageError::Internal(format!(
            "unknown run capture status `{other}`"
        ))),
    }
}
