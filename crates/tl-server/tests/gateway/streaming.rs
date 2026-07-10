#[tokio::test]
async fn openai_gateway_streams_guarded_response_as_sse() {
    let provider = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .and(wire_header("authorization", "Bearer provider-secret"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "chatcmpl_mock",
            "object": "chat.completion",
            "created": 1,
            "model": "mock-model",
            "choices": [{
                "index": 0,
                "message": { "role": "assistant", "content": "clean reply" },
                "finish_reason": "stop"
            }]
        })))
        .mount(&provider)
        .await;

    let app = build_app().await;
    let workspace = "ws_gateway_openai_stream";
    let runtime_key = create_workspace_key(app.clone(), workspace).await;
    create_common_gateway_config(app.clone(), workspace, &provider.uri(), "openai_compatible")
        .await;

    let resp = app
        .clone()
        .oneshot(json_request(
            "POST",
            "/v1/gateway/route/openai/chat/completions",
            &runtime_key,
            workspace,
            json!({
                "model": "mock-model",
                "stream": true,
                "messages": [{ "role": "user", "content": "hello" }]
            }),
        ))
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    assert_eq!(
        resp.headers().get(header::CONTENT_TYPE).unwrap(),
        "text/event-stream"
    );
    let body = read_text(resp).await;
    assert!(body.contains("chat.completion.chunk"));
    assert!(body.contains("clean reply"));
    assert!(body.trim_end().ends_with("data: [DONE]"));

    // The upstream provider must have been called WITHOUT the streaming flag,
    // since we buffer-then-guard before emitting SSE.
    let reqs = provider.received_requests().await.unwrap();
    assert_eq!(reqs.len(), 1);
    let forwarded: Value = serde_json::from_slice(&reqs[0].body).unwrap();
    assert!(forwarded.get("stream").is_none());

    provider.verify().await;
}

#[tokio::test]
async fn openai_gateway_streams_blocked_output_as_sse() {
    let provider = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .and(wire_header("authorization", "Bearer provider-secret"))
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
        .mount(&provider)
        .await;

    let app = build_app().await;
    let workspace = "ws_gateway_openai_stream_block";
    upsert_block_policy(app.clone(), workspace).await;
    let runtime_key = create_workspace_key(app.clone(), workspace).await;
    create_common_gateway_config(app.clone(), workspace, &provider.uri(), "openai_compatible")
        .await;

    let resp = app
        .oneshot(json_request(
            "POST",
            "/v1/gateway/route/openai/chat/completions",
            &runtime_key,
            workspace,
            json!({
                "model": "mock-model",
                "stream": true,
                "messages": [{ "role": "user", "content": "hello" }]
            }),
        ))
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    assert_eq!(
        resp.headers().get(header::CONTENT_TYPE).unwrap(),
        "text/event-stream"
    );
    assert_eq!(
        resp.headers().get("x-trustloopguard-verdict").unwrap(),
        "blocked"
    );
    let body = read_text(resp).await;
    assert!(body.contains("content_filter"));
    assert!(body.contains("Blocked by TrustLoopGuard."));
    assert!(body.trim_end().ends_with("data: [DONE]"));

    provider.verify().await;
}

#[tokio::test]
async fn anthropic_gateway_streams_guarded_response_as_sse() {
    let provider = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .and(wire_header("x-api-key", "provider-secret"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "msg_mock",
            "type": "message",
            "role": "assistant",
            "model": "mock-model",
            "content": [{ "type": "text", "text": "scheduling hours are 9 to 5" }],
            "stop_reason": "end_turn",
            "stop_sequence": null,
            "usage": { "input_tokens": 1, "output_tokens": 1 }
        })))
        .mount(&provider)
        .await;

    let app = build_app().await;
    let workspace = "ws_gateway_anthropic_stream";
    let runtime_key = create_workspace_key(app.clone(), workspace).await;
    create_common_gateway_config(app.clone(), workspace, &provider.uri(), "anthropic").await;

    let resp = app
        .clone()
        .oneshot(json_request(
            "POST",
            "/v1/gateway/route/anthropic/v1/messages",
            &runtime_key,
            workspace,
            json!({
                "model": "claude-3-5-sonnet-latest",
                "stream": true,
                "messages": [{ "role": "user", "content": "hello" }]
            }),
        ))
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    assert_eq!(
        resp.headers().get(header::CONTENT_TYPE).unwrap(),
        "text/event-stream"
    );
    let body = read_text(resp).await;
    assert!(body.contains("event: message_start"));
    assert!(body.contains("event: content_block_delta"));
    assert!(body.contains("scheduling hours are 9 to 5"));
    assert!(body.contains("event: message_stop"));

    let reqs = provider.received_requests().await.unwrap();
    assert_eq!(reqs.len(), 1);
    let forwarded: Value = serde_json::from_slice(&reqs[0].body).unwrap();
    assert!(forwarded.get("stream").is_none());

    provider.verify().await;
}
