#[tokio::test]
async fn workspace_runtime_key_cannot_manage_gateway_configuration() {
    let provider = MockServer::start().await;
    let app = build_app().await;
    let workspace = "ws_gateway_runtime_key_admin";
    let runtime_key = create_workspace_key(app.clone(), workspace).await;

    let resp = app
        .clone()
        .oneshot(json_request(
            "POST",
            "/v1/gateway/provider-connections",
            &runtime_key,
            "ws_wrong",
            json!({
                "id": "provider",
                "display_name": "Runtime key should not manage config",
                "kind": "openai_compatible",
                "base_url": provider.uri(),
                "default_model": "mock-model",
                "provider_api_key": "provider-secret"
            }),
        ))
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::FORBIDDEN);

    let created = app
        .clone()
        .oneshot(json_request(
            "POST",
            "/v1/gateway/provider-connections",
            "sk-internal",
            workspace,
            json!({
                "id": "provider",
                "display_name": "Admin-created provider",
                "kind": "openai_compatible",
                "base_url": provider.uri(),
                "default_model": "mock-model",
                "provider_api_key": "provider-secret"
            }),
        ))
        .await
        .unwrap();
    assert_eq!(created.status(), StatusCode::CREATED);

    let deleted = app
        .oneshot(json_request(
            "DELETE",
            "/v1/gateway/provider-connections/provider",
            &runtime_key,
            "ws_wrong",
            json!({}),
        ))
        .await
        .unwrap();
    assert_eq!(deleted.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn gateway_blocks_input_before_reaching_provider() {
    let provider = MockServer::start().await;
    // No mock registered. Any call to the provider will fail the test via
    // verify().
    let app = build_app().await;
    let workspace = "ws_gateway_input_block";

    let policy = r#"
id: block-unsafe-input
description: Block unsafe user messages
when:
  channels: [chat]
match:
  literal: unsafe input
action: deny
"#;
    let policy_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/policies")
                .header(header::CONTENT_TYPE, "application/x-yaml")
                .header(header::AUTHORIZATION, "Bearer sk-internal")
                .header("x-featherlane-ai-workspace-id", workspace)
                .header("x-featherlane-ai-user-id", gateway_owner_id().to_string())
                .body(Body::from(policy))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(policy_resp.status(), StatusCode::CREATED);

    let runtime_key = create_workspace_key(app.clone(), workspace).await;
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
                "agent_id": "agent"
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
                "messages": [{ "role": "user", "content": "unsafe input" }]
            }),
        ))
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body = read_body(resp).await;
    assert_eq!(
        body["choices"][0]["message"]["content"],
        "Blocked by Featherlane AI."
    );
    provider.verify().await;
}
