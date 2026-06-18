use tl_core::{AgentAuthority, AgentProfile, AgentScope, AgentTone};

use super::validation::{is_yaml_content_type, validate_profile};
use super::{AgentStore, AgentStoreError, MemoryAgentStore};

fn profile(id: &str) -> AgentProfile {
    AgentProfile {
        agent_id: id.into(),
        display_name: format!("{id} display"),
        scope: AgentScope {
            in_scope: vec!["billing".into()],
            out_of_scope: vec![],
        },
        authority: AgentAuthority::default(),
        tone: AgentTone {
            target: "neutral".into(),
            forbidden: vec![],
        },
        knowledge_sources: vec![],
        escalation_triggers: vec![],
        system_prompt: None,
        workflow_definition: None,
    }
}

#[tokio::test]
async fn memory_store_round_trip() {
    let store = MemoryAgentStore::new();
    store
        .upsert("default", &profile("a"), "yaml")
        .await
        .unwrap();
    let got = store.get("default", "a").await.unwrap();
    assert_eq!(got.agent_id, "a");
}

#[tokio::test]
async fn memory_store_delete_then_get_not_found() {
    let store = MemoryAgentStore::new();
    store
        .upsert("default", &profile("a"), "yaml")
        .await
        .unwrap();
    store.delete("default", "a").await.unwrap();
    assert!(matches!(
        store.get("default", "a").await,
        Err(AgentStoreError::NotFound)
    ));
}

#[tokio::test]
async fn memory_store_list_sorted() {
    let store = MemoryAgentStore::new();
    store.upsert("default", &profile("z"), "y").await.unwrap();
    store.upsert("default", &profile("a"), "y").await.unwrap();
    store.upsert("default", &profile("m"), "y").await.unwrap();
    let ids: Vec<String> = store
        .list("default")
        .await
        .unwrap()
        .iter()
        .map(|profile| profile.agent_id.clone())
        .collect();
    assert_eq!(ids, vec!["a", "m", "z"]);
}

#[tokio::test]
async fn delete_missing_yields_not_found() {
    let store = MemoryAgentStore::new();
    assert!(matches!(
        store.delete("default", "nope").await,
        Err(AgentStoreError::NotFound)
    ));
}

#[test]
fn yaml_content_types() {
    for content_type in [
        "application/yaml",
        "application/x-yaml",
        "text/yaml",
        "text/x-yaml",
        "application/yaml; charset=utf-8",
    ] {
        assert!(
            is_yaml_content_type(content_type),
            "should be YAML: {content_type}"
        );
    }
    for content_type in ["application/json", "text/plain", ""] {
        assert!(
            !is_yaml_content_type(content_type),
            "should NOT be YAML: {content_type}"
        );
    }
}

#[test]
fn validate_rejects_empty_agent_id() {
    let mut profile = profile("ok");
    profile.agent_id = String::new();
    assert!(validate_profile(&profile).is_err());
}

#[test]
fn validate_rejects_empty_in_scope() {
    let mut profile = profile("ok");
    profile.scope.in_scope.clear();
    assert!(validate_profile(&profile).is_err());
}
