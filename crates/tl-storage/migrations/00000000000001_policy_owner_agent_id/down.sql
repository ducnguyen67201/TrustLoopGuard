DROP INDEX IF EXISTS policies_owner_agent_idx;
ALTER TABLE policies DROP COLUMN IF EXISTS owner_agent_id;
