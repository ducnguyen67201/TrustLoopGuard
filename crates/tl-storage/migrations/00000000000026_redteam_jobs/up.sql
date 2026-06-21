-- Durable red-team dispatch jobs + per-attack sessions (Rust-owned).
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

CREATE TABLE IF NOT EXISTS redteam_attack_sessions (
    workspace_id       TEXT    NOT NULL,
    job_id             UUID    NOT NULL,
    session_id         TEXT    NOT NULL,
    runner_session_id  TEXT,
    seq                INTEGER NOT NULL,
    case_id            TEXT,
    track              TEXT,
    kind               TEXT,
    trial_index        INTEGER,
    attack             TEXT    NOT NULL,
    goal               TEXT    NOT NULL,
    status             TEXT    NOT NULL,
    outcome            TEXT    NOT NULL,
    landed             BOOLEAN NOT NULL,
    trace_id           TEXT,
    error              TEXT,
    PRIMARY KEY (workspace_id, job_id, session_id)
);

CREATE INDEX IF NOT EXISTS redteam_attack_sessions_job_seq_idx
    ON redteam_attack_sessions (workspace_id, job_id, seq);

CREATE INDEX IF NOT EXISTS redteam_attack_sessions_workspace_outcome_idx
    ON redteam_attack_sessions (workspace_id, outcome);

CREATE TABLE IF NOT EXISTS redteam_session_events (
    workspace_id  TEXT        NOT NULL,
    job_id        UUID        NOT NULL,
    session_id    TEXT        NOT NULL,
    event_id      TEXT        NOT NULL,
    seq           INTEGER     NOT NULL,
    kind          TEXT        NOT NULL,
    actor         TEXT        NOT NULL,
    label         TEXT,
    content_text  TEXT,
    payload       JSONB       NOT NULL DEFAULT '{}'::jsonb,
    trace_id      TEXT,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (workspace_id, job_id, session_id, event_id)
);

CREATE INDEX IF NOT EXISTS redteam_session_events_session_seq_idx
    ON redteam_session_events (workspace_id, job_id, session_id, seq);
