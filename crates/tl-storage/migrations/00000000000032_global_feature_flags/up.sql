CREATE TABLE IF NOT EXISTS global_feature_flags (
    key TEXT PRIMARY KEY,
    enabled BOOLEAN NOT NULL DEFAULT false,
    config JSONB NOT NULL DEFAULT '{}'::jsonb,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_by TEXT
);

INSERT INTO global_feature_flags (key, enabled, config, updated_by)
VALUES ('knowledge_grounding', false, '{}'::jsonb, 'migration')
ON CONFLICT (key) DO NOTHING;
