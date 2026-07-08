-- Durable evolving-eval cases promoted from verified harden survivors.
-- The full source trace stays in red-team session storage; this table is the
-- suite/index row a regression runner can select and refresh idempotently.
CREATE TABLE IF NOT EXISTS redteam_regression_cases (
    workspace_id         TEXT        NOT NULL,
    id                   UUID        NOT NULL,
    case_key             TEXT        NOT NULL,
    environment_id       TEXT        NOT NULL,
    agent_id             TEXT,
    source               TEXT        NOT NULL,
    source_job_id        UUID,
    source_session_seqs  JSONB       NOT NULL,
    substrate            TEXT        NOT NULL,
    artifact_id          TEXT        NOT NULL,
    expected_outcome     TEXT        NOT NULL,
    attack               TEXT        NOT NULL,
    goal                 TEXT        NOT NULL,
    created_at           TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at           TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (workspace_id, case_key)
);

CREATE INDEX IF NOT EXISTS redteam_regression_cases_agent_updated_idx
    ON redteam_regression_cases (workspace_id, agent_id, updated_at DESC);

CREATE INDEX IF NOT EXISTS redteam_regression_cases_job_idx
    ON redteam_regression_cases (workspace_id, source_job_id);
