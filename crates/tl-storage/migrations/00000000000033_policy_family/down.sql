DROP INDEX IF EXISTS policies_family_idx;
ALTER TABLE policies DROP COLUMN IF EXISTS family;
