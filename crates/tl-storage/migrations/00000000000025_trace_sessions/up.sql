-- Promote the monitoring session id from the trace JSON payload to a
-- queryable column. Session ids are SDK-formatted opaque strings
-- (`sess_<uuid>`), deliberately TEXT rather than UUID: the server never
-- parses them.
ALTER TABLE traces
    ADD COLUMN IF NOT EXISTS session_id TEXT;

CREATE INDEX IF NOT EXISTS traces_workspace_session_created_idx
    ON traces (workspace_id, session_id, created_at DESC)
    WHERE session_id IS NOT NULL;
