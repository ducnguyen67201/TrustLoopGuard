#[tokio::test]
async fn malformed_yaml_yields_400() {
    let app = build_app();
    let resp = app
        .oneshot(
            workspace_request()
                .method("POST")
                .uri("/v1/agents")
                .header(header::CONTENT_TYPE, "application/yaml")
                .body(Body::from("not: valid: yaml: ["))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    let body: ApiError = serde_json::from_value(read_body(resp).await).unwrap();
    assert!(matches!(body.code, tl_core::ApiErrorCode::Invalid));
}

#[tokio::test]
async fn yaml_missing_required_fields_yields_400() {
    // Validation in tl_policy::load_agent_str catches this before we
    // hit our own validate_profile check.
    let app = build_app();
    let body = "agent_id: \"\"\ndisplay_name: x\nscope:\n  in_scope: [a]\nauthority: {}\ntone:\n  target: x\n";
    let resp = app
        .oneshot(
            workspace_request()
                .method("POST")
                .uri("/v1/agents")
                .header(header::CONTENT_TYPE, "application/yaml")
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn json_with_empty_agent_id_yields_422() {
    // JSON path skips tl_policy::load_agent_str so our own validate_profile
    // is what catches the empty agent_id — different status (422 vs 400).
    let app = build_app();
    let body = serde_json::json!({
        "agent_id": "",
        "display_name": "x",
        "scope": { "in_scope": ["a"], "out_of_scope": [] },
        "authority": { "can_promise": [], "cannot_promise": [] },
        "tone": { "target": "x", "forbidden": [] },
    });
    let resp = app
        .oneshot(
            workspace_request()
                .method("POST")
                .uri("/v1/agents")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
}
