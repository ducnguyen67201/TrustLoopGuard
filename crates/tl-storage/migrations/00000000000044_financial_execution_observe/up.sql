ALTER TABLE workspace_settings
    ADD COLUMN financial_action_mode TEXT NOT NULL DEFAULT 'enforce';

ALTER TABLE workspace_settings
    ADD CONSTRAINT workspace_settings_financial_action_mode_check
    CHECK (financial_action_mode IN ('observe', 'enforce'));

ALTER TABLE environment_checker_modes
    ADD COLUMN financial_action_mode TEXT NULL;

ALTER TABLE environment_checker_modes
    ADD CONSTRAINT environment_checker_modes_financial_action_mode_check
    CHECK (financial_action_mode IS NULL OR financial_action_mode IN ('observe', 'enforce'));

ALTER TABLE financial_actions ADD COLUMN environment_id TEXT;

UPDATE financial_actions AS action
SET environment_id = (
    SELECT environment.id
    FROM workspace_environments AS environment
    WHERE environment.workspace_id = action.workspace_id
      AND environment.deleted_at IS NULL
    ORDER BY environment.is_default DESC, environment.created_at ASC
    LIMIT 1
);

ALTER TABLE financial_actions ALTER COLUMN environment_id SET NOT NULL;
ALTER TABLE financial_actions
    ADD CONSTRAINT financial_actions_environment_fk
    FOREIGN KEY (workspace_id, environment_id)
    REFERENCES workspace_environments(workspace_id, id);

CREATE INDEX financial_actions_workspace_environment_created_idx
    ON financial_actions (workspace_id, environment_id, created_at DESC);

CREATE TABLE financial_action_evaluations (
    workspace_id TEXT NOT NULL,
    action_id UUID NOT NULL,
    environment_id TEXT NOT NULL,
    runtime_mode TEXT NOT NULL,
    outcome TEXT NOT NULL,
    reason TEXT NOT NULL,
    risks JSONB NOT NULL DEFAULT '[]'::jsonb,
    policy_ids JSONB NOT NULL DEFAULT '[]'::jsonb,
    amount_minor BIGINT NOT NULL CHECK (amount_minor >= 0),
    currency TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (workspace_id, action_id),
    FOREIGN KEY (workspace_id, action_id)
        REFERENCES financial_actions(workspace_id, id) ON DELETE CASCADE,
    FOREIGN KEY (workspace_id, environment_id)
        REFERENCES workspace_environments(workspace_id, id),
    CHECK (runtime_mode IN ('observe', 'enforce')),
    CHECK (outcome IN ('allow', 'hold', 'block', 'would_allow', 'would_hold', 'would_block'))
);

CREATE INDEX financial_action_evaluations_observation_idx
    ON financial_action_evaluations
       (workspace_id, environment_id, runtime_mode, outcome, created_at DESC);

CREATE TABLE financial_observation_reviews (
    workspace_id TEXT NOT NULL,
    id UUID NOT NULL,
    action_id UUID NOT NULL,
    outcome TEXT NOT NULL,
    note TEXT,
    reviewed_by TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (workspace_id, id),
    FOREIGN KEY (workspace_id, action_id)
        REFERENCES financial_action_evaluations(workspace_id, action_id) ON DELETE CASCADE,
    CHECK (outcome IN ('confirmed_risk', 'false_positive'))
);

CREATE INDEX financial_observation_reviews_action_created_idx
    ON financial_observation_reviews (workspace_id, action_id, created_at DESC, id DESC);

CREATE TABLE financial_execution_grants (
    workspace_id TEXT NOT NULL,
    id UUID NOT NULL,
    action_id UUID NOT NULL,
    action_hash TEXT NOT NULL,
    binding TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'issued',
    expires_at TIMESTAMPTZ NOT NULL,
    claim_id UUID,
    claimed_at TIMESTAMPTZ,
    commit_idempotency_key TEXT,
    attestation_hash TEXT,
    committed_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (workspace_id, id),
    UNIQUE (workspace_id, action_id),
    FOREIGN KEY (workspace_id, action_id)
        REFERENCES financial_actions(workspace_id, id) ON DELETE CASCADE,
    CHECK (binding IN ('managed_executor', 'external_attestation')),
    CHECK (status IN ('issued', 'claimed', 'committed', 'failed', 'expired')),
    CHECK ((status = 'claimed') = (claim_id IS NOT NULL AND claimed_at IS NOT NULL)),
    CHECK ((status = 'committed') = (committed_at IS NOT NULL))
);

CREATE INDEX financial_execution_grants_status_expiry_idx
    ON financial_execution_grants (workspace_id, status, expires_at);

CREATE TABLE financial_execution_connectors (
    workspace_id TEXT NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    id UUID NOT NULL,
    display_name TEXT NOT NULL,
    encrypted_secret TEXT NOT NULL,
    allowed_rails JSONB NOT NULL DEFAULT '[]'::jsonb,
    allowed_operations JSONB NOT NULL DEFAULT '[]'::jsonb,
    status TEXT NOT NULL DEFAULT 'active',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    revoked_at TIMESTAMPTZ,
    PRIMARY KEY (workspace_id, id),
    CHECK (status IN ('active', 'revoked')),
    CHECK ((status = 'revoked') = (revoked_at IS NOT NULL))
);

CREATE INDEX financial_execution_connectors_status_idx
    ON financial_execution_connectors (workspace_id, status, created_at DESC);
