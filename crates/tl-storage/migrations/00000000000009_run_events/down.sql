DROP INDEX IF EXISTS traces_workspace_run_event_created_idx;

ALTER TABLE traces
    DROP COLUMN IF EXISTS run_event_id;

DROP INDEX IF EXISTS run_events_workspace_run_occurred_idx;
DROP INDEX IF EXISTS run_events_workspace_run_sequence_idx;
DROP TABLE IF EXISTS run_events;
