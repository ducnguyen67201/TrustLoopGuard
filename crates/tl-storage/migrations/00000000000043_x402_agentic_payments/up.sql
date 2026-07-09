CREATE TABLE IF NOT EXISTS financial_payment_sessions (
    workspace_id TEXT NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    id TEXT NOT NULL,
    principal_id TEXT NOT NULL,
    currency TEXT NOT NULL,
    max_amount_minor BIGINT NOT NULL CHECK (max_amount_minor >= 0),
    reserved_minor BIGINT NOT NULL DEFAULT 0 CHECK (reserved_minor >= 0),
    committed_minor BIGINT NOT NULL DEFAULT 0 CHECK (committed_minor >= 0),
    released_minor BIGINT NOT NULL DEFAULT 0 CHECK (released_minor >= 0),
    status TEXT NOT NULL DEFAULT 'active',
    expires_at TIMESTAMPTZ NOT NULL,
    metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (workspace_id, id),
    CHECK (status IN ('active', 'closed', 'expired'))
);

CREATE INDEX IF NOT EXISTS financial_payment_sessions_principal_idx
    ON financial_payment_sessions (workspace_id, principal_id, currency, expires_at);

CREATE INDEX IF NOT EXISTS financial_payment_sessions_status_expiry_idx
    ON financial_payment_sessions (workspace_id, status, expires_at);

CREATE TABLE IF NOT EXISTS financial_payment_reservations (
    workspace_id TEXT NOT NULL,
    id UUID NOT NULL,
    action_id UUID NOT NULL,
    session_id TEXT NOT NULL,
    principal_id TEXT NOT NULL,
    payment_requirement_hash TEXT NOT NULL,
    amount_minor BIGINT NOT NULL CHECK (amount_minor >= 0),
    currency TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'reserved',
    expires_at TIMESTAMPTZ NOT NULL,
    commit_proof JSONB,
    metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    committed_at TIMESTAMPTZ,
    released_at TIMESTAMPTZ,
    PRIMARY KEY (workspace_id, id),
    UNIQUE (workspace_id, action_id),
    UNIQUE (workspace_id, session_id, payment_requirement_hash),
    FOREIGN KEY (workspace_id, action_id)
        REFERENCES financial_actions(workspace_id, id)
        ON DELETE CASCADE,
    FOREIGN KEY (workspace_id, session_id)
        REFERENCES financial_payment_sessions(workspace_id, id)
        ON DELETE CASCADE,
    CHECK (status IN ('reserved', 'committed', 'released', 'expired'))
);

CREATE INDEX IF NOT EXISTS financial_payment_reservations_session_idx
    ON financial_payment_reservations (workspace_id, session_id, status, expires_at);

CREATE INDEX IF NOT EXISTS financial_payment_reservations_action_idx
    ON financial_payment_reservations (workspace_id, action_id);

CREATE INDEX IF NOT EXISTS financial_payment_reservations_hash_idx
    ON financial_payment_reservations (workspace_id, payment_requirement_hash);
