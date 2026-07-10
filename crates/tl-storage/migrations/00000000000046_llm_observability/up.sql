-- Precise model prices, typed LLM usage ownership, and meter-scoped alerts.
-- Existing rows retain their exact historical cent-derived values.
ALTER TABLE llm_model_prices
    ADD COLUMN input_per_million_nanos BIGINT,
    ADD COLUMN output_per_million_nanos BIGINT;

UPDATE llm_model_prices
SET input_per_million_nanos = LEAST(input_per_million_minor, 922337203685) * 10000000,
    output_per_million_nanos = LEAST(output_per_million_minor, 922337203685) * 10000000;

ALTER TABLE llm_model_prices
    ALTER COLUMN input_per_million_nanos SET NOT NULL,
    ALTER COLUMN output_per_million_nanos SET NOT NULL,
    ADD CONSTRAINT llm_model_prices_input_nanos_nonnegative
        CHECK (input_per_million_nanos >= 0),
    ADD CONSTRAINT llm_model_prices_output_nanos_nonnegative
        CHECK (output_per_million_nanos >= 0);

ALTER TABLE llm_usage_events
    ADD COLUMN usage_kind TEXT NOT NULL DEFAULT 'customer_inference'
        CHECK (usage_kind IN ('customer_inference', 'guardrail'));

CREATE INDEX llm_usage_events_kind_window_idx
    ON llm_usage_events (workspace_id, usage_kind, effective_at);

ALTER TABLE budget_alert_configs
    ADD COLUMN meter TEXT NOT NULL DEFAULT 'actions'
        CHECK (meter IN ('actions', 'llm_usage'));

ALTER TABLE budget_alert_configs
    DROP CONSTRAINT budget_alert_configs_workspace_id_name_key,
    ADD CONSTRAINT budget_alert_configs_workspace_meter_name_key
        UNIQUE (workspace_id, meter, name);

ALTER TABLE budget_alert_firings
    ADD COLUMN meter TEXT NOT NULL DEFAULT 'actions'
        CHECK (meter IN ('actions', 'llm_usage'));

DROP INDEX IF EXISTS budget_alert_configs_enabled_idx;
CREATE INDEX budget_alert_configs_enabled_idx
    ON budget_alert_configs (workspace_id, meter) WHERE enabled;
