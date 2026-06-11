use std::path::PathBuf;

use anyhow::{anyhow, bail, Context};
use reqwest::StatusCode;
use tl_core::{ApiError, PolicyDocument};
use tl_policy::AnyPolicy;

use crate::{http, PolicyCmd};

pub(super) async fn run(cmd: PolicyCmd) -> anyhow::Result<()> {
    match cmd {
        PolicyCmd::Validate { path } => {
            match load_policy_file(&path)? {
                AnyPolicy::Content(policy) => println!("ok: policy `{}` valid", policy.id),
                AnyPolicy::Family(policy) => println!(
                    "ok: family policy `{}` valid (parse/validate only; runtime evaluation \
                     is not implemented yet)",
                    policy.id()
                ),
            }
            Ok(())
        }
        PolicyCmd::Push { path, url, api_key } => {
            let src = std::fs::read_to_string(&path)
                .with_context(|| format!("read policy {}", path.display()))?;
            match tl_policy::load_any_str(&src)
                .with_context(|| format!("validate policy {}", path.display()))?
            {
                AnyPolicy::Content(_) => {}
                AnyPolicy::Family(policy) => bail!(
                    "family policy `{}` cannot be pushed yet: POST /v1/policies stores \
                     content policies only",
                    policy.id()
                ),
            }
            let document = push_policy(&http::server_url(url), api_key, src).await?;
            println!("ok: pushed policy `{}`", document.id);
            Ok(())
        }
        PolicyCmd::Pull {
            policy_id,
            output,
            url,
            api_key,
        } => {
            let document = pull_policy(&http::server_url(url), api_key, &policy_id).await?;
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

fn load_policy_file(path: &PathBuf) -> anyhow::Result<AnyPolicy> {
    let src =
        std::fs::read_to_string(path).with_context(|| format!("read policy {}", path.display()))?;
    tl_policy::load_any_str(&src).with_context(|| format!("validate policy {}", path.display()))
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
    if let Some(api_key) = http::resolve_api_key(api_key) {
        req = req.bearer_auth(api_key);
    }
    let response = req.send().await.context("send policy push")?;
    decode_policy_response(response).await
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
    if let Some(api_key) = http::resolve_api_key(api_key) {
        req = req.bearer_auth(api_key);
    }
    let response = req.send().await.context("send policy pull")?;
    decode_policy_response(response).await
}

async fn decode_policy_response(response: reqwest::Response) -> anyhow::Result<PolicyDocument> {
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
