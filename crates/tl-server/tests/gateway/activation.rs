#[tokio::test]
async fn activation_is_idempotent_and_reports_exact_verification_as_pending() {
    let app = build_app().await;
    let workspace = "ws_gateway_activation";
    let _runtime_key = create_workspace_key(app.clone(), workspace).await;

    let fallback = app
        .clone()
        .oneshot(json_request(
            "POST",
            "/v1/gateway/provider-connections",
            "sk-internal",
            workspace,
            json!({
                "id": "provider-fallback",
                "display_name": "Fallback",
                "kind": "openai_compatible",
                "base_url": "https://fallback.example.com",
                "default_model": "fallback-model",
                "provider_api_key": "fallback-secret"
            }),
        ))
        .await
        .unwrap();
    assert_eq!(fallback.status(), StatusCode::CREATED);

    let body = json!({
        "provider": {
            "display_name": "Primary",
            "kind": "openai_compatible",
            "base_url": "https://primary.example.com",
            "default_model": "primary-model",
            "provider_api_key": "primary-secret"
        },
        "agent": {
            "mode": "new",
            "name": "Activation agent",
            "purpose": "Verify production traffic"
        },
        "route_display_name": "Activation production",
        "alert_email": "ops@example.com",
        "alerts_deferred": false,
        "verification_session_id": "verify-activation-1",
        "data_handling_mode": "no_body_retention",
        "confirm_workspace_privacy_change": true,
        "reliability_mode": "standard",
        "fallback_provider_connection_ids": ["provider-fallback"]
    });

    let first = app
        .clone()
        .oneshot(json_request(
            "POST",
            "/v1/gateway/activations",
            "sk-internal",
            workspace,
            body.clone(),
        ))
        .await
        .unwrap();
    let first_status = first.status();
    let first_body = read_body(first).await;
    assert_eq!(first_status, StatusCode::CREATED, "{first_body}");
    assert_eq!(first_body["verification_session_id"], "verify-activation-1");
    assert_eq!(first_body["route"]["reliability_mode"], "standard");
    assert_eq!(
        first_body["route"]["fallback_provider_connection_ids"],
        json!(["provider-fallback"])
    );
    assert!(first_body["notification_rule"]["id"].is_string());
    assert_eq!(first_body["readiness"]["status"], "needs_attention");
    assert_eq!(
        first_body["readiness"]["checks"]
            .as_array()
            .unwrap()
            .iter()
            .find(|check| check["id"] == "traffic_seen")
            .unwrap()["ready"],
        false
    );

    let second = app
        .clone()
        .oneshot(json_request(
            "POST",
            "/v1/gateway/activations",
            "sk-internal",
            workspace,
            body,
        ))
        .await
        .unwrap();
    assert_eq!(second.status(), StatusCode::CREATED);
    let second_body = read_body(second).await;
    assert_eq!(second_body["route"]["id"], first_body["route"]["id"]);
    assert_eq!(
        second_body["notification_rule"]["id"],
        first_body["notification_rule"]["id"]
    );

    let rules = app
        .oneshot(json_request(
            "GET",
            "/v1/notification-rules",
            "sk-internal",
            workspace,
            json!({}),
        ))
        .await
        .unwrap();
    let rules_body = read_body(rules).await;
    assert_eq!(rules_body["notification_rules"].as_array().unwrap().len(), 1);
}

#[tokio::test]
async fn activation_supports_explicit_alert_deferral_without_claiming_ready() {
    let app = build_app().await;
    let workspace = "ws_gateway_activation_deferred";
    let _runtime_key = create_workspace_key(app.clone(), workspace).await;
    let response = app
        .oneshot(json_request(
            "POST",
            "/v1/gateway/activations",
            "sk-internal",
            workspace,
            json!({
                "provider": {
                    "display_name": "Deferred provider",
                    "kind": "openai_compatible",
                    "base_url": "https://provider.example.com",
                    "default_model": "model",
                    "provider_api_key": "provider-secret"
                },
                "agent": {
                    "mode": "new",
                    "name": "Deferred agent",
                    "purpose": "Exercise alert deferral"
                },
                "route_display_name": "Deferred production",
                "alert_email": "",
                "alerts_deferred": true,
                "verification_session_id": "verify-deferred",
                "data_handling_mode": "no_body_retention",
                "confirm_workspace_privacy_change": true,
                "reliability_mode": "standard",
                "fallback_provider_connection_ids": []
            }),
        ))
        .await
        .unwrap();
    let response_status = response.status();
    let body = read_body(response).await;
    assert_eq!(response_status, StatusCode::CREATED, "{body}");
    assert!(body.get("notification_rule").is_none());
    assert_eq!(body["alerts_deferred"], true);
    assert_eq!(body["readiness"]["status"], "needs_attention");
}

