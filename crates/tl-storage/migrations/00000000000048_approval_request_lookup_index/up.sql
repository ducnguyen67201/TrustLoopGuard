CREATE INDEX CONCURRENTLY IF NOT EXISTS approval_requests_workspace_action_status_expiry_idx
    ON approval_requests (workspace_id, action_id, status, expires_at);
