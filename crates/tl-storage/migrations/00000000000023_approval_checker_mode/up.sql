-- Per-workspace rollout mode for the approval checker. Default 'off':
-- no workspace gets approval escalations without opting in.
ALTER TABLE workspace_settings
    ADD COLUMN approval_checker_mode TEXT NOT NULL DEFAULT 'off';

ALTER TABLE workspace_settings
    ADD CONSTRAINT workspace_settings_approval_checker_mode_check CHECK (
        approval_checker_mode IN ('off', 'shadow', 'enforce')
    );
