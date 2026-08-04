use std::collections::HashSet;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use std::time::Duration;

use rmcp::model::{CallToolRequestParams, CallToolResult, PaginatedRequestParams, Tool};
use rmcp::service::RunningService;
use rmcp::transport::streamable_http_client::{
    StreamableHttpClientTransport, StreamableHttpClientTransportConfig,
};
use rmcp::{RoleClient, ServiceExt};
use sha2::{Digest, Sha256};
use url::Url;

use super::bounded_http::BoundedHttpClient;
use super::naming::public_tool_names;
use super::{CatalogToolInput, McpGatewayStoreError};

const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
const OPERATION_TIMEOUT: Duration = Duration::from_secs(30);
const MAX_TOOLS: usize = 500;
const MAX_DESCRIPTION_BYTES: usize = 4 * 1024;
const MAX_SCHEMA_BYTES: usize = 64 * 1024;
const MAX_SCHEMA_DEPTH: usize = 32;
const MAX_CATALOG_HTTP_BYTES: usize = 72 * 1024 * 1024;
const MAX_TOOL_RESULT_HTTP_BYTES: usize = 1024 * 1024 + 64 * 1024;

pub(super) struct PreparedUpstream {
    service: RunningService<RoleClient, ()>,
}

impl PreparedUpstream {
    pub(super) async fn list_tools(&self) -> Result<Vec<Tool>, McpGatewayStoreError> {
        tokio::time::timeout(OPERATION_TIMEOUT, async {
            let mut tools = Vec::new();
            let mut cursor = None;
            let mut seen_cursors = HashSet::new();
            let mut page_count = 0usize;
            loop {
                if page_count >= MAX_TOOLS {
                    return Err(rmcp::ServiceError::McpError(
                        rmcp::model::ErrorData::invalid_request(
                            "upstream returned too many catalog pages",
                            None,
                        ),
                    ));
                }
                page_count += 1;
                let page = self
                    .service
                    .list_tools(Some(
                        PaginatedRequestParams::default().with_cursor(cursor.clone()),
                    ))
                    .await?;
                if !catalog_page_fits(tools.len(), page.tools.len()) {
                    return Err(rmcp::ServiceError::McpError(
                        rmcp::model::ErrorData::invalid_request(
                            "upstream exposes more than 500 tools",
                            None,
                        ),
                    ));
                }
                tools.extend(page.tools);
                let Some(next) = page.next_cursor else {
                    return Ok(tools);
                };
                if next.is_empty() || !seen_cursors.insert(next.clone()) {
                    return Err(rmcp::ServiceError::McpError(
                        rmcp::model::ErrorData::invalid_request(
                            "upstream returned an invalid tools cursor",
                            None,
                        ),
                    ));
                }
                cursor = Some(next);
            }
        })
        .await
        .map_err(|_| safe_upstream("upstream tool listing timed out"))?
        .map_err(|_| safe_upstream("upstream tool listing failed"))
    }

    pub(super) async fn call_tool(
        &self,
        request: CallToolRequestParams,
    ) -> Result<CallToolResult, McpGatewayStoreError> {
        tokio::time::timeout(OPERATION_TIMEOUT, self.service.call_tool(request))
            .await
            .map_err(|_| safe_upstream("upstream tool call timed out"))?
            .map_err(|_| safe_upstream("upstream tool call failed"))
    }

    pub(super) async fn close(mut self) {
        let _ = self
            .service
            .close_with_timeout(Duration::from_secs(2))
            .await;
    }
}

async fn prepare_upstream(
    endpoint: &str,
    bearer: Option<&str>,
    max_response_bytes: usize,
) -> Result<PreparedUpstream, McpGatewayStoreError> {
    let (url, host, addresses) = validate_and_resolve_endpoint(endpoint).await?;
    let client = reqwest::Client::builder()
        .connect_timeout(CONNECT_TIMEOUT)
        .timeout(OPERATION_TIMEOUT)
        .redirect(reqwest::redirect::Policy::none())
        .retry(reqwest::retry::never())
        .no_proxy()
        .resolve_to_addrs(&host, &addresses)
        .build()
        .map_err(|_| safe_upstream("upstream transport could not be configured"))?;
    let mut config = StreamableHttpClientTransportConfig::with_uri(url.as_str().to_string());
    config.allow_stateless = true;
    config.reinit_on_expired_session = false;
    if let Some(bearer) = bearer {
        config = config.auth_header(bearer.to_string());
    }
    let transport = StreamableHttpClientTransport::with_client(
        BoundedHttpClient::new(client, max_response_bytes),
        config,
    );
    let service = tokio::time::timeout(CONNECT_TIMEOUT, ().serve(transport))
        .await
        .map_err(|_| safe_upstream("upstream connection timed out"))?
        .map_err(|_| safe_upstream("upstream connection failed"))?;
    Ok(PreparedUpstream { service })
}

