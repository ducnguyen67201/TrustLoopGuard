ALTER TABLE gateway_routes
    ADD COLUMN reliability_mode TEXT NOT NULL DEFAULT 'none',
    ADD COLUMN fallback_provider_connection_id TEXT;

ALTER TABLE gateway_routes
    ADD CONSTRAINT gateway_routes_reliability_mode_check
    CHECK (reliability_mode IN ('none', 'standard'));

ALTER TABLE gateway_routes
    ADD CONSTRAINT gateway_routes_fallback_provider_connection_fk
    FOREIGN KEY (workspace_id, fallback_provider_connection_id)
    REFERENCES gateway_provider_connections (workspace_id, id) ON DELETE RESTRICT;

CREATE UNIQUE INDEX runs_gateway_active_session_unique
    ON runs (workspace_id, environment_id, agent_id, external_id)
    WHERE status = 'running'
      AND external_id IS NOT NULL
      AND kind = 'chat_session'
      AND metadata @> '{"integration_mode":"gateway"}'::jsonb;

CREATE INDEX runs_gateway_active_activity_idx
    ON runs (COALESCE(last_evidence_at, started_at))
    WHERE status = 'running'
      AND kind = 'chat_session'
      AND metadata @> '{"integration_mode":"gateway"}'::jsonb;
