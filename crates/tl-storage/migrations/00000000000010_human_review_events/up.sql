CREATE TABLE IF NOT EXISTS human_review_events (
    workspace_id TEXT NOT NULL,
    id UUID NOT NULL,
    trace_id UUID NOT NULL,
    run_id UUID,
    run_event_id UUID,
    outcome TEXT NOT NULL CHECK (
        outcome IN (
            'accepted',
            'corrected',
            'rejected',
            'false_positive',
            'missed_issue',
            'ignored'
        )
    ),
    reviewer_id TEXT,
    reason_codes JSONB NOT NULL DEFAULT '[]'::jsonb,
    note TEXT,
    metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (workspace_id, id)
);

CREATE INDEX IF NOT EXISTS human_review_events_workspace_trace_created_idx
    ON human_review_events (workspace_id, trace_id, created_at DESC);

CREATE INDEX IF NOT EXISTS human_review_events_workspace_created_idx
    ON human_review_events (workspace_id, created_at DESC);

CREATE INDEX IF NOT EXISTS human_review_events_workspace_outcome_created_idx
    ON human_review_events (workspace_id, outcome, created_at DESC);
