CREATE TABLE tool_approval_requests (
    workspace_id TEXT NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    environment_id TEXT NOT NULL,
    id UUID NOT NULL,
    invocation_id TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending',
    fingerprint TEXT NOT NULL,
    fingerprint_version INT NOT NULL,
    agent_id TEXT NOT NULL,
    operation TEXT NOT NULL,
    server_id TEXT NOT NULL,
    tool_name TEXT NOT NULL,
    schema_hash TEXT NOT NULL,
    action_snapshot JSONB NOT NULL,
    approver_roles JSONB NOT NULL DEFAULT '[]'::jsonb,
    reason TEXT NOT NULL,
    trace_id TEXT NOT NULL,
    decided_by TEXT,
    decided_at TIMESTAMPTZ,
    expires_at TIMESTAMPTZ NOT NULL,
    consumed_at TIMESTAMPTZ,
    consumed_by_attempt_id TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (workspace_id, id),
    UNIQUE (workspace_id, environment_id, invocation_id),
    CHECK (status IN ('pending', 'approved', 'denied', 'expired', 'canceled', 'consumed'))
);

CREATE INDEX tool_approval_requests_workspace_environment_status_created_idx
    ON tool_approval_requests (workspace_id, environment_id, status, created_at DESC);

CREATE INDEX tool_approval_requests_workspace_environment_agent_idx
    ON tool_approval_requests (workspace_id, environment_id, agent_id, created_at DESC);
