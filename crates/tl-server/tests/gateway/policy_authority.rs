async fn upsert_gateway_policy(app: axum::Router, workspace: &str, policy: &str) {
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/policies")
                .header(header::CONTENT_TYPE, "application/x-yaml")
                .header(header::AUTHORIZATION, "Bearer sk-internal")
                .header("x-tlg-workspace-id", workspace)
                .header("x-tlg-user-id", gateway_owner_id().to_string())
                .body(Body::from(policy.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);
}

#[tokio::test]
async fn gateway_applies_policy_input_rewrite_without_a_rule_set() {
    let provider = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "chatcmpl_mock",
            "object": "chat.completion",
            "created": 1,
            "model": "mock-model",
            "choices": [{
                "index": 0,
                "message": { "role": "assistant", "content": "safe reply" },
                "finish_reason": "stop"
            }]
        })))
        .expect(1)
        .mount(&provider)
        .await;

    let app = build_app().await;
    let workspace = "ws_gateway_policy_rewrite_input";
    upsert_gateway_policy(
        app.clone(),
        workspace,
        r#"
id: rewrite-unsafe-input
description: Rewrite unsafe input
when:
  channels: [chat]
match:
  literal: unsafe input
action: transform
rewrite: safe input
"#,
    )
    .await;
    let runtime_key = create_workspace_key(app.clone(), workspace).await;
    create_common_gateway_config(app.clone(), workspace, &provider.uri(), "openai_compatible")
        .await;

    let response = app
        .oneshot(json_request(
            "POST",
            "/v1/gateway/route/openai/chat/completions",
            &runtime_key,
            workspace,
            json!({
                "model": "mock-model",
                "messages": [{ "role": "user", "content": "unsafe input" }]
            }),
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let requests = provider.received_requests().await.unwrap();
    let forwarded: Value = serde_json::from_slice(&requests[0].body).unwrap();
    assert_eq!(forwarded["messages"][0]["content"], "safe input");
}

#[tokio::test]
async fn gateway_applies_policy_output_rewrite_without_regeneration() {
    let provider = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "chatcmpl_mock",
            "object": "chat.completion",
            "created": 1,
            "model": "mock-model",
            "choices": [{
                "index": 0,
                "message": { "role": "assistant", "content": "unsafe reply" },
                "finish_reason": "stop"
            }]
        })))
        .expect(1)
        .mount(&provider)
        .await;

    let app = build_app().await;
    let workspace = "ws_gateway_policy_rewrite_output";
    upsert_gateway_policy(
        app.clone(),
        workspace,
        r#"
id: rewrite-unsafe-output
description: Rewrite unsafe output
when:
  channels: [chat]
match:
  literal: unsafe reply
action: transform
rewrite: safe replacement
"#,
    )
    .await;
    let runtime_key = create_workspace_key(app.clone(), workspace).await;
    create_common_gateway_config(app.clone(), workspace, &provider.uri(), "openai_compatible")
        .await;

    let response = app
        .oneshot(json_request(
            "POST",
            "/v1/gateway/route/openai/chat/completions",
            &runtime_key,
            workspace,
            json!({
                "model": "mock-model",
                "messages": [{ "role": "user", "content": "hello" }]
            }),
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.headers().get("x-trustloopguard-effect").unwrap(),
        "transform"
    );
    let body = read_body(response).await;
    assert_eq!(body["choices"][0]["message"]["content"], "safe replacement");
    provider.verify().await;
}

