-- Migration 51 unified the runtime decision vocabulary, but existing policy
-- rows still contain the legacy allow/block/rewrite/escalate actions. Convert
-- both the parsed JSON source of truth and its authoring YAML representation.

UPDATE policies
SET parsed_policy = jsonb_set(
        parsed_policy,
        '{action}',
        to_jsonb(
            CASE parsed_policy ->> 'action'
                WHEN 'allow' THEN 'permit'
                WHEN 'block' THEN 'deny'
                WHEN 'rewrite' THEN 'transform'
                WHEN 'escalate' THEN
                    CASE
                        WHEN family = 'approval' THEN 'require_approval'
                        ELSE 'defer'
                    END
            END
        )
    ),
    updated_at = now()
WHERE family <> 'financial'
  AND parsed_policy ->> 'action' IN ('allow', 'block', 'rewrite', 'escalate');

UPDATE policies
SET policy_yaml = regexp_replace(
        regexp_replace(
            regexp_replace(
                policy_yaml,
                '(^|\n)([[:space:]]*action:[[:space:]]*)allow([[:space:]]*)($|\n)',
                E'\\1\\2permit\\3\\4',
                'g'
            ),
            '(^|\n)([[:space:]]*action:[[:space:]]*)block([[:space:]]*)($|\n)',
            E'\\1\\2deny\\3\\4',
            'g'
        ),
        '(^|\n)([[:space:]]*action:[[:space:]]*)rewrite([[:space:]]*)($|\n)',
        E'\\1\\2transform\\3\\4',
        'g'
    ),
    updated_at = now()
WHERE family <> 'financial';

UPDATE policies
SET policy_yaml = regexp_replace(
        policy_yaml,
        '(^|\n)([[:space:]]*action:[[:space:]]*)escalate([[:space:]]*)($|\n)',
        CASE
            WHEN family = 'approval' THEN E'\\1\\2require_approval\\3\\4'
            ELSE E'\\1\\2defer\\3\\4'
        END,
        'g'
    ),
    updated_at = now()
WHERE family <> 'financial';

-- Financial policies also renamed fields whose old names overloaded approval
-- and evidence uncertainty. Missing/failed evidence cannot be bypassed by a
-- grant, while a threshold breach can explicitly request approval.
UPDATE policies
SET parsed_policy =
        (parsed_policy
            - 'hold_above_minor'
            - 'hold_new_counterparty'
            - 'mandate_required'
            - 'missing_evidence_action'
            - 'failed_precondition_action')
        || jsonb_strip_nulls(jsonb_build_object(
            'approval_threshold_minor',
                CASE
                    WHEN parsed_policy ? 'approval_threshold_minor'
                        THEN parsed_policy -> 'approval_threshold_minor'
                    ELSE parsed_policy -> 'hold_above_minor'
                END,
            'require_approval_for_new_counterparty',
                CASE
                    WHEN parsed_policy ? 'require_approval_for_new_counterparty'
                        THEN parsed_policy -> 'require_approval_for_new_counterparty'
                    ELSE parsed_policy -> 'hold_new_counterparty'
                END,
            'grant_required',
                CASE
                    WHEN parsed_policy ? 'grant_required'
                        THEN parsed_policy -> 'grant_required'
                    ELSE parsed_policy -> 'mandate_required'
                END,
            'missing_evidence_effect',
                CASE
                    WHEN parsed_policy ? 'missing_evidence_effect'
                        THEN parsed_policy -> 'missing_evidence_effect'
                    WHEN parsed_policy ->> 'missing_evidence_action' = 'escalate'
                        THEN to_jsonb('defer'::text)
                    WHEN parsed_policy ? 'missing_evidence_action'
                        THEN to_jsonb('deny'::text)
                END,
            'failed_precondition_effect',
                CASE
                    WHEN parsed_policy ? 'failed_precondition_effect'
                        THEN parsed_policy -> 'failed_precondition_effect'
                    WHEN parsed_policy ->> 'failed_precondition_action' = 'escalate'
                        THEN to_jsonb('defer'::text)
                    WHEN parsed_policy ? 'failed_precondition_action'
                        THEN to_jsonb('deny'::text)
                END,
            'on_breach',
                CASE parsed_policy ->> 'on_breach'
                    WHEN 'escalate' THEN to_jsonb('require_approval'::text)
                    WHEN 'block' THEN to_jsonb('deny'::text)
                    WHEN 'allow' THEN to_jsonb('deny'::text)
                    WHEN 'rewrite' THEN to_jsonb('deny'::text)
                    ELSE parsed_policy -> 'on_breach'
                END
        )),
    updated_at = now()
WHERE family = 'financial';

UPDATE policies
SET policy_yaml = regexp_replace(
        regexp_replace(
            regexp_replace(
                regexp_replace(
                    regexp_replace(
                        policy_yaml,
                        '(^|\n)([[:space:]]*)hold_above_minor:',
                        E'\\1\\2approval_threshold_minor:',
                        'g'
                    ),
                    '(^|\n)([[:space:]]*)hold_new_counterparty:',
                    E'\\1\\2require_approval_for_new_counterparty:',
                    'g'
                ),
                '(^|\n)([[:space:]]*)mandate_required:',
                E'\\1\\2grant_required:',
                'g'
            ),
            '(^|\n)([[:space:]]*)missing_evidence_action:',
            E'\\1\\2missing_evidence_effect:',
            'g'
        ),
        '(^|\n)([[:space:]]*)failed_precondition_action:',
        E'\\1\\2failed_precondition_effect:',
        'g'
    ),
    updated_at = now()
WHERE family = 'financial';

UPDATE policies
SET policy_yaml = regexp_replace(
        regexp_replace(
            regexp_replace(
                policy_yaml,
                '(^|\n)([[:space:]]*missing_evidence_effect:[[:space:]]*)escalate([[:space:]]*)($|\n)',
                E'\\1\\2defer\\3\\4',
                'g'
            ),
            '(^|\n)([[:space:]]*failed_precondition_effect:[[:space:]]*)block([[:space:]]*)($|\n)',
            E'\\1\\2deny\\3\\4',
            'g'
        ),
        '(^|\n)([[:space:]]*failed_precondition_effect:[[:space:]]*)escalate([[:space:]]*)($|\n)',
        E'\\1\\2defer\\3\\4',
        'g'
    ),
    updated_at = now()
WHERE family = 'financial';

UPDATE policies
SET policy_yaml = regexp_replace(
        regexp_replace(
            regexp_replace(
                regexp_replace(
                    policy_yaml,
                    '(^|\n)([[:space:]]*on_breach:[[:space:]]*)escalate([[:space:]]*)($|\n)',
                    E'\\1\\2require_approval\\3\\4',
                    'g'
                ),
                '(^|\n)([[:space:]]*on_breach:[[:space:]]*)block([[:space:]]*)($|\n)',
                E'\\1\\2deny\\3\\4',
                'g'
            ),
            '(^|\n)([[:space:]]*on_breach:[[:space:]]*)allow([[:space:]]*)($|\n)',
            E'\\1\\2deny\\3\\4',
            'g'
        ),
        '(^|\n)([[:space:]]*on_breach:[[:space:]]*)rewrite([[:space:]]*)($|\n)',
        E'\\1\\2deny\\3\\4',
        'g'
    ),
    updated_at = now()
WHERE family = 'financial';
