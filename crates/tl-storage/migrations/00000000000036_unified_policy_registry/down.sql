DROP INDEX IF EXISTS policies_workspace_family_active_idx;

ALTER TABLE policies
    ALTER COLUMN family DROP DEFAULT;
