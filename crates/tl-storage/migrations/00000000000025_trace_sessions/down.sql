DROP INDEX IF EXISTS traces_workspace_session_created_idx;

ALTER TABLE traces
    DROP COLUMN IF EXISTS session_id;
