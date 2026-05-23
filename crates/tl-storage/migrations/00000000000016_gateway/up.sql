CREATE TABLE gateway_provider_connections (
    workspace_id TEXT NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    id TEXT NOT NULL,
    display_name TEXT NOT NULL,
    kind TEXT NOT NULL,
    base_url TEXT NULL,
    default_model TEXT NOT NULL,
    encrypted_api_key TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    deleted_at TIMESTAMPTZ NULL,
    PRIMARY KEY (workspace_id, id)
);

CREATE TABLE enforcement_profiles (
    workspace_id TEXT NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    id TEXT NOT NULL,
    display_name TEXT NOT NULL,
    input_action TEXT NOT NULL,
    output_action TEXT NOT NULL,
    fail_mode TEXT NOT NULL,
    retention_mode TEXT NOT NULL,
    fallback_message TEXT NOT NULL,
    max_regenerations INT NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    deleted_at TIMESTAMPTZ NULL,
    PRIMARY KEY (workspace_id, id)
);

CREATE TABLE gateway_routes (
    workspace_id TEXT NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    id TEXT NOT NULL,
    display_name TEXT NOT NULL,
    provider_connection_id TEXT NOT NULL,
    agent_id TEXT NOT NULL,
    enforcement_profile_id TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    deleted_at TIMESTAMPTZ NULL,
    PRIMARY KEY (workspace_id, id),
    FOREIGN KEY (workspace_id, provider_connection_id)
        REFERENCES gateway_provider_connections(workspace_id, id) ON DELETE RESTRICT,
    FOREIGN KEY (workspace_id, enforcement_profile_id)
        REFERENCES enforcement_profiles(workspace_id, id) ON DELETE RESTRICT
);

CREATE INDEX gateway_provider_connections_active_idx
    ON gateway_provider_connections (workspace_id, created_at DESC)
    WHERE deleted_at IS NULL;

CREATE INDEX enforcement_profiles_active_idx
    ON enforcement_profiles (workspace_id, created_at DESC)
    WHERE deleted_at IS NULL;

CREATE INDEX gateway_routes_active_idx
    ON gateway_routes (workspace_id, created_at DESC)
    WHERE deleted_at IS NULL;

CREATE INDEX gateway_routes_provider_connection_idx
    ON gateway_routes (workspace_id, provider_connection_id)
    WHERE deleted_at IS NULL;

CREATE INDEX gateway_routes_enforcement_profile_idx
    ON gateway_routes (workspace_id, enforcement_profile_id)
    WHERE deleted_at IS NULL;
