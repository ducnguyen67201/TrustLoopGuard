#[tokio::test]
async fn openai_blocked_output_has_content_filter_finish_reason_and_headers() {
    let provider = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "chatcmpl_mock",
            "object": "chat.completion",
            "created": 1,
            "model": "mock-model",
            "choices": [{ "index": 0, "message": { "role": "assistant", "content": "unsafe reply" }, "finish_reason": "stop" }]
        })))
        .mount(&provider)
        .await;

    let app = build_app();
    let workspace = "ws_sig_openai_output";
    upsert_block_policy(app.clone(), workspace).await;
    let runtime_key = create_workspace_key(app.clone(), workspace).await;
    create_common_gateway_config(app.clone(), workspace, &provider.uri(), "openai_compatible")
        .await;

    let resp = app
        .oneshot(json_request(
            "POST",
            "/v1/gateway/route/openai/chat/completions",
            &runtime_key,
            "ws_wrong",
            json!({ "model": "mock-model", "messages": [{ "role": "user", "content": "hello" }] }),
        ))
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let verdict = resp
        .headers()
        .get("x-trustloopguard-verdict")
        .and_then(|v| v.to_str().ok())
        .map(str::to_owned);
    let phase = resp
        .headers()
        .get("x-trustloopguard-phase")
        .and_then(|v| v.to_str().ok())
        .map(str::to_owned);
    let trace_id = resp
        .headers()
        .get("x-trustloopguard-trace-id")
        .and_then(|v| v.to_str().ok())
        .map(str::to_owned);
    let body = read_body(resp).await;

    assert_eq!(body["choices"][0]["finish_reason"], "content_filter");
    assert_eq!(verdict.as_deref(), Some("blocked"));
    assert_eq!(phase.as_deref(), Some("output"));
    assert!(trace_id.is_some());
}

#[tokio::test]
async fn anthropic_blocked_output_has_content_filter_stop_reason_and_headers() {
    let provider = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "msg_mock",
            "type": "message",
            "role": "assistant",
            "model": "mock-model",
            "content": [{ "type": "text", "text": "unsafe reply" }],
            "stop_reason": "end_turn",
            "stop_sequence": null,
            "usage": { "input_tokens": 1, "output_tokens": 1 }
        })))
        .mount(&provider)
        .await;

    let app = build_app();
    let workspace = "ws_sig_anthropic_output";
    upsert_block_policy(app.clone(), workspace).await;
    let runtime_key = create_workspace_key(app.clone(), workspace).await;
    create_common_gateway_config(app.clone(), workspace, &provider.uri(), "anthropic").await;

    let resp = app
        .oneshot(json_request(
            "POST",
            "/v1/gateway/route/anthropic/v1/messages",
            &runtime_key,
            "ws_wrong",
            json!({ "model": "mock-model", "messages": [{ "role": "user", "content": "hello" }] }),
        ))
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let verdict = resp
        .headers()
        .get("x-trustloopguard-verdict")
        .and_then(|v| v.to_str().ok())
        .map(str::to_owned);
    let phase = resp
        .headers()
        .get("x-trustloopguard-phase")
        .and_then(|v| v.to_str().ok())
        .map(str::to_owned);
    let body = read_body(resp).await;

    assert_eq!(body["stop_reason"], "content_filter");
    assert_eq!(verdict.as_deref(), Some("blocked"));
    assert_eq!(phase.as_deref(), Some("output"));
}

