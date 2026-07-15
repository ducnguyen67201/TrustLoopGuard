ALTER TABLE financial_actions
    ADD COLUMN mandate JSONB,
    ADD COLUMN status TEXT NOT NULL DEFAULT 'proposed'
        CHECK (status IN ('proposed', 'authorized', 'held', 'executed', 'denied', 'failed', 'reversed', 'expired'));

DROP INDEX IF EXISTS financial_actions_workspace_execution_created_idx;
ALTER TABLE financial_actions
    DROP CONSTRAINT IF EXISTS financial_actions_workspace_environment_idempotency_key_key,
    ADD CONSTRAINT financial_actions_workspace_id_idempotency_key_key
        UNIQUE (workspace_id, idempotency_key);
ALTER TABLE financial_actions
    DROP CONSTRAINT IF EXISTS financial_actions_authorization_intent_fk;
ALTER TABLE financial_actions
    DROP COLUMN IF EXISTS execution_status,
    DROP COLUMN IF EXISTS authorization_intent_id,
    DROP COLUMN IF EXISTS environment_id;

CREATE INDEX financial_actions_workspace_status_created_idx
    ON financial_actions (workspace_id, status, created_at DESC);

CREATE TABLE mandates (
    workspace_id TEXT NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    id TEXT NOT NULL,
    version INT NOT NULL DEFAULT 1,
    status TEXT NOT NULL DEFAULT 'active',
    principal_id TEXT NOT NULL,
    scope JSONB NOT NULL DEFAULT '{}'::jsonb,
    metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
    starts_at TIMESTAMPTZ,
    expires_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (workspace_id, id, version),
    CHECK (status IN ('active', 'revoked', 'expired'))
);

CREATE TABLE approval_requests (
    workspace_id TEXT NOT NULL,
    id UUID NOT NULL,
    action_id UUID NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending',
    reason TEXT NOT NULL,
    approver_roles JSONB NOT NULL DEFAULT '[]'::jsonb,
    decided_by TEXT,
    decided_at TIMESTAMPTZ,
    expires_at TIMESTAMPTZ,
    metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (workspace_id, id),
    FOREIGN KEY (workspace_id, action_id)
        REFERENCES financial_actions(workspace_id, id) ON DELETE CASCADE,
    CHECK (status IN ('pending', 'approved', 'denied', 'expired', 'canceled'))
);

CREATE INDEX approval_requests_workspace_status_created_idx
    ON approval_requests (workspace_id, status, created_at DESC);

CREATE TABLE tool_approval_requests (
    workspace_id TEXT NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    environment_id TEXT NOT NULL,
    id UUID NOT NULL,
    principal_id TEXT NOT NULL,
    invocation_id TEXT NOT NULL,
    fingerprint TEXT NOT NULL,
    fingerprint_version INT NOT NULL,
    status TEXT NOT NULL,
    event_snapshot JSONB NOT NULL,
    approver_roles JSONB NOT NULL DEFAULT '[]'::jsonb,
    reason TEXT,
    decided_by TEXT,
    decided_at TIMESTAMPTZ,
    expires_at TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (workspace_id, environment_id, id)
);

ALTER TABLE financial_receipts
    DROP CONSTRAINT IF EXISTS financial_receipts_authorization_receipt_fk;
ALTER TABLE financial_receipts
    DROP COLUMN IF EXISTS authorization_receipt_id,
    DROP COLUMN IF EXISTS environment_id;

DROP TABLE IF EXISTS authorization_receipts;
DROP TABLE IF EXISTS authorization_leases;
DROP TABLE IF EXISTS authorization_grants;
DROP TABLE IF EXISTS authorization_approvals;
DROP TABLE IF EXISTS authorization_intents;
