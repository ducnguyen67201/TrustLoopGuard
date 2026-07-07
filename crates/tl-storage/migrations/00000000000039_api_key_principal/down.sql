DROP INDEX IF EXISTS workspace_api_keys_principal_idx;

ALTER TABLE workspace_api_keys
    DROP COLUMN IF EXISTS principal_id;
