ALTER TABLE traces DROP COLUMN workspace_id;

ALTER TABLE policies DROP CONSTRAINT IF EXISTS policies_owner_agent_workspace_fk;
DROP INDEX IF EXISTS policies_active_idx;
DROP INDEX IF EXISTS policies_enabled_idx;
DROP INDEX IF EXISTS policies_owner_agent_idx;
ALTER TABLE policies DROP CONSTRAINT policies_pkey;
ALTER TABLE policies ADD PRIMARY KEY (id);
CREATE INDEX policies_active_idx ON policies (id) WHERE deleted_at IS NULL;
CREATE INDEX policies_enabled_idx ON policies (enabled, id)
    WHERE deleted_at IS NULL AND enabled = TRUE;
CREATE INDEX policies_owner_agent_idx
    ON policies (owner_agent_id)
    WHERE deleted_at IS NULL AND owner_agent_id IS NOT NULL;
ALTER TABLE policies DROP COLUMN workspace_id;

DROP INDEX IF EXISTS agents_active_idx;
ALTER TABLE agents DROP CONSTRAINT agents_pkey;
ALTER TABLE agents ADD PRIMARY KEY (id);
CREATE INDEX agents_active_idx ON agents (id) WHERE deleted_at IS NULL;
ALTER TABLE agents DROP COLUMN workspace_id;

ALTER TABLE policies
    ADD CONSTRAINT policies_owner_agent_id_fkey
    FOREIGN KEY (owner_agent_id)
    REFERENCES agents(id)
    ON DELETE RESTRICT;
