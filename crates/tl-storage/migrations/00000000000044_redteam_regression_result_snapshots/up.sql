-- Durable historical summaries for the evolving eval loop.
-- One row is kept per regression result query key so CI/dashboard refreshes are
-- idempotent while still producing a trend over distinct regression jobs.
CREATE TABLE IF NOT EXISTS redteam_regression_result_snapshots (
    workspace_id      TEXT        NOT NULL,
    snapshot_key      TEXT        NOT NULL,
    id                UUID        NOT NULL,
    job_id            UUID        NOT NULL,
    source_job_id     UUID        NOT NULL,
    environment_id    TEXT        NOT NULL,
    agent_id          TEXT,
    case_keys         JSONB       NOT NULL,
    total             INTEGER     NOT NULL,
    passed            INTEGER     NOT NULL,
    failed            INTEGER     NOT NULL,
    missing           INTEGER     NOT NULL,
    inconclusive      INTEGER     NOT NULL,
    created_at        TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at        TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (workspace_id, snapshot_key)
);

CREATE INDEX IF NOT EXISTS redteam_regression_result_snapshots_source_updated_idx
    ON redteam_regression_result_snapshots (workspace_id, source_job_id, updated_at DESC);

CREATE INDEX IF NOT EXISTS redteam_regression_result_snapshots_job_idx
    ON redteam_regression_result_snapshots (workspace_id, job_id);
