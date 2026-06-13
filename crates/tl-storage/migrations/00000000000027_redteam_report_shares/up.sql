-- Capability tokens granting public, read-only access to one red-team
-- vulnerability report (a single job, optionally a same-agent comparison run).
--
-- The token is a high-entropy random string and is the sole bearer credential
-- for the public report endpoint. `workspace_id` scopes the job lookup on read
-- but is never exposed to the public client. A share is valid while
-- `revoked_at` is null and `expires_at` is null or in the future.
CREATE TABLE IF NOT EXISTS redteam_report_shares (
    token          TEXT        NOT NULL PRIMARY KEY,
    workspace_id   TEXT        NOT NULL,
    job_id         UUID        NOT NULL,
    compare_job_id UUID,
    created_at     TIMESTAMPTZ NOT NULL DEFAULT now(),
    expires_at     TIMESTAMPTZ,
    revoked_at     TIMESTAMPTZ
);

-- Lists a workspace's shares newest-first (future revoke/management UI).
CREATE INDEX IF NOT EXISTS redteam_report_shares_workspace_idx
    ON redteam_report_shares (workspace_id, created_at DESC);
