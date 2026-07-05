#[tokio::test]
async fn internal_bearer_with_forwarded_user_can_issue_workspace_key_used_by_sdk_runtime() {
    let (app, user_id) = build_app_with_approved_user(Some(AuthConfig::new("sk-internal"))).await;
    let workspace_id = create_workspace_for_user(app.clone(), user_id, "Runtime Workspace").await;

    let create_resp = app
        .clone()
        .oneshot(create_api_key_request_with_user(
            "sk-internal",
            &workspace_id,
            "SDK integration",
            user_id,
        ))
        .await
        .unwrap();
    assert_eq!(create_resp.status(), StatusCode::CREATED);
    let created = read_body(create_resp).await;
    let plaintext = created["plaintext_key"]
        .as_str()
        .expect("plaintext key is returned once");
    assert!(plaintext.starts_with("tl_live_"));
    assert_eq!(created["api_key"]["name"], "SDK integration");
    assert_eq!(created["api_key"]["status"], "active");
    assert_eq!(created["api_key"]["last_used_at"], serde_json::Value::Null);
    assert!(created["api_key"]["prefix"]
        .as_str()
        .unwrap()
        .starts_with("tl_live_"));
    assert!(created.get("key_hash").is_none());

    let list_resp = app
        .clone()
        .oneshot(list_api_keys_request_with_user(
            "sk-internal",
            &workspace_id,
            user_id,
        ))
        .await
        .unwrap();
    assert_eq!(list_resp.status(), StatusCode::OK);
    let listed = read_body(list_resp).await;
    assert_eq!(listed["api_keys"].as_array().unwrap().len(), 1);
    assert_eq!(
        listed["api_keys"][0]["prefix"],
        created["api_key"]["prefix"]
    );
    assert!(!listed.to_string().contains(plaintext));

    let other_workspace_policy = r#"
id: wrong-workspace-block
description: Would block if caller-controlled workspace won
when:
  channels: [chat]
match:
  literal: deny me
action: block
"#;
    let upsert_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/policies")
                .header(header::CONTENT_TYPE, "application/x-yaml")
                .header(header::AUTHORIZATION, "Bearer sk-internal")
                .header("x-tlg-workspace-id", "ws_wrong")
                .body(Body::from(other_workspace_policy))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(upsert_resp.status(), StatusCode::CREATED);

    let event_body = serde_json::json!({
        "kind": "output.proposed",
        "principal": {
            "workspace_id": "ws_wrong",
            "environment_id": "production",
            "agent_id": "a"
        },
        "action": {
            "operation": "output",
            "parameters": { "text": "deny me" },
            "side_effect": "none"
        },
        "sources": [{ "id": "input", "origin": "user", "labels": {} }],
        "provenance": { "text": ["input"] },
        "context": { "channel": "chat", "domain": "customer_support" }
    });
    let event_resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/events")
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::AUTHORIZATION, format!("Bearer {plaintext}"))
                .header("x-tlg-workspace-id", "ws_wrong")
                .body(Body::from(event_body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(event_resp.status(), StatusCode::OK);
    let decision = read_body(event_resp).await;
    assert_eq!(decision["verdict"], serde_json::json!(Verdict::Allow));
}

#[tokio::test]
async fn local_dev_without_auth_config_can_manage_api_keys_with_forwarded_user() {
    let app = build_app(None);
    let user_id = Uuid::new_v4();
    let workspace_id = create_workspace_for_user(app.clone(), user_id, "Local Dev Workspace").await;

    let create_resp = app
        .clone()
        .oneshot(create_api_key_request_with_user(
            "unused-local-token",
            &workspace_id,
            "SDK integration",
            user_id,
        ))
        .await
        .unwrap();
    assert_eq!(create_resp.status(), StatusCode::CREATED);

    let list_resp = app
        .oneshot(list_api_keys_request_with_user(
            "unused-local-token",
            &workspace_id,
            user_id,
        ))
        .await
        .unwrap();
    assert_eq!(list_resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn local_dev_missing_forwarded_user_is_unauthorized() {
    let app = build_app(None);
    let user_id = Uuid::new_v4();
    let workspace_id = create_workspace_for_user(app.clone(), user_id, "Local Dev Workspace").await;

    let create_resp = app
        .clone()
        .oneshot(create_api_key_request(
            "unused-local-token",
            &workspace_id,
            "SDK integration",
        ))
        .await
        .unwrap();
    assert_eq!(create_resp.status(), StatusCode::UNAUTHORIZED);

    let list_resp = app
        .oneshot(list_api_keys_request("unused-local-token", &workspace_id))
        .await
        .unwrap();
    assert_eq!(list_resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn internal_bearer_can_revoke_workspace_keys() {
    let (app, user_id) = build_app_with_approved_user(Some(AuthConfig::new("sk-internal"))).await;
    let workspace_id = create_workspace_for_user(app.clone(), user_id, "Runtime Workspace").await;
    let create_resp = app
        .clone()
        .oneshot(create_api_key_request_with_user(
            "sk-internal",
            &workspace_id,
            "SDK integration",
            user_id,
        ))
        .await
        .unwrap();
    assert_eq!(create_resp.status(), StatusCode::CREATED);
    let created = read_body(create_resp).await;
    let key_id = created["api_key"]["id"].as_str().unwrap();
    let plaintext = created["plaintext_key"].as_str().unwrap();

    let revoke_resp = app
        .clone()
        .oneshot(revoke_api_keys_request_with_user(
            "sk-internal",
            &workspace_id,
            &[key_id],
            user_id,
        ))
        .await
        .unwrap();
    assert_eq!(revoke_resp.status(), StatusCode::OK);
    let revoked = read_body(revoke_resp).await;
    assert_eq!(revoked["api_keys"][0]["status"], "revoked");

    let event_body = serde_json::json!({
        "kind": "output.proposed",
        "principal": {
            "workspace_id": "default",
            "environment_id": "production",
            "agent_id": "a"
        },
        "action": {
            "operation": "output",
            "parameters": { "text": "hello" },
            "side_effect": "none"
        }
    });
    let event_resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/events")
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::AUTHORIZATION, format!("Bearer {plaintext}"))
                .body(Body::from(event_body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(event_resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn revoke_missing_workspace_key_returns_404() {
    let (app, user_id) = build_app_with_approved_user(Some(AuthConfig::new("sk-internal"))).await;
    let workspace_id = create_workspace_for_user(app.clone(), user_id, "Runtime Workspace").await;
    let resp = app
        .oneshot(revoke_api_keys_request_with_user(
            "sk-internal",
            &workspace_id,
            &["apk_missing"],
            user_id,
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn internal_bearer_without_forwarded_user_cannot_issue_workspace_key() {
    let app = build_app(Some(AuthConfig::new("sk-internal")));
    let resp = app
        .oneshot(create_api_key_request(
            "sk-internal",
            "ws_runtime",
            "SDK integration",
        ))
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn user_jwt_can_manage_keys_only_for_owned_workspace() {
    let (app, signer, owner_id, outsider_id) = build_app_with_jwt_and_approved_users().await;
    let workspace_id = create_workspace_for_user(app.clone(), owner_id, "JWT Workspace").await;
    let owner_token = signer.mint(owner_id, "owner@example.com").unwrap();

    let create_resp = app
        .clone()
        .oneshot(create_api_key_request(
            &owner_token,
            &workspace_id,
            "SDK integration",
        ))
        .await
        .unwrap();
    assert_eq!(create_resp.status(), StatusCode::CREATED);
    let created = read_body(create_resp).await;
    let key_id = created["api_key"]["id"].as_str().unwrap();

    let list_resp = app
        .clone()
        .oneshot(list_api_keys_request(&owner_token, &workspace_id))
        .await
        .unwrap();
    assert_eq!(list_resp.status(), StatusCode::OK);

    let revoke_resp = app
        .clone()
        .oneshot(revoke_api_keys_request(
            &owner_token,
            &workspace_id,
            &[key_id],
        ))
        .await
        .unwrap();
    assert_eq!(revoke_resp.status(), StatusCode::OK);

    let outsider_token = signer.mint(outsider_id, "outsider@example.com").unwrap();
    let denied_resp = app
        .oneshot(create_api_key_request(
            &outsider_token,
            &workspace_id,
            "stolen workspace",
        ))
        .await
        .unwrap();
    assert_eq!(denied_resp.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn workspace_runtime_key_cannot_manage_api_keys() {
    let (app, user_id) = build_app_with_approved_user(Some(AuthConfig::new("sk-internal"))).await;
    let workspace_id = create_workspace_for_user(app.clone(), user_id, "Runtime Workspace").await;

    let create_resp = app
        .clone()
        .oneshot(create_api_key_request_with_user(
            "sk-internal",
            &workspace_id,
            "SDK integration",
            user_id,
        ))
        .await
        .unwrap();
    assert_eq!(create_resp.status(), StatusCode::CREATED);
    let created = read_body(create_resp).await;
    let plaintext = created["plaintext_key"].as_str().unwrap();
    let key_id = created["api_key"]["id"].as_str().unwrap();

    let create_with_runtime_key = app
        .clone()
        .oneshot(create_api_key_request(
            plaintext,
            &workspace_id,
            "recursive key",
        ))
        .await
        .unwrap();
    assert_eq!(create_with_runtime_key.status(), StatusCode::FORBIDDEN);

    let list_with_runtime_key = app
        .clone()
        .oneshot(list_api_keys_request(plaintext, &workspace_id))
        .await
        .unwrap();
    assert_eq!(list_with_runtime_key.status(), StatusCode::FORBIDDEN);

    let revoke_with_runtime_key = app
        .oneshot(revoke_api_keys_request(plaintext, &workspace_id, &[key_id]))
        .await
        .unwrap();
    assert_eq!(revoke_with_runtime_key.status(), StatusCode::FORBIDDEN);
}