pub(super) async fn prepare_catalog_upstream(
    endpoint: &str,
    bearer: Option<&str>,
) -> Result<PreparedUpstream, McpGatewayStoreError> {
    prepare_upstream(endpoint, bearer, MAX_CATALOG_HTTP_BYTES).await
}

pub(super) async fn prepare_tool_upstream(
    endpoint: &str,
    bearer: Option<&str>,
) -> Result<PreparedUpstream, McpGatewayStoreError> {
    prepare_upstream(endpoint, bearer, MAX_TOOL_RESULT_HTTP_BYTES).await
}

pub(super) async fn sync_catalog(
    endpoint: &str,
    bearer: Option<&str>,
    server_slug: &str,
    connection_id: uuid::Uuid,
) -> Result<Vec<CatalogToolInput>, McpGatewayStoreError> {
    let peer = prepare_catalog_upstream(endpoint, bearer).await?;
    let result = async {
        let tools = peer.list_tools().await?;
        normalize_catalog(server_slug, connection_id, tools)
    }
    .await;
    peer.close().await;
    result
}

pub(super) fn normalize_catalog(
    server_slug: &str,
    connection_id: uuid::Uuid,
    tools: Vec<Tool>,
) -> Result<Vec<CatalogToolInput>, McpGatewayStoreError> {
    if tools.len() > MAX_TOOLS {
        return Err(McpGatewayStoreError::Conflict(
            "upstream exposes more than 500 tools".into(),
        ));
    }
    let upstream_names = tools
        .iter()
        .map(|tool| tool.name.to_string())
        .collect::<Vec<_>>();
    let aliases = public_tool_names(server_slug, connection_id, &upstream_names)
        .map_err(|message| McpGatewayStoreError::Conflict(message.into()))?;
    tools
        .into_iter()
        .zip(aliases)
        .map(|(tool, public_name)| normalize_tool(tool, public_name))
        .collect()
}

fn normalize_tool(
    tool: Tool,
    public_name: String,
) -> Result<CatalogToolInput, McpGatewayStoreError> {
    if tool.description.as_deref().map(str::len).unwrap_or(0) > MAX_DESCRIPTION_BYTES {
        return Err(McpGatewayStoreError::Conflict(
            "upstream tool description is too large".into(),
        ));
    }
    let input_schema = serde_json::Value::Object((*tool.input_schema).clone());
    validate_schema(&input_schema, true)?;
    if input_schema
        .get("properties")
        .and_then(serde_json::Value::as_object)
        .is_some_and(|properties| properties.contains_key(super::governance::RESERVED_FIELD))
    {
        return Err(McpGatewayStoreError::Conflict(
            "upstream tool uses the reserved __featherlane_ai argument".into(),
        ));
    }
    let output_schema = tool
        .output_schema
        .map(|schema| serde_json::Value::Object((*schema).clone()));
    if let Some(schema) = output_schema.as_ref() {
        validate_schema(schema, false)?;
    }
    let annotations = tool
        .annotations
        .map(|value| serde_json::to_value(value).unwrap_or_else(|_| serde_json::json!({})))
        .unwrap_or_else(|| serde_json::json!({}));
    Ok(CatalogToolInput {
        upstream_name: tool.name.into_owned(),
        public_name,
        title: tool.title,
        description: tool.description.map(|value| value.into_owned()),
        schema_hash: schema_hash(&input_schema),
        input_schema,
        output_schema,
        annotations,
    })
}

pub(super) fn schema_hash(schema: &serde_json::Value) -> String {
    let canonical = crate::authorization::canonical_json(schema);
    let digest = Sha256::digest(canonical.as_bytes());
    format!(
        "sha256:v1:{}",
        digest
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>()
    )
}

pub(super) fn validate_schema(
    schema: &serde_json::Value,
    require_object: bool,
) -> Result<(), McpGatewayStoreError> {
    if serde_json::to_vec(schema)
        .map_err(|_| McpGatewayStoreError::Conflict("schema is invalid".into()))?
        .len()
        > MAX_SCHEMA_BYTES
    {
        return Err(McpGatewayStoreError::Conflict(
            "upstream tool schema is too large".into(),
        ));
    }
    if require_object && schema.get("type").and_then(serde_json::Value::as_str) != Some("object") {
        return Err(McpGatewayStoreError::Conflict(
            "tool input schema must have object type".into(),
        ));
    }
    inspect_schema(schema, 0)?;
    jsonschema::validator_for(schema)
        .map_err(|_| McpGatewayStoreError::Conflict("upstream tool schema is invalid".into()))?;
    Ok(())
}

fn inspect_schema(value: &serde_json::Value, depth: usize) -> Result<(), McpGatewayStoreError> {
    if depth > MAX_SCHEMA_DEPTH {
        return Err(McpGatewayStoreError::Conflict(
            "upstream tool schema is too deeply nested".into(),
        ));
    }
    match value {
        serde_json::Value::Object(object) => {
            if let Some(reference) = object.get("$ref").and_then(serde_json::Value::as_str) {
                if !reference.starts_with('#') {
                    return Err(McpGatewayStoreError::Conflict(
                        "external schema references are not allowed".into(),
                    ));
                }
            }
            for child in object.values() {
                inspect_schema(child, depth + 1)?;
            }
        }
        serde_json::Value::Array(values) => {
            for child in values {
                inspect_schema(child, depth + 1)?;
            }
        }
        _ => {}
    }
    Ok(())
}

