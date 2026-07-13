-- Serialize action-ledger budget admission across replicas. Currency is
-- included because financial policy windows are evaluated per currency.
CREATE TABLE financial_budget_principal_locks (
    workspace_id TEXT NOT NULL,
    principal_id TEXT NOT NULL,
    currency TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (workspace_id, principal_id, currency)
);

CREATE INDEX approval_requests_workspace_action_status_expiry_idx
    ON approval_requests (workspace_id, action_id, status, expires_at);
