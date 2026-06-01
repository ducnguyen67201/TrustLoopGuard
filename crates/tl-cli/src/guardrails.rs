use anyhow::Context;
use tl_core::{GuardrailGenerateResponse, GuardrailListResponse};

use crate::{http, AgentsCmd, GuardrailsCmd};

pub(super) async fn run_agents(cmd: AgentsCmd) -> anyhow::Result<()> {
    match cmd {
        AgentsCmd::Guardrails { cmd } => run_guardrails(cmd).await,
    }
}

async fn run_guardrails(cmd: GuardrailsCmd) -> anyhow::Result<()> {
    match cmd {
        GuardrailsCmd::Generate {
            agent_id,
            url,
            api_key,
        } => {
            let resp = generate_guardrails(&http::server_url(url), api_key, &agent_id).await?;
            if resp.generated.is_empty() {
                println!("no guardrails generated for `{agent_id}`");
                return Ok(());
            }
            println!(
                "ok: generated {} guardrail(s) for `{agent_id}` (enabled=false; review then enable):",
                resp.generated.len()
            );
            for doc in &resp.generated {
                let summary = doc.description.as_deref().unwrap_or("(no description)");
                println!("  - {} [{:?}]: {summary}", doc.id, doc.severity);
            }
            println!(
                "\nEnable a policy with: tl policy push <yaml> or PATCH /v1/policies/<id>/enabled"
            );
            Ok(())
        }
        GuardrailsCmd::List {
            agent_id,
            url,
            api_key,
        } => {
            let resp = list_guardrails(&http::server_url(url), api_key, &agent_id).await?;
            if resp.policies.is_empty() {
                println!("no guardrails owned by `{agent_id}`");
                return Ok(());
            }
            println!(
                "ok: {} guardrail(s) owned by `{agent_id}`:",
                resp.policies.len()
            );
            for p in &resp.policies {
                let enabled_marker = if p.enabled { "ON " } else { "off" };
                let summary = p.description.as_deref().unwrap_or("(no description)");
                println!(
                    "  [{enabled_marker}] {} [{:?}]: {summary}",
                    p.id, p.severity
                );
            }
            Ok(())
        }
    }
}

async fn generate_guardrails(
    base_url: &str,
    api_key: Option<String>,
    agent_id: &str,
) -> anyhow::Result<GuardrailGenerateResponse> {
    let url = format!(
        "{}/v1/agents/{}/guardrails/generate",
        base_url.trim_end_matches('/'),
        http::urlencode_id(agent_id),
    );
    let mut req = reqwest::Client::new().post(url);
    if let Some(key) = http::resolve_api_key(api_key) {
        req = req.bearer_auth(key);
    }
    let response = req.send().await.context("send guardrails generate")?;
    http::decode_typed_response(response, "guardrail generate response").await
}

async fn list_guardrails(
    base_url: &str,
    api_key: Option<String>,
    agent_id: &str,
) -> anyhow::Result<GuardrailListResponse> {
    let url = format!(
        "{}/v1/agents/{}/guardrails",
        base_url.trim_end_matches('/'),
        http::urlencode_id(agent_id),
    );
    let mut req = reqwest::Client::new().get(url);
    if let Some(key) = http::resolve_api_key(api_key) {
        req = req.bearer_auth(key);
    }
    let response = req.send().await.context("send guardrails list")?;
    http::decode_typed_response(response, "guardrail list response").await
}
