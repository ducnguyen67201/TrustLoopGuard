-- GitHub-assisted agent installation control-plane state (Rust-owned).
CREATE TABLE IF NOT EXISTS github_installation_states (
    state_hash   BYTEA       PRIMARY KEY,
    workspace_id TEXT        NOT NULL,
    user_id      UUID        NOT NULL,
    expires_at   TIMESTAMPTZ NOT NULL,
    consumed_at  TIMESTAMPTZ,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS github_installation_states_workspace_created_idx
    ON github_installation_states (workspace_id, created_at DESC);

CREATE TABLE IF NOT EXISTS github_installations (
    workspace_id          TEXT        NOT NULL,
    id                    UUID        NOT NULL,
    installation_id       BIGINT      NOT NULL,
    account_login         TEXT        NOT NULL,
    account_type          TEXT        NOT NULL,
    repository_selection  TEXT        NOT NULL,
    status                TEXT        NOT NULL DEFAULT 'active',
    installed_by_user_id  UUID        NOT NULL,
    created_at            TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at            TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (workspace_id, id),
    UNIQUE (workspace_id, installation_id)
);

CREATE INDEX IF NOT EXISTS github_installations_github_id_idx
    ON github_installations (installation_id);

CREATE TABLE IF NOT EXISTS github_repository_connections (
    workspace_id     TEXT        NOT NULL,
    id               UUID        NOT NULL,
    installation_id  UUID        NOT NULL,
    repository_id    BIGINT      NOT NULL,
    owner            TEXT        NOT NULL,
    name             TEXT        NOT NULL,
    default_branch   TEXT        NOT NULL,
    root_path        TEXT        NOT NULL,
    agent_id         TEXT        NOT NULL,
    environment_id   TEXT        NOT NULL,
    status           TEXT        NOT NULL DEFAULT 'active',
    recipe_version   TEXT        NOT NULL,
    created_at       TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at       TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (workspace_id, id),
    CONSTRAINT github_repository_connections_installation_fk
        FOREIGN KEY (workspace_id, installation_id)
        REFERENCES github_installations (workspace_id, id)
        ON DELETE CASCADE,
    CONSTRAINT github_repository_connections_agent_fk
        FOREIGN KEY (workspace_id, agent_id)
        REFERENCES agents (workspace_id, id)
        ON DELETE RESTRICT,
    CONSTRAINT github_repository_connections_environment_fk
        FOREIGN KEY (workspace_id, environment_id)
        REFERENCES workspace_environments (workspace_id, id)
        ON DELETE RESTRICT
);

CREATE UNIQUE INDEX IF NOT EXISTS github_repository_connections_active_unique_idx
    ON github_repository_connections (
        workspace_id,
        repository_id,
        root_path,
        agent_id,
        environment_id
    )
    WHERE status <> 'disconnected';

CREATE INDEX IF NOT EXISTS github_repository_connections_agent_idx
    ON github_repository_connections (workspace_id, agent_id, created_at DESC);

CREATE TABLE IF NOT EXISTS github_integration_jobs (
    workspace_id              TEXT        NOT NULL,
    id                        UUID        NOT NULL,
    connection_id             UUID        NOT NULL,
    status                    TEXT        NOT NULL DEFAULT 'queued',
    risk_statement            TEXT        NOT NULL,
    base_branch               TEXT        NOT NULL,
    base_sha                  TEXT,
    recipe_version            TEXT        NOT NULL,
    analysis_summary          JSONB,
    proposed_changes          JSONB       NOT NULL DEFAULT '[]'::jsonb,
    manual_steps              JSONB       NOT NULL DEFAULT '[]'::jsonb,
    branch_name               TEXT,
    commit_sha                TEXT,
    pull_request_number       BIGINT,
    pull_request_url          TEXT,
    error_code                TEXT,
    error_message             TEXT,
    attempt_count             INTEGER     NOT NULL DEFAULT 0,
    installation_connected_at TIMESTAMPTZ,
    repository_connected_at   TIMESTAMPTZ,
    analysis_completed_at     TIMESTAMPTZ,
    pr_opened_at              TIMESTAMPTZ,
    pr_merged_at              TIMESTAMPTZ,
    first_verified_trace_at   TIMESTAMPTZ,
    created_at                TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at                TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (workspace_id, id),
    CONSTRAINT github_integration_jobs_connection_fk
        FOREIGN KEY (workspace_id, connection_id)
        REFERENCES github_repository_connections (workspace_id, id)
        ON DELETE CASCADE
);

CREATE UNIQUE INDEX IF NOT EXISTS github_integration_jobs_one_active_idx
    ON github_integration_jobs (workspace_id, connection_id)
    WHERE status NOT IN ('verified', 'closed_unmerged', 'error', 'cancelled');

CREATE INDEX IF NOT EXISTS github_integration_jobs_connection_created_idx
    ON github_integration_jobs (workspace_id, connection_id, created_at DESC);

CREATE INDEX IF NOT EXISTS github_integration_jobs_recoverable_idx
    ON github_integration_jobs (status, updated_at)
    WHERE status IN ('queued', 'analyzing', 'applying');

CREATE INDEX IF NOT EXISTS traces_github_integration_marker_idx
    ON traces (
        workspace_id,
        environment_id,
        ((payload #>> '{event,principal,agent_id}')),
        ((payload #>> '{event,context,tlg_integration_id}')),
        created_at DESC
    );
