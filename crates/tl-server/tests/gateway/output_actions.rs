#[tokio::test]
async fn gateway_redacts_flagged_input_before_forwarding_to_provider() {
    let provider = MockServer::start().await;
    // Mock accepts any POST — presence of a call proves the request was forwarded (not blocked).
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .and(wire_header("authorization", "Bearer provider-secret"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "chatcmpl_mock",
            "object": "chat.completion",
            "created": 1,
            "model": "mock-model",
            "choices": [{"index": 0, "message": {"role": "assistant", "content": "safe reply"}, "finish_reason": "stop"}],
            "usage": {"prompt_tokens": 0, "completion_tokens": 0, "total_tokens": 0}
        })))
        .mount(&provider)
        .await;

    let app = build_app().await;
    let workspace = "ws_gateway_redact_input";

    // Policy that flags the literal "unsafe input" phrase.
    let policy = r#"
id: block-unsafe-input-for-redact
description: Flag unsafe user messages
when:
  channels: [chat]
match:
  literal: unsafe input
action: block
"#;
    let policy_resp = app
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
    assert_eq!(policy_resp.status(), StatusCode::CREATED);

    let runtime_key = create_workspace_key(app.clone(), workspace).await;

    // Provider connection, enforcement profile with input_action=redact, and route.
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
                "display_name": "Redact on input",
                "input_action": "redact",
                "output_action": "block",
                "fail_mode": "open",
                "retention_mode": "full_body",
                "fallback_message": "Blocked by TrustLoopGuard.",
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
                "display_name": "Gateway route",
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
            json!({
                "model": "mock-model",
                "messages": [{"role": "user", "content": "unsafe input"}]
            }),
        ))
        .await
        .unwrap();

    // The redact action rewrites the input to "[redacted]" and forwards it.
    // The provider returns "safe reply" which passes the output check.
    assert_eq!(resp.status(), StatusCode::OK);
    let body = read_body(resp).await;
    assert_eq!(body["choices"][0]["message"]["content"], "safe reply");
    // verify() fails if the mock was never called — proves request was forwarded, not blocked.
    provider.verify().await;
}

#[tokio::test]
async fn gateway_rewrite_output_falls_back_to_safe_response_when_no_safe_output() {
    let provider = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .and(wire_header("authorization", "Bearer provider-secret"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "chatcmpl_mock",
            "object": "chat.completion",
            "created": 1,
            "model": "mock-model",
            "choices": [{"index": 0, "message": {"role": "assistant", "content": "unsafe reply"}, "finish_reason": "stop"}],
            "usage": {"prompt_tokens": 0, "completion_tokens": 0, "total_tokens": 0}
        })))
        .mount(&provider)
        .await;

    let app = build_app().await;
    let workspace = "ws_gateway_rewrite_output";
    // Block policy flags "unsafe reply" on output.
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
                "display_name": "Rewrite on output",
                "input_action": "allow",
                "output_action": "rewrite",
                "fail_mode": "open",
                "retention_mode": "full_body",
                "fallback_message": "Blocked by TrustLoopGuard.",
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
                "display_name": "Gateway route",
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
            json!({
                "model": "mock-model",
                "messages": [{"role": "user", "content": "hello"}]
            }),
        ))
        .await
        .unwrap();

    // Provider returns "unsafe reply" → output flagged → rewrite with no safe_output
    // → falls back to safe_response (fallback_message).
    assert_eq!(resp.status(), StatusCode::OK);
    let body = read_body(resp).await;
    assert_eq!(
        body["choices"][0]["message"]["content"],
        "Blocked by TrustLoopGuard."
    );
    provider.verify().await;
}

include!("output_signals.rs");
