-- TrustLoopGuard storage schema.
--
-- Diesel-friendly conventions:
-- - snake_case table and column names
-- - no quoted identifiers
-- - JSONB for parsed policy/profile/decision payloads
-- - soft deletes for authoring records

CREATE TABLE agents (
    id              TEXT PRIMARY KEY,
    profile_yaml    TEXT        NOT NULL,
    parsed_profile  JSONB       NOT NULL,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    deleted_at      TIMESTAMPTZ
);

CREATE INDEX agents_active_idx ON agents (id) WHERE deleted_at IS NULL;

CREATE TABLE policies (
    id             TEXT        PRIMARY KEY,
    policy_yaml    TEXT        NOT NULL,
    parsed_policy  JSONB       NOT NULL,
    enabled        BOOLEAN     NOT NULL DEFAULT TRUE,
    created_at     TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at     TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    deleted_at     TIMESTAMPTZ
);

CREATE INDEX policies_active_idx ON policies (id) WHERE deleted_at IS NULL;
CREATE INDEX policies_enabled_idx ON policies (enabled, id)
    WHERE deleted_at IS NULL AND enabled = TRUE;

CREATE TABLE traces (
    trace_id    UUID        NOT NULL,
    domain      TEXT        NOT NULL,
    decision    TEXT        NOT NULL,
    elapsed_ms  INTEGER     NOT NULL,
    payload     JSONB       NOT NULL,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (trace_id, created_at)
) PARTITION BY RANGE (created_at);

CREATE TABLE traces_default PARTITION OF traces DEFAULT;

CREATE INDEX traces_decision_idx ON traces (decision, created_at DESC);
CREATE INDEX traces_domain_idx ON traces (domain, created_at DESC);

CREATE TABLE escalations (
    id          UUID        PRIMARY KEY,
    trace_id    UUID        NOT NULL,
    webhook_url TEXT        NOT NULL,
    status      TEXT        NOT NULL CHECK (status IN ('pending', 'sent', 'failed')),
    attempts    INTEGER     NOT NULL DEFAULT 0,
    payload     JSONB       NOT NULL,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    sent_at     TIMESTAMPTZ
);

CREATE INDEX escalations_pending_idx ON escalations (status, created_at)
    WHERE status = 'pending';
