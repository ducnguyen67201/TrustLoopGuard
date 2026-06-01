use tl_core::{
    CreateEnforcementProfileRequest, CreateGatewayProviderConnectionRequest,
    CreateGatewayRouteRequest, GatewayProviderKind, ResponseMode, RetentionMode,
    UpdateEnforcementProfileRequest, UpdateGatewayProviderConnectionRequest,
    UpdateGatewayRouteRequest,
};
#[cfg(feature = "postgres")]
use tl_core::{FailMode, GatewayInputAction, GatewayOutputAction};
use url::Url;
use uuid::Uuid;

use super::crypto::seal_provider_key;
use super::store::{
    EnforcementProfilePatch, GatewayRoutePatch, NewEnforcementProfile,
    NewGatewayProviderConnection, NewGatewayRoute, ProviderConnectionPatch,
};

pub(super) fn normalize_provider_connection(
    workspace_id: &str,
    req: CreateGatewayProviderConnectionRequest,
    seal_key: &[u8; 32],
) -> Result<NewGatewayProviderConnection, String> {
    let display_name = required_trimmed(req.display_name, "display_name")?;
    let default_model = required_trimmed(req.default_model, "default_model")?;
    let provider_api_key = required_trimmed(req.provider_api_key, "provider_api_key")?;
    Ok(NewGatewayProviderConnection {
        id: req.id.unwrap_or_else(|| format!("gpc_{}", Uuid::now_v7())),
        workspace_id: workspace_id.to_string(),
        display_name,
        kind: req.kind,
        base_url: normalize_optional_url(req.base_url)?,
        default_model,
        encrypted_api_key: seal_provider_key(&provider_api_key, seal_key),
    })
}

pub(super) fn normalize_provider_connection_patch(
    req: UpdateGatewayProviderConnectionRequest,
    seal_key: &[u8; 32],
) -> Result<ProviderConnectionPatch, String> {
    Ok(ProviderConnectionPatch {
        display_name: normalize_optional_text(req.display_name, "display_name")?,
        base_url: match req.base_url {
            None => None,
            Some(v) => Some(normalize_optional_url(Some(v))?),
        },
        default_model: normalize_optional_text(req.default_model, "default_model")?,
        encrypted_api_key: req
            .provider_api_key
            .map(|value| {
                required_trimmed(value, "provider_api_key")
                    .map(|key| seal_provider_key(&key, seal_key))
            })
            .transpose()?,
    })
}

pub(super) fn normalize_enforcement_profile(
    workspace_id: &str,
    req: CreateEnforcementProfileRequest,
) -> Result<NewEnforcementProfile, String> {
    Ok(NewEnforcementProfile {
        id: req.id.unwrap_or_else(|| format!("ep_{}", Uuid::now_v7())),
        workspace_id: workspace_id.to_string(),
        display_name: required_trimmed(req.display_name, "display_name")?,
        input_action: req.input_action,
        output_action: req.output_action,
        fail_mode: req.fail_mode,
        retention_mode: req.retention_mode,
        response_mode: req.response_mode,
        fallback_message: required_trimmed(req.fallback_message, "fallback_message")?,
        max_regenerations: req.max_regenerations,
    })
}

pub(super) fn normalize_enforcement_profile_patch(
    req: UpdateEnforcementProfileRequest,
) -> Result<EnforcementProfilePatch, String> {
    Ok(EnforcementProfilePatch {
        display_name: normalize_optional_text(req.display_name, "display_name")?,
        input_action: req.input_action,
        output_action: req.output_action,
        fail_mode: req.fail_mode,
        retention_mode: req.retention_mode,
        response_mode: req.response_mode,
        fallback_message: normalize_optional_text(req.fallback_message, "fallback_message")?,
        max_regenerations: req.max_regenerations,
    })
}

pub(super) fn normalize_gateway_route(
    workspace_id: &str,
    req: CreateGatewayRouteRequest,
) -> Result<NewGatewayRoute, String> {
    Ok(NewGatewayRoute {
        id: req.id.unwrap_or_else(|| format!("gr_{}", Uuid::now_v7())),
        workspace_id: workspace_id.to_string(),
        display_name: required_trimmed(req.display_name, "display_name")?,
        provider_connection_id: required_trimmed(
            req.provider_connection_id,
            "provider_connection_id",
        )?,
        agent_id: required_trimmed(req.agent_id, "agent_id")?,
        enforcement_profile_id: required_trimmed(
            req.enforcement_profile_id,
            "enforcement_profile_id",
        )?,
    })
}

pub(super) fn normalize_gateway_route_patch(
    req: UpdateGatewayRouteRequest,
) -> Result<GatewayRoutePatch, String> {
    Ok(GatewayRoutePatch {
        display_name: normalize_optional_text(req.display_name, "display_name")?,
        provider_connection_id: normalize_optional_text(
            req.provider_connection_id,
            "provider_connection_id",
        )?,
        agent_id: normalize_optional_text(req.agent_id, "agent_id")?,
        enforcement_profile_id: normalize_optional_text(
            req.enforcement_profile_id,
            "enforcement_profile_id",
        )?,
    })
}

