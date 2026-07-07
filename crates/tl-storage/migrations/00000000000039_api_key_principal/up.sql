ALTER TABLE workspace_api_keys
    ADD COLUMN IF NOT EXISTS principal_id TEXT;

CREATE INDEX IF NOT EXISTS workspace_api_keys_principal_idx
    ON workspace_api_keys (workspace_id, principal_id)
    WHERE principal_id IS NOT NULL;
