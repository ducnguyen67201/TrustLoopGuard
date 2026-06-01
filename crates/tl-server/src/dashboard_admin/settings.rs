use serde_json::json;
use tl_core::{DataHandlingMode, WorkspaceSettings};

pub fn default_settings() -> WorkspaceSettings {
    WorkspaceSettings {
        default_action: "allow".to_string(),
        escalation_webhook_url: None,
        telemetry_enabled: true,
        retention_days: "30".to_string(),
        data_handling_mode: DataHandlingMode::RawAllowed,
        config: json!({}),
        updated_at: None,
    }
}
