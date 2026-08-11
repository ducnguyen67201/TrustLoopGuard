CREATE TABLE notification_rules (
    workspace_id TEXT NOT NULL,
    id UUID NOT NULL,
    environment_id TEXT NOT NULL,
    agent_id TEXT,
    email TEXT NOT NULL,
    event_kinds TEXT[] NOT NULL,
    enabled BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    deleted_at TIMESTAMPTZ,
    PRIMARY KEY (workspace_id, id),
    FOREIGN KEY (workspace_id, environment_id)
        REFERENCES workspace_environments (workspace_id, id) ON DELETE CASCADE,
    FOREIGN KEY (workspace_id, agent_id)
        REFERENCES agents (workspace_id, id) ON DELETE RESTRICT,
    CHECK (cardinality(event_kinds) > 0),
    CHECK (event_kinds <@ ARRAY[
        'evaluation_failed', 'evaluation_inconclusive', 'evaluation_error',
        'provider_terminal_failure', 'test'
    ]::TEXT[])
);

CREATE TABLE notification_deliveries (
    workspace_id TEXT NOT NULL,
    id UUID NOT NULL,
    rule_id UUID NOT NULL,
    environment_id TEXT NOT NULL,
    run_id UUID,
    event_kind TEXT NOT NULL,
    subject_id TEXT NOT NULL,
    subject_version TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending',
    payload JSONB NOT NULL DEFAULT '{}'::jsonb,
    attempt_count INT NOT NULL DEFAULT 0,
    next_attempt_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    lease_owner TEXT,
    lease_expires_at TIMESTAMPTZ,
    last_error_code TEXT,
    last_error_message TEXT,
    sent_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (workspace_id, id),
    UNIQUE (workspace_id, rule_id, event_kind, subject_id, subject_version),
    FOREIGN KEY (workspace_id, rule_id)
        REFERENCES notification_rules (workspace_id, id) ON DELETE CASCADE,
    FOREIGN KEY (workspace_id, environment_id)
        REFERENCES workspace_environments (workspace_id, id) ON DELETE CASCADE,
    FOREIGN KEY (workspace_id, run_id)
        REFERENCES runs (workspace_id, id) ON DELETE CASCADE,
    CHECK (event_kind IN (
        'evaluation_failed', 'evaluation_inconclusive', 'evaluation_error',
        'provider_terminal_failure', 'test'
    )),
    CHECK (status IN ('pending', 'sending', 'sent', 'failed')),
    CHECK (attempt_count >= 0)
);

CREATE INDEX notification_rules_active_idx
    ON notification_rules (workspace_id, environment_id, created_at DESC)
    WHERE deleted_at IS NULL;

CREATE INDEX notification_deliveries_claim_idx
    ON notification_deliveries (next_attempt_at, created_at)
    WHERE status = 'pending';

CREATE INDEX notification_deliveries_run_idx
    ON notification_deliveries (workspace_id, run_id, created_at DESC)
    WHERE run_id IS NOT NULL;