async fn validate_and_resolve_endpoint(
    endpoint: &str,
) -> Result<(Url, String, Vec<SocketAddr>), McpGatewayStoreError> {
    let url = validate_endpoint_url(endpoint)
        .map_err(|message| McpGatewayStoreError::Conflict(message.into()))?;
    let allow_http = insecure_http_allowed();
    let host = url
        .host_str()
        .ok_or_else(|| McpGatewayStoreError::Conflict("endpoint URL requires a hostname".into()))?
        .to_string();
    let port = url
        .port_or_known_default()
        .ok_or_else(|| McpGatewayStoreError::Conflict("endpoint URL requires a port".into()))?;
    let addresses = tokio::net::lookup_host((host.as_str(), port))
        .await
        .map_err(|_| safe_upstream("upstream hostname could not be resolved"))?
        .collect::<Vec<_>>();
    if addresses.is_empty() {
        return Err(safe_upstream("upstream hostname could not be resolved"));
    }
    if addresses
        .iter()
        .any(|address| !endpoint_address_allowed(url.scheme(), allow_http, address.ip()))
    {
        return Err(McpGatewayStoreError::Conflict(
            "endpoint resolves to a non-public network".into(),
        ));
    }
    Ok((url, host, addresses))
}

pub(super) fn validate_endpoint_url(endpoint: &str) -> Result<Url, &'static str> {
    let url = Url::parse(endpoint).map_err(|_| "endpoint URL is invalid")?;
    if !url.username().is_empty()
        || url.password().is_some()
        || url.fragment().is_some()
        || url.query().is_some()
    {
        return Err("endpoint URL cannot contain credentials, query, or fragment");
    }
    let loopback_http = url.scheme() == "http"
        && insecure_http_allowed()
        && matches!(
            url.host_str(),
            Some("localhost") | Some("127.0.0.1") | Some("::1")
        );
    if url.scheme() != "https" && !loopback_http {
        return Err("endpoint URL must use HTTPS");
    }
    if url.host_str().is_none() {
        return Err("endpoint URL requires a hostname");
    }
    Ok(url)
}

fn insecure_http_allowed() -> bool {
    std::env::var("TL_MCP_GATEWAY_ALLOW_INSECURE_HTTP")
        .map(|value| value == "1" || value.eq_ignore_ascii_case("true"))
        .unwrap_or(false)
}

pub(super) fn endpoint_address_allowed(scheme: &str, allow_http: bool, ip: IpAddr) -> bool {
    match scheme {
        "https" => is_public_ip(ip),
        "http" => allow_http && ip.is_loopback(),
        _ => false,
    }
}

pub(super) fn catalog_page_fits(current: usize, page: usize) -> bool {
    current
        .checked_add(page)
        .is_some_and(|total| total <= MAX_TOOLS)
}

fn is_public_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(ip) => is_public_ipv4(ip),
        IpAddr::V6(ip) => ip
            .to_ipv4_mapped()
            .map(is_public_ipv4)
            .unwrap_or_else(|| is_public_ipv6(ip)),
    }
}

fn is_public_ipv4(ip: Ipv4Addr) -> bool {
    let [a, b, c, _] = ip.octets();
    !(a == 0
        || a == 10
        || a == 127
        || a >= 224
        || (a == 100 && (64..=127).contains(&b))
        || (a == 169 && b == 254)
        || (a == 172 && (16..=31).contains(&b))
        || (a == 192 && (b == 168 || (b == 0 && c <= 2)))
        || (a == 198 && (b == 18 || b == 19 || (b == 51 && c == 100)))
        || (a == 203 && b == 0 && c == 113))
}

fn is_public_ipv6(ip: Ipv6Addr) -> bool {
    let segments = ip.segments();
    !(ip.is_unspecified()
        || ip.is_loopback()
        || ip.is_multicast()
        || segments[0] & 0xfe00 == 0xfc00
        || segments[0] & 0xffc0 == 0xfe80
        || segments[0] == 0x2001 && segments[1] == 0x0db8)
}

fn safe_upstream(message: &str) -> McpGatewayStoreError {
    McpGatewayStoreError::Internal(message.into())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_private_and_metadata_ranges() {
        for ip in [
            "10.0.0.1",
            "169.254.169.254",
            "192.168.1.1",
            "::1",
            "fc00::1",
            "2001:db8::1",
        ] {
            assert!(!is_public_ip(ip.parse().unwrap()), "{ip}");
        }
    }
    #[test]
    fn accepts_public_addresses() {
        assert!(is_public_ip("8.8.8.8".parse().unwrap()));
        assert!(is_public_ip("2606:4700:4700::1111".parse().unwrap()));
    }
}
