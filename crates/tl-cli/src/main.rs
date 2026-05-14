use std::path::PathBuf;

use anyhow::{anyhow, Context};
use clap::{Parser, Subcommand};
use reqwest::StatusCode;
use tl_core::{ApiError, GuardrailGenerateResponse, GuardrailListResponse, PolicyDocument};

#[derive(Parser)]
#[command(name = "tl", about = "TrustLoopGuard CLI", version)]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Work with policy YAML locally or against tl-server.
    Policy {
        #[command(subcommand)]
        cmd: PolicyCmd,
    },
    /// Work with agents against tl-server.
    Agents {
        #[command(subcommand)]
        cmd: AgentsCmd,
    },
    /// Validate a policy YAML file.
    PolicyLint { path: PathBuf },
    /// Validate an agent profile YAML file.
    AgentLint { path: PathBuf },
}

#[derive(Subcommand)]
enum AgentsCmd {
    /// Manage guardrail policies attached to a specific agent.
    Guardrails {
        #[command(subcommand)]
        cmd: GuardrailsCmd,
    },
}

#[derive(Subcommand)]
enum GuardrailsCmd {
    /// Derive a guardrail policy set from the agent's stored
    /// `system_prompt` and persist each draft with `enabled=false`.
    /// Operators review the set and enable individual policies after.
    Generate {
        /// Agent identifier. Must already be registered via POST /v1/agents
        /// with a non-empty `system_prompt`.
        agent_id: String,
        /// tl-server base URL. Defaults to TL_SERVER_URL or http://localhost:8080.
        #[arg(long)]
        url: Option<String>,
        /// Bearer API key. Defaults to TL_API_KEY when set.
        #[arg(long)]
        api_key: Option<String>,
    },
    /// List guardrail policies owned by an agent.
    List {
        agent_id: String,
        /// tl-server base URL. Defaults to TL_SERVER_URL or http://localhost:8080.
        #[arg(long)]
        url: Option<String>,
        /// Bearer API key. Defaults to TL_API_KEY when set.
        #[arg(long)]
        api_key: Option<String>,
    },
}

#[derive(Subcommand)]
enum PolicyCmd {
    /// Validate a policy YAML file locally.
    Validate { path: PathBuf },
    /// Publish a policy YAML file to tl-server.
    Push {
        path: PathBuf,
        /// tl-server base URL. Defaults to TL_SERVER_URL or http://localhost:8080.
        #[arg(long)]
        url: Option<String>,
        /// Bearer API key. Defaults to TL_API_KEY when set.
        #[arg(long)]
        api_key: Option<String>,
    },
    /// Pull a policy YAML document from tl-server.
    Pull {
        policy_id: String,
        /// Destination YAML path.
        #[arg(short, long)]
        output: PathBuf,
        /// tl-server base URL. Defaults to TL_SERVER_URL or http://localhost:8080.
        #[arg(long)]
        url: Option<String>,
        /// Bearer API key. Defaults to TL_API_KEY when set.
        #[arg(long)]
        api_key: Option<String>,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.cmd {
        Cmd::Policy { cmd } => run_policy(cmd).await,
        Cmd::Agents { cmd } => run_agents(cmd).await,
        Cmd::PolicyLint { path } => {
            let src = std::fs::read_to_string(&path)?;
            let policy = tl_policy::load_str(&src)?;
            println!("ok: policy `{}` parsed", policy.id);
            Ok(())
        }
        Cmd::AgentLint { path } => {
            let src = std::fs::read_to_string(&path)?;
            let profile = tl_policy::load_agent_str(&src)?;
            println!(
                "ok: agent `{}` ({}) parsed",
                profile.agent_id, profile.display_name
            );
            Ok(())
        }
    }
}

async fn run_policy(cmd: PolicyCmd) -> anyhow::Result<()> {
    match cmd {
        PolicyCmd::Validate { path } => {
            let policy = load_policy_file(&path)?;
            println!("ok: policy `{}` valid", policy.id);
            Ok(())
        }
        PolicyCmd::Push { path, url, api_key } => {
            let src = std::fs::read_to_string(&path)
                .with_context(|| format!("read policy {}", path.display()))?;
            tl_policy::load_str(&src)
                .with_context(|| format!("validate policy {}", path.display()))?;
            let document = push_policy(&server_url(url), api_key, src).await?;
            println!("ok: pushed policy `{}`", document.id);
            Ok(())
        }
        PolicyCmd::Pull {
            policy_id,
            output,
            url,
            api_key,
        } => {
            let document = pull_policy(&server_url(url), api_key, &policy_id).await?;
            std::fs::write(&output, document.source_yaml)
                .with_context(|| format!("write policy {}", output.display()))?;
            println!(
                "ok: pulled policy `{}` to {}",
                document.id,
                output.display()
            );
            Ok(())
        }
    }
}

async fn run_agents(cmd: AgentsCmd) -> anyhow::Result<()> {
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
            let resp = generate_guardrails(&server_url(url), api_key, &agent_id).await?;
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
            let resp = list_guardrails(&server_url(url), api_key, &agent_id).await?;
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
        urlencode_id(agent_id),
    );
    let mut req = reqwest::Client::new().post(url);
    if let Some(key) = resolve_api_key(api_key) {
        req = req.bearer_auth(key);
    }
    let response = req.send().await.context("send guardrails generate")?;
    decode_typed_response(response, "guardrail generate response").await
}

