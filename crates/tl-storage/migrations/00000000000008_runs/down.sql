DROP INDEX IF EXISTS traces_workspace_run_created_idx;

ALTER TABLE traces
    DROP COLUMN IF EXISTS run_id;

DROP INDEX IF EXISTS runs_workspace_external_idx;
DROP INDEX IF EXISTS runs_workspace_agent_created_idx;
DROP INDEX IF EXISTS runs_workspace_created_idx;
DROP TABLE IF EXISTS runs;
