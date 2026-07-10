DROP INDEX IF EXISTS llm_budget_reservations_active_window_idx;
DROP TABLE IF EXISTS llm_budget_reservations;
DROP TABLE IF EXISTS llm_budget_principal_locks;

ALTER TABLE llm_usage_events
    DROP CONSTRAINT IF EXISTS llm_usage_events_cost_nanos_nonnegative,
    DROP COLUMN IF EXISTS cost_nanos;
