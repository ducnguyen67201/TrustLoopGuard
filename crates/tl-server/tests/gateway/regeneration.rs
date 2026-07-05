// ── Max regenerations tests ──────────────────────────────────────────────────

#[tokio::test]
async fn max_regenerations_self_heals_on_second_attempt() {
    let provider = MockServer::start().await;

    // Wiremock uses first-registered = highest priority.
    // Register the unsafe-reply mock first so it handles the initial request.
    // It responds at most once, then the safe-reply mock below takes over for the retry.
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "chatcmpl_1",
            "object": "chat.completion",
            "created": 1,
            "model": "mock-model",
            "choices": [{ "index": 0, "message": { "role": "assistant", "content": "unsafe reply" }, "finish_reason": "stop" }]
        })))
        .up_to_n_times(1)
        .expect(1)
        .mount(&provider)
        .await;

    // Register the safe-reply mock second (lower priority). It handles the regeneration
    // request once the unsafe mock is exhausted.
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "chatcmpl_2",
            "object": "chat.completion",
            "created": 2,
            "model": "mock-model",
            "choices": [{ "index": 0, "message": { "role": "assistant", "content": "safe reply" }, "finish_reason": "stop" }]
        })))
        .expect(1)
        .mount(&provider)
        .await;

    let app = build_app().await;
    let workspace = "ws_regen_self_heal";
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
                "display_name": "Rewrite with regen",
                "input_action": "allow",
                "output_action": "rewrite",
                "fail_mode": "open",
                "retention_mode": "full_body",
                "fallback_message": "Blocked by TrustLoopGuard.",
                "max_regenerations": 2
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
            json!({ "model": "mock-model", "messages": [{ "role": "user", "content": "hello" }] }),
        ))
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    // No enforcement headers — the clean response passes through unmodified.
    assert!(resp.headers().get("x-trustloopguard-verdict").is_none());
    let body = read_body(resp).await;
    assert_eq!(body["choices"][0]["message"]["content"], "safe reply");
    assert_eq!(body["choices"][0]["finish_reason"], "stop"); // real finish_reason from provider
    provider.verify().await; // exactly 2 provider calls
}

#[tokio::test]
async fn max_regenerations_exhausted_falls_back_to_safe_response() {
    let provider = MockServer::start().await;

    // Both the original call and the retry return "unsafe reply" (always blocked).
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "chatcmpl_mock",
            "object": "chat.completion",
            "created": 1,
            "model": "mock-model",
            "choices": [{ "index": 0, "message": { "role": "assistant", "content": "unsafe reply" }, "finish_reason": "stop" }]
        })))
        .expect(2) // original + 1 regeneration attempt
        .mount(&provider)
        .await;

    let app = build_app().await;
    let workspace = "ws_regen_exhausted";
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
                "display_name": "Rewrite with regen",
                "input_action": "allow",
                "output_action": "rewrite",
                "fail_mode": "open",
                "retention_mode": "full_body",
                "fallback_message": "Blocked by TrustLoopGuard.",
                "max_regenerations": 1
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

    assert_eq!(
        body["choices"][0]["message"]["content"],
        "Blocked by TrustLoopGuard."
    );
    assert_eq!(body["choices"][0]["finish_reason"], "content_filter");
    assert_eq!(verdict.as_deref(), Some("blocked"));
    assert_eq!(phase.as_deref(), Some("output"));
    provider.verify().await; // exactly 2 provider calls
}
