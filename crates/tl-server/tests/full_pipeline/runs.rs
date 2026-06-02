#[tokio::test]
async fn run_lifecycle_endpoints_group_execution_state() {
    let state = memory_app_state(Arc::new(Engine::empty()));
    let app = router(state, None, [0u8; 32]);

    let create_body = serde_json::json!({
        "agent_id": "acme-support-v3",
        "kind": "chat_session",
        "external_id": "chat-123",
        "metadata": { "tier": "enterprise" }
    });
    let created = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/runs")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(create_body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(created.status(), StatusCode::CREATED);
    let created_body = read_body(created).await;
    let run_id = created_body["id"].as_str().unwrap();
    assert_eq!(created_body["status"], "running");
    assert_eq!(created_body["external_id"], "chat-123");

    let encoded_create_body = serde_json::json!({
        "agent_id": "acme-support-v3",
        "kind": "chat_session",
        "external_id": "chat/encoded"
    });
    let encoded_created = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/runs")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(encoded_create_body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(encoded_created.status(), StatusCode::CREATED);

    let listed = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/v1/runs?external_id=chat-123")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(listed.status(), StatusCode::OK);
    let listed_body = read_body(listed).await;
    assert_eq!(listed_body["runs"].as_array().unwrap().len(), 1);

    let encoded_listed = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/v1/runs?external_id=chat%2Fencoded")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(encoded_listed.status(), StatusCode::OK);
    let encoded_listed_body = read_body(encoded_listed).await;
    assert_eq!(encoded_listed_body["runs"].as_array().unwrap().len(), 1);

    let event = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/v1/runs/{run_id}/events"))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    serde_json::json!({
                        "kind": "user_turn",
                        "label": "Turn 1",
                        "input_summary": "Customer asks for a refund"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(event.status(), StatusCode::CREATED);
    let event_body = read_body(event).await;
    assert_eq!(event_body["sequence"], 1);
    assert_eq!(event_body["kind"], "user_turn");

    let updated = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri(format!("/v1/runs/{run_id}"))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(r#"{"status":"completed"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(updated.status(), StatusCode::OK);
    let updated_body = read_body(updated).await;
    assert_eq!(updated_body["status"], "completed");
    assert!(updated_body["ended_at"].as_str().is_some());

    let detail = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/v1/runs/{run_id}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(detail.status(), StatusCode::OK);
    let detail_body = read_body(detail).await;
    assert_eq!(detail_body["run"]["id"], run_id);
    assert_eq!(detail_body["events"].as_array().unwrap().len(), 1);
    assert!(detail_body["traces"].as_array().unwrap().is_empty());
}

#[tokio::test]
async fn check_can_create_run_event_inline() {
    let state = memory_app_state(Arc::new(Engine::empty()));
    let app = router(state, None, [0u8; 32]);

    let created = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/runs")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    serde_json::json!({
                        "agent_id": "acme-support-v3",
                        "kind": "chat_session"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(created.status(), StatusCode::CREATED);
    let run_id = read_body(created).await["id"].as_str().unwrap().to_string();

    let checked = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/check")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    serde_json::json!({
                        "run_id": run_id,
                        "run_event": {
                            "kind": "assistant_turn",
                            "label": "Turn 1",
                            "input_summary": "Customer asks about a refund",
                            "output_summary": "Agent drafts refund answer"
                        },
                        "agent_id": "acme-support-v3",
                        "channel": "chat",
                        "input": "Can I get a refund?",
                        "proposed_output": "I can help check refund options."
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(checked.status(), StatusCode::OK);

    let detail = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/v1/runs/{run_id}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(detail.status(), StatusCode::OK);
    let detail_body = read_body(detail).await;
    let events = detail_body["events"].as_array().unwrap();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0]["kind"], "assistant_turn");
    assert_eq!(events[0]["label"], "Turn 1");
}

#[tokio::test]
async fn check_rejects_malformed_run_id_before_engine_execution() {
    let state = memory_app_state(Arc::new(Engine::empty()));
    let app = router(state, None, [0u8; 32]);

    let body = serde_json::json!({
        "run_id": "not-a-uuid",
        "agent_id": "anon",
        "channel": "chat",
        "input": "hi",
        "proposed_output": "hello"
    });
    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/check")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    let body = read_body(resp).await;
    assert_eq!(body["code"], "invalid");
}

#[tokio::test]
async fn check_rejects_run_event_id_without_run_id() {
    let state = memory_app_state(Arc::new(Engine::empty()));
    let app = router(state, None, [0u8; 32]);

    let body = serde_json::json!({
        "run_event_id": "018f2222-2222-7222-8222-222222222222",
        "agent_id": "anon",
        "channel": "chat",
        "input": "hi",
        "proposed_output": "hello"
    });
    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/check")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    let body = read_body(resp).await;
    assert_eq!(
        body["message"],
        "run_id is required when run_event_id is provided"
    );
}
