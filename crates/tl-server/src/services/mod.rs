pub(crate) mod event_service;
pub(crate) mod guard_service;

use tl_core::WorkspaceSettings;
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
