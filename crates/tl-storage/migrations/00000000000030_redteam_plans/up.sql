-- Saved, named attack plans per agent (Rust-owned). The generated vectors +
-- analyzer paths are persisted as a JSONB blob so a plan can be re-selected and
-- re-run rather than regenerated each time.
CREATE TABLE IF NOT EXISTS redteam_plans (
    workspace_id   TEXT        NOT NULL,
    id             UUID        NOT NULL,
    environment_id TEXT        NOT NULL,
    agent_id       TEXT        NOT NULL,
    name           TEXT        NOT NULL,
    plan           JSONB       NOT NULL,
    created_at     TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (workspace_id, id)
);

CREATE INDEX IF NOT EXISTS redteam_plans_agent_created_idx
    ON redteam_plans (workspace_id, agent_id, created_at DESC);