async fn list_guardrails(
    base_url: &str,
    api_key: Option<String>,
    agent_id: &str,
) -> anyhow::Result<GuardrailListResponse> {
    let url = format!(
        "{}/v1/agents/{}/guardrails",
        base_url.trim_end_matches('/'),
        urlencode_id(agent_id),
    );
    let mut req = reqwest::Client::new().get(url);
    if let Some(key) = resolve_api_key(api_key) {
        req = req.bearer_auth(key);
    }
    let response = req.send().await.context("send guardrails list")?;
    decode_typed_response(response, "guardrail list response").await
}

/// Minimal hand-rolled path-segment encoder. The CLI doesn't pull in a
/// URL crate just for this; agent ids are kebab-case in practice, and
/// this still keeps slashes/spaces safe.
fn urlencode_id(id: &str) -> String {
    let mut out = String::with_capacity(id.len());
    for b in id.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char);
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

async fn decode_typed_response<T: serde::de::DeserializeOwned>(
    response: reqwest::Response,
    label: &str,
) -> anyhow::Result<T> {
    let status = response.status();
    let body = response.text().await.context("read response body")?;
    if status.is_success() {
        return serde_json::from_str(&body).with_context(|| format!("decode {label}"));
    }
    if let Ok(api_error) = serde_json::from_str::<ApiError>(&body) {
        return Err(anyhow!(
            "server returned {} ({:?}): {}",
            status,
            api_error.code,
            api_error.message
        ));
    }
    Err(anyhow!("server returned {status}: {body}"))
}

fn load_policy_file(path: &PathBuf) -> anyhow::Result<tl_policy::Policy> {
    let src =
        std::fs::read_to_string(path).with_context(|| format!("read policy {}", path.display()))?;
    tl_policy::load_str(&src).with_context(|| format!("validate policy {}", path.display()))
}

async fn push_policy(
    base_url: &str,
    api_key: Option<String>,
    source_yaml: String,
) -> anyhow::Result<PolicyDocument> {
    let url = format!("{}/v1/policies", base_url.trim_end_matches('/'));
    let mut req = reqwest::Client::new()
        .post(url)
        .header(reqwest::header::CONTENT_TYPE, "application/yaml")
        .body(source_yaml);
    if let Some(api_key) = resolve_api_key(api_key) {
        req = req.bearer_auth(api_key);
    }
    let response = req.send().await.context("send policy push")?;
    decode_json_response(response).await
}

async fn pull_policy(
    base_url: &str,
    api_key: Option<String>,
    policy_id: &str,
) -> anyhow::Result<PolicyDocument> {
    let url = format!(
        "{}/v1/policies/{}",
        base_url.trim_end_matches('/'),
        policy_id
    );
    let mut req = reqwest::Client::new().get(url);
    if let Some(api_key) = resolve_api_key(api_key) {
        req = req.bearer_auth(api_key);
    }
    let response = req.send().await.context("send policy pull")?;
    decode_json_response(response).await
}

async fn decode_json_response(response: reqwest::Response) -> anyhow::Result<PolicyDocument> {
    let status = response.status();
    let body = response.text().await.context("read response body")?;
    if status == StatusCode::OK || status == StatusCode::CREATED {
        return serde_json::from_str(&body).context("decode policy document");
    }
    if let Ok(api_error) = serde_json::from_str::<ApiError>(&body) {
        return Err(anyhow!(
            "server returned {} ({:?}): {}",
            status,
            api_error.code,
            api_error.message
        ));
    }
    Err(anyhow!("server returned {status}: {body}"))
}

fn server_url(url: Option<String>) -> String {
    url.or_else(|| std::env::var("TL_SERVER_URL").ok())
        .unwrap_or_else(|| "http://localhost:8080".to_string())
}

fn resolve_api_key(api_key: Option<String>) -> Option<String> {
    api_key
        .or_else(|| std::env::var("TL_API_KEY").ok())
        .filter(|value| !value.trim().is_empty())
}
