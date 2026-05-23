CREATE TABLE IF NOT EXISTS analytics_dashboard_views (
    workspace_id TEXT NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    id TEXT NOT NULL,
    name TEXT NOT NULL,
    is_default BOOLEAN NOT NULL DEFAULT FALSE,
    config JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (workspace_id, id),
    UNIQUE (workspace_id, name)
);

CREATE UNIQUE INDEX IF NOT EXISTS analytics_dashboard_views_one_default
    ON analytics_dashboard_views (workspace_id)
    WHERE is_default;

CREATE OR REPLACE FUNCTION set_updated_at()
RETURNS trigger AS $$
BEGIN
    NEW.updated_at = now();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER set_analytics_dashboard_views_updated_at
BEFORE UPDATE ON analytics_dashboard_views
FOR EACH ROW EXECUTE FUNCTION set_updated_at();
