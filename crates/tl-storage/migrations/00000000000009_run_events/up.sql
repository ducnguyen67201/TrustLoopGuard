CREATE TABLE IF NOT EXISTS run_events (
    workspace_id TEXT NOT NULL,
    id UUID NOT NULL,
    run_id UUID NOT NULL,
    sequence INTEGER NOT NULL,
    kind TEXT NOT NULL,
    label TEXT,
    input_summary TEXT,
    output_summary TEXT,
    metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
    occurred_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (workspace_id, id),
    FOREIGN KEY (workspace_id, run_id)
        REFERENCES runs (workspace_id, id)
        ON DELETE CASCADE
);

CREATE UNIQUE INDEX IF NOT EXISTS run_events_workspace_run_sequence_idx
    ON run_events (workspace_id, run_id, sequence);

CREATE INDEX IF NOT EXISTS run_events_workspace_run_occurred_idx
    ON run_events (workspace_id, run_id, occurred_at);

ALTER TABLE traces
    ADD COLUMN IF NOT EXISTS run_event_id UUID;

CREATE INDEX IF NOT EXISTS traces_workspace_run_event_created_idx
    ON traces (workspace_id, run_event_id, created_at DESC)
    WHERE run_event_id IS NOT NULL;
