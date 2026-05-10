//! Escalation webhook worker.
//!
//! When `Decision::Escalate` lands, the request handler fires a payload
//! into the escalation channel. This worker drains the channel,
//! optionally persists the row in `"Escalations"` (postgres feature),
//! POSTs the JSON to the configured webhook with retries + backoff,
//! and updates the row to `sent` / `failed` based on the outcome.
//!
//! Backoff schedule defaults to the v0 plan: 1s, 5s, 30s, 2m, 10m
//! (5 attempts total). Tests can override via `RetryPolicy` so the
//! suite doesn't sleep for 12 minutes per failed-delivery test.

use std::time::Duration;

use serde::{Deserialize, Serialize};
use tl_core::Decision;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

#[cfg(feature = "postgres")]
use std::sync::Arc;
#[cfg(feature = "postgres")]
use tl_storage::EscalationRepo;
#[cfg(feature = "postgres")]
use uuid::Uuid;

/// One queued escalation. The check handler fills this in when a
/// `Decision::Escalate` is produced.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EscalationPayload {
    pub trace_id: String,
    pub agent_id: String,
    pub domain: String,
    pub decision: Decision,
}

#[derive(Debug, Clone)]
pub struct EscalationConfig {
    pub webhook_url: String,
    pub retry: RetryPolicy,
    pub channel_capacity: usize,
}

impl EscalationConfig {
    pub fn new(webhook_url: impl Into<String>) -> Self {
        Self {
            webhook_url: webhook_url.into(),
            retry: RetryPolicy::default(),
            channel_capacity: 1024,
        }
    }
}

#[derive(Debug, Clone)]
pub struct RetryPolicy {
    pub delays: Vec<Duration>,
}

impl RetryPolicy {
    pub fn max_attempts(&self) -> usize {
        // First attempt + one per delay slot.
        1 + self.delays.len()
    }
}

impl Default for RetryPolicy {
    fn default() -> Self {
        // 1s, 5s, 30s, 2m → 5 attempts total (initial + 4 retries).
        // Total in-memory wait ≈ 2 min 36 s before giving up. Anything
        // longer is picked up by the boot replay path that drains
        // EscalationRepo::list_stale_pending — no need to keep an
        // in-process timer alive for >10 min.
        Self {
            delays: vec![
                Duration::from_secs(1),
                Duration::from_secs(5),
                Duration::from_secs(30),
                Duration::from_secs(120),
            ],
        }
    }
}

/// Spawn the worker. Returns the sender to plug into AppState plus the
/// JoinHandle so the server can flush on shutdown by dropping the tx
/// and awaiting completion.
pub fn spawn_escalation_worker(
    config: EscalationConfig,
    #[cfg(feature = "postgres")] repo: Option<Arc<EscalationRepo>>,
) -> (mpsc::Sender<EscalationPayload>, JoinHandle<()>) {
    let (tx, rx) = mpsc::channel(config.channel_capacity);
    let http = reqwest::Client::builder()
        .build()
        .expect("reqwest client init");
    #[cfg(not(feature = "postgres"))]
    let handle = tokio::spawn(worker_loop(http, config, rx));
    #[cfg(feature = "postgres")]
    let handle = tokio::spawn(worker_loop(http, config, rx, repo));
    (tx, handle)
}

async fn worker_loop(
    http: reqwest::Client,
    config: EscalationConfig,
    mut rx: mpsc::Receiver<EscalationPayload>,
    #[cfg(feature = "postgres")] repo: Option<Arc<EscalationRepo>>,
) {
    while let Some(payload) = rx.recv().await {
        let http = http.clone();
        let config = config.clone();
        #[cfg(feature = "postgres")]
        let repo = repo.clone();
        tokio::spawn(async move {
            #[cfg(feature = "postgres")]
            deliver_one(&http, &config, payload, repo).await;
            #[cfg(not(feature = "postgres"))]
            deliver_one(&http, &config, payload).await;
        });
    }
}

async fn deliver_one(
    http: &reqwest::Client,
    config: &EscalationConfig,
    payload: EscalationPayload,
    #[cfg(feature = "postgres")] repo: Option<Arc<EscalationRepo>>,
) {
    #[cfg(feature = "postgres")]
    let row_id = persist_pending(&payload, &config.webhook_url, repo.as_deref()).await;

    let body = match serde_json::to_value(&payload) {
        Ok(v) => v,
        Err(e) => {
            tracing::error!(
                trace_id = %payload.trace_id,
                error = %e,
                "escalation payload serialize failed; dropping"
            );
            return;
        }
    };

    let max = config.retry.max_attempts();
    for attempt_idx in 0..max {
        if attempt_idx > 0 {
            // Sleep through the backoff slot before this retry.
            let delay = config.retry.delays[attempt_idx - 1];
            tokio::time::sleep(delay).await;
        }

        #[cfg(feature = "postgres")]
        if let (Some(rid), Some(repo)) = (row_id, repo.as_ref()) {
            let _ = repo.record_attempt(rid).await;
        }

        match http
            .post(&config.webhook_url)
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .await
        {
            Ok(resp) if resp.status().is_success() => {
                tracing::info!(
                    trace_id = %payload.trace_id,
                    status = resp.status().as_u16(),
                    attempts = attempt_idx + 1,
                    "escalation delivered"
                );
                #[cfg(feature = "postgres")]
                if let (Some(rid), Some(repo)) = (row_id, repo.as_ref()) {
                    let _ = repo.mark_sent(rid).await;
                }
                return;
            }
            Ok(resp) => {
                tracing::warn!(
                    trace_id = %payload.trace_id,
                    status = resp.status().as_u16(),
                    attempt = attempt_idx + 1,
                    "escalation non-2xx; will retry"
                );
            }
            Err(e) => {
                tracing::warn!(
                    trace_id = %payload.trace_id,
                    error = %e,
                    attempt = attempt_idx + 1,
                    "escalation transport error; will retry"
                );
            }
        }
    }

    tracing::error!(
        trace_id = %payload.trace_id,
        attempts = max,
        "escalation exhausted retries"
    );
    #[cfg(feature = "postgres")]
    if let (Some(rid), Some(repo)) = (row_id, repo.as_ref()) {
        let _ = repo.mark_failed(rid).await;
    }
}

#[cfg(feature = "postgres")]
async fn persist_pending(
    payload: &EscalationPayload,
    webhook_url: &str,
    repo: Option<&EscalationRepo>,
) -> Option<Uuid> {
    let repo = repo?;
    let id = Uuid::now_v7();
    let trace_uuid = Uuid::parse_str(&payload.trace_id).ok()?;
    let body = serde_json::to_value(payload).ok()?;
    if let Err(e) = repo
        .insert_pending(id, trace_uuid, webhook_url, &body)
        .await
    {
        tracing::warn!(error = %e, "could not persist pending escalation");
        return None;
    }
    Some(id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_retry_policy_is_five_attempts() {
        let p = RetryPolicy::default();
        assert_eq!(p.max_attempts(), 5);
        assert_eq!(p.delays.len(), 4);
        assert_eq!(p.delays[0], Duration::from_secs(1));
        assert_eq!(p.delays[3], Duration::from_secs(120));
    }

    #[test]
    fn empty_retry_policy_means_one_attempt_only() {
        let p = RetryPolicy { delays: vec![] };
        assert_eq!(p.max_attempts(), 1);
    }
}
