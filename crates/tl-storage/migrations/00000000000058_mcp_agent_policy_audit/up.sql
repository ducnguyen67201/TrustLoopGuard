CREATE TABLE mcp_agent_tool_assignments (
    workspace_id TEXT NOT NULL,
    tool_id UUID NOT NULL,
    user_id UUID NOT NULL,
    agent_id TEXT NOT NULL,
    created_by UUID NULL REFERENCES users(id) ON DELETE SET NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (workspace_id, tool_id, user_id, agent_id),
    FOREIGN KEY (workspace_id, tool_id)
        REFERENCES mcp_tools(workspace_id, id) ON DELETE CASCADE,
    FOREIGN KEY (workspace_id, user_id)
        REFERENCES workspace_members(workspace_id, user_id) ON DELETE CASCADE,
    FOREIGN KEY (workspace_id, agent_id)
        REFERENCES agents(workspace_id, id) ON DELETE CASCADE
);

CREATE INDEX mcp_agent_tool_assignments_member_idx
    ON mcp_agent_tool_assignments (workspace_id, user_id, agent_id, tool_id);
CREATE INDEX mcp_agent_tool_assignments_tool_idx
    ON mcp_agent_tool_assignments (workspace_id, tool_id, agent_id, user_id);

ALTER TABLE mcp_oauth_authorization_codes
    ADD COLUMN agent_id TEXT NULL;
ALTER TABLE mcp_oauth_refresh_tokens
    ADD COLUMN agent_id TEXT NULL;

ALTER TABLE mcp_oauth_authorization_codes
    ADD CONSTRAINT mcp_oauth_authorization_codes_agent_fk
    FOREIGN KEY (workspace_id, agent_id)
    REFERENCES agents(workspace_id, id) ON DELETE CASCADE;
ALTER TABLE mcp_oauth_refresh_tokens
    ADD CONSTRAINT mcp_oauth_refresh_tokens_agent_fk
    FOREIGN KEY (workspace_id, agent_id)
    REFERENCES agents(workspace_id, id) ON DELETE CASCADE;

ALTER TABLE authorization_receipts
    ADD COLUMN principal_id TEXT NULL,
    ADD COLUMN operation TEXT NULL,
    ADD COLUMN run_id UUID NULL;

UPDATE authorization_receipts AS receipt
SET principal_id = intent.principal_id,
    operation = intent.operation
FROM authorization_intents AS intent
WHERE receipt.workspace_id = intent.workspace_id
  AND receipt.environment_id = intent.environment_id
  AND receipt.intent_id = intent.id;

UPDATE authorization_receipts AS receipt
SET run_id = trace.run_id
FROM traces AS trace
WHERE receipt.workspace_id = trace.workspace_id
  AND receipt.environment_id = trace.environment_id
  AND receipt.trace_id = trace.trace_id::text
  AND trace.run_id IS NOT NULL;

CREATE INDEX authorization_receipts_created_idx
    ON authorization_receipts (workspace_id, environment_id, created_at DESC);
CREATE INDEX authorization_receipts_principal_created_idx
    ON authorization_receipts (workspace_id, environment_id, principal_id, created_at DESC);
