use std::time::Duration;

use chrono::{Duration as ChronoDuration, Utc};
use tl_core::{FinalizeRunRequest, RunBoundarySource, RunStatus};

use crate::AppState;

pub(crate) fn spawn_gateway_session_worker(state: AppState) -> tokio::task::JoinHandle<()> {
    let idle_seconds = env_seconds("TL_GATEWAY_SESSION_IDLE_SECONDS", 300);
    let max_seconds = env_seconds("TL_GATEWAY_SESSION_MAX_SECONDS", 86_400);
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(30));
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            interval.tick().await;
            let now = Utc::now();
            let stale = match state
                .run_store
                .list_stale_gateway_runs(
                    now - ChronoDuration::seconds(idle_seconds),
                    now - ChronoDuration::seconds(max_seconds),
                    100,
                )
                .await
            {
                Ok(value) => value,
                Err(error) => {
                    tracing::warn!(error = %error, "gateway session sweep failed");
                    continue;
                }
            };
            for run in stale {
                let boundary_source = if run.max_duration_exceeded {
                    RunBoundarySource::MaxDuration
                } else {
                    RunBoundarySource::IdleTimeout
                };
                let status = if run.max_duration_exceeded {
                    RunStatus::TimedOut
                } else {
                    RunStatus::Completed
                };
                let capture_wait_ms = state
                    .evaluation_store
                    .get_profile(&run.workspace_id, &run.environment_id, &run.agent_id)
                    .await
                    .ok()
                    .flatten()
                    .filter(|profile| profile.enabled)
                    .map_or(30_000, |profile| profile.max_capture_wait_ms);
                match state
                    .run_store
                    .finalize(
                        &run.workspace_id,
                        &run.environment_id,
                        &run.run_id,
                        FinalizeRunRequest {
                            status,
                            ended_at: None,
                            boundary_source,
                            expected_flush_id: None,
                            last_event_sequence: None,
                        },
                        capture_wait_ms,
                    )
                    .await
                {
                    Ok(_) => {
                        tracing::info!(workspace_id = %run.workspace_id, run_id = %run.run_id, ?boundary_source, "gateway session automatically finalized")
                    }
                    Err(
                        crate::runs::RunStoreError::Conflict | crate::runs::RunStoreError::NotFound,
                    ) => {
                        tracing::debug!(run_id = %run.run_id, "gateway session finalization race lost")
                    }
                    Err(error) => {
                        tracing::warn!(run_id = %run.run_id, error = %error, "gateway session finalization failed")
                    }
                }
            }
        }
    })
}

fn env_seconds(name: &str, default: i64) -> i64 {
    std::env::var(name)
        .ok()
        .and_then(|value| value.parse::<i64>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(default)
}
