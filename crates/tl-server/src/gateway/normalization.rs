use tl_core::{
    CreateGatewayProviderConnectionRequest, CreateGatewayRouteRequest, GatewayProviderKind,
    UpdateGatewayProviderConnectionRequest, UpdateGatewayRouteRequest,
};
use url::Url;
use uuid::Uuid;

use super::crypto::seal_provider_key;
use super::store::{
    GatewayRoutePatch, NewGatewayProviderConnection, NewGatewayRoute, ProviderConnectionPatch,
};

pub(super) fn normalize_provider_connection(
    workspace_id: &str,
    req: CreateGatewayProviderConnectionRequest,
    seal_key: &[u8; 32],
) -> Result<NewGatewayProviderConnection, String> {
    let display_name = required_trimmed(req.display_name, "display_name")?;
    let provider_api_key = required_trimmed(req.provider_api_key, "provider_api_key")?;
    let base_url = normalize_optional_url(req.base_url)?;
    // Payment connections have no model and no default host — the endpoint
    // IS the configuration, so require it. LLM kinds keep requiring a model.
    let default_model = if req.kind == GatewayProviderKind::PaymentHttp {
        if base_url.is_none() {
            return Err("base_url is required for payment_http connections".to_string());
        }
        req.default_model.trim().to_string()
    } else {
        required_trimmed(req.default_model, "default_model")?
    };
    Ok(NewGatewayProviderConnection {
        id: req.id.unwrap_or_else(|| format!("gpc_{}", Uuid::now_v7())),
        workspace_id: workspace_id.to_string(),
        display_name,
        kind: req.kind,
        base_url,
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
        GatewayProviderKind::PaymentHttp => "payment_http",
    }
}

#[cfg(feature = "postgres")]
pub(crate) fn provider_kind_storage_text(kind: GatewayProviderKind) -> &'static str {
    provider_kind_text(kind)
}