pub(super) fn normalize_optional_url(value: Option<String>) -> Result<Option<String>, String> {
    let Some(raw) = value else { return Ok(None) };
    let raw = raw.trim().trim_end_matches('/').to_string();
    if raw.is_empty() {
        return Ok(None);
    }
    let parsed = Url::parse(&raw).map_err(|_| "base_url must be a valid URL".to_string())?;
    match parsed.scheme() {
        "https" | "http" => {}
        scheme => {
            return Err(format!(
                "base_url scheme '{scheme}' is not allowed; use https or http"
            ))
        }
    }
    let host = parsed
        .host()
        .ok_or_else(|| "base_url must have a host".to_string())?;
    match host {
        url::Host::Ipv4(addr) => {
            let [a, b, ..] = addr.octets();
            // Hard-block cloud metadata endpoints (AWS IMDSv1, GCP, Azure).
            if a == 169 && b == 254 {
                return Err(
                    "base_url cannot point to a link-local address (169.254.x.x)".to_string(),
                );
            }
            // Warn for other private ranges — some on-premise deployments are legitimate.
            if a == 127 || a == 10 || (a == 172 && (16..=31).contains(&b)) || (a == 192 && b == 168)
            {
                tracing::warn!(
                    base_url = %raw,
                    "SECURITY: provider base_url targets a private network address; \
                     ensure this deployment intentionally routes to an on-premise provider"
                );
            }
        }
        url::Host::Ipv6(addr) => {
            if addr.is_loopback() || addr.is_unspecified() {
                tracing::warn!(
                    base_url = %raw,
                    "SECURITY: provider base_url targets a loopback IPv6 address"
                );
            }
        }
        url::Host::Domain(host) => {
            // Hard-block k8s/mDNS internal domains.
            if host.ends_with(".local")
                || host.ends_with(".internal")
                || host.ends_with(".cluster.local")
            {
                return Err("base_url cannot point to an internal cluster domain".to_string());
            }
            if host == "localhost" || host.ends_with(".localhost") {
                tracing::warn!(
                    base_url = %raw,
                    "SECURITY: provider base_url targets localhost; \
                     ensure this is intentional (local dev or test environment only)"
                );
            }
        }
    }
    Ok(Some(raw))
}

pub(super) fn normalize_optional_text(
    value: Option<String>,
    field: &str,
) -> Result<Option<String>, String> {
    value
        .map(|value| required_trimmed(value, field))
        .transpose()
}

fn required_trimmed(value: String, field: &str) -> Result<String, String> {
    let value = value.trim().to_string();
    if value.is_empty() {
        Err(format!("{field} is required"))
    } else {
        Ok(value)
    }
}

pub(super) fn provider_kind_text(kind: GatewayProviderKind) -> &'static str {
    match kind {
        GatewayProviderKind::OpenaiCompatible => "openai_compatible",
        GatewayProviderKind::Anthropic => "anthropic",
    }
}

pub(super) fn retention_mode_text(mode: RetentionMode) -> &'static str {
    match mode {
        RetentionMode::MetadataOnly => "metadata_only",
        RetentionMode::RedactedBody => "redacted_body",
        RetentionMode::FullBody => "full_body",
    }
}

#[cfg(feature = "postgres")]
pub(crate) fn provider_kind_storage_text(kind: GatewayProviderKind) -> &'static str {
    provider_kind_text(kind)
}

#[cfg(feature = "postgres")]
pub(crate) fn input_action_storage_text(action: GatewayInputAction) -> &'static str {
    match action {
        GatewayInputAction::Allow => "allow",
        GatewayInputAction::Block => "block",
        GatewayInputAction::Redact => "redact",
    }
}

#[cfg(feature = "postgres")]
pub(crate) fn output_action_storage_text(action: GatewayOutputAction) -> &'static str {
    match action {
        GatewayOutputAction::Allow => "allow",
        GatewayOutputAction::Block => "block",
        GatewayOutputAction::Rewrite => "rewrite",
        GatewayOutputAction::Escalate => "escalate",
    }
}

#[cfg(feature = "postgres")]
pub(crate) fn fail_mode_storage_text(mode: FailMode) -> &'static str {
    match mode {
        FailMode::Open => "open",
        FailMode::Closed => "closed",
    }
}

#[cfg(feature = "postgres")]
pub(crate) fn retention_mode_storage_text(mode: RetentionMode) -> &'static str {
    retention_mode_text(mode)
}

#[cfg(feature = "postgres")]
pub(crate) fn response_mode_storage_text(mode: ResponseMode) -> &'static str {
    match mode {
        ResponseMode::Regular => "regular",
        ResponseMode::Streaming => "streaming",
    }
}
