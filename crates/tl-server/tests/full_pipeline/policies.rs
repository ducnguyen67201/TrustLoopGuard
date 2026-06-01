#[tokio::test]
async fn disabled_policy_no_longer_changes_check_decision() {
    let state = memory_app_state(Arc::new(Engine::empty()));
    let app = router(state, None, [0u8; 32]);

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/policies")
                .header(header::CONTENT_TYPE, "application/yaml")
                .body(Body::from(REFUND_POLICY_YAML))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);

    let check_body = serde_json::json!({
        "agent_id": "acme-support-v3",
        "channel": "chat",
        "input": "Can I get my money back?",
        "proposed_output": "Yes, I can promise a guaranteed refund."
    });

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/check")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(check_body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let decision: Decision = serde_json::from_value(read_body(resp).await).unwrap();
    assert_eq!(decision.verdict, Verdict::Block);
    assert_eq!(decision.triggered_policies[0].id, "refund-guarantee");

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri("/v1/policies/refund-guarantee/enabled")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(r#"{"enabled":false}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/check")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(check_body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let decision: Decision = serde_json::from_value(read_body(resp).await).unwrap();
    assert_eq!(decision.verdict, Verdict::Allow);
    assert!(decision.triggered_policies.is_empty());
}

#[tokio::test]
async fn same_agent_can_have_different_policy_deployments_per_environment() {
    let state = memory_app_state(Arc::new(Engine::empty()));
    let app = router(state, None, [0u8; 32]);

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/environments")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    serde_json::json!({
                        "slug": "dev",
                        "name": "Development"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
    let environment = read_body(resp).await;
    let dev_environment_id = environment["id"].as_str().unwrap();

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/policies")
                .header(header::CONTENT_TYPE, "application/yaml")
                .body(Body::from(REFUND_POLICY_YAML))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);

    let check_body = serde_json::json!({
        "agent_id": "acme-support-v3",
        "channel": "chat",
        "input": "Can I get my money back?",
        "proposed_output": "Yes, I can promise a guaranteed refund."
    });

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/check")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(check_body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let decision: Decision = serde_json::from_value(read_body(resp).await).unwrap();
    assert_eq!(decision.verdict, Verdict::Block);

    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/check")
                .header(header::CONTENT_TYPE, "application/json")
                .header("x-tlg-environment-id", dev_environment_id)
                .body(Body::from(check_body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let decision: Decision = serde_json::from_value(read_body(resp).await).unwrap();
    assert_eq!(decision.verdict, Verdict::Allow);
    assert!(decision.triggered_policies.is_empty());
}

#[tokio::test]
async fn check_allows_sensitive_text_when_no_policy_is_deployed() {
    // Runtime decisions come from stored policies. With no deployed
    // policies, sensitive-looking text does not trigger a hardcoded block.
    let state = memory_app_state(Arc::new(Engine::empty()));
    let app = router(state, None, [0u8; 32]);

    let body = serde_json::json!({
        "agent_id": "anon",
        "channel": "chat",
        "input": "send my number",
        "proposed_output": "Sure — call me at 415-555-1212"
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
    let decision: Decision = serde_json::from_value(read_body(resp).await).unwrap();
    assert_eq!(decision.verdict, Verdict::Allow);
    assert!(decision.triggered_policies.is_empty());
}

#[tokio::test]
async fn check_uses_enabled_policy_created_through_authoring_api() {
    let state = memory_app_state(Arc::new(Engine::empty()));
    let app = router(state, None, [0u8; 32]);

    let publish = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/policies")
                .header(header::CONTENT_TYPE, "application/yaml")
                .body(Body::from(REFUND_POLICY_YAML))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(publish.status(), StatusCode::CREATED);

    let check_body = serde_json::json!({
        "agent_id": "acme-support-v3",
        "channel": "chat",
        "input": "can I get a refund?",
        "proposed_output": "Yes, you get a guaranteed refund."
    });
    let blocked = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/check")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(check_body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(blocked.status(), StatusCode::OK);
    let decision: Decision = serde_json::from_value(read_body(blocked).await).unwrap();
    assert_eq!(decision.verdict, Verdict::Block);
    assert!(decision
        .triggered_policies
        .iter()
        .any(|policy| policy.id == "refund-guarantee"));

    let disabled = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri("/v1/policies/refund-guarantee/enabled")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(r#"{"enabled":false}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(disabled.status(), StatusCode::OK);

    let allowed = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/check")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(check_body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(allowed.status(), StatusCode::OK);
    let decision: Decision = serde_json::from_value(read_body(allowed).await).unwrap();
    assert_eq!(decision.verdict, Verdict::Allow);
    assert!(!decision
        .triggered_policies
        .iter()
        .any(|policy| policy.id == "refund-guarantee"));
}

#[tokio::test]
async fn check_uses_only_runtime_policies_from_request_workspace() {
    let state = memory_app_state(Arc::new(Engine::empty()));
    let app = router(state, None, [0u8; 32]);

    let publish = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/policies")
                .header(header::CONTENT_TYPE, "application/yaml")
                .header("x-tlg-workspace-id", "ws_alpha")
                .body(Body::from(REFUND_POLICY_YAML))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(publish.status(), StatusCode::CREATED);

    let check_body = serde_json::json!({
        "agent_id": "acme-support-v3",
        "channel": "chat",
        "input": "can I get a refund?",
        "proposed_output": "Yes, you get a guaranteed refund."
    });

    let blocked = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/check")
                .header(header::CONTENT_TYPE, "application/json")
                .header("x-tlg-workspace-id", "ws_alpha")
                .body(Body::from(check_body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(blocked.status(), StatusCode::OK);
    let decision: Decision = serde_json::from_value(read_body(blocked).await).unwrap();
    assert_eq!(decision.verdict, Verdict::Block);

    let allowed = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/check")
                .header(header::CONTENT_TYPE, "application/json")
                .header("x-tlg-workspace-id", "ws_beta")
                .body(Body::from(check_body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(allowed.status(), StatusCode::OK);
    let decision: Decision = serde_json::from_value(read_body(allowed).await).unwrap();
    assert_eq!(decision.verdict, Verdict::Allow);
    assert!(!decision
        .triggered_policies
        .iter()
        .any(|policy| policy.id == "refund-guarantee"));
}
