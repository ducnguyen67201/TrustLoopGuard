-- Configurable budget alert thresholds + once-per-window firing log.
--
-- `budget_alert_configs` is the user-authored threshold ("warn me at
-- 80% of the weekly cap"); `budget_alert_firings` records each
-- delivery-worthy crossing. The UNIQUE (config_id, principal_id,
-- window_start) key is the dedup gate: insert-first with ON CONFLICT
-- DO NOTHING makes firing at-most-once per window per principal,
-- race-safe across concurrent spends.
CREATE TABLE IF NOT EXISTS budget_alert_configs (
    id UUID PRIMARY KEY,
    workspace_id TEXT NOT NULL,
    name TEXT NOT NULL,
    -- Window keyword is reserved in Postgres; quoted throughout.
    "window" TEXT NOT NULL CHECK ("window" IN ('day', 'week', 'month')),
    -- NULL = any principal (evaluated per acting principal).
    principal_id TEXT,
    threshold_type TEXT NOT NULL CHECK (threshold_type IN ('percent', 'absolute')),
    threshold_value BIGINT NOT NULL,
    -- NULL falls back to the workspace escalation_webhook_url.
    webhook_url TEXT,
    enabled BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (workspace_id, name)
);

-- The spend-time hook's single indexed lookup: enabled configs per
-- workspace.
CREATE INDEX IF NOT EXISTS budget_alert_configs_enabled_idx
    ON budget_alert_configs (workspace_id) WHERE enabled;

CREATE TABLE IF NOT EXISTS budget_alert_firings (
    id UUID PRIMARY KEY,
    workspace_id TEXT NOT NULL,
    config_id UUID NOT NULL REFERENCES budget_alert_configs (id) ON DELETE CASCADE,
    principal_id TEXT NOT NULL,
    window_start TIMESTAMPTZ NOT NULL,
    cap_minor BIGINT NOT NULL,
    spent_minor BIGINT NOT NULL,
    currency TEXT NOT NULL,
    payload JSONB NOT NULL,
    fired_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    -- A workspace-wide config fires once per principal per window,
    -- not once globally.
    UNIQUE (config_id, principal_id, window_start)
);

CREATE INDEX IF NOT EXISTS budget_alert_firings_workspace_idx
    ON budget_alert_firings (workspace_id, fired_at);
