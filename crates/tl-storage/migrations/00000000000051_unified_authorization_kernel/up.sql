-- Unified authorization kernel. This is a destructive pre-launch cutover:
-- legacy approval and mandate state is intentionally not migrated.

CREATE TABLE authorization_intents (
    workspace_id TEXT NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    environment_id TEXT NOT NULL,
    id UUID NOT NULL,
    domain TEXT NOT NULL CHECK (domain IN ('content', 'tool', 'financial')),
    subject_id TEXT NOT NULL,
    idempotency_key TEXT NOT NULL,
    principal_id TEXT NOT NULL,
    operation TEXT NOT NULL,
    fingerprint TEXT NOT NULL,
    fingerprint_version INT NOT NULL,
    subject_snapshot JSONB NOT NULL,
    status TEXT NOT NULL CHECK (status IN (
        'evaluating', 'pending_approval', 'authorized', 'denied',
        'deferred', 'canceled', 'expired'
    )),
    current_effect TEXT NOT NULL CHECK (current_effect IN (
        'permit', 'deny', 'transform', 'require_approval', 'defer'
    )),
    reason TEXT NOT NULL,
    trace_id TEXT,
    expires_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (workspace_id, environment_id, id),
    UNIQUE (workspace_id, environment_id, domain, subject_id),
    UNIQUE (workspace_id, environment_id, domain, idempotency_key)
);

CREATE INDEX authorization_intents_status_created_idx
    ON authorization_intents (workspace_id, environment_id, status, created_at DESC);

CREATE TABLE authorization_approvals (
    workspace_id TEXT NOT NULL,
    environment_id TEXT NOT NULL,
    id UUID NOT NULL,
    intent_id UUID NOT NULL,
    fingerprint TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending'
        CHECK (status IN ('pending', 'approved', 'denied', 'canceled', 'expired')),
    envelope JSONB NOT NULL,
    envelope_hash TEXT NOT NULL,
    requirement_ids JSONB NOT NULL DEFAULT '[]'::jsonb,
    approver_roles JSONB NOT NULL DEFAULT '[]'::jsonb,
    decided_by TEXT,
    decided_at TIMESTAMPTZ,
    decision_reason TEXT,
    expires_at TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (workspace_id, environment_id, id),
    FOREIGN KEY (workspace_id, environment_id, intent_id)
        REFERENCES authorization_intents(workspace_id, environment_id, id)
        ON DELETE CASCADE
);

CREATE UNIQUE INDEX authorization_approvals_pending_intent_fingerprint_idx
    ON authorization_approvals (workspace_id, environment_id, intent_id, fingerprint)
    WHERE status = 'pending';
CREATE INDEX authorization_approvals_status_created_idx
    ON authorization_approvals (workspace_id, environment_id, status, created_at DESC);

CREATE TABLE authorization_grants (
    workspace_id TEXT NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    environment_id TEXT NOT NULL,
    id UUID NOT NULL,
    principal_id TEXT NOT NULL,
    domain TEXT NOT NULL CHECK (domain IN ('content', 'tool', 'financial')),
    capability TEXT NOT NULL,
    mode TEXT NOT NULL CHECK (mode IN ('exact_once', 'scoped')),
    status TEXT NOT NULL DEFAULT 'active'
        CHECK (status IN ('active', 'revoked', 'expired', 'exhausted')),
    source TEXT NOT NULL CHECK (source IN ('user_intent', 'reviewer_approval', 'workspace_admin')),
    scope_schema TEXT NOT NULL,
    scope JSONB,
    exact_fingerprint TEXT,
    fingerprint_version INT NOT NULL,
    source_approval_id UUID,
    requirement_ids JSONB NOT NULL DEFAULT '[]'::jsonb,
    max_uses INT,
    use_count INT NOT NULL DEFAULT 0 CHECK (use_count >= 0),
    starts_at TIMESTAMPTZ,
    expires_at TIMESTAMPTZ,
    revoked_at TIMESTAMPTZ,
    revoked_by TEXT,
    created_by TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (workspace_id, environment_id, id),
    CHECK (
        (mode = 'exact_once' AND exact_fingerprint IS NOT NULL AND scope IS NULL AND max_uses = 1)
        OR (mode = 'scoped' AND exact_fingerprint IS NULL AND scope IS NOT NULL)
    ),
    FOREIGN KEY (workspace_id, environment_id, source_approval_id)
        REFERENCES authorization_approvals(workspace_id, environment_id, id)
        ON DELETE RESTRICT
);

CREATE INDEX authorization_grants_match_idx
    ON authorization_grants (
        workspace_id, environment_id, principal_id, domain, capability, status, expires_at
    );

