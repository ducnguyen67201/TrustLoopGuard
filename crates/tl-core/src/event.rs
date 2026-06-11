use serde::{Deserialize, Serialize};

use crate::{CheckerRun, LabelResolution, ProvenanceMap, SignalEvidence, Source, ToolResolution};

#[cfg(feature = "schema")]
use schemars::JsonSchema;
#[cfg(feature = "ts-export")]
use ts_rs::TS;
#[cfg(feature = "openapi")]
use utoipa::ToSchema;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct GuardEvent {
    pub kind: EventKind,
    pub principal: Principal,
    pub action: Action,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sources: Vec<Source>,
    #[serde(default, skip_serializing_if = "ProvenanceMap::is_empty")]
    pub provenance: ProvenanceMap,
    /// Registry resolution evidence attached by the event pipeline.
    /// `None` until the pipeline has run.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub resolution: Option<ToolResolution>,
    /// Label resolution evidence attached by the event pipeline.
    /// `None` until the pipeline has run.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub label_resolution: Option<LabelResolution>,
    /// Checker evaluation evidence attached by the event pipeline.
    /// Server-populated: the pipeline resets this before evaluating, so
    /// collector-submitted values never survive.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts-export", ts(as = "Option<Vec<CheckerRun>>", optional))]
    pub checks: Vec<CheckerRun>,
    /// Advisory signal evidence attached by the event pipeline.
    /// Server-populated: the pipeline resets this before evaluating, so
    /// collector-submitted values never survive. Signals never change
    /// the decision.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(
        feature = "ts-export",
        ts(as = "Option<Vec<SignalEvidence>>", optional)
    )]
    pub signals: Vec<SignalEvidence>,
    #[serde(default)]
    #[cfg_attr(feature = "ts-export", ts(type = "Record<string, unknown> | null"))]
    pub context: serde_json::Value,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub enum EventKind {
    #[serde(rename = "output.proposed")]
    OutputProposed,
    #[serde(rename = "tool.call.proposed")]
    ToolCallProposed,
    #[serde(rename = "memory.write.proposed")]
    MemoryWriteProposed,
    #[serde(rename = "memory.retrieval.used_for_action")]
    MemoryRetrievalUsedForAction,
    #[serde(rename = "file.action.proposed")]
    FileActionProposed,
    #[serde(rename = "shell.action.proposed")]
    ShellActionProposed,
    #[serde(rename = "network.request.proposed")]
    NetworkRequestProposed,
    #[serde(rename = "browser.action.proposed")]
    BrowserActionProposed,
    #[serde(rename = "database.mutation.proposed")]
    DatabaseMutationProposed,
    #[serde(rename = "api.mutation.proposed")]
    ApiMutationProposed,
    #[serde(rename = "external_message.proposed")]
    ExternalMessageProposed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct Principal {
    pub workspace_id: String,
    pub environment_id: String,
    pub agent_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub user_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub session_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub task_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub run_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub run_event_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct Action {
    pub operation: String,
    #[serde(default)]
    #[cfg_attr(feature = "ts-export", ts(type = "Record<string, unknown> | null"))]
    pub parameters: serde_json::Value,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub side_effect: Option<SideEffectClass>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub enum SideEffectClass {
    None,
    Read,
    ExternalCommunication,
    FileWrite,
    ShellExec,
    NetworkCall,
    DbMutation,
    ApiMutation,
    MemoryWrite,
    Publish,
}
