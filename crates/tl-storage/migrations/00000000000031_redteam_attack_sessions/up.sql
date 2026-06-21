-- Forward migration for databases that already applied migration 26 before
-- red-team results moved from flat rows to per-attack sessions/events.
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

DROP TABLE IF EXISTS redteam_job_results;
