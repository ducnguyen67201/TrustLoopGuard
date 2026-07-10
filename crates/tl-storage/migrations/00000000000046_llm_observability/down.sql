DROP INDEX IF EXISTS budget_alert_configs_enabled_idx;
CREATE INDEX budget_alert_configs_enabled_idx
    ON budget_alert_configs (workspace_id) WHERE enabled;

ALTER TABLE budget_alert_firings DROP COLUMN IF EXISTS meter;
ALTER TABLE budget_alert_configs
    DROP CONSTRAINT IF EXISTS budget_alert_configs_workspace_meter_name_key,
    ADD CONSTRAINT budget_alert_configs_workspace_id_name_key UNIQUE (workspace_id, name);
ALTER TABLE budget_alert_configs DROP COLUMN IF EXISTS meter;

DROP INDEX IF EXISTS llm_usage_events_kind_window_idx;
ALTER TABLE llm_usage_events DROP COLUMN IF EXISTS usage_kind;

ALTER TABLE llm_model_prices
    DROP CONSTRAINT IF EXISTS llm_model_prices_output_nanos_nonnegative,
    DROP CONSTRAINT IF EXISTS llm_model_prices_input_nanos_nonnegative,
    DROP COLUMN IF EXISTS output_per_million_nanos,
    DROP COLUMN IF EXISTS input_per_million_nanos;
