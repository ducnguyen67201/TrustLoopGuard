//! Bounded OTLP/HTTP trace ingestion for Run-correlated agent evidence.

use std::collections::{BTreeMap, HashMap};
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use axum::{
    body::Bytes,
    extract::State,
    http::{header, HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use chrono::{DateTime, Utc};
use opentelemetry_proto::tonic::{
    collector::trace::v1::{
        ExportTracePartialSuccess, ExportTraceServiceRequest, ExportTraceServiceResponse,
    },
    common::v1::{any_value, AnyValue, KeyValue},
    trace::v1::Span,
};
use prost::Message;
use serde_json::{json, Value};
use tl_core::{ContentCaptureMode, DataHandlingMode, RunParticipantRole};

pub const ATTR_RUN_ID: &str = "featherlane.run.id";
pub const ATTR_AGENT_ID: &str = "featherlane.agent.id";
pub const ATTR_RUN_EVENT_ID: &str = "featherlane.run.event.id";
pub const ATTR_FLUSH_ID: &str = "featherlane.flush.id";
pub const ATTR_REDACTED: &str = "featherlane.content.redacted";
pub const ATTR_ARTIFACT_REF: &str = "featherlane.content.artifact_ref";
pub const ATTR_ARTIFACT_CHECKSUM: &str = "featherlane.content.artifact_checksum";

#[derive(Debug, Clone)]
pub struct NormalizedSpan {
    pub run_id: String,
    pub agent_id: String,
    pub run_event_id: Option<String>,
    pub flush_id: Option<String>,
    pub otel_trace_id: String,
    pub otel_span_id: String,
    pub parent_span_id: Option<String>,
    pub name: String,
    pub span_kind: i32,
    pub operation_name: Option<String>,
    pub conversation_id: Option<String>,
    pub external_agent_id: Option<String>,
    pub started_at: DateTime<Utc>,
    pub ended_at: DateTime<Utc>,
    pub status_code: i32,
    pub status_message: Option<String>,
    pub resource: Value,
    pub attributes: Value,
    pub events: Value,
    pub links: Value,
    pub content_capture_status: String,
    pub dropped_attribute_count: i32,
}

#[derive(Debug)]
pub struct IngestSpanBatch {
    pub workspace_id: String,
    pub environment_id: String,
    pub run_id: String,
    pub flush_id: Option<String>,
    pub rejected_span_count: i32,
    pub spans: Vec<NormalizedSpan>,
}

#[derive(Debug, Clone, Copy)]
pub struct IngestSpanResult {
    pub accepted_span_count: i32,
    pub late_span_count: i32,
}

#[derive(Debug, thiserror::Error)]
pub enum OtelStoreError {
    #[error("not found")]
    NotFound,
    #[error("conflict")]
    Conflict,
    #[error("internal: {0}")]
    Internal(String),
}

#[async_trait]
pub trait OtelStore: Send + Sync {
    async fn ingest(&self, batch: IngestSpanBatch) -> Result<IngestSpanResult, OtelStoreError>;
}

#[derive(Debug, Default)]
pub struct MemoryOtelStore {
    spans: Mutex<std::collections::HashSet<(String, String, String, String)>>,
}

#[async_trait]
impl OtelStore for MemoryOtelStore {
    async fn ingest(&self, batch: IngestSpanBatch) -> Result<IngestSpanResult, OtelStoreError> {
        let mut stored = self.spans.lock().expect("OTLP memory store lock");
        let mut accepted = 0;
        for span in batch.spans {
            if stored.insert((
                batch.workspace_id.clone(),
                batch.environment_id.clone(),
                span.otel_trace_id,
                span.otel_span_id,
            )) {
                accepted += 1;
            }
        }
        Ok(IngestSpanResult {
            accepted_span_count: accepted,
            late_span_count: 0,
        })
    }
}

#[derive(Debug, Clone, Copy)]
pub struct OtlpIngestConfig {
    pub max_body_bytes: usize,
    pub max_spans: usize,
    pub max_attributes: usize,
    pub max_events: usize,
    pub max_links: usize,
    pub max_value_bytes: usize,
}

impl Default for OtlpIngestConfig {
    fn default() -> Self {
        Self {
            max_body_bytes: 4 * 1024 * 1024,
            max_spans: 10_000,
            max_attributes: 128,
            max_events: 128,
            max_links: 128,
            max_value_bytes: 16 * 1024,
        }
    }
}

#[derive(Clone)]
pub struct OtelState {
    pub store: Arc<dyn OtelStore>,
    pub run_store: Arc<dyn crate::runs::RunStore>,
    pub evaluation_store: Arc<dyn crate::evaluations::EvaluationStore>,
    pub environment_store: Arc<dyn crate::environments::EnvironmentStore>,
    pub settings_store: Arc<dyn crate::dashboard_admin::SettingsStore>,
    pub config: OtlpIngestConfig,
}

pub async fn export_traces(
    State(state): State<OtelState>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    if body.len() > state.config.max_body_bytes {
        return api_error(
            StatusCode::PAYLOAD_TOO_LARGE,
            "OTLP body exceeds configured limit",
        );
    }
    let workspace_id = match crate::policies::workspace_id_from_headers(&headers) {
        Ok(workspace_id) => workspace_id,
        Err(response) => return response,
    };
    let environment_id = match crate::environments::resolve_environment_id(
        &headers,
        state.environment_store.as_ref(),
        &workspace_id,
    )
    .await
    {
        Ok(environment_id) => environment_id,
        Err(error) => return crate::environments::environment_error_response(error),
    };
    let settings = match state.settings_store.get(&workspace_id).await {
        Ok(settings) => settings,
        Err(error) => {
            tracing::error!(workspace_id, error = %error, "OTLP workspace settings lookup failed");
            return api_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "workspace settings unavailable",
            );
        }
    };
    let request = match ExportTraceServiceRequest::decode(body) {
        Ok(request) => request,
        Err(error) => {
            return api_error(
                StatusCode::BAD_REQUEST,
                &format!("invalid OTLP protobuf: {error}"),
            )
        }
    };

    let mut rejected = 0_i64;
    let mut normalized = Vec::new();
    let mut seen = 0_usize;
    for resource_spans in request.resource_spans {
        let resource_attributes = resource_spans
            .resource
            .map(|resource| resource.attributes)
            .unwrap_or_default();
        for scope_spans in resource_spans.scope_spans {
            let scope = scope_spans.scope.as_ref().map(|scope| {
                json!({
                    "name": truncate(&scope.name, state.config.max_value_bytes),
                    "version": truncate(&scope.version, state.config.max_value_bytes),
                })
            });
            for span in scope_spans.spans {
                seen += 1;
                if seen > state.config.max_spans {
                    rejected += 1;
                    continue;
                }
                let run_id = string_attribute(&span.attributes, ATTR_RUN_ID)
                    .or_else(|| string_attribute(&resource_attributes, ATTR_RUN_ID))
                    .map(str::to_string);
                let agent_id = string_attribute(&span.attributes, ATTR_AGENT_ID)
                    .or_else(|| string_attribute(&resource_attributes, ATTR_AGENT_ID))
                    .map(str::to_string);
                let Some(run_id) = run_id else {
                    rejected += 1;
                    continue;
                };
                let Some(agent_id) = agent_id else {
                    rejected += 1;
                    continue;
                };
                let profile = state
                    .evaluation_store
                    .get_profile(&workspace_id, &environment_id, &agent_id)
                    .await
                    .ok()
                    .flatten();
                let content_mode = effective_content_mode(
                    settings.data_handling_mode,
                    profile.map_or(ContentCaptureMode::MetadataOnly, |profile| {
                        profile.content_mode
                    }),
                );
                match normalize_span(
                    span,
                    &resource_attributes,
                    scope.clone(),
                    content_mode,
                    state.config,
                ) {
                    Ok(span) if span.run_id == run_id && span.agent_id == agent_id => {
                        normalized.push(span)
                    }
                    _ => rejected += 1,
                }
            }
        }
    }

    let mut grouped: HashMap<(String, String, Option<String>), Vec<NormalizedSpan>> =
        HashMap::new();
    for span in normalized {
        let flush_id = span.flush_id.clone();
        grouped
            .entry((span.run_id.clone(), span.agent_id.clone(), flush_id))
            .or_default()
            .push(span);
    }

    let mut storage_failed = false;
    for ((run_id, agent_id, flush_id), spans) in grouped {
        let run = match state
            .run_store
            .get(&workspace_id, &environment_id, &run_id)
            .await
        {
            Ok(run) => run,
            Err(_) => {
                rejected += spans.len() as i64;
                continue;
            }
        };
        let role = if run.agent_id == agent_id {
            RunParticipantRole::Primary
        } else {
            RunParticipantRole::Participant
        };
        if state
            .evaluation_store
            .register_participant_and_freeze_manifest(
                &workspace_id,
                &environment_id,
                &run_id,
                &agent_id,
                role,
            )
            .await
            .is_err()
        {
            rejected += spans.len() as i64;
            continue;
        }
        let span_count = spans.len();
        match state
            .store
            .ingest(IngestSpanBatch {
                workspace_id: workspace_id.clone(),
                environment_id: environment_id.clone(),
                run_id,
                flush_id,
                rejected_span_count: i32::try_from(rejected).unwrap_or(i32::MAX),
                spans,
            })
            .await
        {
            Ok(result) => {
                tracing::debug!(
                    accepted = result.accepted_span_count,
                    late = result.late_span_count,
                    "OTLP span batch committed"
                );
            }
            Err(error) => {
                tracing::error!(error = %error, "OTLP span commit failed");
                storage_failed = true;
                rejected += span_count as i64;
            }
        }
    }

    if storage_failed {
        return api_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "OTLP evidence storage is temporarily unavailable",
        );
    }

    let partial_success = (rejected > 0).then(|| ExportTracePartialSuccess {
        rejected_spans: rejected,
        error_message: "one or more spans were rejected by correlation, size, or storage checks"
            .into(),
    });
    let response = ExportTraceServiceResponse { partial_success }.encode_to_vec();
    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "application/x-protobuf")],
        response,
    )
        .into_response()
}

