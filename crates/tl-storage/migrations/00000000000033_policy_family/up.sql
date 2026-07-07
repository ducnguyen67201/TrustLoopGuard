-- Family policies share the policies table; the
-- `family` tag distinguishes them from content policies. NULL = content.
ALTER TABLE policies ADD COLUMN IF NOT EXISTS family TEXT;

CREATE INDEX IF NOT EXISTS policies_family_idx
    ON policies (workspace_id, family)
    WHERE family IS NOT NULL AND deleted_at IS NULL;
