#[tokio::test]
async fn oauth_session_requires_internal_bearer() {
    let app = build_app(Some(AuthConfig::new("sk-internal")));

    let resp = app
        .oneshot(oauth_session_request(
            None,
            "google",
            "google-subject",
            "user@example.com",
        ))
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn oauth_session_rejects_user_session_jwt() {
    let (app, signer) = build_app_with_jwt();
    let jwt = signer
        .mint(Uuid::new_v4(), "attacker@example.com")
        .expect("mint attacker jwt");

    let resp = app
        .oneshot(oauth_session_request(
            Some(&jwt),
            "google",
            "google-subject",
            "victim@example.com",
        ))
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn oauth_session_links_google_and_github_to_same_local_user_by_email() {
    let app = build_app(Some(AuthConfig::new("sk-internal")));

    let google_resp = app
        .clone()
        .oneshot(oauth_session_request(
            Some("sk-internal"),
            "google",
            "google-subject",
            "user@example.com",
        ))
        .await
        .unwrap();
    assert_eq!(google_resp.status(), StatusCode::OK);
    let google = read_body(google_resp).await;

    let github_resp = app
        .oneshot(oauth_session_request(
            Some("sk-internal"),
            "github",
            "github-subject",
            "user@example.com",
        ))
        .await
        .unwrap();
    assert_eq!(github_resp.status(), StatusCode::OK);
    let github = read_body(github_resp).await;

    assert_eq!(google["user_id"], github["user_id"]);
    assert_eq!(github["username"], "user@example.com");
}

#[tokio::test]
async fn oauth_session_rejects_workspace_runtime_api_key() {
    let (app, user_id) = build_app_with_approved_user(Some(AuthConfig::new("sk-internal"))).await;
    let workspace_id = create_workspace_for_user(app.clone(), user_id, "OAuth Workspace").await;

    let create_resp = app
        .clone()
        .oneshot(create_api_key_request_with_user(
            "sk-internal",
            &workspace_id,
            "Runtime key",
            user_id,
        ))
        .await
        .unwrap();
    assert_eq!(create_resp.status(), StatusCode::CREATED);
    let created = read_body(create_resp).await;
    let runtime_key = created["plaintext_key"].as_str().expect("runtime key");

    let resp = app
        .oneshot(oauth_session_request(
            Some(runtime_key),
            "google",
            "google-subject",
            "victim@example.com",
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn oauth_authorization_rejects_workspace_runtime_api_key_before_forwarded_identity() {
    let (app, user_id) = build_app_with_approved_user(Some(AuthConfig::new("sk-internal"))).await;
    let workspace_id =
        create_workspace_for_user(app.clone(), user_id, "OAuth Authorization Workspace").await;

    let create_resp = app
        .clone()
        .oneshot(create_api_key_request_with_user(
            "sk-internal",
            &workspace_id,
            "Runtime key",
            user_id,
        ))
        .await
        .unwrap();
    assert_eq!(create_resp.status(), StatusCode::CREATED);
    let created = read_body(create_resp).await;
    let runtime_key = created["plaintext_key"].as_str().expect("runtime key");

    let response = app
        .oneshot(oauth_authorize_request(
            runtime_key,
            &workspace_id,
            user_id,
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}
