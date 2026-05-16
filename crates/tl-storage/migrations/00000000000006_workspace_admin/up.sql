DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_type WHERE typname = 'organization_role') THEN
        CREATE TYPE organization_role AS ENUM ('owner', 'admin', 'member');
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_type WHERE typname = 'workspace_role') THEN
        CREATE TYPE workspace_role AS ENUM ('owner', 'admin', 'editor', 'viewer');
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_type WHERE typname = 'invite_status') THEN
        CREATE TYPE invite_status AS ENUM ('pending', 'accepted', 'revoked', 'expired');
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_type WHERE typname = 'api_key_status') THEN
        CREATE TYPE api_key_status AS ENUM ('active', 'revoked');
    END IF;
END $$;

CREATE TABLE IF NOT EXISTS organizations (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    slug TEXT NOT NULL UNIQUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS organization_members (
    organization_id TEXT NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    role organization_role NOT NULL DEFAULT 'member',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (organization_id, user_id)
);

CREATE INDEX IF NOT EXISTS organization_members_user_idx
    ON organization_members (user_id);

CREATE TABLE IF NOT EXISTS workspaces (
    id TEXT PRIMARY KEY,
    organization_id TEXT NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    slug TEXT NOT NULL,
    description TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    deleted_at TIMESTAMPTZ,
    UNIQUE (organization_id, slug)
);

CREATE INDEX IF NOT EXISTS workspaces_active_idx
    ON workspaces (organization_id, deleted_at);

CREATE TABLE IF NOT EXISTS workspace_members (
    workspace_id TEXT NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    role workspace_role NOT NULL DEFAULT 'viewer',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (workspace_id, user_id)
);

CREATE INDEX IF NOT EXISTS workspace_members_user_idx
    ON workspace_members (user_id);

CREATE TABLE IF NOT EXISTS workspace_invites (
    id TEXT PRIMARY KEY,
    workspace_id TEXT NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    email TEXT NOT NULL,
    role workspace_role NOT NULL DEFAULT 'viewer',
    status invite_status NOT NULL DEFAULT 'pending',
    invited_by_user_id UUID REFERENCES users(id) ON DELETE SET NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    expires_at TIMESTAMPTZ NOT NULL
);

CREATE INDEX IF NOT EXISTS workspace_invites_pending_email_idx
    ON workspace_invites (workspace_id, email, status);

CREATE TABLE IF NOT EXISTS workspace_settings (
    workspace_id TEXT PRIMARY KEY REFERENCES workspaces(id) ON DELETE CASCADE,
    default_action TEXT NOT NULL DEFAULT 'allow',
    escalation_webhook_url TEXT,
    telemetry_enabled BOOLEAN NOT NULL DEFAULT true,
    retention_days TEXT NOT NULL DEFAULT '30',
    config JSONB NOT NULL DEFAULT '{}'::jsonb,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS workspace_api_keys (
    id TEXT PRIMARY KEY,
    workspace_id TEXT NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    key_prefix TEXT NOT NULL,
    key_hash TEXT NOT NULL UNIQUE,
    status api_key_status NOT NULL DEFAULT 'active',
    created_by_user_id UUID REFERENCES users(id) ON DELETE SET NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    last_used_at TIMESTAMPTZ,
    revoked_at TIMESTAMPTZ
);

CREATE INDEX IF NOT EXISTS workspace_api_keys_status_idx
    ON workspace_api_keys (workspace_id, status);

CREATE INDEX IF NOT EXISTS workspace_api_keys_prefix_idx
    ON workspace_api_keys (key_prefix);
