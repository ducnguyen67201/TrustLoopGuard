CREATE TABLE source_label_policy (
    workspace_id TEXT        NOT NULL,
    origin       TEXT        NOT NULL,
    spec         JSONB       NOT NULL,
    enabled      BOOLEAN     NOT NULL DEFAULT TRUE,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    deleted_at   TIMESTAMPTZ,
    PRIMARY KEY (workspace_id, origin),
    CONSTRAINT source_label_policy_origin_spec_consistent
        CHECK ((spec ->> 'origin') = origin)
);

CREATE INDEX source_label_policy_active_idx
    ON source_label_policy (workspace_id)
    WHERE deleted_at IS NULL;

ALTER TABLE source_label_policy
    ADD CONSTRAINT source_label_policy_origin_check CHECK (origin IN (
        'user', 'system', 'tool', 'memory', 'file', 'web', 'email', 'api', 'unknown'
    ));

INSERT INTO source_label_policy (
    workspace_id,
    origin,
    spec,
    enabled,
    created_at,
    updated_at,
    deleted_at
)
SELECT
    policies.workspace_id,
    policies.parsed_policy ->> 'origin',
    jsonb_strip_nulls(jsonb_build_object(
        'origin', policies.parsed_policy ->> 'origin',
        'trust', policies.parsed_policy ->> 'trust',
        'confidentiality', policies.parsed_policy ->> 'confidentiality',
        'integrity', policies.parsed_policy ->> 'integrity'
    )),
    policies.enabled,
    policies.created_at,
    policies.updated_at,
    policies.deleted_at
FROM policies
WHERE policies.family = 'source_label'
  AND policies.parsed_policy ? 'origin';

DELETE FROM entity_versions
WHERE entity_type = 'policy'
  AND entity_id IN (
      SELECT id
      FROM policies
      WHERE family = 'source_label'
  );

DELETE FROM policies
WHERE family = 'source_label';
