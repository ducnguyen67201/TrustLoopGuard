-- Workspace-scoped per-origin label overrides (event engine label
-- resolution). spec holds the full serialized SourceLabelPolicy.
-- enabled=false hides a row from runtime resolution without losing it.
CREATE TABLE source_label_policy (
    workspace_id TEXT        NOT NULL,
    origin       TEXT        NOT NULL,
    spec         JSONB       NOT NULL,
    enabled      BOOLEAN     NOT NULL DEFAULT TRUE,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    deleted_at   TIMESTAMPTZ,
    PRIMARY KEY (workspace_id, origin)
);

CREATE INDEX source_label_policy_active_idx
    ON source_label_policy (workspace_id)
    WHERE deleted_at IS NULL;
