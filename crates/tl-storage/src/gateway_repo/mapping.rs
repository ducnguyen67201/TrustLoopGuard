use chrono::{DateTime, Utc};
use tl_core::{
    GatewayCredentialStatus, GatewayProviderConnection, GatewayProviderKind, GatewayRoute,
};

use crate::{
    models::{GatewayProviderConnectionRecord, GatewayRouteRecord},
    StorageError,
};

pub(super) fn provider_record_to_wire(
    row: GatewayProviderConnectionRecord,
) -> Result<GatewayProviderConnection, StorageError> {
    Ok(GatewayProviderConnection {
        id: row.id,
        display_name: row.display_name,
        kind: parse_provider_kind(&row.kind)?,
        base_url: row.base_url,
        default_model: row.default_model,
        credential_status: GatewayCredentialStatus::Configured,
        created_at: to_rfc3339(row.created_at),
        updated_at: to_rfc3339(row.updated_at),
    })
}

pub(super) fn route_record_to_wire(row: GatewayRouteRecord) -> GatewayRoute {
    GatewayRoute {
        id: row.id,
        display_name: row.display_name,
        provider_connection_id: row.provider_connection_id,
        agent_id: row.agent_id,
        created_at: to_rfc3339(row.created_at),
        updated_at: to_rfc3339(row.updated_at),
    }
}

fn to_rfc3339(value: DateTime<Utc>) -> String {
    value.to_rfc3339()
}

fn parse_provider_kind(value: &str) -> Result<GatewayProviderKind, StorageError> {
    match value {
        "openai_compatible" => Ok(GatewayProviderKind::OpenaiCompatible),
        "anthropic" => Ok(GatewayProviderKind::Anthropic),
        "payment_http" => Ok(GatewayProviderKind::PaymentHttp),
        other => Err(StorageError::Internal(format!(
            "unknown provider kind: {other}"
        ))),
    }
}
