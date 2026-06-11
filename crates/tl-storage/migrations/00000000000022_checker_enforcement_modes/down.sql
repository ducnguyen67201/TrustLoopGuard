ALTER TABLE workspace_settings DROP CONSTRAINT IF EXISTS workspace_settings_param_checker_mode_check;
ALTER TABLE workspace_settings DROP CONSTRAINT IF EXISTS workspace_settings_memory_checker_mode_check;
ALTER TABLE workspace_settings DROP CONSTRAINT IF EXISTS workspace_settings_flow_checker_mode_check;
ALTER TABLE workspace_settings DROP COLUMN IF EXISTS param_checker_mode;
ALTER TABLE workspace_settings DROP COLUMN IF EXISTS memory_checker_mode;
ALTER TABLE workspace_settings DROP COLUMN IF EXISTS flow_checker_mode;
