-- Durable benchmark parent runs + raw/guarded arm mapping (Rust-owned).
ALTER TABLE redteam_job_results
    ADD COLUMN IF NOT EXISTS case_id TEXT,
    ADD COLUMN IF NOT EXISTS track TEXT,
    ADD COLUMN IF NOT EXISTS kind TEXT,
    ADD COLUMN IF NOT EXISTS trial_index INTEGER;

CREATE TABLE IF NOT EXISTS bench_runs (
    workspace_id   TEXT        NOT NULL,
    id             UUID        NOT NULL,
    environment_id TEXT        NOT NULL,
    status         TEXT        NOT NULL DEFAULT 'queued',
    profile        TEXT        NOT NULL,
    generator      TEXT        NOT NULL DEFAULT 'deterministic',
    agent_id       TEXT,
    seed           TEXT,
    error          TEXT,
    created_at     TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at     TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (workspace_id, id)
);

CREATE INDEX IF NOT EXISTS bench_runs_workspace_created_idx
    ON bench_runs (workspace_id, created_at DESC);

CREATE TABLE IF NOT EXISTS bench_run_arms (
    workspace_id    TEXT        NOT NULL,
    run_id          UUID        NOT NULL,
    arm             TEXT        NOT NULL,
    label           TEXT        NOT NULL,
    target          TEXT        NOT NULL,
    redteam_job_id  UUID,
    checker_config  TEXT,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (workspace_id, run_id, arm),
    CONSTRAINT bench_run_arms_arm_check
        CHECK (arm IN ('raw', 'guarded')),
    CONSTRAINT bench_run_arms_run_fk
        FOREIGN KEY (workspace_id, run_id)
        REFERENCES bench_runs (workspace_id, id)
        ON DELETE CASCADE,
    CONSTRAINT bench_run_arms_redteam_job_fk
        FOREIGN KEY (workspace_id, redteam_job_id)
        REFERENCES redteam_jobs (workspace_id, id)
);

CREATE INDEX IF NOT EXISTS bench_run_arms_workspace_run_idx
    ON bench_run_arms (workspace_id, run_id);
