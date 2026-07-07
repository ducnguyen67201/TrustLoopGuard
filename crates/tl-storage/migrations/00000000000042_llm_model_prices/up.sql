-- Workspace-editable LLM model prices for gateway metering. One row per
-- (workspace, model); prices are integer USD minor units per 1M tokens.
-- The built-in default table in code seeds/fallbacks models with no row.
-- Non-negative CHECKs: a negative price would subtract from accumulated
-- spend and quietly defeat the budget gate.
CREATE TABLE IF NOT EXISTS llm_model_prices (
    workspace_id TEXT NOT NULL,
    model TEXT NOT NULL,
    input_per_million_minor BIGINT NOT NULL CHECK (input_per_million_minor >= 0),
    output_per_million_minor BIGINT NOT NULL CHECK (output_per_million_minor >= 0),
    currency TEXT NOT NULL DEFAULT 'USD',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (workspace_id, model)
);
