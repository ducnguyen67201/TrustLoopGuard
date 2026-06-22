-- Workspace-level data handling mode. Drives `/v1/events` rejection logic
-- when set to `redacted_only`, and reserves namespace for `no_body_retention`
-- and `private_deployment` modes. See docs/specs/check-redaction.md.
ALTER TABLE workspace_settings
    ADD COLUMN data_handling_mode TEXT NOT NULL DEFAULT 'raw_allowed';
