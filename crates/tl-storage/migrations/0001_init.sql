-- 0001_init.sql — initial schema for the v0 deployment.
--
-- Three tables:
--   "Agent"        — registered agent profiles (one row per agent_id)
--   "Traces"       — append-only decision log, partitioned by day
--   "Escalations"  — pending/sent webhook deliveries
--
-- Note on the PascalCase identifiers: Postgres normalises unquoted
-- identifiers to lowercase, so to preserve the casing we double-quote
-- them everywhere — both in this migration and in every query that
-- references them (see crates/tl-storage/src/postgres.rs).
--
-- See docs/concept/v0-design-decisions.md §8 for the rationale on
-- partitioning, JSONB payloads, and column choices.

-- Agent ------------------------------------------------------------
--
-- profile_yaml is the source of truth — parsed_profile is a
-- materialised JSONB view kept in sync by the AgentRepo (PR 12) so
-- API consumers can fetch without re-parsing YAML.

CREATE TABLE "Agent" (
    id              TEXT PRIMARY KEY,
    profile_yaml    TEXT       NOT NULL,
    parsed_profile  JSONB      NOT NULL,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    deleted_at      TIMESTAMPTZ
);

-- Active-agent lookup is the hot path for resolution; a partial index
-- on the `deleted_at IS NULL` predicate keeps it small.
CREATE INDEX "Agent_active_idx" ON "Agent" (id) WHERE deleted_at IS NULL;

-- Traces -----------------------------------------------------------
--
-- trace_id is UUIDv7 (time-ordered) so each row's id implicitly carries
-- a creation timestamp; we still store created_at explicitly so the
-- partitioning predicate doesn't depend on a UUID-decoding routine.
--
-- Composite primary key (trace_id, created_at) is required by Postgres
-- partitioned tables — the partition key must appear in every unique
-- constraint.

CREATE TABLE "Traces" (
    trace_id    UUID        NOT NULL,
    domain      TEXT        NOT NULL,
    decision    TEXT        NOT NULL,
    elapsed_ms  INTEGER     NOT NULL,
    payload     JSONB       NOT NULL,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (trace_id, created_at)
) PARTITION BY RANGE (created_at);

-- Default partition catches anything outside named ranges. v0
-- operators run with this only; v1 / pg_cron will create daily
-- partitions for retention and cheap DROP-based purges.
CREATE TABLE "Traces_default" PARTITION OF "Traces" DEFAULT;

CREATE INDEX "Traces_decision_idx" ON "Traces" (decision, created_at DESC);
CREATE INDEX "Traces_domain_idx"   ON "Traces" (domain,   created_at DESC);

-- Escalations -----------------------------------------------------
--
-- One row per Decision::Escalate. status moves pending → sent | failed.
-- The pending partial index makes the worker's drain query trivial.

CREATE TABLE "Escalations" (
    id          UUID        PRIMARY KEY,
    trace_id    UUID        NOT NULL,
    webhook_url TEXT        NOT NULL,
    status      TEXT        NOT NULL CHECK (status IN ('pending', 'sent', 'failed')),
    attempts    INTEGER     NOT NULL DEFAULT 0,
    payload     JSONB       NOT NULL,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    sent_at     TIMESTAMPTZ
);

CREATE INDEX "Escalations_pending_idx" ON "Escalations" (status, created_at)
    WHERE status = 'pending';
