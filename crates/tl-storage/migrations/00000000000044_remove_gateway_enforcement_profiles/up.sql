DROP INDEX IF EXISTS gateway_routes_enforcement_profile_idx;

ALTER TABLE gateway_routes
    DROP CONSTRAINT IF EXISTS gateway_routes_workspace_id_enforcement_profile_id_fkey,
    DROP COLUMN enforcement_profile_id;

DROP TABLE enforcement_profiles;
