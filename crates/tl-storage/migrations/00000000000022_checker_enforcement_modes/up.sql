-- Per-workspace rollout modes for the event-pipeline checkers. Default
-- 'off': no workspace gets evaluation or decision changes without opting
-- in.
ALTER TABLE workspace_settings
    ADD COLUMN flow_checker_mode TEXT NOT NULL DEFAULT 'off',
    ADD COLUMN memory_checker_mode TEXT NOT NULL DEFAULT 'off',
    ADD COLUMN param_checker_mode TEXT NOT NULL DEFAULT 'off';

ALTER TABLE workspace_settings
    ADD CONSTRAINT workspace_settings_flow_checker_mode_check CHECK (
        flow_checker_mode IN ('off', 'shadow', 'enforce')
    ),
    ADD CONSTRAINT workspace_settings_memory_checker_mode_check CHECK (
        memory_checker_mode IN ('off', 'shadow', 'enforce')
    ),
    ADD CONSTRAINT workspace_settings_param_checker_mode_check CHECK (
        param_checker_mode IN ('off', 'shadow', 'enforce')
    );