fn normalize_span(
    span: Span,
    resource_attributes: &[KeyValue],
    scope: Option<Value>,
    content_mode: ContentCaptureMode,
    config: OtlpIngestConfig,
) -> Result<NormalizedSpan, ()> {
    if span.attributes.len() > config.max_attributes
        || span.events.len() > config.max_events
        || span.links.len() > config.max_links
        || span.name.len() > config.max_value_bytes
    {
        return Err(());
    }
    let run_id = required_attribute(&span.attributes, resource_attributes, ATTR_RUN_ID)?;
    let agent_id = required_attribute(&span.attributes, resource_attributes, ATTR_AGENT_ID)?;
    uuid::Uuid::parse_str(&run_id).map_err(|_| ())?;
    let run_event_id = optional_attribute(&span.attributes, resource_attributes, ATTR_RUN_EVENT_ID);
    let flush_id = optional_attribute(&span.attributes, resource_attributes, ATTR_FLUSH_ID);
    if run_event_id
        .as_deref()
        .is_some_and(|value| uuid::Uuid::parse_str(value).is_err())
    {
        return Err(());
    }
    let otel_trace_id = fixed_hex(&span.trace_id, 16)?;
    let otel_span_id = fixed_hex(&span.span_id, 8)?;
    let parent_span_id = if span.parent_span_id.is_empty() {
        None
    } else {
        Some(fixed_hex(&span.parent_span_id, 8)?)
    };
    let started_at = timestamp(span.start_time_unix_nano)?;
    let ended_at = timestamp(span.end_time_unix_nano)?;
    if ended_at < started_at {
        return Err(());
    }
    let (attributes, dropped_attributes, content_capture_status) =
        normalize_attributes(&span.attributes, content_mode, config.max_value_bytes)?;
    let (resource, resource_dropped, _) = normalize_attributes(
        resource_attributes,
        ContentCaptureMode::MetadataOnly,
        config.max_value_bytes,
    )?;
    let events = span
        .events
        .into_iter()
        .map(|event| {
            let (attributes, _, _) = normalize_attributes(
                &event.attributes,
                ContentCaptureMode::MetadataOnly,
                config.max_value_bytes,
            )?;
            Ok(json!({
                "time_unix_nano": event.time_unix_nano.to_string(),
                "name": truncate(&event.name, config.max_value_bytes),
                "attributes": attributes,
            }))
        })
        .collect::<Result<Vec<_>, ()>>()?;
    let links = span
        .links
        .into_iter()
        .take(config.max_links)
        .map(|link| {
            Ok(json!({
                "trace_id": fixed_hex(&link.trace_id, 16)?,
                "span_id": fixed_hex(&link.span_id, 8)?,
            }))
        })
        .collect::<Result<Vec<_>, ()>>()?;
    let status_code = span.status.as_ref().map_or(0, |status| status.code);
    let status_message = span
        .status
        .and_then(|status| (!status.message.is_empty()).then_some(status.message));
    Ok(NormalizedSpan {
        run_id,
        agent_id,
        run_event_id,
        flush_id,
        otel_trace_id,
        otel_span_id,
        parent_span_id,
        name: span.name,
        span_kind: span.kind,
        operation_name: string_value(&attributes, "gen_ai.operation.name"),
        conversation_id: string_value(&attributes, "gen_ai.conversation.id"),
        external_agent_id: string_value(&attributes, "gen_ai.agent.id"),
        started_at,
        ended_at,
        status_code,
        status_message,
        resource: json!({ "attributes": resource, "scope": scope }),
        attributes,
        events: Value::Array(events),
        links: Value::Array(links),
        content_capture_status,
        dropped_attribute_count: i32::try_from(
            span.dropped_attributes_count as usize + dropped_attributes + resource_dropped,
        )
        .unwrap_or(i32::MAX),
    })
}