#[tokio::test]
async fn openai_blocked_input_returns_content_filter_with_input_phase_header() {
    let provider = MockServer::start().await;
    // No mock registered. Provider must never be called on an input block.

    let app = build_app();
    let workspace = "ws_sig_input_phase";

    let policy = r#"
id: block-bad-input
description: Block bad input
when:
  channels: [chat]
match:
  literal: forbidden input
action: block
"#;
    let _ = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/policies")
                .header(header::CONTENT_TYPE, "application/x-yaml")
                .header(header::AUTHORIZATION, "Bearer sk-internal")
                .header("x-tlg-workspace-id", workspace)
                .body(Body::from(policy))
                .unwrap(),
        )
        .await
        .unwrap();

    let runtime_key = create_workspace_key(app.clone(), workspace).await;
    let _ = app
        .clone()
        .oneshot(json_request(
            "POST",
            "/v1/enforcement-profiles",
            "sk-internal",
            workspace,
            json!({
                "id": "profile",
                "display_name": "Block on input",
                "input_action": "block",
                "output_action": "block",
                "fail_mode": "open",
                "retention_mode": "full_body",
                "fallback_message": "Blocked by input guard.",
                "max_regenerations": 0
            }),
        ))
        .await
        .unwrap();
    let _ = app
        .clone()
        .oneshot(json_request(
            "POST",
            "/v1/gateway/provider-connections",
            "sk-internal",
            workspace,
            json!({
                "id": "provider",
                "display_name": "Mock provider",
                "kind": "openai_compatible",
                "base_url": provider.uri(),
                "default_model": "mock-model",
                "provider_api_key": "provider-secret"
            }),
        ))
        .await
        .unwrap();
    let _ = app
        .clone()
        .oneshot(json_request(
            "POST",
            "/v1/gateway/routes",
            "sk-internal",
            workspace,
            json!({
                "id": "route",
                "display_name": "Route",
                "provider_connection_id": "provider",
                "agent_id": "agent",
                "enforcement_profile_id": "profile"
            }),
        ))
        .await
        .unwrap();

    let resp = app
        .oneshot(json_request(
            "POST",
            "/v1/gateway/route/openai/chat/completions",
            &runtime_key,
            workspace,
            json!({ "model": "mock-model", "messages": [{ "role": "user", "content": "forbidden input" }] }),
        ))
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let phase = resp
        .headers()
        .get("x-trustloopguard-phase")
        .and_then(|v| v.to_str().ok())
        .map(str::to_owned);
    let verdict = resp
        .headers()
        .get("x-trustloopguard-verdict")
        .and_then(|v| v.to_str().ok())
        .map(str::to_owned);
    let body = read_body(resp).await;

    assert_eq!(body["choices"][0]["finish_reason"], "content_filter");
    assert_eq!(phase.as_deref(), Some("input"));
    assert_eq!(verdict.as_deref(), Some("blocked"));
    provider.verify().await;
}

#[tokio::test]
async fn escalate_output_action_returns_escalated_verdict_header() {
    let provider = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "chatcmpl_mock",
            "object": "chat.completion",
            "created": 1,
            "model": "mock-model",
            "choices": [{ "index": 0, "message": { "role": "assistant", "content": "unsafe reply" }, "finish_reason": "stop" }]
        })))
        .mount(&provider)
        .await;

    let app = build_app();
    let workspace = "ws_escalate_header";
    upsert_block_policy(app.clone(), workspace).await;
    let runtime_key = create_workspace_key(app.clone(), workspace).await;

    let _ = app
        .clone()
        .oneshot(json_request(
            "POST",
            "/v1/gateway/provider-connections",
            "sk-internal",
            workspace,
            json!({
                "id": "provider",
                "display_name": "Mock provider",
                "kind": "openai_compatible",
                "base_url": provider.uri(),
                "default_model": "mock-model",
                "provider_api_key": "provider-secret"
            }),
        ))
        .await
        .unwrap();
    let _ = app
        .clone()
        .oneshot(json_request(
            "POST",
            "/v1/enforcement-profiles",
            "sk-internal",
            workspace,
            json!({
                "id": "profile",
                "display_name": "Escalate on output",
                "input_action": "allow",
                "output_action": "escalate",
                "fail_mode": "open",
                "retention_mode": "full_body",
                "fallback_message": "Under review.",
                "max_regenerations": 0
            }),
        ))
        .await
        .unwrap();
    let _ = app
        .clone()
        .oneshot(json_request(
            "POST",
            "/v1/gateway/routes",
            "sk-internal",
            workspace,
            json!({
                "id": "route",
                "display_name": "Route",
                "provider_connection_id": "provider",
                "agent_id": "agent",
                "enforcement_profile_id": "profile"
            }),
        ))
        .await
        .unwrap();

    let resp = app
        .oneshot(json_request(
            "POST",
            "/v1/gateway/route/openai/chat/completions",
            &runtime_key,
            "ws_wrong",
            json!({ "model": "mock-model", "messages": [{ "role": "user", "content": "hello" }] }),
        ))
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let verdict = resp
        .headers()
        .get("x-trustloopguard-verdict")
        .and_then(|v| v.to_str().ok())
        .map(str::to_owned);
    let body = read_body(resp).await;

    assert_eq!(body["choices"][0]["finish_reason"], "content_filter");
    assert_eq!(verdict.as_deref(), Some("escalated"));
}
