//! Regression gate for the delivery-worker generalization: the bytes
//! an escalation webhook receiver sees must be exactly the serialized
//! `EscalationPayload` — no envelope, no reordering, no rewrapping.

use std::time::Duration;

use tl_core::{AuthorizationEffect, Decision};
use tl_server::{spawn_escalation_worker, EscalationConfig, EscalationPayload, RetryPolicy};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn escalation_webhook_body_is_byte_identical_to_the_payload_serialization() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/escalations"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&server)
        .await;

    let mut decision = Decision::allow("trace-compat".to_string());
    decision.effect = AuthorizationEffect::RequireApproval;
    decision.reason = "tier 3 LLM judge timed out".into();
    let payload = EscalationPayload {
        trace_id: "trace-compat".into(),
        agent_id: "acme-support-v3".into(),
        domain: "customer_support".into(),
        decision,
    };
    // The historical pipeline: `serde_json::to_value(&payload)` then
    // reqwest's `.json(&body)` (which is `serde_json::to_vec`).
    let expected_bytes = serde_json::to_vec(&serde_json::to_value(&payload).unwrap()).unwrap();

    let cfg = EscalationConfig {
        webhook_url: format!("{}/escalations", server.uri()),
        retry: RetryPolicy { delays: vec![] },
        channel_capacity: 16,
    };
    #[cfg(feature = "postgres")]
    let (tx, _handle) = spawn_escalation_worker(cfg, None);
    #[cfg(not(feature = "postgres"))]
    let (tx, _handle) = spawn_escalation_worker(cfg);
    tx.send(payload).await.expect("send");

    let deadline = std::time::Instant::now() + Duration::from_secs(1);
    while server
        .received_requests()
        .await
        .unwrap_or_default()
        .is_empty()
    {
        if std::time::Instant::now() > deadline {
            panic!("no POST received within 1s");
        }
        tokio::time::sleep(Duration::from_millis(5)).await;
    }

    let received = server.received_requests().await.unwrap();
    assert_eq!(received.len(), 1);
    assert_eq!(
        received[0].body, expected_bytes,
        "escalation webhook body drifted from the payload serialization"
    );
}