fn normalize_attributes(
    attributes: &[KeyValue],
    content_mode: ContentCaptureMode,
    max_value_bytes: usize,
) -> Result<(Value, usize, String), ()> {
    let redacted = bool_attribute(attributes, ATTR_REDACTED).unwrap_or(false);
    let mut output = BTreeMap::new();
    let mut dropped = 0;
    for attribute in attributes {
        if attribute.key.len() > 256 {
            dropped += 1;
            continue;
        }
        let sensitive = is_sensitive_key(&attribute.key);
        let allowed = if !sensitive {
            true
        } else {
            match content_mode {
                ContentCaptureMode::MetadataOnly => false,
                ContentCaptureMode::Redacted => redacted,
                ContentCaptureMode::EncryptedArtifactRef => matches!(
                    attribute.key.as_str(),
                    ATTR_ARTIFACT_REF | ATTR_ARTIFACT_CHECKSUM
                ),
            }
        };
        if !allowed {
            dropped += 1;
            continue;
        }
        let value = attribute
            .value
            .as_ref()
            .map(|value| any_value_to_json(value, 0, max_value_bytes))
            .transpose()?
            .unwrap_or(Value::Null);
        output.insert(attribute.key.clone(), value);
    }
    let status = match content_mode {
        ContentCaptureMode::MetadataOnly => "omitted_by_policy",
        ContentCaptureMode::Redacted if redacted => "redacted",
        ContentCaptureMode::Redacted => "missing_redaction_evidence",
        ContentCaptureMode::EncryptedArtifactRef => "encrypted_artifact_ref",
    };
    Ok((
        serde_json::to_value(output).map_err(|_| ())?,
        dropped,
        status.into(),
    ))
}

