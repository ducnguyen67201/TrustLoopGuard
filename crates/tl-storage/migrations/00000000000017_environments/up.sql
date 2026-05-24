CREATE TABLE IF NOT EXISTS workspace_environments (
    workspace_id TEXT NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    id TEXT NOT NULL,
    slug TEXT NOT NULL,
    name TEXT NOT NULL,
    description TEXT,
    is_default BOOLEAN NOT NULL DEFAULT false,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    deleted_at TIMESTAMPTZ,
    PRIMARY KEY (workspace_id, id)
);

CREATE UNIQUE INDEX IF NOT EXISTS workspace_environments_active_slug_idx
    ON workspace_environments (workspace_id, slug)
    WHERE deleted_at IS NULL;

CREATE UNIQUE INDEX IF NOT EXISTS workspace_environments_one_default_idx
    ON workspace_environments (workspace_id)
    WHERE deleted_at IS NULL AND is_default;

INSERT INTO workspace_environments (
    workspace_id,
    id,
    slug,
    name,
    is_default
)
SELECT
    workspaces.id,
    'production',
    'production',
    'Production',
    true
FROM workspaces
ON CONFLICT (workspace_id, id) DO NOTHING;

ALTER TABLE workspace_api_keys
    ADD COLUMN IF NOT EXISTS environment_id TEXT NOT NULL DEFAULT 'production';

ALTER TABLE runs
    ADD COLUMN IF NOT EXISTS environment_id TEXT NOT NULL DEFAULT 'production';

ALTER TABLE traces
    ADD COLUMN IF NOT EXISTS environment_id TEXT NOT NULL DEFAULT 'production';

CREATE TABLE IF NOT EXISTS policy_environment_deployments (
    workspace_id TEXT NOT NULL,
    environment_id TEXT NOT NULL,
    policy_id TEXT NOT NULL,
    enabled BOOLEAN NOT NULL DEFAULT false,
    deployed_version INTEGER,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (workspace_id, environment_id, policy_id),
    FOREIGN KEY (workspace_id, environment_id)
        REFERENCES workspace_environments (workspace_id, id)
        ON DELETE CASCADE,
    FOREIGN KEY (workspace_id, policy_id)
        REFERENCES policies (workspace_id, id)
        ON DELETE CASCADE
);

INSERT INTO policy_environment_deployments (
    workspace_id,
    environment_id,
    policy_id,
    enabled
)
SELECT
    policies.workspace_id,
    'production',
    policies.id,
    policies.enabled
FROM policies
JOIN workspace_environments
    ON workspace_environments.workspace_id = policies.workspace_id
   AND workspace_environments.id = 'production'
WHERE policies.deleted_at IS NULL
ON CONFLICT (workspace_id, environment_id, policy_id) DO UPDATE SET
    enabled = EXCLUDED.enabled,
    updated_at = now();

ALTER TABLE workspace_api_keys
    ADD CONSTRAINT workspace_api_keys_environment_fk
    FOREIGN KEY (workspace_id, environment_id)
    REFERENCES workspace_environments (workspace_id, id)
    ON DELETE RESTRICT;

ALTER TABLE runs
    ADD CONSTRAINT runs_environment_fk
    FOREIGN KEY (workspace_id, environment_id)
    REFERENCES workspace_environments (workspace_id, id)
    ON DELETE RESTRICT;

CREATE INDEX IF NOT EXISTS workspace_api_keys_environment_status_idx
    ON workspace_api_keys (workspace_id, environment_id, status);

CREATE INDEX IF NOT EXISTS runs_workspace_environment_created_idx
    ON runs (workspace_id, environment_id, created_at DESC);

CREATE INDEX IF NOT EXISTS traces_workspace_environment_created_idx
    ON traces (workspace_id, environment_id, created_at DESC);

CREATE INDEX IF NOT EXISTS policy_environment_deployments_enabled_idx
    ON policy_environment_deployments (workspace_id, environment_id, enabled, policy_id);