CREATE TABLE authorization_leases (
    workspace_id TEXT NOT NULL,
    environment_id TEXT NOT NULL,
    id UUID NOT NULL,
    intent_id UUID NOT NULL,
    grant_id UUID,
    attempt_id TEXT NOT NULL,
    fingerprint TEXT NOT NULL,
    status TEXT NOT NULL CHECK (status IN ('claimed', 'consumed', 'canceled', 'expired')),
    claimed_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    consumed_at TIMESTAMPTZ,
    canceled_at TIMESTAMPTZ,
    expires_at TIMESTAMPTZ NOT NULL,
    outcome JSONB NOT NULL DEFAULT '{}'::jsonb,
    PRIMARY KEY (workspace_id, environment_id, id),
    UNIQUE (workspace_id, environment_id, intent_id, attempt_id),
    FOREIGN KEY (workspace_id, environment_id, intent_id)
        REFERENCES authorization_intents(workspace_id, environment_id, id)
        ON DELETE CASCADE,
    FOREIGN KEY (workspace_id, environment_id, grant_id)
        REFERENCES authorization_grants(workspace_id, environment_id, id)
        ON DELETE RESTRICT
);

CREATE INDEX authorization_leases_intent_status_idx
    ON authorization_leases (workspace_id, environment_id, intent_id, status);

CREATE TABLE authorization_receipts (
    workspace_id TEXT NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    environment_id TEXT NOT NULL,
    id UUID NOT NULL,
    intent_id UUID,
    trace_id TEXT,
    domain TEXT NOT NULL CHECK (domain IN ('content', 'tool', 'financial')),
    effect TEXT NOT NULL CHECK (effect IN ('permit', 'deny', 'transform', 'require_approval', 'defer')),
    intent_status TEXT,
    subject_hash TEXT NOT NULL,
    reason TEXT NOT NULL,
    findings JSONB NOT NULL DEFAULT '[]'::jsonb,
    policy_versions JSONB NOT NULL DEFAULT '[]'::jsonb,
    approval_id UUID,
    grant_id UUID,
    lease_id UUID,
    domain_evidence JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (workspace_id, environment_id, id),
    FOREIGN KEY (workspace_id, environment_id, intent_id)
        REFERENCES authorization_intents(workspace_id, environment_id, id)
        ON DELETE RESTRICT,
    FOREIGN KEY (workspace_id, environment_id, approval_id)
        REFERENCES authorization_approvals(workspace_id, environment_id, id)
        ON DELETE RESTRICT,
    FOREIGN KEY (workspace_id, environment_id, grant_id)
        REFERENCES authorization_grants(workspace_id, environment_id, id)
        ON DELETE RESTRICT,
    FOREIGN KEY (workspace_id, environment_id, lease_id)
        REFERENCES authorization_leases(workspace_id, environment_id, id)
        ON DELETE RESTRICT
);

CREATE INDEX authorization_receipts_domain_created_idx
    ON authorization_receipts (workspace_id, environment_id, domain, created_at DESC);

ALTER TABLE financial_actions
    ADD COLUMN environment_id TEXT NOT NULL DEFAULT 'production',
    ADD COLUMN authorization_intent_id UUID,
    ADD COLUMN execution_status TEXT NOT NULL DEFAULT 'not_started'
        CHECK (execution_status IN ('not_started', 'executing', 'succeeded', 'failed', 'canceled', 'reversed'));

ALTER TABLE financial_actions
    DROP CONSTRAINT IF EXISTS financial_actions_workspace_id_idempotency_key_key,
    ADD CONSTRAINT financial_actions_workspace_environment_idempotency_key_key
        UNIQUE (workspace_id, environment_id, idempotency_key);

ALTER TABLE financial_actions
    ADD CONSTRAINT financial_actions_authorization_intent_fk
    FOREIGN KEY (workspace_id, environment_id, authorization_intent_id)
    REFERENCES authorization_intents(workspace_id, environment_id, id)
    ON DELETE RESTRICT;

DROP INDEX IF EXISTS financial_actions_workspace_status_created_idx;
CREATE INDEX financial_actions_workspace_execution_created_idx
    ON financial_actions (workspace_id, environment_id, execution_status, created_at DESC);

ALTER TABLE financial_actions
    DROP COLUMN status,
    DROP COLUMN mandate;

DROP INDEX IF EXISTS approval_requests_workspace_status_created_idx;
DROP TABLE IF EXISTS approval_requests;
DROP TABLE IF EXISTS mandates;
DROP TABLE IF EXISTS tool_approval_requests;

ALTER TABLE financial_receipts
    ADD COLUMN environment_id TEXT NOT NULL DEFAULT 'production',
    ADD COLUMN authorization_receipt_id UUID;
ALTER TABLE financial_receipts
    ADD CONSTRAINT financial_receipts_authorization_receipt_fk
    FOREIGN KEY (workspace_id, environment_id, authorization_receipt_id)
    REFERENCES authorization_receipts(workspace_id, environment_id, id)
    ON DELETE RESTRICT;
