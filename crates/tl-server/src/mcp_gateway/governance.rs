use std::collections::BTreeSet;

use rmcp::model::CallToolResult;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};

pub(super) const RESERVED_FIELD: &str = "__featherlane_ai";
pub(super) const MAX_ARGUMENT_BYTES: usize = 64 * 1024;
pub(super) const MAX_RESULT_BYTES: usize = 1024 * 1024;
pub(super) const MAX_POLICY_TEXT_BYTES: usize = 128 * 1024;
const MAX_INTENT_CHARS: usize = 8192;
const MAX_DESTINATION_CHARS: usize = 2048;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum GovernancePurpose {
    AnswerUser,
    Analysis,
    Automation,
    ModelTraining,
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct GovernanceContext {
    pub user_intent: String,
    pub purpose: GovernancePurpose,
    pub destination: Option<String>,
}

impl GovernanceContext {
    pub(super) fn policy_parameters(&self, mut upstream: Map<String, Value>) -> Value {
        let mut governance = Map::new();
        governance.insert(
            "policy_text".into(),
            Value::String(self.user_intent.clone()),
        );
        governance.insert(
            "purpose".into(),
            serde_json::to_value(self.purpose).expect("purpose serializes"),
        );
        if let Some(destination) = &self.destination {
            governance.insert("destination".into(), Value::String(destination.clone()));
        }
        upstream.insert(RESERVED_FIELD.into(), Value::Object(governance));
        Value::Object(upstream)
    }
}

#[derive(Debug, Clone)]
pub(super) struct GovernedArguments {
    pub context: GovernanceContext,
    pub upstream: Map<String, Value>,
}

#[derive(Debug, Clone)]
pub(super) struct GovernedResult {
    pub policy_text: String,
    pub result_bytes: usize,
    pub digest: String,
    pub content_types: Vec<String>,
    pub text_only: bool,
}

pub(super) fn governed_input_schema(original: &Value) -> Result<Value, String> {
    let mut schema = original
        .as_object()
        .cloned()
        .ok_or_else(|| "The pinned input schema is not an object".to_string())?;
    let properties = schema
        .entry("properties")
        .or_insert_with(|| Value::Object(Map::new()))
        .as_object_mut()
        .ok_or_else(|| "The pinned input schema has invalid properties".to_string())?;
    if properties.contains_key(RESERVED_FIELD) {
        return Err("The upstream tool uses the reserved __featherlane_ai argument".into());
    }
    properties.insert(RESERVED_FIELD.into(), governance_schema());
    let required = schema
        .entry("required")
        .or_insert_with(|| Value::Array(Vec::new()))
        .as_array_mut()
        .ok_or_else(|| "The pinned input schema has invalid required fields".to_string())?;
    if !required
        .iter()
        .any(|value| value.as_str() == Some(RESERVED_FIELD))
    {
        required.push(Value::String(RESERVED_FIELD.into()));
    }
    Ok(Value::Object(schema))
}

pub(super) fn split_governance_arguments(
    original_schema: &Value,
    arguments: &Map<String, Value>,
) -> Result<GovernedArguments, String> {
    let full = Value::Object(arguments.clone());
    if serde_json::to_vec(&full)
        .map_err(|_| "Tool arguments are invalid".to_string())?
        .len()
        > MAX_ARGUMENT_BYTES
    {
        return Err("Tool arguments exceed 64 KiB".into());
    }
    let public_schema = governed_input_schema(original_schema)?;
    validate(
        &public_schema,
        &full,
        "Tool arguments do not match the managed schema",
    )?;

    let mut upstream = arguments.clone();
    let raw_context = upstream
        .remove(RESERVED_FIELD)
        .ok_or_else(|| "Managed policy context is required".to_string())?;
    let context: GovernanceContext = serde_json::from_value(raw_context)
        .map_err(|_| "Managed policy context is invalid".to_string())?;
    validate(
        original_schema,
        &Value::Object(upstream.clone()),
        "Tool arguments do not match the assigned schema",
    )?;
    Ok(GovernedArguments { context, upstream })
}

pub(super) fn extract_result_policy_text(
    result: &CallToolResult,
    output_schema: Option<&Value>,
) -> Result<GovernedResult, String> {
    let bytes = serde_json::to_vec(result)
        .map_err(|_| "The upstream result could not be serialized".to_string())?;
    if bytes.len() > MAX_RESULT_BYTES {
        return Err("The upstream tool result exceeded the 1 MiB response limit".into());
    }
    match (output_schema, result.structured_content.as_ref()) {
        (Some(schema), Some(structured)) => validate(
            schema,
            structured,
            "The upstream result did not match the approved output schema",
        )?,
        (Some(_), None) => {
            return Err("The upstream result omitted its required structured content".into())
        }
        (None, _) => {}
    }

    let serialized = serde_json::to_value(result)
        .map_err(|_| "The upstream result could not be inspected".to_string())?;
    let content = serialized
        .get("content")
        .and_then(Value::as_array)
        .ok_or_else(|| "The upstream result content is invalid".to_string())?;
    let mut segments = Vec::new();
    let mut seen = BTreeSet::new();
    let mut content_types = BTreeSet::new();
    let mut text_only = result.structured_content.is_none();
    for block in content {
        let kind = block
            .get("type")
            .and_then(Value::as_str)
            .ok_or_else(|| "The upstream result contains an unknown content block".to_string())?;
        content_types.insert(kind.to_string());
        match kind {
            "text" => push_segment(
                &mut segments,
                &mut seen,
                block
                    .get("text")
                    .and_then(Value::as_str)
                    .ok_or_else(|| "The upstream text result is invalid".to_string())?
                    .to_string(),
            ),
            "resource" => {
                text_only = false;
                let resource = block
                    .get("resource")
                    .and_then(Value::as_object)
                    .ok_or_else(|| "The upstream resource result is invalid".to_string())?;
                if resource.get("blob").is_some() {
                    return Err("Binary embedded resources cannot be policy evaluated".into());
                }
                let text = resource
                    .get("text")
                    .and_then(Value::as_str)
                    .ok_or_else(|| {
                        "The upstream resource result is not inspectable text".to_string()
                    })?;
                let uri = resource
                    .get("uri")
                    .and_then(Value::as_str)
                    .unwrap_or_default();
                let mime = resource
                    .get("mimeType")
                    .and_then(Value::as_str)
                    .unwrap_or("text/plain");
                push_segment(
                    &mut segments,
                    &mut seen,
                    format!("Resource: {uri}\nMIME: {mime}\n{text}"),
                );
            }
            "resource_link" => {
                text_only = false;
                let metadata = ["uri", "name", "title", "description", "mimeType"]
                    .into_iter()
                    .filter_map(|key| {
                        block
                            .get(key)
                            .and_then(Value::as_str)
                            .map(|value| format!("{key}: {value}"))
                    })
                    .collect::<Vec<_>>()
                    .join("\n");
                push_segment(&mut segments, &mut seen, metadata);
            }
            "image" | "audio" => {
                return Err(format!(
                    "The upstream {kind} result cannot be policy evaluated"
                ))
            }
            _ => return Err("The upstream result contains an unsupported content block".into()),
        }
    }
    if let Some(structured) = &result.structured_content {
        text_only = false;
        push_segment(
            &mut segments,
            &mut seen,
            crate::authorization::canonical_json(structured),
        );
        content_types.insert("structured_content".into());
    }
    let policy_text = segments.join("\n\n");
    if policy_text.len() > MAX_POLICY_TEXT_BYTES {
        return Err("The inspectable upstream result exceeded 128 KiB".into());
    }
    let digest = Sha256::digest(&bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    Ok(GovernedResult {
        policy_text,
        result_bytes: bytes.len(),
        digest: format!("sha256:v1:{digest}"),
        content_types: content_types.into_iter().collect(),
        text_only,
    })
}

pub(super) fn managed_description(description: Option<&str>) -> String {
    let instruction = "Managed by Featherlane AI. Every call must include __featherlane_ai.user_intent (the latest user instruction), __featherlane_ai.purpose, and optional __featherlane_ai.destination. Featherlane AI removes this object before forwarding the call.";
    match description.map(str::trim).filter(|value| !value.is_empty()) {
        Some(description) => format!("{description}\n\n{instruction}"),
        None => instruction.to_string(),
    }
}

fn governance_schema() -> Value {
    serde_json::json!({
        "type": "object",
        "additionalProperties": false,
        "required": ["user_intent", "purpose"],
        "properties": {
            "user_intent": {
                "type": "string",
                "minLength": 1,
                "maxLength": MAX_INTENT_CHARS,
                "pattern": "\\S",
                "description": "Verbatim latest user instruction that caused this tool call."
            },
            "purpose": {
                "type": "string",
                "enum": ["answer_user", "analysis", "automation", "model_training", "other"]
            },
            "destination": {
                "type": "string",
                "minLength": 1,
                "maxLength": MAX_DESTINATION_CHARS,
                "pattern": "\\S"
            }
        }
    })
}

fn validate(schema: &Value, value: &Value, message: &str) -> Result<(), String> {
    let validator = jsonschema::JSONSchema::compile(schema)
        .map_err(|_| "The pinned schema is invalid".to_string())?;
    if validator.is_valid(value) {
        Ok(())
    } else {
        Err(message.to_string())
    }
}

fn push_segment(segments: &mut Vec<String>, seen: &mut BTreeSet<String>, value: String) {
    if !value.is_empty() && seen.insert(value.clone()) {
        segments.push(value);
    }
}

#[cfg(test)]
mod tests {
    use rmcp::model::{CallToolResult, ContentBlock};

    use super::*;

    fn schema() -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {"query": {"type": "string"}},
            "required": ["query"],
            "additionalProperties": false
        })
    }

    #[test]
    fn managed_schema_and_split_preserve_upstream_contract() {
        let public = governed_input_schema(&schema()).unwrap();
        assert!(public["required"]
            .as_array()
            .unwrap()
            .iter()
            .any(|value| value == RESERVED_FIELD));
        let args = serde_json::json!({
            "query": "customers",
            "__featherlane_ai": {
                "user_intent": "List active customers",
                "purpose": "answer_user"
            }
        })
        .as_object()
        .unwrap()
        .clone();
        let split = split_governance_arguments(&schema(), &args).unwrap();
        assert_eq!(
            split.upstream,
            serde_json::json!({"query":"customers"})
                .as_object()
                .unwrap()
                .clone()
        );
        assert_eq!(split.context.purpose, GovernancePurpose::AnswerUser);
    }

    #[test]
    fn reserved_schema_collision_is_rejected() {
        let collision = serde_json::json!({
            "type":"object",
            "properties":{"__featherlane_ai":{"type":"string"}}
        });
        assert!(governed_input_schema(&collision).is_err());
    }

    #[test]
    fn text_result_is_inspectable_and_binary_result_is_not() {
        let result = CallToolResult::success(vec![ContentBlock::text("customer data")]);
        let governed = extract_result_policy_text(&result, None).unwrap();
        assert_eq!(governed.policy_text, "customer data");
        assert!(governed.text_only);

        let binary = CallToolResult::success(vec![ContentBlock::image("AA==", "image/png")]);
        assert!(extract_result_policy_text(&binary, None).is_err());
    }
}
