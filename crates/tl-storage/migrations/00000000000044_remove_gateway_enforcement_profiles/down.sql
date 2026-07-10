CREATE TABLE enforcement_profiles (
    workspace_id TEXT NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    id TEXT NOT NULL,
    display_name TEXT NOT NULL,
    input_action TEXT NOT NULL,
    output_action TEXT NOT NULL,
    fail_mode TEXT NOT NULL,
    retention_mode TEXT NOT NULL,
    response_mode TEXT NOT NULL DEFAULT 'regular',
    fallback_message TEXT NOT NULL,
    max_regenerations INT NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    deleted_at TIMESTAMPTZ NULL,
    PRIMARY KEY (workspace_id, id)
);

INSERT INTO enforcement_profiles (
    workspace_id,
    id,
    display_name,
    input_action,
    output_action,
    fail_mode,
    retention_mode,
    response_mode,
    fallback_message,
    max_regenerations
)
SELECT DISTINCT
    workspace_id,
    'legacy-default',
    'Legacy default',
    'block',
    'block',
    'closed',
    'metadata_only',
    'regular',
    'Blocked by TrustLoopGuard.',
    0
FROM gateway_routes;

ALTER TABLE gateway_routes
    ADD COLUMN enforcement_profile_id TEXT;

UPDATE gateway_routes
SET enforcement_profile_id = 'legacy-default';

ALTER TABLE gateway_routes
    ALTER COLUMN enforcement_profile_id SET NOT NULL,
    ADD CONSTRAINT gateway_routes_workspace_id_enforcement_profile_id_fkey
        FOREIGN KEY (workspace_id, enforcement_profile_id)
        REFERENCES enforcement_profiles(workspace_id, id) ON DELETE RESTRICT;

CREATE INDEX enforcement_profiles_active_idx
    ON enforcement_profiles (workspace_id, created_at DESC)
    WHERE deleted_at IS NULL;

CREATE INDEX gateway_routes_enforcement_profile_idx
    ON gateway_routes (workspace_id, enforcement_profile_id)
    WHERE deleted_at IS NULL;
