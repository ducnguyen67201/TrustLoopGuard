DROP TABLE IF EXISTS financial_execution_connectors;
DROP TABLE IF EXISTS financial_execution_grants;
DROP TABLE IF EXISTS financial_observation_reviews;
DROP TABLE IF EXISTS financial_action_evaluations;

DROP INDEX IF EXISTS financial_actions_workspace_environment_created_idx;
ALTER TABLE financial_actions DROP CONSTRAINT IF EXISTS financial_actions_environment_fk;
ALTER TABLE financial_actions DROP COLUMN IF EXISTS environment_id;

ALTER TABLE environment_checker_modes
    DROP CONSTRAINT IF EXISTS environment_checker_modes_financial_action_mode_check;
ALTER TABLE environment_checker_modes DROP COLUMN IF EXISTS financial_action_mode;

ALTER TABLE workspace_settings
    DROP CONSTRAINT IF EXISTS workspace_settings_financial_action_mode_check;
ALTER TABLE workspace_settings DROP COLUMN IF EXISTS financial_action_mode;
