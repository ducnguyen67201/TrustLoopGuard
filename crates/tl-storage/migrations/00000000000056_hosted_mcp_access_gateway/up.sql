ALTER TABLE workspaces
    ADD COLUMN is_mcp_gateway_enabled BOOLEAN NOT NULL DEFAULT false;

CREATE TABLE mcp_server_connections (
    workspace_id TEXT NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    id UUID NOT NULL,
    display_name TEXT NOT NULL,
    server_slug TEXT NOT NULL,
    endpoint_url TEXT NOT NULL,
    auth_kind TEXT NOT NULL CHECK (auth_kind IN ('none', 'static_bearer')),
    encrypted_credential TEXT NULL,
    enabled BOOLEAN NOT NULL DEFAULT true,
    last_sync_status TEXT NOT NULL DEFAULT 'never'
        CHECK (last_sync_status IN ('never', 'succeeded', 'failed')),
    last_sync_error TEXT NULL,
    last_synced_at TIMESTAMPTZ NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (workspace_id, id),
    UNIQUE (workspace_id, server_slug)
);

CREATE TABLE mcp_tools (
    workspace_id TEXT NOT NULL,
    id UUID NOT NULL,
    connection_id UUID NOT NULL,
    upstream_name TEXT NOT NULL,
    public_name TEXT NOT NULL,
    title TEXT NULL,
    description TEXT NULL,
    input_schema JSONB NOT NULL,
    output_schema JSONB NULL,
    annotations JSONB NOT NULL DEFAULT '{}'::jsonb,
    schema_hash TEXT NOT NULL,
    side_effect TEXT NOT NULL DEFAULT 'api_mutation',
    catalog_status TEXT NOT NULL DEFAULT 'active'
        CHECK (catalog_status IN ('active', 'schema_changed', 'missing')),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (workspace_id, id),
    FOREIGN KEY (workspace_id, connection_id)
        REFERENCES mcp_server_connections(workspace_id, id) ON DELETE CASCADE,
    UNIQUE (workspace_id, connection_id, upstream_name),
    UNIQUE (workspace_id, public_name)
);

CREATE TABLE mcp_tool_assignments (
    workspace_id TEXT NOT NULL,
    tool_id UUID NOT NULL,
    user_id UUID NOT NULL,
    created_by UUID NULL REFERENCES users(id) ON DELETE SET NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (workspace_id, tool_id, user_id),
    FOREIGN KEY (workspace_id, tool_id)
        REFERENCES mcp_tools(workspace_id, id) ON DELETE CASCADE,
    FOREIGN KEY (workspace_id, user_id)
        REFERENCES workspace_members(workspace_id, user_id) ON DELETE CASCADE
);

CREATE INDEX mcp_tool_assignments_member_idx
    ON mcp_tool_assignments (workspace_id, user_id, tool_id);
CREATE INDEX mcp_tools_public_name_idx
    ON mcp_tools (workspace_id, public_name);
CREATE INDEX mcp_tools_connection_status_idx
    ON mcp_tools (workspace_id, connection_id, catalog_status);
CREATE INDEX mcp_server_connections_status_idx
    ON mcp_server_connections (workspace_id, enabled, last_sync_status);
