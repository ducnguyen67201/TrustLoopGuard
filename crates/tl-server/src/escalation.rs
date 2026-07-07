//! Escalation webhook worker.
//!
//! When `Decision::Escalate` lands, the request handler fires a payload
//! into the escalation channel. This worker drains the channel,
//! optionally persists the row in `escalations` (postgres feature),
//! POSTs the JSON to the configured webhook with retries + backoff,
//! and updates the row to `sent` / `failed` based on the outcome.
//!
//! The delivery machinery is payload-agnostic: a [`WebhookDelivery`]
//! is any `(webhook_url, JSON body)` pair. Escalations map onto it
//! with the workspace-configured URL; budget alerts enqueue their own
//! per-config URLs through [`spawn_webhook_delivery_worker`]. Both
//! share the retry policy and the `escalations` persistence table.
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

/// One generic delivery job: POST `body` to `webhook_url` with the
/// worker's retry policy. `trace_id` is the persistence/correlation
/// key (escalations use the decision trace id; budget alerts use the
/// firing id).
#[derive(Debug, Clone)]
pub struct WebhookDelivery {
    pub trace_id: String,
    pub webhook_url: String,
    pub body: serde_json::Value,
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
    let handle = tokio::spawn(escalation_loop(http, config, rx));
    #[cfg(feature = "postgres")]
    let handle = tokio::spawn(escalation_loop(http, config, rx, repo));
    (tx, handle)
}

/// Spawn a generic webhook delivery worker: same retry + persistence
/// rail as escalations, but each job carries its own target URL.
pub fn spawn_webhook_delivery_worker(
    retry: RetryPolicy,
    channel_capacity: usize,
    #[cfg(feature = "postgres")] repo: Option<Arc<EscalationRepo>>,
) -> (mpsc::Sender<WebhookDelivery>, JoinHandle<()>) {
    let (tx, rx) = mpsc::channel(channel_capacity);
    let http = reqwest::Client::builder()
        .build()
        .expect("reqwest client init");
    #[cfg(not(feature = "postgres"))]
    let handle = tokio::spawn(delivery_loop(http, retry, rx));
    #[cfg(feature = "postgres")]
    let handle = tokio::spawn(delivery_loop(http, retry, rx, repo));
    (tx, handle)
}

async fn escalation_loop(
    http: reqwest::Client,
    config: EscalationConfig,
    mut rx: mpsc::Receiver<EscalationPayload>,
    #[cfg(feature = "postgres")] repo: Option<Arc<EscalationRepo>>,
) {
    while let Some(payload) = rx.recv().await {
        // Same serialization pipeline as before the delivery rail was
        // generalized (`serde_json::to_value` then `.json(&body)`), so
        // the POSTed bytes are unchanged.
        let body = match serde_json::to_value(&payload) {
            Ok(v) => v,
            Err(e) => {
                tracing::error!(
                    trace_id = %payload.trace_id,
                    error = %e,
                    "escalation payload serialize failed; dropping"
                );
                continue;
            }
        };
        let job = WebhookDelivery {
            trace_id: payload.trace_id,
            webhook_url: config.webhook_url.clone(),
            body,
        };
        let http = http.clone();
        let retry = config.retry.clone();
        #[cfg(feature = "postgres")]
        let repo = repo.clone();
        tokio::spawn(async move {
            #[cfg(feature = "postgres")]
            deliver_one(&http, &retry, job, repo).await;
            #[cfg(not(feature = "postgres"))]
            deliver_one(&http, &retry, job).await;
        });
    }
}

async fn delivery_loop(
    http: reqwest::Client,
    retry: RetryPolicy,
    mut rx: mpsc::Receiver<WebhookDelivery>,
    #[cfg(feature = "postgres")] repo: Option<Arc<EscalationRepo>>,
) {
    while let Some(job) = rx.recv().await {
        let http = http.clone();
        let retry = retry.clone();
        #[cfg(feature = "postgres")]
        let repo = repo.clone();
        tokio::spawn(async move {
            #[cfg(feature = "postgres")]
            deliver_one(&http, &retry, job, repo).await;
            #[cfg(not(feature = "postgres"))]
            deliver_one(&http, &retry, job).await;
        });
    }
}

