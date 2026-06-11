-- Enum-backed TEXT columns are written only through the Rust repos, so
-- invalid values previously surfaced as runtime deserialize errors.
-- Enforce the closed value sets at the database too, so rows written
-- outside the repos are rejected at insert instead.
ALTER TABLE tool_metadata
    ADD CONSTRAINT tool_metadata_side_effect_check CHECK (side_effect IN (
        'none', 'read', 'external_communication', 'file_write', 'shell_exec',
        'network_call', 'db_mutation', 'api_mutation', 'memory_write', 'publish'
    ));

ALTER TABLE source_label_policy
    ADD CONSTRAINT source_label_policy_origin_check CHECK (origin IN (
        'user', 'system', 'tool', 'memory', 'file', 'web', 'email', 'api', 'unknown'
    ));
