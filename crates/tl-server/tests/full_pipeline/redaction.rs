#[tokio::test]
async fn check_redacts_before_engine_evaluation() {
    let state = memory_app_state(Arc::new(Engine::empty()));
    let app = router(state, None, [0u8; 32]);

    let body = serde_json::json!({
        "agent_id": "acme-support-v3",
        "channel": "chat",
        "input": "My SIN is 123-456-789 and my email is alice@example.com.",
        "proposed_output": "I will email alice@example.com with the update.",
        "context": {
            "document_type": "T4",
            "notes": "Alice Example earns $82,000."
        },
        "redaction": {
            "mode": "server",
            "status": "not_requested",
            "entities": [],
            "input_redacted": false,
            "proposed_output_redacted": false,
            "context_redacted": false
        }
    });

    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/check")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let response_body = read_body(resp).await;
    let response_text = response_body.to_string();
    assert!(!response_text.contains("alice@example.com"));
    assert!(!response_text.contains("123-456-789"));
    assert!(!response_text.contains("$82,000"));
    assert_eq!(response_body["verdict"], "allow");
    assert_eq!(response_body["redaction"]["status"], "applied");
    // Positive assertions: every sensitive value type produced a stable
    // token. Without these, a regression that silently stops redacting
    // SIN or INCOME_AMOUNT would still pass the negative checks above.
    assert!(response_text.contains("[EMAIL_1]"));
    assert!(response_text.contains("[SIN_1]"));
    assert!(response_text.contains("[INCOME_AMOUNT_1]"));
}

#[derive(Debug)]
struct RedactedOnlySettingsStore;

#[async_trait]
impl SettingsStore for RedactedOnlySettingsStore {
    async fn get(
        &self,
        _workspace_id: &str,
    ) -> Result<WorkspaceSettings, dashboard_admin::DashboardAdminStoreError> {
        let mut settings = dashboard_admin::default_settings();
        settings.data_handling_mode = DataHandlingMode::RedactedOnly;
        Ok(settings)
    }
}

#[tokio::test]
async fn redacted_only_workspace_rejects_obvious_raw_sensitive_content() {
    let mut state = memory_app_state(Arc::new(Engine::empty()));
    state.settings_store = Arc::new(RedactedOnlySettingsStore);
    let app = router(state, None, [0u8; 32]);

    let body = serde_json::json!({
        "agent_id": "tax-document-agent",
        "channel": "chat",
        "input": "My SIN is 123-456-789.",
        "proposed_output": "Email alice@example.com."
    });

    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/check")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    let response_body = read_body(resp).await;
    assert_eq!(response_body["code"], "invalid");
    assert_eq!(
        response_body["message"],
        "workspace requires redacted check content"
    );
}

#[tokio::test]
async fn redacted_only_workspace_rejects_client_asserted_applied_with_raw_values() {
    // A misconfigured or hostile client could flip `status: applied` while
    // still shipping raw sensitive values. The server must verify by
    // scanning content; client-asserted status is not load-bearing.
    let mut state = memory_app_state(Arc::new(Engine::empty()));
    state.settings_store = Arc::new(RedactedOnlySettingsStore);
    let app = router(state, None, [0u8; 32]);

    let body = serde_json::json!({
        "agent_id": "tax-document-agent",
        "channel": "chat",
        "input": "My SIN is 123-456-789.",
        "proposed_output": "Email alice@example.com.",
        "redaction": {
            "mode": "sdk_local",
            "status": "applied",
            "entities": [],
            "input_redacted": false,
            "proposed_output_redacted": false,
            "context_redacted": false
        }
    });

    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/check")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    let response_body = read_body(resp).await;
    assert_eq!(response_body["code"], "invalid");
}

#[tokio::test]
async fn check_rejects_redaction_info_with_contradictory_fields() {
    // The wire shape lets a client claim `status: failed` while still
    // listing entities or flipping `input_redacted` - incoherent. The
    // boundary check must reject those combinations.
    let state = memory_app_state(Arc::new(Engine::empty()));
    let app = router(state, None, [0u8; 32]);

    let body = serde_json::json!({
        "agent_id": "acme-support-v3",
        "channel": "chat",
        "input": "ok",
        "proposed_output": "ok",
        "redaction": {
            "mode": "sdk_local",
            "status": "failed",
            "entities": [{
                "entity_type": "EMAIL",
                "token": "[EMAIL_1]",
                "count": 1
            }],
            "input_redacted": false,
            "proposed_output_redacted": false,
            "context_redacted": false
        }
    });

    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/check")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    let response_body = read_body(resp).await;
    assert_eq!(response_body["code"], "invalid");
    assert!(response_body["message"]
        .as_str()
        .unwrap()
        .contains("invalid redaction info"));
}

#[tokio::test]
async fn server_redaction_produces_stable_tokens_across_fields() {
    // Same raw value in input, proposed_output, and context must get the
    // same token. Without this, policies and humans can't correlate
    // sanitized references back to one entity.
    let state = memory_app_state(Arc::new(Engine::empty()));
    let app = router(state, None, [0u8; 32]);

    let body = serde_json::json!({
        "agent_id": "acme-support-v3",
        "channel": "chat",
        "input": "Contact alice@example.com today.",
        "proposed_output": "Will email alice@example.com tomorrow.",
        "context": {
            "notes": "Customer alice@example.com requested a callback."
        },
        "redaction": {
            "mode": "server",
            "status": "not_requested",
            "entities": [],
            "input_redacted": false,
            "proposed_output_redacted": false,
            "context_redacted": false
        }
    });

    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/check")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let response_body = read_body(resp).await;
    let response_text = response_body.to_string();
    assert!(!response_text.contains("alice@example.com"));
    // Token is allocated once for the value and reused across all three
    // surfaces; the entity count records three occurrences.
    let entities = response_body["redaction"]["entities"]
        .as_array()
        .expect("entities array");
    let email = entities
        .iter()
        .find(|entity| entity["entity_type"] == "EMAIL")
        .expect("EMAIL entity present");
    assert_eq!(email["token"], "[EMAIL_1]");
    assert_eq!(email["count"], 3);
}
