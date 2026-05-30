DROP INDEX IF EXISTS policy_environment_deployments_enabled_idx;
DROP INDEX IF EXISTS traces_workspace_environment_created_idx;
DROP INDEX IF EXISTS runs_workspace_environment_created_idx;
DROP INDEX IF EXISTS workspace_api_keys_environment_status_idx;

ALTER TABLE runs DROP CONSTRAINT IF EXISTS runs_environment_fk;
ALTER TABLE traces DROP CONSTRAINT IF EXISTS traces_environment_fk;
ALTER TABLE workspace_api_keys DROP CONSTRAINT IF EXISTS workspace_api_keys_environment_fk;

DROP TABLE IF EXISTS policy_environment_deployments;

ALTER TABLE traces DROP COLUMN IF EXISTS environment_id;
ALTER TABLE runs DROP COLUMN IF EXISTS environment_id;
ALTER TABLE workspace_api_keys DROP COLUMN IF EXISTS environment_id;

DROP INDEX IF EXISTS workspace_environments_one_default_idx;
DROP INDEX IF EXISTS workspace_environments_active_slug_idx;
DROP TABLE IF EXISTS workspace_environments;