fn any_value_to_json(value: &AnyValue, depth: usize, max_value_bytes: usize) -> Result<Value, ()> {
    if depth > 3 {
        return Err(());
    }
    match value.value.as_ref() {
        None => Ok(Value::Null),
        Some(any_value::Value::StringValue(value)) => {
            if value.len() > max_value_bytes {
                Err(())
            } else {
                Ok(Value::String(value.clone()))
            }
        }
        Some(any_value::Value::BoolValue(value)) => Ok(Value::Bool(*value)),
        Some(any_value::Value::IntValue(value)) => Ok(json!(value)),
        Some(any_value::Value::DoubleValue(value)) if value.is_finite() => Ok(json!(value)),
        Some(any_value::Value::DoubleValue(_)) => Err(()),
        Some(any_value::Value::BytesValue(value)) => {
            if value.len() > max_value_bytes {
                Err(())
            } else {
                Ok(Value::String(base64::Engine::encode(
                    &base64::engine::general_purpose::STANDARD,
                    value,
                )))
            }
        }
        Some(any_value::Value::ArrayValue(values)) if values.values.len() <= 128 => {
            Ok(Value::Array(
                values
                    .values
                    .iter()
                    .map(|value| any_value_to_json(value, depth + 1, max_value_bytes))
                    .collect::<Result<Vec<_>, _>>()?,
            ))
        }
        Some(any_value::Value::KvlistValue(values)) if values.values.len() <= 128 => {
            let mut object = serde_json::Map::new();
            for item in &values.values {
                object.insert(
                    item.key.clone(),
                    item.value
                        .as_ref()
                        .map(|value| any_value_to_json(value, depth + 1, max_value_bytes))
                        .transpose()?
                        .unwrap_or(Value::Null),
                );
            }
            Ok(Value::Object(object))
        }
        _ => Err(()),
    }
}

