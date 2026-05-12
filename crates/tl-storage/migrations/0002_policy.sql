-- 0002_policy.sql — cloud-backed policy storage.
--
-- "Policy" stores the authored YAML as the audit/source-of-truth text and
-- the parsed policy as JSONB for API reads and runtime loading. Parsing and
-- validation stay in tl-policy; this table is the durable policy catalog.

CREATE TABLE "Policy" (
    id             TEXT        PRIMARY KEY,
    policy_yaml    TEXT        NOT NULL,
    parsed_policy  JSONB       NOT NULL,
    enabled        BOOLEAN     NOT NULL DEFAULT TRUE,
    created_at     TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at     TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    deleted_at     TIMESTAMPTZ
);

CREATE INDEX "Policy_active_idx" ON "Policy" (id) WHERE deleted_at IS NULL;
CREATE INDEX "Policy_enabled_idx" ON "Policy" (enabled, id)
    WHERE deleted_at IS NULL AND enabled = TRUE;
