CREATE TABLE IF NOT EXISTS runs (
    workspace_id TEXT NOT NULL,
    id UUID NOT NULL,
    agent_id TEXT NOT NULL,
    kind TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'running',
    external_id TEXT,
    metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
    started_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    ended_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (workspace_id, id)
);

CREATE INDEX IF NOT EXISTS runs_workspace_created_idx
    ON runs (workspace_id, created_at DESC);

CREATE INDEX IF NOT EXISTS runs_workspace_agent_created_idx
    ON runs (workspace_id, agent_id, created_at DESC);

CREATE INDEX IF NOT EXISTS runs_workspace_external_idx
    ON runs (workspace_id, external_id)
    WHERE external_id IS NOT NULL;

ALTER TABLE traces
    ADD COLUMN IF NOT EXISTS run_id UUID;

CREATE INDEX IF NOT EXISTS traces_workspace_run_created_idx
    ON traces (workspace_id, run_id, created_at DESC)
    WHERE run_id IS NOT NULL;
