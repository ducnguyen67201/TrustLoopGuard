ALTER TABLE gateway_routes
    ADD COLUMN reliability_mode TEXT NOT NULL DEFAULT 'none';

ALTER TABLE gateway_routes
    ADD CONSTRAINT gateway_routes_reliability_mode_check
    CHECK (reliability_mode IN ('none', 'standard'));

CREATE TABLE gateway_route_fallbacks (
    workspace_id TEXT NOT NULL,
    route_id TEXT NOT NULL,
    position INT NOT NULL,
    provider_connection_id TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (workspace_id, route_id, position),
    UNIQUE (workspace_id, route_id, provider_connection_id),
    FOREIGN KEY (workspace_id, route_id)
        REFERENCES gateway_routes (workspace_id, id) ON DELETE CASCADE,
    FOREIGN KEY (workspace_id, provider_connection_id)
        REFERENCES gateway_provider_connections (workspace_id, id) ON DELETE RESTRICT,
    CHECK (position BETWEEN 1 AND 8)
);

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
