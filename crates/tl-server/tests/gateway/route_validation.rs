#[tokio::test]
async fn gateway_returns_404_for_unknown_route() {
    let app = build_app().await;
    let workspace = "ws_gateway_unknown_route";
    let runtime_key = create_workspace_key(app.clone(), workspace).await;

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

    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn gateway_returns_400_for_provider_kind_mismatch() {
    let provider = MockServer::start().await;
    let app = build_app().await;
    let workspace = "ws_gateway_kind_mismatch";
    let runtime_key = create_workspace_key(app.clone(), workspace).await;
    // Create an Anthropic-kind route but call the OpenAI endpoint.
    create_common_gateway_config(app.clone(), workspace, &provider.uri(), "anthropic").await;

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

    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    // Provider must not have been contacted.
    provider.verify().await;
}

#[tokio::test]
async fn gateway_patch_non_existent_resource_returns_404() {
    let app = build_app().await;
    for (path, body) in [
        (
            "/v1/gateway/provider-connections/no-such",
            json!({"display_name": "x"}),
        ),
        (
            "/v1/enforcement-profiles/no-such",
            json!({"display_name": "x"}),
        ),
        ("/v1/gateway/routes/no-such", json!({"display_name": "x"})),
    ] {
        let resp = app
            .clone()
            .oneshot(json_request(
                "PATCH",
                path,
                "sk-internal",
                "ws_404_test",
                body,
            ))
            .await
            .unwrap();
        assert_eq!(
            resp.status(),
            StatusCode::NOT_FOUND,
            "expected 404 for PATCH {path}"
        );
    }
}

#[tokio::test]
async fn gateway_create_rejects_empty_required_fields() {
    let app = build_app().await;

    // Empty display_name on provider connection.
    let resp = app
        .clone()
        .oneshot(json_request(
            "POST",
            "/v1/gateway/provider-connections",
            "sk-internal",
            "ws_validation",
            json!({
                "display_name": "",
                "kind": "openai_compatible",
                "default_model": "gpt-4",
                "provider_api_key": "sk-test"
            }),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

    // Empty provider_api_key.
    let resp = app
        .clone()
        .oneshot(json_request(
            "POST",
            "/v1/gateway/provider-connections",
            "sk-internal",
            "ws_validation",
            json!({
                "display_name": "Test",
                "kind": "openai_compatible",
                "default_model": "gpt-4",
                "provider_api_key": "  "
            }),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

    // Empty fallback_message on enforcement profile.
    let resp = app
        .clone()
        .oneshot(json_request(
            "POST",
            "/v1/enforcement-profiles",
            "sk-internal",
            "ws_validation",
            json!({
                "display_name": "Test",
                "input_action": "block",
                "output_action": "block",
                "fail_mode": "open",
                "retention_mode": "full_body",
                "fallback_message": "",
                "max_regenerations": 0
            }),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}
