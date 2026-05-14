-- Attach policies to the agent that owns them.
--
-- Populated by `POST /v1/agents/{id}/guardrails:generate`; consumed by
-- `GET /v1/agents/{id}/guardrails` and the cascade-delete handler.
--
-- The FK uses ON DELETE RESTRICT because the application layer never
-- hard-deletes `agents` rows — it sets `deleted_at`. RESTRICT means a
-- hard-delete attempted out-of-band fails loudly instead of silently
-- nuking unrelated policies. The soft-delete cascade is implemented in
-- the server's DELETE /v1/agents/{id} handler, not by the FK.

ALTER TABLE policies
    ADD COLUMN owner_agent_id TEXT NULL
        REFERENCES agents(id) ON DELETE RESTRICT;

-- Partial index: cascade-delete lookups + `GET /agents/{id}/guardrails`
-- only care about active rows owned by some agent. Most policies are
-- global (NULL owner_agent_id) so a partial index keeps it tight.
CREATE INDEX policies_owner_agent_idx
    ON policies (owner_agent_id)
    WHERE deleted_at IS NULL AND owner_agent_id IS NOT NULL;