fn effective_content_mode(
    workspace_mode: DataHandlingMode,
    profile_mode: ContentCaptureMode,
) -> ContentCaptureMode {
    match workspace_mode {
        DataHandlingMode::NoBodyRetention => ContentCaptureMode::MetadataOnly,
        DataHandlingMode::RedactedOnly => match profile_mode {
            ContentCaptureMode::EncryptedArtifactRef => ContentCaptureMode::EncryptedArtifactRef,
            _ => ContentCaptureMode::Redacted,
        },
        DataHandlingMode::RawAllowed | DataHandlingMode::PrivateDeployment => profile_mode,
    }
}

fn required_attribute(span: &[KeyValue], resource: &[KeyValue], key: &str) -> Result<String, ()> {
    optional_attribute(span, resource, key).ok_or(())
}

fn optional_attribute(span: &[KeyValue], resource: &[KeyValue], key: &str) -> Option<String> {
    string_attribute(span, key)
        .or_else(|| string_attribute(resource, key))
        .map(str::to_string)
}

fn string_attribute<'a>(attributes: &'a [KeyValue], key: &str) -> Option<&'a str> {
    attributes.iter().find_map(|attribute| {
        if attribute.key != key {
            return None;
        }
        match attribute.value.as_ref()?.value.as_ref()? {
            any_value::Value::StringValue(value) => Some(value.as_str()),
            _ => None,
        }
    })
}

fn bool_attribute(attributes: &[KeyValue], key: &str) -> Option<bool> {
    attributes.iter().find_map(|attribute| {
        if attribute.key != key {
            return None;
        }
        match attribute.value.as_ref()?.value.as_ref()? {
            any_value::Value::BoolValue(value) => Some(*value),
            _ => None,
        }
    })
}

