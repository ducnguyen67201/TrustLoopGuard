-- Scope runtime authoring and traces by workspace.
--
-- Existing self-hosted rows land in the backwards-compatible `default`
-- workspace so older clients keep working until they send workspace context.

ALTER TABLE agents
    ADD COLUMN workspace_id TEXT NOT NULL DEFAULT 'default';

ALTER TABLE policies DROP CONSTRAINT IF EXISTS policies_owner_agent_id_fkey;
DROP INDEX IF EXISTS agents_active_idx;
ALTER TABLE agents DROP CONSTRAINT agents_pkey;
ALTER TABLE agents ADD PRIMARY KEY (workspace_id, id);
CREATE INDEX agents_active_idx ON agents (workspace_id, id) WHERE deleted_at IS NULL;

ALTER TABLE policies
    ADD COLUMN workspace_id TEXT NOT NULL DEFAULT 'default';

DROP INDEX IF EXISTS policies_active_idx;
DROP INDEX IF EXISTS policies_enabled_idx;
DROP INDEX IF EXISTS policies_owner_agent_idx;
ALTER TABLE policies DROP CONSTRAINT policies_pkey;
ALTER TABLE policies ADD PRIMARY KEY (workspace_id, id);
ALTER TABLE policies
    ADD CONSTRAINT policies_owner_agent_workspace_fk
    FOREIGN KEY (workspace_id, owner_agent_id)
    REFERENCES agents(workspace_id, id)
    ON DELETE RESTRICT;
CREATE INDEX policies_active_idx ON policies (workspace_id, id) WHERE deleted_at IS NULL;
CREATE INDEX policies_enabled_idx ON policies (workspace_id, enabled, id)
    WHERE deleted_at IS NULL AND enabled = TRUE;
CREATE INDEX policies_owner_agent_idx
    ON policies (workspace_id, owner_agent_id)
    WHERE deleted_at IS NULL AND owner_agent_id IS NOT NULL;

ALTER TABLE traces
    ADD COLUMN workspace_id TEXT NOT NULL DEFAULT 'default';

CREATE INDEX traces_workspace_decision_idx ON traces (workspace_id, decision, created_at DESC);
CREATE INDEX traces_workspace_domain_idx ON traces (workspace_id, domain, created_at DESC);
