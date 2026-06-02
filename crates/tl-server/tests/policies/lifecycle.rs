#[tokio::test]
async fn disable_policy_updates_document_but_get_still_works() {
    let app = build_app();
    let resp = request(
        app.clone(),
        Method::POST,
        "/v1/policies",
        Body::from(SAMPLE_POLICY_YAML),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::CREATED);

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri("/v1/policies/refund-guarantee/enabled")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(r#"{"enabled":false}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = read_body(resp).await;
    assert_eq!(body["enabled"], false);

    let resp = request(
        app,
        Method::GET,
        "/v1/policies/refund-guarantee",
        Body::empty(),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body = read_body(resp).await;
    assert_eq!(body["enabled"], false);
}

#[tokio::test]
async fn batch_disable_updates_multiple_policies() {
    let app = build_app();
    for id in ["refund-guarantee", "refund-promise"] {
        let yaml = SAMPLE_POLICY_YAML.replace("refund-guarantee", id);
        let resp = request(app.clone(), Method::POST, "/v1/policies", Body::from(yaml)).await;
        assert_eq!(resp.status(), StatusCode::CREATED);
    }

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri("/v1/policies/batch/enabled")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    r#"{"ids":["refund-guarantee","refund-promise"],"enabled":false}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = read_body(resp).await;
    assert_eq!(body["policies"].as_array().unwrap().len(), 2);
    assert!(body["policies"]
        .as_array()
        .unwrap()
        .iter()
        .all(|policy| policy["enabled"] == false));

    let resp = request(app, Method::GET, "/v1/policies", Body::empty()).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body = read_body(resp).await;
    assert!(body["policies"]
        .as_array()
        .unwrap()
        .iter()
        .all(|policy| policy["enabled"] == false));
}

#[tokio::test]
async fn batch_disable_missing_policy_does_not_partially_update() {
    let app = build_app();
    let resp = request(
        app.clone(),
        Method::POST,
        "/v1/policies",
        Body::from(SAMPLE_POLICY_YAML),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::CREATED);

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri("/v1/policies/batch/enabled")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    r#"{"ids":["refund-guarantee","missing"],"enabled":false}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);

    let resp = request(
        app,
        Method::GET,
        "/v1/policies/refund-guarantee",
        Body::empty(),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body = read_body(resp).await;
    assert_eq!(body["enabled"], true);
}

#[tokio::test]
async fn disable_policy_with_malformed_json_returns_api_error() {
    let app = build_app();
    let resp = request(
        app.clone(),
        Method::POST,
        "/v1/policies",
        Body::from(SAMPLE_POLICY_YAML),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::CREATED);

    let resp = app
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri("/v1/policies/refund-guarantee/enabled")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(r#"{"enabled":"no"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    let body = read_body(resp).await;
    assert_eq!(body["code"], "invalid");
    assert_eq!(body["retriable"], false);
}

#[tokio::test]
async fn delete_policy_makes_get_return_404() {
    let app = build_app();
    let resp = request(
        app.clone(),
        Method::POST,
        "/v1/policies",
        Body::from(SAMPLE_POLICY_YAML),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::CREATED);

    let resp = request(
        app.clone(),
        Method::DELETE,
        "/v1/policies/refund-guarantee",
        Body::empty(),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    let resp = request(
        app,
        Method::GET,
        "/v1/policies/refund-guarantee",
        Body::empty(),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}