#[tokio::test]
async fn gateway_returns_bad_gateway_for_provider_failure() {
    let provider = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(ResponseTemplate::new(503).set_body_string("provider unavailable"))
        .expect(1)
        .mount(&provider)
        .await;

    let app = build_app().await;
    let workspace = "ws_gateway_provider_failure";
    let runtime_key = create_workspace_key(app.clone(), workspace).await;
    create_common_gateway_config(app.clone(), workspace, &provider.uri(), "openai_compatible")
        .await;

    let response = app
        .oneshot(json_request(
            "POST",
            "/v1/gateway/route/openai/chat/completions",
            &runtime_key,
            workspace,
            json!({
                "model": "mock-model",
                "messages": [{ "role": "user", "content": "hello" }]
            }),
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_GATEWAY);
    assert!(response
        .headers()
        .get("x-trustloopguard-effect")
        .is_none());
    let body = read_body(response).await;
    assert_eq!(body["message"], "upstream provider request failed");
    provider.verify().await;
}

#[tokio::test]
async fn gateway_defers_before_provider_call() {
    let provider = MockServer::start().await;
    let app = build_app().await;
    let workspace = "ws_gateway_policy_escalate_input";
    upsert_gateway_policy(
        app.clone(),
        workspace,
        r#"
id: escalate-sensitive-input
description: Escalate sensitive input
when:
  channels: [chat]
match:
  literal: sensitive input
action: defer
"#,
    )
    .await;
    let runtime_key = create_workspace_key(app.clone(), workspace).await;
    create_common_gateway_config(app.clone(), workspace, &provider.uri(), "openai_compatible")
        .await;

    let response = app
        .oneshot(json_request(
            "POST",
            "/v1/gateway/route/openai/chat/completions",
            &runtime_key,
            workspace,
            json!({
                "model": "mock-model",
                "messages": [{ "role": "user", "content": "sensitive input" }]
            }),
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.headers().get("x-trustloopguard-effect").unwrap(),
        "defer"
    );
    let body = read_body(response).await;
    assert_eq!(
        body["choices"][0]["message"]["content"],
        "Blocked by TrustLoopGuard."
    );
    provider.verify().await;
}

#[tokio::test]
async fn gateway_and_events_share_the_same_policy_decision() {
    let provider = MockServer::start().await;
    let app = build_app().await;
    let workspace = "ws_gateway_event_parity";
    upsert_gateway_policy(
        app.clone(),
        workspace,
        r#"
id: parity-block
description: Block parity input
when:
  channels: [chat]
match:
  literal: parity input
action: deny
"#,
    )
    .await;
    let runtime_key = create_workspace_key(app.clone(), workspace).await;
    create_common_gateway_config(app.clone(), workspace, &provider.uri(), "openai_compatible")
        .await;

    let direct = app
        .clone()
        .oneshot(json_request(
            "POST",
            "/v1/events",
            &runtime_key,
            workspace,
            json!({
                "kind": "output.proposed",
                "principal": {
                    "workspace_id": workspace,
                    "environment_id": "production",
                    "agent_id": "agent"
                },
                "action": {
                    "operation": "output",
                    "parameters": { "text": "user: parity input" },
                    "side_effect": "none"
                },
                "sources": [
                    { "id": "input.observed", "origin": "unknown", "labels": {} },
                    { "id": "model.output", "origin": "unknown", "labels": {} }
                ],
                "provenance": { "text": ["input.observed"] },
                "context": { "channel": "chat", "domain": "gateway_input_check" }
            }),
        ))
        .await
        .unwrap();
    assert_eq!(direct.status(), StatusCode::OK);
    let direct_body = read_body(direct).await;
    assert_eq!(direct_body["effect"], "deny");
    assert_eq!(direct_body["findings"][0]["policy_id"], "parity-block");

    let gateway = app
        .oneshot(json_request(
            "POST",
            "/v1/gateway/route/openai/chat/completions",
            &runtime_key,
            workspace,
            json!({
                "model": "mock-model",
                "messages": [{ "role": "user", "content": "parity input" }]
            }),
        ))
        .await
        .unwrap();
    assert_eq!(gateway.status(), StatusCode::OK);
    assert_eq!(
        gateway.headers().get("x-trustloopguard-effect").unwrap(),
        "deny"
    );
    assert_eq!(
        gateway.headers().get("x-trustloopguard-policy-id").unwrap(),
        "parity-block"
    );
    provider.verify().await;
}