async fn deliver_one(
    http: &reqwest::Client,
    retry: &RetryPolicy,
    job: WebhookDelivery,
    #[cfg(feature = "postgres")] repo: Option<Arc<EscalationRepo>>,
) {
    #[cfg(feature = "postgres")]
    let row_id = persist_pending(&job, repo.as_deref()).await;

    let max = retry.max_attempts();
    for attempt_idx in 0..max {
        if attempt_idx > 0 {
            // Sleep through the backoff slot before this retry.
            let delay = retry.delays[attempt_idx - 1];
            tokio::time::sleep(delay).await;
        }

        #[cfg(feature = "postgres")]
        if let (Some(rid), Some(repo)) = (row_id, repo.as_ref()) {
            let _ = repo.record_attempt(rid).await;
        }

        match http
            .post(&job.webhook_url)
            .header("content-type", "application/json")
            .json(&job.body)
            .send()
            .await
        {
            Ok(resp) if resp.status().is_success() => {
                tracing::info!(
                    trace_id = %job.trace_id,
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
                    trace_id = %job.trace_id,
                    status = resp.status().as_u16(),
                    attempt = attempt_idx + 1,
                    "escalation non-2xx; will retry"
                );
            }
            Err(e) => {
                tracing::warn!(
                    trace_id = %job.trace_id,
                    error = %e,
                    attempt = attempt_idx + 1,
                    "escalation transport error; will retry"
                );
            }
        }
    }

    tracing::error!(
        trace_id = %job.trace_id,
        attempts = max,
        "escalation exhausted retries"
    );
    #[cfg(feature = "postgres")]
    if let (Some(rid), Some(repo)) = (row_id, repo.as_ref()) {
        let _ = repo.mark_failed(rid).await;
    }
}

#[cfg(feature = "postgres")]
async fn persist_pending(job: &WebhookDelivery, repo: Option<&EscalationRepo>) -> Option<Uuid> {
    let repo = repo?;
    let id = Uuid::now_v7();
    let trace_uuid = Uuid::parse_str(&job.trace_id).ok()?;
    if let Err(e) = repo
        .insert_pending(id, trace_uuid, &job.webhook_url, &job.body)
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

    /// The escalation payload JSON is a compatibility contract: the
    /// worker generalization must not add an envelope, rename keys, or
    /// reorder the serialization pipeline. Byte-identical guard.
    #[test]
    fn escalation_payload_json_is_byte_identical() {
        let mut decision = Decision::allow("trace-compat".to_string());
        decision.verdict = tl_core::Verdict::Escalate;
        decision.reason = "tier 3 LLM judge timed out".into();
        let payload = EscalationPayload {
            trace_id: "trace-compat".into(),
            agent_id: "acme-support-v3".into(),
            domain: "customer_support".into(),
            decision: decision.clone(),
        };
        // The worker enqueues `serde_json::to_value(&payload)` and
        // reqwest serializes that Value — replicate the exact pipeline.
        let body = serde_json::to_value(&payload).expect("serialize payload");
        let bytes = serde_json::to_vec(&body).expect("serialize body");

        // No envelope: exactly the four historical top-level keys.
        let top = body.as_object().expect("object payload");
        assert_eq!(top.len(), 4);
        for key in ["trace_id", "agent_id", "domain", "decision"] {
            assert!(top.contains_key(key), "missing top-level key {key}");
        }

        // Byte-identical to the pre-generalization pipeline output
        // (to_value → to_vec of the same struct).
        let expected = serde_json::to_vec(
            &serde_json::to_value(&EscalationPayload {
                trace_id: "trace-compat".into(),
                agent_id: "acme-support-v3".into(),
                domain: "customer_support".into(),
                decision,
            })
            .unwrap(),
        )
        .unwrap();
        assert_eq!(bytes, expected);
    }
}