#[tokio::test]
async fn activation_requires_explicit_deferral_when_email_transport_is_unavailable() {
    let app = build_app_with_notification_transport(false).await;
    let workspace = "ws_gateway_activation_missing_smtp";
    let _runtime_key = create_workspace_key(app.clone(), workspace).await;
    let response = app
        .clone()
        .oneshot(json_request(
            "POST",
            "/v1/gateway/activations",
            "sk-internal",
            workspace,
            json!({
                "provider": {
                    "display_name": "SMTP provider",
                    "kind": "openai_compatible",
                    "base_url": "https://provider.example.com",
                    "default_model": "model",
                    "provider_api_key": "provider-secret"
                },
                "agent": {
                    "mode": "new",
                    "name": "SMTP agent",
                    "purpose": "Require explicit alert deferral"
                },
                "route_display_name": "SMTP production",
                "alert_email": "ops@example.com",
                "alerts_deferred": false,
                "data_handling_mode": "no_body_retention",
                "confirm_workspace_privacy_change": true,
                "reliability_mode": "standard",
                "fallback_provider_connection_ids": []
            }),
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    let body = read_body(response).await;
    assert_eq!(body["details"]["activation_step"], "notification_transport");
    assert_eq!(body["details"]["ready_resource_ids"], json!([]));

    let providers = app
        .oneshot(json_request(
            "GET",
            "/v1/gateway/provider-connections",
            "sk-internal",
            workspace,
            json!({}),
        ))
        .await
        .unwrap();
    assert!(read_body(providers).await["provider_connections"]
        .as_array()
        .unwrap()
        .is_empty());
}

#[tokio::test]
async fn activation_validates_fallbacks_before_persisting_the_primary_provider() {
    let app = build_app().await;
    let workspace = "ws_gateway_activation_invalid_fallback";
    let _runtime_key = create_workspace_key(app.clone(), workspace).await;
    let response = app
        .clone()
        .oneshot(json_request(
            "POST",
            "/v1/gateway/activations",
            "sk-internal",
            workspace,
            json!({
                "provider": {
                    "display_name": "Uncommitted primary",
                    "kind": "openai_compatible",
                    "base_url": "https://provider.example.com",
                    "default_model": "model",
                    "provider_api_key": "provider-secret"
                },
                "agent": {
                    "mode": "new",
                    "name": "Fallback agent",
                    "purpose": "Validate before writes"
                },
                "route_display_name": "Fallback validation",
                "alert_email": "ops@example.com",
                "data_handling_mode": "no_body_retention",
                "confirm_workspace_privacy_change": true,
                "reliability_mode": "standard",
                "fallback_provider_connection_ids": ["missing-fallback"]
            }),
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let body = read_body(response).await;
    assert_eq!(body["details"]["activation_step"], "gateway_route");
    assert_eq!(body["details"]["ready_resource_ids"], json!([]));

    let providers = app
        .oneshot(json_request(
            "GET",
            "/v1/gateway/provider-connections",
            "sk-internal",
            workspace,
            json!({}),
        ))
        .await
        .unwrap();
    assert!(read_body(providers).await["provider_connections"]
        .as_array()
        .unwrap()
        .is_empty());
}

#[tokio::test]
async fn activation_conflict_preserves_ready_resources_and_agent_meaning() {
    let app = build_app().await;
    let workspace = "ws_gateway_activation_conflict";
    let _runtime_key = create_workspace_key(app.clone(), workspace).await;
    let mut body = json!({
        "provider": {
            "display_name": "Conflict provider",
            "kind": "openai_compatible",
            "base_url": "https://provider.example.com",
            "default_model": "model",
            "provider_api_key": "provider-secret"
        },
        "agent": {
            "mode": "new",
            "name": "Conflict agent",
            "purpose": "Original purpose"
        },
        "route_display_name": "Conflict production",
        "alert_email": "ops@example.com",
        "data_handling_mode": "no_body_retention",
        "confirm_workspace_privacy_change": true,
        "reliability_mode": "standard",
        "fallback_provider_connection_ids": []
    });
    let first = app
        .clone()
        .oneshot(json_request(
            "POST",
            "/v1/gateway/activations",
            "sk-internal",
            workspace,
            body.clone(),
        ))
        .await
        .unwrap();
    let first_status = first.status();
    let first_body = read_body(first).await;
    assert_eq!(first_status, StatusCode::CREATED, "{first_body}");

    body["agent"]["purpose"] = json!("Different purpose");
    let conflict = app
        .oneshot(json_request(
            "POST",
            "/v1/gateway/activations",
            "sk-internal",
            workspace,
            body,
        ))
        .await
        .unwrap();
    assert_eq!(conflict.status(), StatusCode::CONFLICT);
    let conflict_body = read_body(conflict).await;
    assert_eq!(conflict_body["code"], "conflict");
    assert_eq!(conflict_body["details"]["activation_step"], "agent");
    assert_eq!(
        conflict_body["details"]["ready_resource_ids"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert_eq!(conflict_body["details"]["retriable"], false);
}
