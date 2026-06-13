-- Durable red-team dispatch jobs + per-attack results (Rust-owned).
CREATE TABLE IF NOT EXISTS redteam_jobs (
    workspace_id   TEXT        NOT NULL,
    id             UUID        NOT NULL,
    environment_id TEXT        NOT NULL,
    status         TEXT        NOT NULL DEFAULT 'queued',
    target         TEXT        NOT NULL,
    profile        TEXT        NOT NULL,
    generator      TEXT        NOT NULL DEFAULT 'deterministic',
    agent_id       TEXT,
    attacks        BIGINT      NOT NULL DEFAULT 0,
    landed         BIGINT      NOT NULL DEFAULT 0,
    blocked        BIGINT      NOT NULL DEFAULT 0,
    error          TEXT,
    created_at     TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at     TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (workspace_id, id)
);

CREATE INDEX IF NOT EXISTS redteam_jobs_workspace_created_idx
    ON redteam_jobs (workspace_id, created_at DESC);

CREATE TABLE IF NOT EXISTS redteam_job_results (
    workspace_id TEXT    NOT NULL,
    job_id       UUID    NOT NULL,
    seq          INTEGER NOT NULL,
    attack       TEXT    NOT NULL,
    goal         TEXT    NOT NULL,
    outcome      TEXT    NOT NULL,
    landed       BOOLEAN NOT NULL,
    prompt       TEXT,
    reply        TEXT    NOT NULL,
    trace_id     TEXT,
    PRIMARY KEY (workspace_id, job_id, seq)
);
