-- Append-only LLM gateway metering log. One row per metered chat
-- completion; the gateway budget check sums cost_minor over the same
-- day/week/month windows the financial ledger uses. Deliberately NOT
-- a financial_ledger_entries row: usage is a fact, not a
-- reserve/release lifecycle.
CREATE TABLE IF NOT EXISTS llm_usage_events (
    workspace_id TEXT NOT NULL,
    id UUID NOT NULL,
    principal_id TEXT NOT NULL,
    api_key_id TEXT NOT NULL,
    model TEXT NOT NULL,
    prompt_tokens BIGINT NOT NULL CHECK (prompt_tokens >= 0),
    completion_tokens BIGINT NOT NULL CHECK (completion_tokens >= 0),
    cost_minor BIGINT NOT NULL CHECK (cost_minor >= 0),
    currency TEXT NOT NULL,
    request_id TEXT NOT NULL,
    metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
    effective_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (workspace_id, id),
    -- Retried metering writes are safe: one row per gateway request.
    UNIQUE (workspace_id, request_id)
);

CREATE INDEX IF NOT EXISTS llm_usage_events_window_idx
    ON llm_usage_events (workspace_id, principal_id, currency, effective_at);

CREATE INDEX IF NOT EXISTS llm_usage_events_model_idx
    ON llm_usage_events (workspace_id, model, effective_at);
