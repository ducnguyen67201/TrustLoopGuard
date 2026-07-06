CREATE TABLE IF NOT EXISTS financial_actions (
    workspace_id TEXT NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    id UUID NOT NULL,
    idempotency_key TEXT NOT NULL,
    principal_id TEXT NOT NULL,
    action_kind TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'proposed',
    amount_minor BIGINT NOT NULL CHECK (amount_minor >= 0),
    currency TEXT NOT NULL,
    counterparty JSONB,
    mandate JSONB,
    rail TEXT NOT NULL,
    memo TEXT,
    metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
    evidence JSONB NOT NULL DEFAULT '[]'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (workspace_id, id),
    UNIQUE (workspace_id, idempotency_key),
    CHECK (status IN ('proposed', 'authorized', 'held', 'executed', 'denied', 'failed', 'reversed', 'expired'))
);

CREATE INDEX IF NOT EXISTS financial_actions_workspace_status_created_idx
    ON financial_actions (workspace_id, status, created_at DESC);

CREATE INDEX IF NOT EXISTS financial_actions_workspace_principal_created_idx
    ON financial_actions (workspace_id, principal_id, created_at DESC);

CREATE TABLE IF NOT EXISTS financial_action_events (
    workspace_id TEXT NOT NULL,
    id UUID NOT NULL,
    action_id UUID NOT NULL,
    event_type TEXT NOT NULL,
    from_status TEXT,
    to_status TEXT,
    actor_id TEXT,
    reason TEXT,
    metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (workspace_id, id),
    FOREIGN KEY (workspace_id, action_id)
        REFERENCES financial_actions(workspace_id, id)
        ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS financial_action_events_action_created_idx
    ON financial_action_events (workspace_id, action_id, created_at ASC);

CREATE TABLE IF NOT EXISTS financial_ledger_entries (
    workspace_id TEXT NOT NULL,
    id UUID NOT NULL,
    action_id UUID NOT NULL,
    entry_kind TEXT NOT NULL,
    amount_minor BIGINT NOT NULL CHECK (amount_minor >= 0),
    currency TEXT NOT NULL,
    idempotency_key TEXT NOT NULL,
    metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
    effective_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (workspace_id, id),
    UNIQUE (workspace_id, idempotency_key),
    FOREIGN KEY (workspace_id, action_id)
        REFERENCES financial_actions(workspace_id, id)
        ON DELETE CASCADE,
    CHECK (entry_kind IN ('reserved', 'released', 'executed', 'reversed'))
);

CREATE INDEX IF NOT EXISTS financial_ledger_entries_window_idx
    ON financial_ledger_entries (workspace_id, currency, effective_at);

CREATE INDEX IF NOT EXISTS financial_ledger_entries_action_idx
    ON financial_ledger_entries (workspace_id, action_id);

CREATE TABLE IF NOT EXISTS mandates (
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
    PRIMARY KEY (workspace_id, id, version)
);

CREATE TABLE IF NOT EXISTS approval_requests (
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
        REFERENCES financial_actions(workspace_id, id)
        ON DELETE CASCADE,
    CHECK (status IN ('pending', 'approved', 'denied', 'expired', 'canceled'))
);

CREATE INDEX IF NOT EXISTS approval_requests_workspace_status_created_idx
    ON approval_requests (workspace_id, status, created_at DESC);

CREATE TABLE IF NOT EXISTS financial_receipts (
    workspace_id TEXT NOT NULL,
    id UUID NOT NULL,
    action_id UUID NOT NULL,
    trace_id UUID,
    ledger_event_ids JSONB NOT NULL DEFAULT '[]'::jsonb,
    proof JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (workspace_id, id),
    FOREIGN KEY (workspace_id, action_id)
        REFERENCES financial_actions(workspace_id, id)
        ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS financial_action_outcomes (
    workspace_id TEXT NOT NULL,
    id UUID NOT NULL,
    action_id UUID NOT NULL,
    status TEXT NOT NULL,
    reversal_capability TEXT NOT NULL,
    recovery_status TEXT NOT NULL,
    provider_status TEXT,
    provider_reference TEXT,
    final_loss_amount_minor BIGINT CHECK (final_loss_amount_minor >= 0),
    final_loss_currency TEXT,
    occurred_at TIMESTAMPTZ NOT NULL,
    metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (workspace_id, id),
    FOREIGN KEY (workspace_id, action_id)
        REFERENCES financial_actions(workspace_id, id)
        ON DELETE CASCADE,
    CHECK (status IN ('pending', 'succeeded', 'failed', 'canceled', 'reversed', 'recovery_started', 'recovered', 'disputed', 'loss_recorded', 'unknown')),
    CHECK (reversal_capability IN ('none', 'cancel_before_capture', 'cancel_pending_refund', 'provider_reversal', 'compensating_charge', 'internal_balance_adjustment', 'manual_recovery')),
    CHECK (recovery_status IN ('not_needed', 'not_available', 'available', 'started', 'recovered', 'failed', 'manual_required')),
    CHECK ((final_loss_amount_minor IS NULL AND final_loss_currency IS NULL) OR (final_loss_amount_minor IS NOT NULL AND final_loss_currency IS NOT NULL))
);

CREATE INDEX IF NOT EXISTS financial_action_outcomes_action_created_idx
    ON financial_action_outcomes (workspace_id, action_id, occurred_at DESC, created_at DESC);

CREATE TABLE IF NOT EXISTS counterparties (
    workspace_id TEXT NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    id TEXT NOT NULL,
    kind TEXT NOT NULL,
    display_name TEXT,
    country TEXT,
    metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (workspace_id, id)
);
