#[tokio::test]
async fn gateway_fail_mode_open_returns_502_on_provider_error() {
    let provider = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(ResponseTemplate::new(500).set_body_string("internal error"))
        .mount(&provider)
        .await;

    let app = build_app();
    let workspace = "ws_gateway_fail_open";
    let runtime_key = create_workspace_key(app.clone(), workspace).await;

    let profile_resp = app
        .clone()
        .oneshot(json_request(
            "POST",
            "/v1/enforcement-profiles",
            "sk-internal",
            workspace,
            json!({
                "id": "profile",
                "display_name": "Open fail mode",
                "input_action": "allow",
                "output_action": "allow",
                "fail_mode": "open",
                "retention_mode": "full_body",
                "fallback_message": "Fallback.",
                "max_regenerations": 0
            }),
        ))
        .await
        .unwrap();
    assert_eq!(profile_resp.status(), StatusCode::CREATED);

    let provider_resp = app
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
    assert_eq!(provider_resp.status(), StatusCode::CREATED);

    let route_resp = app
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
    assert_eq!(route_resp.status(), StatusCode::CREATED);

    let resp = app
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

    assert_eq!(resp.status(), StatusCode::BAD_GATEWAY);
}

#[tokio::test]
async fn gateway_fail_mode_closed_returns_safe_response_on_provider_error() {
    let provider = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(ResponseTemplate::new(503).set_body_string("service unavailable"))
        .mount(&provider)
        .await;

    let app = build_app();
    let workspace = "ws_gateway_fail_closed";
    let runtime_key = create_workspace_key(app.clone(), workspace).await;

    let profile_resp = app
        .clone()
        .oneshot(json_request(
            "POST",
            "/v1/enforcement-profiles",
            "sk-internal",
            workspace,
            json!({
                "id": "profile",
                "display_name": "Closed fail mode",
                "input_action": "allow",
                "output_action": "allow",
                "fail_mode": "closed",
                "retention_mode": "full_body",
                "fallback_message": "Service temporarily unavailable.",
                "max_regenerations": 0
            }),
        ))
        .await
        .unwrap();
    assert_eq!(profile_resp.status(), StatusCode::CREATED);

    let provider_resp = app
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
    assert_eq!(provider_resp.status(), StatusCode::CREATED);

    let route_resp = app
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
    assert_eq!(route_resp.status(), StatusCode::CREATED);

    let resp = app
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

    assert_eq!(resp.status(), StatusCode::OK);
    let body = read_body(resp).await;
    assert_eq!(
        body["choices"][0]["message"]["content"],
        "Service temporarily unavailable."
    );
}
