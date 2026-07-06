-- Normalize policy family into a first-class registry discriminator.
-- Older content policies used NULL; new code writes `content` explicitly.
UPDATE policies
SET family = 'content'
WHERE family IS NULL;

ALTER TABLE policies
    ALTER COLUMN family SET DEFAULT 'content';

CREATE INDEX IF NOT EXISTS policies_workspace_family_active_idx
    ON policies (workspace_id, family, id)
    WHERE deleted_at IS NULL;

-- Family policies created before this migration were workspace-scoped. Give
-- every active policy explicit deployment state in every active environment so
-- runtime loading can use one environment-aware path for all families.
INSERT INTO policy_environment_deployments (
    workspace_id,
    environment_id,
    policy_id,
    enabled
)
SELECT
    policies.workspace_id,
    workspace_environments.id,
    policies.id,
    policies.enabled
FROM policies
JOIN workspace_environments
  ON workspace_environments.workspace_id = policies.workspace_id
 AND workspace_environments.deleted_at IS NULL
WHERE policies.deleted_at IS NULL
ON CONFLICT (workspace_id, environment_id, policy_id) DO NOTHING;

-- Older family-policy writes skipped entity_versions. Seed version 1 from the
-- current YAML when no policy version exists yet.
INSERT INTO entity_versions (
    workspace_id,
    entity_type,
    entity_id,
    version,
    content
)
SELECT
    policies.workspace_id,
    'policy',
    policies.id,
    1,
    policies.policy_yaml
FROM policies
WHERE policies.deleted_at IS NULL
  AND NOT EXISTS (
      SELECT 1
      FROM entity_versions
      WHERE entity_versions.workspace_id = policies.workspace_id
        AND entity_versions.entity_type = 'policy'
        AND entity_versions.entity_id = policies.id
  );
