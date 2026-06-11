-- Per-environment checker-mode overrides. NULL columns inherit the
-- workspace-level mode from workspace_settings.
CREATE TABLE environment_checker_modes (
    workspace_id TEXT NOT NULL,
    environment_id TEXT NOT NULL,
    flow_checker_mode TEXT NULL,
    memory_checker_mode TEXT NULL,
    param_checker_mode TEXT NULL,
    approval_checker_mode TEXT NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (workspace_id, environment_id),
    CONSTRAINT environment_checker_modes_flow_check CHECK (
        flow_checker_mode IS NULL OR flow_checker_mode IN ('off', 'shadow', 'enforce')
    ),
    CONSTRAINT environment_checker_modes_memory_check CHECK (
        memory_checker_mode IS NULL OR memory_checker_mode IN ('off', 'shadow', 'enforce')
    ),
    CONSTRAINT environment_checker_modes_param_check CHECK (
        param_checker_mode IS NULL OR param_checker_mode IN ('off', 'shadow', 'enforce')
    ),
    CONSTRAINT environment_checker_modes_approval_check CHECK (
        approval_checker_mode IS NULL OR approval_checker_mode IN ('off', 'shadow', 'enforce')
    )
);
