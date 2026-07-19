use chrono::Utc;
use tl_core::{
    Action, AuthorizationClaim, EventKind, GuardEvent, McpGatewayAuthKind, McpGatewayCatalogStatus,
    McpGatewayTool, Principal, SideEffectClass,
};

use super::{handler, upstream, EntitledMcpTool};

fn event() -> GuardEvent {
    GuardEvent {
        kind: EventKind::ToolCallProposed,
        principal: Principal {
            workspace_id: "workspace".into(),
            environment_id: "production".into(),
            agent_id: "hosted-mcp".into(),
            user_id: Some("user".into()),
            session_id: Some("client".into()),
            task_id: None,
            run_id: None,
            run_event_id: None,
        },
        action: Action {
            operation: "mcp:server:tool".into(),
            parameters: serde_json::json!({}),
            side_effect: Some(SideEffectClass::Read),
            invocation_id: Some("stable-invocation".into()),
            tool_identity: None,
            authorization: None,
        },
        sources: Vec::new(),
        provenance: Default::default(),
        resolution: None,
        label_resolution: None,
        checks: Vec::new(),
        signals: Vec::new(),
        context: serde_json::Value::Null,
    }
}

fn entitled(side_effect: SideEffectClass) -> EntitledMcpTool {
    EntitledMcpTool {
        tool: McpGatewayTool {
            id: "019f7b7b-8f97-7000-8000-000000000001".into(),
            connection_id: "019f7b7b-8f97-7000-8000-000000000002".into(),
            connection_name: "Company tools".into(),
            upstream_name: "read_docs".into(),
            public_name: "company__read_docs".into(),
            title: None,
            description: None,
            input_schema: serde_json::json!({"type":"object"}),
            output_schema: None,
            annotations: serde_json::json!({}),
            schema_hash: "sha256:v1:test".into(),
            side_effect,
            catalog_status: McpGatewayCatalogStatus::Active,
            assigned_user_ids: vec!["user".into()],
            created_at: "2026-07-19T00:00:00Z".into(),
            updated_at: "2026-07-19T00:00:00Z".into(),
        },
        endpoint_url: "https://tools.example/mcp".into(),
        auth_kind: McpGatewayAuthKind::StaticBearer,
        encrypted_credential: Some("sealed".into()),
        connection_updated_at: Utc::now(),
    }
}

#[test]
fn approval_resume_preserves_the_original_invocation() {
    let original = event();
    let resumed = handler::resume_authorized_event(
        original.clone(),
        AuthorizationClaim {
            grant_id: "grant".into(),
            attempt_id: "attempt".into(),
        },
    );

    assert_eq!(resumed.action.invocation_id, original.action.invocation_id);
    assert_eq!(
        resumed
            .action
            .authorization
            .as_ref()
            .map(|claim| claim.grant_id.as_str()),
        Some("grant")
    );
}

#[test]
fn side_effect_reclassification_invalidates_execution_authority() {
    let original = entitled(SideEffectClass::Read);
    let current = entitled(SideEffectClass::ApiMutation);
    assert!(handler::require_same_authority(&original, &current).is_err());
}

#[test]
fn insecure_http_is_only_valid_for_loopback_addresses() {
    assert!(upstream::endpoint_address_allowed(
        "http",
        true,
        "127.0.0.1".parse().unwrap()
    ));
    assert!(!upstream::endpoint_address_allowed(
        "http",
        true,
        "8.8.8.8".parse().unwrap()
    ));
    assert!(upstream::endpoint_address_allowed(
        "https",
        false,
        "8.8.8.8".parse().unwrap()
    ));
}

#[test]
fn catalog_pagination_stops_before_accumulating_too_many_tools() {
    assert!(upstream::catalog_page_fits(499, 1));
    assert!(!upstream::catalog_page_fits(500, 1));
}
