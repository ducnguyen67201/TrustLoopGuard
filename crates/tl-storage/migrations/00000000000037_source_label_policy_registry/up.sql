INSERT INTO policies (
    workspace_id,
    id,
    policy_yaml,
    parsed_policy,
    enabled,
    family,
    created_at,
    updated_at,
    deleted_at
)
SELECT
    source_label_policy.workspace_id,
    'source-label-' || source_label_policy.origin,
    'family: source_label
id: source-label-' || source_label_policy.origin || '
description: Source label override for ' || source_label_policy.origin || '
severity: low
origin: ' || source_label_policy.origin || '
trust: ' || COALESCE(source_label_policy.spec ->> 'trust', 'null') || '
confidentiality: ' || COALESCE(source_label_policy.spec ->> 'confidentiality', 'null') || '
integrity: ' || COALESCE(source_label_policy.spec ->> 'integrity', 'null') || '
',
    jsonb_strip_nulls(jsonb_build_object(
        'family', 'source_label',
        'id', 'source-label-' || source_label_policy.origin,
        'description', 'Source label override for ' || source_label_policy.origin,
        'severity', 'low',
        'origin', source_label_policy.origin,
        'trust', source_label_policy.spec ->> 'trust',
        'confidentiality', source_label_policy.spec ->> 'confidentiality',
        'integrity', source_label_policy.spec ->> 'integrity'
    )),
    source_label_policy.enabled,
    'source_label',
    source_label_policy.created_at,
    source_label_policy.updated_at,
    source_label_policy.deleted_at
FROM source_label_policy
ON CONFLICT (workspace_id, id) DO UPDATE SET
    policy_yaml = EXCLUDED.policy_yaml,
    parsed_policy = EXCLUDED.parsed_policy,
    enabled = EXCLUDED.enabled,
    family = EXCLUDED.family,
    updated_at = EXCLUDED.updated_at,
    deleted_at = EXCLUDED.deleted_at;

INSERT INTO policy_environment_deployments (
    workspace_id,
    environment_id,
    policy_id,
    enabled
)
SELECT
    source_label_policy.workspace_id,
    workspace_environments.id,
    'source-label-' || source_label_policy.origin,
    source_label_policy.enabled
FROM source_label_policy
JOIN workspace_environments
    ON workspace_environments.workspace_id = source_label_policy.workspace_id
WHERE source_label_policy.deleted_at IS NULL
ON CONFLICT (workspace_id, environment_id, policy_id) DO UPDATE SET
    enabled = EXCLUDED.enabled,
    updated_at = NOW();

INSERT INTO entity_versions (
    workspace_id,
    entity_type,
    entity_id,
    version,
    content,
    created_at
)
SELECT
    source_label_policy.workspace_id,
    'policy',
    'source-label-' || source_label_policy.origin,
    1,
    'family: source_label
id: source-label-' || source_label_policy.origin || '
description: Source label override for ' || source_label_policy.origin || '
severity: low
origin: ' || source_label_policy.origin || '
trust: ' || COALESCE(source_label_policy.spec ->> 'trust', 'null') || '
confidentiality: ' || COALESCE(source_label_policy.spec ->> 'confidentiality', 'null') || '
integrity: ' || COALESCE(source_label_policy.spec ->> 'integrity', 'null') || '
',
    source_label_policy.created_at
FROM source_label_policy
WHERE NOT EXISTS (
    SELECT 1
    FROM entity_versions existing
    WHERE existing.workspace_id = source_label_policy.workspace_id
      AND existing.entity_type = 'policy'
      AND existing.entity_id = 'source-label-' || source_label_policy.origin
);

DROP TABLE source_label_policy;