fn string_value(value: &Value, key: &str) -> Option<String> {
    value.get(key).and_then(Value::as_str).map(str::to_string)
}

fn is_sensitive_key(key: &str) -> bool {
    let key = key.to_ascii_lowercase();
    key.contains("prompt")
        || key.contains("completion")
        || key.contains("tool.arguments")
        || key.ends_with(".body")
        || key.ends_with(".content")
        || key == ATTR_ARTIFACT_REF
        || key == ATTR_ARTIFACT_CHECKSUM
}

fn fixed_hex(value: &[u8], expected_len: usize) -> Result<String, ()> {
    if value.len() != expected_len || value.iter().all(|value| *value == 0) {
        return Err(());
    }
    Ok(value.iter().map(|value| format!("{value:02x}")).collect())
}

fn timestamp(nanos: u64) -> Result<DateTime<Utc>, ()> {
    let seconds = i64::try_from(nanos / 1_000_000_000).map_err(|_| ())?;
    let subsec = u32::try_from(nanos % 1_000_000_000).map_err(|_| ())?;
    DateTime::from_timestamp(seconds, subsec).ok_or(())
}

fn truncate(value: &str, max_bytes: usize) -> String {
    if value.len() <= max_bytes {
        return value.to_string();
    }
    value.chars().take(max_bytes).collect()
}

fn api_error(status: StatusCode, message: &str) -> Response {
    let code = tl_core::ApiErrorCode::from_http_status(status.as_u16());
    (
        status,
        axum::Json(tl_core::ApiError {
            code,
            message: message.to_string(),
            retriable: code.default_retriable(),
            details: Value::Null,
        }),
    )
        .into_response()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn string_kv(key: &str, value: &str) -> KeyValue {
        KeyValue {
            key: key.into(),
            value: Some(AnyValue {
                value: Some(any_value::Value::StringValue(value.into())),
            }),
            ..KeyValue::default()
        }
    }

    fn span(attributes: Vec<KeyValue>) -> Span {
        Span {
            trace_id: vec![1; 16],
            span_id: vec![2; 8],
            name: "agent turn".into(),
            start_time_unix_nano: 1_700_000_000_000_000_000,
            end_time_unix_nano: 1_700_000_000_100_000_000,
            attributes,
            ..Span::default()
        }
    }

    #[test]
    fn metadata_only_strips_prompt_content() {
        let normalized = normalize_span(
            span(vec![
                string_kv(ATTR_RUN_ID, "018f1111-1111-7111-8111-111111111111"),
                string_kv(ATTR_AGENT_ID, "agent-a"),
                string_kv("gen_ai.prompt", "private prompt"),
                string_kv("gen_ai.operation.name", "chat"),
            ]),
            &[],
            None,
            ContentCaptureMode::MetadataOnly,
            OtlpIngestConfig::default(),
        )
        .expect("normalized span");
        assert!(normalized.attributes.get("gen_ai.prompt").is_none());
        assert_eq!(normalized.operation_name.as_deref(), Some("chat"));
        assert_eq!(normalized.content_capture_status, "omitted_by_policy");
    }

    #[test]
    fn resource_correlation_carries_flush_receipt_without_authorizing_tenant() {
        let resource = vec![
            string_kv(ATTR_RUN_ID, "018f1111-1111-7111-8111-111111111111"),
            string_kv(ATTR_AGENT_ID, "agent-a"),
            string_kv(ATTR_FLUSH_ID, "flush-a"),
        ];
        let normalized = normalize_span(
            span(Vec::new()),
            &resource,
            None,
            ContentCaptureMode::MetadataOnly,
            OtlpIngestConfig::default(),
        )
        .expect("resource-correlated span");
        assert_eq!(normalized.flush_id.as_deref(), Some("flush-a"));
        assert_eq!(normalized.agent_id, "agent-a");
    }
}
