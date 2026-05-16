-- Move dashboard runtime data into the Rust-owned runtime tables.
--
-- After this migration, `agents`, `policies`, and `traces` are the canonical
-- source for runtime/dashboard agent, policy, and decision data. The old web
-- tables are copied when present, then removed so the database has one runtime
-- model.

DO $$
BEGIN
    IF to_regclass('public.workspace_agents') IS NOT NULL THEN
        EXECUTE $copy_agents$
            INSERT INTO agents (
                workspace_id,
                id,
                profile_yaml,
                parsed_profile,
                created_at,
                updated_at,
                deleted_at
            )
            SELECT
                workspace_id,
                id,
                CONCAT(
                    'agent_id: ', id, E'\n',
                    'display_name: ', name, E'\n',
                    'system_prompt: ', COALESCE(to_json(system_prompt)::text, '""'), E'\n',
                    'scope:', E'\n',
                    '  in_scope:', E'\n',
                    '    - ', scope, E'\n',
                    '  out_of_scope: []', E'\n',
                    'authority:', E'\n',
                    '  can_promise: []', E'\n',
                    '  cannot_promise: []', E'\n',
                    'tone:', E'\n',
                    '  target: clear-professional', E'\n',
                    '  forbidden: []', E'\n',
                    'knowledge_sources: []', E'\n',
                    'escalation_triggers: []', E'\n'
                ),
                jsonb_build_object(
                    'agent_id', id,
                    'display_name', name,
                    'system_prompt', system_prompt,
                    'scope', jsonb_build_object(
                        'in_scope', jsonb_build_array(scope),
                        'out_of_scope', jsonb_build_array()
                    ),
                    'authority', jsonb_build_object(
                        'can_promise', jsonb_build_array(),
                        'cannot_promise', jsonb_build_array()
                    ),
                    'tone', jsonb_build_object(
                        'target', 'clear-professional',
                        'forbidden', jsonb_build_array()
                    ),
                    'knowledge_sources', jsonb_build_array(),
                    'escalation_triggers', jsonb_build_array()
                ),
                created_at,
                updated_at,
                deleted_at
            FROM workspace_agents
            ON CONFLICT (workspace_id, id) DO UPDATE SET
                profile_yaml = EXCLUDED.profile_yaml,
                parsed_profile = EXCLUDED.parsed_profile,
                updated_at = EXCLUDED.updated_at,
                deleted_at = EXCLUDED.deleted_at
        $copy_agents$;
    END IF;

    IF to_regclass('public.workspace_policies') IS NOT NULL THEN
        EXECUTE $copy_policies$
            INSERT INTO policies (
                workspace_id,
                id,
                policy_yaml,
                parsed_policy,
                enabled,
                owner_agent_id,
                created_at,
                updated_at,
                deleted_at
            )
            SELECT
                workspace_id,
                policy_key,
                COALESCE(
                    source_yaml,
                    CONCAT(
                        'id: ', policy_key, E'\n',
                        'description: ', COALESCE(description, ''), E'\n',
                        'match:', E'\n',
                        '  literal: "', policy_key, '"', E'\n',
                        'action: ', action, E'\n',
                        'severity: ', severity, E'\n'
                    )
                ),
                jsonb_build_object(
                    'id', policy_key,
                    'description', description,
                    'match', jsonb_build_object('literal', policy_key),
                    'action', action,
                    'severity', severity,
                    'owner_agent_id', agent_id
                ),
                enabled,
                agent_id,
                created_at,
                updated_at,
                deleted_at
            FROM workspace_policies
            ON CONFLICT (workspace_id, id) DO UPDATE SET
                policy_yaml = EXCLUDED.policy_yaml,
                parsed_policy = EXCLUDED.parsed_policy,
                enabled = EXCLUDED.enabled,
                owner_agent_id = EXCLUDED.owner_agent_id,
                updated_at = EXCLUDED.updated_at,
                deleted_at = EXCLUDED.deleted_at
        $copy_policies$;
    END IF;

    IF to_regclass('public.guardrail_decisions') IS NOT NULL THEN
        EXECUTE $copy_traces$
            WITH moved AS (
                SELECT
                    gd.workspace_id,
                    (
                        substr(md5(gd.trace_id), 1, 8) || '-' ||
                        substr(md5(gd.trace_id), 9, 4) || '-' ||
                        substr(md5(gd.trace_id), 13, 4) || '-' ||
                        substr(md5(gd.trace_id), 17, 4) || '-' ||
                        substr(md5(gd.trace_id), 21, 12)
                    )::uuid AS trace_uuid,
                    gd.trace_id,
                    gd.agent_id,
                    COALESCE(wp.policy_key, gd.policy_id) AS policy_key,
                    gd.verdict,
                    gd.latency_ms,
                    gd.created_at
                FROM guardrail_decisions gd
                LEFT JOIN workspace_policies wp ON wp.id = gd.policy_id
            )
            INSERT INTO traces (
                workspace_id,
                trace_id,
                domain,
                decision,
                elapsed_ms,
                payload,
                created_at
            )
            SELECT
                workspace_id,
                trace_uuid,
                'customer_support',
                verdict,
                latency_ms::integer,
                jsonb_build_object(
                    'trace_id', trace_uuid::text,
                    'legacy_trace_id', trace_id,
                    'agent_id', agent_id,
                    'verdict', verdict,
                    'reason', CASE
                        WHEN policy_key IS NULL THEN 'no policies triggered'
                        ELSE policy_key || ' triggered'
                    END,
                    'triggered_policies', CASE
                        WHEN policy_key IS NULL THEN jsonb_build_array()
                        ELSE jsonb_build_array(jsonb_build_object(
                            'id', policy_key,
                            'severity', 'medium',
                            'reason', policy_key || ' matched'
                        ))
                    END,
                    'safe_output', NULL,
                    'latency_ms', latency_ms::integer
                ),
                created_at
            FROM moved
            ON CONFLICT (trace_id, created_at) DO NOTHING
        $copy_traces$;
    END IF;
END $$;

DROP TABLE IF EXISTS guardrail_decisions;
DROP TABLE IF EXISTS workspace_policies;
DROP TABLE IF EXISTS workspace_agents;
