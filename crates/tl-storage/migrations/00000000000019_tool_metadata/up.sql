-- Workspace-scoped tool metadata registry (event engine action resolution).
-- spec holds the full serialized ToolMetadata; side_effect/reversible are
-- promoted for queries. enabled=false hides a tool from runtime resolution
-- without losing its definition.
CREATE TABLE tool_metadata (
    workspace_id TEXT        NOT NULL,
    tool         TEXT        NOT NULL,
    side_effect  TEXT        NOT NULL,
    reversible   BOOLEAN     NOT NULL,
    spec         JSONB       NOT NULL,
    enabled      BOOLEAN     NOT NULL DEFAULT TRUE,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    deleted_at   TIMESTAMPTZ,
    PRIMARY KEY (workspace_id, tool)
);

CREATE INDEX tool_metadata_active_idx
    ON tool_metadata (workspace_id, tool)
    WHERE deleted_at IS NULL;
