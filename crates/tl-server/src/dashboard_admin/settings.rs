use serde_json::json;
use tl_core::{
    DataHandlingMode, EnforcementMode, UpdateWorkspaceSettingsRequest, WorkspaceSettings,
};

/// Merge a partial update onto current settings. Absent fields are left
/// unchanged; the result is what stores persist.
pub fn apply_settings_update(
    current: &WorkspaceSettings,
    update: &UpdateWorkspaceSettingsRequest,
) -> WorkspaceSettings {
    WorkspaceSettings {
        default_action: update
            .default_action
            .clone()
            .unwrap_or_else(|| current.default_action.clone()),
        escalation_webhook_url: update
            .escalation_webhook_url
            .clone()
            .or_else(|| current.escalation_webhook_url.clone()),
        telemetry_enabled: update
            .telemetry_enabled
            .unwrap_or(current.telemetry_enabled),
        retention_days: update
            .retention_days
            .clone()
            .unwrap_or_else(|| current.retention_days.clone()),
        data_handling_mode: update
            .data_handling_mode
            .unwrap_or(current.data_handling_mode),
        flow_checker_mode: update
            .flow_checker_mode
            .unwrap_or(current.flow_checker_mode),
        memory_checker_mode: update
            .memory_checker_mode
            .unwrap_or(current.memory_checker_mode),
        param_checker_mode: update
            .param_checker_mode
            .unwrap_or(current.param_checker_mode),
        approval_checker_mode: update
            .approval_checker_mode
            .unwrap_or(current.approval_checker_mode),
        config: current.config.clone(),
        updated_at: current.updated_at.clone(),
    }
}

pub fn default_settings() -> WorkspaceSettings {
    WorkspaceSettings {
        default_action: "allow".to_string(),
        escalation_webhook_url: None,
        telemetry_enabled: true,
        retention_days: "30".to_string(),
        data_handling_mode: DataHandlingMode::RawAllowed,
        flow_checker_mode: EnforcementMode::Off,
        memory_checker_mode: EnforcementMode::Off,
        param_checker_mode: EnforcementMode::Off,
        approval_checker_mode: EnforcementMode::Off,
        config: json!({}),
        updated_at: None,
    }
}
