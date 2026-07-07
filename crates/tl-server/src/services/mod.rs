pub(crate) mod event_service;

use tl_core::{EnvironmentCheckerModes, WorkspaceSettings};
use tl_engine::CheckerModes;

/// Map workspace settings to per-checker enforcement modes. The settings
/// are already fetched on both hot paths, so mode resolution adds no I/O.
pub(crate) fn checker_modes(settings: &WorkspaceSettings) -> CheckerModes {
    CheckerModes {
        information_flow: settings.flow_checker_mode,
        memory: settings.memory_checker_mode,
        parameter_auth: settings.param_checker_mode,
        approval: settings.approval_checker_mode,
    }
}

/// Apply per-environment overrides on top of workspace-level modes.
/// `None` override fields inherit the workspace mode.
pub(crate) fn effective_checker_modes(
    settings: &WorkspaceSettings,
    overrides: Option<&EnvironmentCheckerModes>,
) -> CheckerModes {
    let base = checker_modes(settings);
    let Some(overrides) = overrides else {
        return base;
    };
    CheckerModes {
        information_flow: overrides.flow_checker_mode.unwrap_or(base.information_flow),
        memory: overrides.memory_checker_mode.unwrap_or(base.memory),
        parameter_auth: overrides.param_checker_mode.unwrap_or(base.parameter_auth),
        approval: overrides.approval_checker_mode.unwrap_or(base.approval),
    }
}

/// Resolve the effective modes for one request. An override-lookup error
/// fails the request: an environment may be configured stricter than its
/// workspace, so silently inheriting workspace modes on a store failure
/// would weaken enforcement. This matches how a workspace-settings
/// resolution failure is handled on the same paths.
pub(crate) async fn resolve_checker_modes(
    state: &crate::AppState,
    workspace_id: &str,
    environment_id: &str,
    settings: &WorkspaceSettings,
) -> Result<CheckerModes, axum::response::Response> {
    let overrides = match state
        .settings_store
        .get_environment_modes(workspace_id, environment_id)
        .await
    {
        Ok(overrides) => overrides,
        Err(e) => {
            tracing::error!(
                workspace_id,
                environment_id,
                error = %e,
                "environment checker-mode resolution failed"
            );
            return Err(crate::app::error::api_error_response(
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                tl_core::ApiErrorCode::Internal,
                "environment checker-mode resolution failed".into(),
            ));
        }
    };
    Ok(effective_checker_modes(settings, overrides.as_ref()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tl_core::EnforcementMode;

    fn settings_with(flow: EnforcementMode) -> WorkspaceSettings {
        WorkspaceSettings {
            flow_checker_mode: flow,
            ..crate::dashboard_admin::default_settings()
        }
    }

    #[test]
    fn no_override_inherits_workspace_modes() {
        let settings = settings_with(EnforcementMode::Enforce);
        let modes = effective_checker_modes(&settings, None);
        assert_eq!(modes.information_flow, EnforcementMode::Enforce);
        assert_eq!(modes.memory, EnforcementMode::Off);
    }

    #[test]
    fn all_none_override_inherits_workspace_modes() {
        let settings = settings_with(EnforcementMode::Shadow);
        let overrides = EnvironmentCheckerModes::default();
        let modes = effective_checker_modes(&settings, Some(&overrides));
        assert_eq!(modes.information_flow, EnforcementMode::Shadow);
    }

    #[test]
    fn override_wins_per_checker() {
        let settings = settings_with(EnforcementMode::Off);
        let overrides = EnvironmentCheckerModes {
            flow_checker_mode: Some(EnforcementMode::Enforce),
            memory_checker_mode: None,
            param_checker_mode: Some(EnforcementMode::Shadow),
            approval_checker_mode: None,
            updated_at: None,
        };
        let modes = effective_checker_modes(&settings, Some(&overrides));
        assert_eq!(modes.information_flow, EnforcementMode::Enforce);
        assert_eq!(modes.memory, EnforcementMode::Off);
        assert_eq!(modes.parameter_auth, EnforcementMode::Shadow);
        assert_eq!(modes.approval, EnforcementMode::Off);
    }

    #[test]
    fn override_can_disable_workspace_enforcement() {
        let settings = settings_with(EnforcementMode::Enforce);
        let overrides = EnvironmentCheckerModes {
            flow_checker_mode: Some(EnforcementMode::Off),
            ..EnvironmentCheckerModes::default()
        };
        let modes = effective_checker_modes(&settings, Some(&overrides));
        assert_eq!(modes.information_flow, EnforcementMode::Off);
    }
}
