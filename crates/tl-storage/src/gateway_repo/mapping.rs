use chrono::{DateTime, Utc};
use tl_core::{
    EnforcementProfile, FailMode, GatewayCredentialStatus, GatewayInputAction, GatewayOutputAction,
    GatewayProviderConnection, GatewayProviderKind, GatewayRoute, ResponseMode, RetentionMode,
};

use crate::{
    models::{EnforcementProfileRecord, GatewayProviderConnectionRecord, GatewayRouteRecord},
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

pub(super) fn profile_record_to_wire(
    row: EnforcementProfileRecord,
) -> Result<EnforcementProfile, StorageError> {
    Ok(EnforcementProfile {
        id: row.id,
        display_name: row.display_name,
        input_action: parse_input_action(&row.input_action)?,
        output_action: parse_output_action(&row.output_action)?,
        fail_mode: parse_fail_mode(&row.fail_mode)?,
        retention_mode: parse_retention_mode(&row.retention_mode)?,
        response_mode: parse_response_mode(&row.response_mode)?,
        fallback_message: row.fallback_message,
        max_regenerations: row.max_regenerations.max(0) as u32,
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
        enforcement_profile_id: row.enforcement_profile_id,
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

fn parse_input_action(value: &str) -> Result<GatewayInputAction, StorageError> {
    match value {
        "allow" => Ok(GatewayInputAction::Allow),
        "block" => Ok(GatewayInputAction::Block),
        "redact" => Ok(GatewayInputAction::Redact),
        other => Err(StorageError::Internal(format!(
            "unknown input action: {other}"
        ))),
    }
}

fn parse_output_action(value: &str) -> Result<GatewayOutputAction, StorageError> {
    match value {
        "allow" => Ok(GatewayOutputAction::Allow),
        "block" => Ok(GatewayOutputAction::Block),
        "rewrite" => Ok(GatewayOutputAction::Rewrite),
        "escalate" => Ok(GatewayOutputAction::Escalate),
        other => Err(StorageError::Internal(format!(
            "unknown output action: {other}"
        ))),
    }
}

fn parse_fail_mode(value: &str) -> Result<FailMode, StorageError> {
    match value {
        "open" => Ok(FailMode::Open),
        "closed" => Ok(FailMode::Closed),
        other => Err(StorageError::Internal(format!(
            "unknown fail mode: {other}"
        ))),
    }
}

fn parse_retention_mode(value: &str) -> Result<RetentionMode, StorageError> {
    match value {
        "metadata_only" => Ok(RetentionMode::MetadataOnly),
        "redacted_body" => Ok(RetentionMode::RedactedBody),
        "full_body" => Ok(RetentionMode::FullBody),
        other => Err(StorageError::Internal(format!(
            "unknown retention mode: {other}"
        ))),
    }
}

fn parse_response_mode(value: &str) -> Result<ResponseMode, StorageError> {
    match value {
        "regular" => Ok(ResponseMode::Regular),
        "streaming" => Ok(ResponseMode::Streaming),
        other => Err(StorageError::Internal(format!(
            "unknown response mode: {other}"
        ))),
    }
}
