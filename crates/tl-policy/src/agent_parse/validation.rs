use std::{collections::HashSet, net::IpAddr};

use tl_core::{AgentProfile, KnowledgeSourceKind};
use url::Url;

use crate::policy_parse::PolicyError;

pub(super) fn validate(profile: &AgentProfile) -> Result<(), PolicyError> {
    if profile.agent_id.trim().is_empty() {
        return Err(PolicyError::Validation("agent_id is required".into()));
    }
    if profile.display_name.trim().is_empty() {
        return Err(PolicyError::Validation("display_name is required".into()));
    }
    if profile.scope.in_scope.is_empty() {
        return Err(PolicyError::Validation(
            "scope.in_scope must contain at least one entry".into(),
        ));
    }

    validate_knowledge_sources(profile)
}

fn validate_knowledge_sources(profile: &AgentProfile) -> Result<(), PolicyError> {
    let mut seen = HashSet::new();
    for (idx, source) in profile.knowledge_sources.iter().enumerate() {
        let source_id = source.kb_id.trim();
        if source_id.is_empty() {
            return Err(PolicyError::Validation(format!(
                "knowledge_sources[{idx}].kb_id is required"
            )));
        }
        if !seen.insert(source_id) {
            return Err(PolicyError::Validation(format!(
                "knowledge_sources[{idx}].kb_id duplicate `{}`",
                source.kb_id
            )));
        }
        if source.kind == KnowledgeSourceKind::Web {
            let raw_url = source.url.as_deref().ok_or_else(|| {
                PolicyError::Validation(format!("knowledge_sources[{idx}].url is required"))
            })?;
            validate_public_web_url(idx, raw_url)?;
        }
    }

    Ok(())
}

fn validate_public_web_url(idx: usize, raw_url: &str) -> Result<(), PolicyError> {
    let parsed = Url::parse(raw_url).map_err(|_| public_url_error(idx))?;
    if !matches!(parsed.scheme(), "http" | "https") {
        return Err(public_url_error(idx));
    }

    let Some(host) = parsed.host_str() else {
        return Err(public_url_error(idx));
    };

    if is_private_host(host) {
        return Err(public_url_error(idx));
    }

    Ok(())
}

fn is_private_host(host: &str) -> bool {
    let normalized = host.to_ascii_lowercase();
    let is_private_name = normalized == "localhost" || normalized.ends_with(".localhost");
    let is_private_ip = normalized
        .parse::<IpAddr>()
        .map(is_private_ip)
        .unwrap_or(false);

    is_private_name || is_private_ip
}

fn is_private_ip(addr: IpAddr) -> bool {
    match addr {
        IpAddr::V4(addr) => {
            addr.is_private() || addr.is_loopback() || addr.is_link_local() || addr.is_unspecified()
        }
        IpAddr::V6(addr) => {
            let first = addr.segments()[0];
            let is_unique_local = (first & 0xfe00) == 0xfc00;
            let is_link_local = (first & 0xffc0) == 0xfe80;
            addr.is_loopback() || addr.is_unspecified() || is_unique_local || is_link_local
        }
    }
}

fn public_url_error(idx: usize) -> PolicyError {
    PolicyError::Validation(format!(
        "knowledge_sources[{idx}].url must be a public http(s) URL"
    ))
}
