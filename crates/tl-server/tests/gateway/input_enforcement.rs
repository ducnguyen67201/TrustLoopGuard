#[tokio::test]
async fn workspace_runtime_key_cannot_create_gateway_configuration() {
    let provider = MockServer::start().await;
    let app = build_app();
    let workspace = "ws_gateway_runtime_key_admin";
    let runtime_key = create_workspace_key(app.clone(), workspace).await;

    let resp = app
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
}

#[tokio::test]
async fn gateway_blocks_input_before_reaching_provider() {
    let provider = MockServer::start().await;
    // No mock registered. Any call to the provider will fail the test via
    // verify().
    let app = build_app();
    let workspace = "ws_gateway_input_block";

    let policy = r#"
id: block-unsafe-input
description: Block unsafe user messages
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
    let profile_resp = app
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
                "messages": [{ "role": "user", "content": "unsafe input" }]
            }),
        ))
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body = read_body(resp).await;
    assert_eq!(
        body["choices"][0]["message"]["content"],
        "Blocked by input guard."
    );
    provider.verify().await;
}

#[tokio::test]
async fn metadata_only_retention_still_enforces_input_policy() {
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
                "message": { "role": "assistant", "content": "provider response" },
                "finish_reason": "stop"
            }]
        })))
        .expect(0)
        .mount(&provider)
        .await;

    let app = build_app();
    let workspace = "ws_gateway_metadata_enforces";
    let policy = r#"
id: block-retained-input
description: Block unsafe input even when retention omits body
when:
  channels: [chat]
match:
  literal: unsafe retained input
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
    let profile_resp = app
        .clone()
        .oneshot(json_request(
            "POST",
            "/v1/enforcement-profiles",
            "sk-internal",
            workspace,
            json!({
                "id": "profile",
                "display_name": "Metadata only",
                "input_action": "block",
                "output_action": "allow",
                "fail_mode": "open",
                "retention_mode": "metadata_only",
                "fallback_message": "Blocked despite metadata-only retention.",
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
                "messages": [{ "role": "user", "content": "unsafe retained input" }]
            }),
        ))
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body = read_body(resp).await;
    assert_eq!(
        body["choices"][0]["message"]["content"],
        "Blocked despite metadata-only retention."
    );
    provider.verify().await;
}
