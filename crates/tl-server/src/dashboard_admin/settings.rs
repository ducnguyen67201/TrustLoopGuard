use serde_json::json;
use tl_core::{DataHandlingMode, EnforcementMode, WorkspaceSettings};

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
        config: json!({}),
        updated_at: None,
    }
}
