-- Preserve sub-cent provider cost for hard budget admission. Existing
-- events are exact only to their historical cent value.
ALTER TABLE llm_usage_events
    ADD COLUMN cost_nanos BIGINT;

UPDATE llm_usage_events
SET cost_nanos = LEAST(cost_minor, 922337203685) * 10000000;

ALTER TABLE llm_usage_events
    ALTER COLUMN cost_nanos SET NOT NULL,
    ADD CONSTRAINT llm_usage_events_cost_nanos_nonnegative CHECK (cost_nanos >= 0);

-- One durable row per budget principal gives transactions a stable
-- target to lock before reading spend and inserting a reservation.
CREATE TABLE llm_budget_principal_locks (
    workspace_id TEXT NOT NULL,
    principal_id TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (workspace_id, principal_id)
);

CREATE TABLE llm_budget_reservations (
    workspace_id TEXT NOT NULL,
    request_id TEXT NOT NULL,
    principal_id TEXT NOT NULL,
    api_key_id TEXT NOT NULL,
    currency TEXT NOT NULL,
    reserved_nanos BIGINT NOT NULL CHECK (reserved_nanos >= 0),
    actual_nanos BIGINT CHECK (actual_nanos IS NULL OR actual_nanos >= 0),
    status TEXT NOT NULL CHECK (status IN ('active', 'settled', 'released')),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (workspace_id, request_id)
);

CREATE INDEX llm_budget_reservations_active_window_idx
    ON llm_budget_reservations (workspace_id, principal_id, currency, created_at)
    WHERE status = 'active';
