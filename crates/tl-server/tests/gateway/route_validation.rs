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
async fn gateway_hard_deletes_unreferenced_provider_connection() {
    let app = build_app().await;
    let workspace = "ws_gateway_delete_provider";

    let create = app
        .clone()
        .oneshot(json_request(
            "POST",
            "/v1/gateway/provider-connections",
            "sk-internal",
            workspace,
            json!({
                "id": "provider-to-delete",
                "display_name": "Temporary provider",
                "kind": "openai_compatible",
                "default_model": "mock-model",
                "provider_api_key": "provider-secret"
            }),
        ))
        .await
        .unwrap();
    assert_eq!(create.status(), StatusCode::CREATED);

    let deleted = app
        .clone()
        .oneshot(json_request(
            "DELETE",
            "/v1/gateway/provider-connections/provider-to-delete",
            "sk-internal",
            workspace,
            json!({}),
        ))
        .await
        .unwrap();
    assert_eq!(deleted.status(), StatusCode::NO_CONTENT);

    let listed = app
        .clone()
        .oneshot(json_request(
            "GET",
            "/v1/gateway/provider-connections",
            "sk-internal",
            workspace,
            json!({}),
        ))
        .await
        .unwrap();
    assert_eq!(listed.status(), StatusCode::OK);
    assert_eq!(read_body(listed).await["provider_connections"], json!([]));

    let recreated = app
        .oneshot(json_request(
            "POST",
            "/v1/gateway/provider-connections",
            "sk-internal",
            workspace,
            json!({
                "id": "provider-to-delete",
                "display_name": "Replacement provider",
                "kind": "openai_compatible",
                "default_model": "replacement-model",
                "provider_api_key": "replacement-secret"
            }),
        ))
        .await
        .unwrap();
    assert_eq!(recreated.status(), StatusCode::CREATED);
}

#[tokio::test]
async fn gateway_refuses_to_delete_provider_connection_used_by_a_route() {
    let app = build_app().await;
    let workspace = "ws_gateway_delete_used_provider";
    create_common_gateway_config(
        app.clone(),
        workspace,
        "https://provider.example.com",
        "openai_compatible",
    )
    .await;

    let deleted = app
        .oneshot(json_request(
            "DELETE",
            "/v1/gateway/provider-connections/provider",
            "sk-internal",
            workspace,
            json!({}),
        ))
        .await
        .unwrap();

    assert_eq!(deleted.status(), StatusCode::CONFLICT);
    assert_eq!(
        read_body(deleted).await["message"],
        "provider connection is used by a gateway route"
    );
}

#[tokio::test]
async fn gateway_patch_non_existent_resource_returns_404() {
    let app = build_app().await;
    for (path, body) in [
        (
            "/v1/gateway/provider-connections/no-such",
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

    let resp = app
        .oneshot(json_request(
            "GET",
            "/v1/enforcement-profiles",
            "sk-internal",
            "ws_validation",
            json!({}),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}
