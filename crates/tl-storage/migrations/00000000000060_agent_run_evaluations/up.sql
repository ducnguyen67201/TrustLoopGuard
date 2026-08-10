ALTER TABLE run_events ADD COLUMN agent_id TEXT;

UPDATE run_events
SET agent_id = runs.agent_id
FROM runs
WHERE runs.workspace_id = run_events.workspace_id
  AND runs.id = run_events.run_id;

ALTER TABLE run_events ALTER COLUMN agent_id SET NOT NULL;

ALTER TABLE run_events
    ADD CONSTRAINT run_events_agent_fk
    FOREIGN KEY (workspace_id, agent_id)
    REFERENCES agents (workspace_id, id)
    ON DELETE RESTRICT;

ALTER TABLE traces ADD COLUMN agent_id TEXT;
UPDATE traces
SET agent_id = NULLIF(payload #>> '{event,principal,agent_id}', '');

UPDATE traces
SET agent_id = NULL
WHERE agent_id IS NOT NULL
  AND NOT EXISTS (
      SELECT 1
      FROM agents
      WHERE agents.workspace_id = traces.workspace_id
        AND agents.id = traces.agent_id
  );

ALTER TABLE traces
    ADD CONSTRAINT traces_agent_fk
    FOREIGN KEY (workspace_id, agent_id)
    REFERENCES agents (workspace_id, id)
    ON DELETE RESTRICT;

ALTER TABLE traces ADD COLUMN late_evidence BOOLEAN NOT NULL DEFAULT FALSE;

CREATE INDEX traces_workspace_environment_agent_created_idx
    ON traces (workspace_id, environment_id, agent_id, created_at DESC);
CREATE INDEX traces_workspace_run_agent_idx
    ON traces (workspace_id, run_id, agent_id)
    WHERE run_id IS NOT NULL;
CREATE INDEX run_events_workspace_run_agent_idx
    ON run_events (workspace_id, run_id, agent_id, sequence);

ALTER TABLE runs ADD COLUMN boundary_source TEXT;
ALTER TABLE runs ADD COLUMN boundary_confidence TEXT;
ALTER TABLE runs ADD COLUMN finalized_at TIMESTAMPTZ;
ALTER TABLE runs ADD COLUMN capture_status TEXT NOT NULL DEFAULT 'open';
ALTER TABLE runs ADD COLUMN capture_deadline TIMESTAMPTZ;
ALTER TABLE runs ADD COLUMN expected_flush_id TEXT;
ALTER TABLE runs ADD COLUMN previous_run_id UUID;
ALTER TABLE runs ADD COLUMN last_evidence_at TIMESTAMPTZ;
ALTER TABLE runs ADD COLUMN dropped_trace_count BIGINT NOT NULL DEFAULT 0;
ALTER TABLE runs ADD COLUMN reevaluation_agent_ids TEXT[];
ALTER TABLE runs ADD COLUMN evaluation_eligibility TEXT;

UPDATE runs SET evaluation_eligibility = 'legacy_incomplete';

ALTER TABLE runs ALTER COLUMN evaluation_eligibility SET DEFAULT 'eligible';
ALTER TABLE runs ALTER COLUMN evaluation_eligibility SET NOT NULL;

ALTER TABLE runs
    ADD CONSTRAINT runs_boundary_source_check CHECK (
        boundary_source IS NULL OR boundary_source IN (
            'explicit_sdk', 'framework_adapter', 'otel_session_end',
            'root_span_end', 'idle_timeout', 'max_duration', 'admin', 'legacy_sdk'
        )
    );
ALTER TABLE runs
    ADD CONSTRAINT runs_boundary_confidence_check CHECK (
        boundary_confidence IS NULL OR boundary_confidence IN ('authoritative', 'strong', 'inferred')
    );
ALTER TABLE runs
    ADD CONSTRAINT runs_capture_status_check CHECK (
        capture_status IN ('open', 'waiting', 'complete', 'incomplete')
    );
ALTER TABLE runs
    ADD CONSTRAINT runs_evaluation_eligibility_check CHECK (
        evaluation_eligibility IN ('eligible', 'legacy_incomplete')
    );
ALTER TABLE runs
    ADD CONSTRAINT runs_previous_run_fk
    FOREIGN KEY (workspace_id, previous_run_id)
    REFERENCES runs (workspace_id, id)
    ON DELETE RESTRICT;

CREATE TABLE run_participants (
    workspace_id TEXT NOT NULL,
    environment_id TEXT NOT NULL,
    run_id UUID NOT NULL,
    agent_id TEXT NOT NULL,
    role TEXT NOT NULL,
    joined_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    manifest_frozen_at TIMESTAMPTZ,
    PRIMARY KEY (workspace_id, run_id, agent_id),
    FOREIGN KEY (workspace_id, run_id)
        REFERENCES runs (workspace_id, id) ON DELETE CASCADE,
    FOREIGN KEY (workspace_id, agent_id)
        REFERENCES agents (workspace_id, id) ON DELETE RESTRICT,
    FOREIGN KEY (workspace_id, environment_id)
        REFERENCES workspace_environments (workspace_id, id) ON DELETE RESTRICT,
    CHECK (role IN ('primary', 'participant'))
);

INSERT INTO run_participants (
    workspace_id, environment_id, run_id, agent_id, role, joined_at, manifest_frozen_at
)
SELECT workspace_id, environment_id, id, agent_id, 'primary', started_at, started_at
FROM runs
ON CONFLICT DO NOTHING;

CREATE TABLE agent_evaluation_profiles (
    workspace_id TEXT NOT NULL,
    environment_id TEXT NOT NULL,
    agent_id TEXT NOT NULL,
    enabled BOOLEAN NOT NULL DEFAULT FALSE,
    capture_mode TEXT NOT NULL DEFAULT 'best_effort',
    content_mode TEXT NOT NULL DEFAULT 'metadata_only',
    quiet_period_ms BIGINT NOT NULL DEFAULT 2000,
    max_capture_wait_ms BIGINT NOT NULL DEFAULT 30000,
    on_incomplete TEXT NOT NULL DEFAULT 'inconclusive',
    profile_version INTEGER NOT NULL DEFAULT 1,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (workspace_id, environment_id, agent_id),
    FOREIGN KEY (workspace_id, environment_id)
        REFERENCES workspace_environments (workspace_id, id) ON DELETE CASCADE,
    FOREIGN KEY (workspace_id, agent_id)
        REFERENCES agents (workspace_id, id) ON DELETE CASCADE,
    CHECK (capture_mode IN ('best_effort', 'durable')),
    CHECK (content_mode IN ('metadata_only', 'redacted', 'encrypted_artifact_ref')),
    CHECK (on_incomplete IN ('inconclusive', 'fail')),
    CHECK (quiet_period_ms BETWEEN 0 AND 300000),
    CHECK (max_capture_wait_ms BETWEEN 1000 AND 3600000),
    CHECK (profile_version > 0)
);

CREATE TABLE agent_evaluation_policy_assignments (
    workspace_id TEXT NOT NULL,
    environment_id TEXT NOT NULL,
    agent_id TEXT NOT NULL,
    policy_id TEXT NOT NULL,
    policy_version INTEGER,
    weight INTEGER NOT NULL DEFAULT 1,
    critical BOOLEAN NOT NULL DEFAULT FALSE,
    enabled BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (workspace_id, environment_id, agent_id, policy_id),
    FOREIGN KEY (workspace_id, environment_id, agent_id)
        REFERENCES agent_evaluation_profiles (workspace_id, environment_id, agent_id)
        ON DELETE CASCADE,
    FOREIGN KEY (workspace_id, policy_id)
        REFERENCES policies (workspace_id, id) ON DELETE CASCADE,
    CHECK (policy_version IS NULL OR policy_version > 0),
    CHECK (weight BETWEEN 1 AND 10000)
);

CREATE TABLE run_evaluation_policy_manifest (
    workspace_id TEXT NOT NULL,
    environment_id TEXT NOT NULL,
    run_id UUID NOT NULL,
    agent_id TEXT NOT NULL,
    policy_id TEXT NOT NULL,
    policy_family TEXT NOT NULL,
    policy_version INTEGER NOT NULL,
    policy_hash TEXT NOT NULL,
    policy_yaml TEXT NOT NULL,
    weight INTEGER NOT NULL,
    critical BOOLEAN NOT NULL,
    evidence_requirements JSONB NOT NULL DEFAULT '{}'::jsonb,
    captured_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (workspace_id, run_id, agent_id, policy_id),
    FOREIGN KEY (workspace_id, run_id, agent_id)
        REFERENCES run_participants (workspace_id, run_id, agent_id) ON DELETE CASCADE,
    CHECK (policy_version > 0),
    CHECK (weight BETWEEN 1 AND 10000)
);

CREATE TABLE run_spans (
    workspace_id TEXT NOT NULL,
    environment_id TEXT NOT NULL,
    run_id UUID NOT NULL,
    agent_id TEXT NOT NULL,
    run_event_id UUID,
    otel_trace_id TEXT NOT NULL,
    otel_span_id TEXT NOT NULL,
    parent_span_id TEXT,
    name TEXT NOT NULL,
    span_kind INTEGER NOT NULL,
    operation_name TEXT,
    conversation_id TEXT,
    external_agent_id TEXT,
    started_at TIMESTAMPTZ NOT NULL,
    ended_at TIMESTAMPTZ NOT NULL,
    status_code INTEGER NOT NULL,
    status_message TEXT,
    resource JSONB NOT NULL DEFAULT '{}'::jsonb,
    attributes JSONB NOT NULL DEFAULT '{}'::jsonb,
    events JSONB NOT NULL DEFAULT '[]'::jsonb,
    links JSONB NOT NULL DEFAULT '[]'::jsonb,
    content_capture_status TEXT NOT NULL DEFAULT 'omitted_by_policy',
    dropped_attribute_count INTEGER NOT NULL DEFAULT 0,
    late_evidence BOOLEAN NOT NULL DEFAULT FALSE,
    ingested_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (workspace_id, environment_id, otel_trace_id, otel_span_id),
    FOREIGN KEY (workspace_id, run_id, agent_id)
        REFERENCES run_participants (workspace_id, run_id, agent_id) ON DELETE RESTRICT,
    FOREIGN KEY (workspace_id, run_event_id)
        REFERENCES run_events (workspace_id, id) ON DELETE SET NULL,
    CHECK (char_length(otel_trace_id) = 32),
    CHECK (char_length(otel_span_id) = 16),
    CHECK (parent_span_id IS NULL OR char_length(parent_span_id) = 16),
    CHECK (ended_at >= started_at)
);
CREATE INDEX run_spans_workspace_run_agent_started_idx
    ON run_spans (workspace_id, run_id, agent_id, started_at);

CREATE TABLE otel_flush_receipts (
    workspace_id TEXT NOT NULL,
    environment_id TEXT NOT NULL,
    run_id UUID NOT NULL,
    flush_id TEXT NOT NULL,
    accepted_span_count INTEGER NOT NULL,
    rejected_span_count INTEGER NOT NULL,
    accepted_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (workspace_id, environment_id, run_id, flush_id),
    FOREIGN KEY (workspace_id, run_id)
        REFERENCES runs (workspace_id, id) ON DELETE CASCADE
);

CREATE TABLE run_snapshots (
    workspace_id TEXT NOT NULL,
    environment_id TEXT NOT NULL,
    id UUID NOT NULL,
    run_id UUID NOT NULL,
    snapshot_version INTEGER NOT NULL,
    snapshot_hash TEXT NOT NULL,
    manifest_hash TEXT NOT NULL,
    capture_status TEXT NOT NULL,
    event_cutoff TIMESTAMPTZ NOT NULL,
    event_count BIGINT NOT NULL,
    trace_count BIGINT NOT NULL,
    span_count BIGINT NOT NULL,
    dropped_trace_count BIGINT NOT NULL,
    late_evidence_count BIGINT NOT NULL,
    snapshot JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (workspace_id, id),
    UNIQUE (workspace_id, run_id, snapshot_version),
    UNIQUE (workspace_id, run_id, snapshot_hash, manifest_hash),
    FOREIGN KEY (workspace_id, run_id)
        REFERENCES runs (workspace_id, id) ON DELETE CASCADE,
    CHECK (snapshot_version > 0),
    CHECK (capture_status IN ('complete', 'incomplete'))
);

CREATE TABLE evaluation_jobs (
    workspace_id TEXT NOT NULL,
    environment_id TEXT NOT NULL,
    id UUID NOT NULL,
    run_id UUID NOT NULL,
    agent_id TEXT NOT NULL,
    snapshot_id UUID NOT NULL,
    snapshot_hash TEXT NOT NULL,
    manifest_hash TEXT NOT NULL,
    evaluator_version TEXT NOT NULL,
    status TEXT NOT NULL,
    attempts INTEGER NOT NULL DEFAULT 0,
    available_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    lease_owner TEXT,
    lease_expires_at TIMESTAMPTZ,
    error TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (workspace_id, id),
    UNIQUE (workspace_id, run_id, agent_id, snapshot_hash, manifest_hash, evaluator_version),
    FOREIGN KEY (workspace_id, run_id, agent_id)
        REFERENCES run_participants (workspace_id, run_id, agent_id) ON DELETE CASCADE,
    FOREIGN KEY (workspace_id, snapshot_id)
        REFERENCES run_snapshots (workspace_id, id) ON DELETE CASCADE,
    CHECK (status IN ('waiting_capture', 'queued', 'running', 'completed', 'failed', 'inconclusive', 'error')),
    CHECK (attempts >= 0)
);
CREATE INDEX evaluation_jobs_claim_idx
    ON evaluation_jobs (status, available_at, lease_expires_at, created_at);

CREATE TABLE evaluation_results (
    workspace_id TEXT NOT NULL,
    environment_id TEXT NOT NULL,
    id UUID NOT NULL,
    job_id UUID NOT NULL,
    run_id UUID NOT NULL,
    agent_id TEXT NOT NULL,
    snapshot_hash TEXT NOT NULL,
    manifest_hash TEXT NOT NULL,
    evaluator_version TEXT NOT NULL,
    verdict TEXT NOT NULL,
    score_bps INTEGER,
    capture_status TEXT NOT NULL,
    llm_audit JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (workspace_id, id),
    UNIQUE (workspace_id, job_id),
    FOREIGN KEY (workspace_id, job_id)
        REFERENCES evaluation_jobs (workspace_id, id) ON DELETE CASCADE,
    CHECK (verdict IN ('passed', 'failed', 'inconclusive', 'error', 'not_configured')),
    CHECK (score_bps IS NULL OR score_bps BETWEEN 0 AND 10000)
);

CREATE TABLE evaluation_findings (
    workspace_id TEXT NOT NULL,
    environment_id TEXT NOT NULL,
    id UUID NOT NULL,
    result_id UUID NOT NULL,
    run_id UUID NOT NULL,
    agent_id TEXT NOT NULL,
    policy_id TEXT NOT NULL,
    policy_version INTEGER NOT NULL,
    severity TEXT NOT NULL,
    critical BOOLEAN NOT NULL,
    status TEXT NOT NULL,
    score_bps INTEGER,
    reason TEXT NOT NULL,
    evidence JSONB NOT NULL DEFAULT '[]'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (workspace_id, id),
    UNIQUE (workspace_id, result_id, policy_id),
    FOREIGN KEY (workspace_id, result_id)
        REFERENCES evaluation_results (workspace_id, id) ON DELETE CASCADE,
    CHECK (status IN ('passed', 'failed', 'inconclusive', 'error', 'not_applicable')),
    CHECK (score_bps IS NULL OR score_bps BETWEEN 0 AND 10000)
);

CREATE TABLE evaluation_datasets (
    workspace_id TEXT NOT NULL,
    id UUID NOT NULL,
    agent_id TEXT NOT NULL,
    name TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (workspace_id, id),
    FOREIGN KEY (workspace_id, agent_id)
        REFERENCES agents (workspace_id, id) ON DELETE RESTRICT
);

CREATE TABLE evaluation_dataset_versions (
    workspace_id TEXT NOT NULL,
    dataset_id UUID NOT NULL,
    version INTEGER NOT NULL,
    manifest_hash TEXT NOT NULL,
    manifest JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (workspace_id, dataset_id, version),
    UNIQUE (workspace_id, dataset_id, manifest_hash),
    FOREIGN KEY (workspace_id, dataset_id)
        REFERENCES evaluation_datasets (workspace_id, id) ON DELETE CASCADE,
    CHECK (version > 0)
);

CREATE TABLE evaluation_cases (
    workspace_id TEXT NOT NULL,
    dataset_id UUID NOT NULL,
    dataset_version INTEGER NOT NULL,
    case_id TEXT NOT NULL,
    case_hash TEXT NOT NULL,
    scoring_mode TEXT NOT NULL,
    weight INTEGER NOT NULL DEFAULT 1,
    spec JSONB NOT NULL,
    PRIMARY KEY (workspace_id, dataset_id, dataset_version, case_id),
    FOREIGN KEY (workspace_id, dataset_id, dataset_version)
        REFERENCES evaluation_dataset_versions (workspace_id, dataset_id, version) ON DELETE CASCADE,
    CHECK (scoring_mode IN ('trajectory', 'endstate')),
    CHECK (weight BETWEEN 1 AND 10000)
);

CREATE TABLE evaluation_campaigns (
    workspace_id TEXT NOT NULL,
    environment_id TEXT NOT NULL,
    id UUID NOT NULL,
    dataset_id UUID NOT NULL,
    dataset_version INTEGER NOT NULL,
    agent_id TEXT NOT NULL,
    status TEXT NOT NULL,
    case_runs JSONB NOT NULL DEFAULT '{}'::jsonb,
    aggregate JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (workspace_id, id),
    FOREIGN KEY (workspace_id, dataset_id, dataset_version)
        REFERENCES evaluation_dataset_versions (workspace_id, dataset_id, version) ON DELETE RESTRICT,
    FOREIGN KEY (workspace_id, environment_id)
        REFERENCES workspace_environments (workspace_id, id) ON DELETE RESTRICT,
    FOREIGN KEY (workspace_id, agent_id)
        REFERENCES agents (workspace_id, id) ON DELETE RESTRICT,
    CHECK (status IN ('queued', 'running', 'completed', 'failed', 'incomplete'))
);

CREATE TABLE evaluation_release_gates (
    workspace_id TEXT NOT NULL,
    environment_id TEXT NOT NULL,
    id UUID NOT NULL,
    agent_id TEXT NOT NULL,
    campaign_id UUID,
    manifest_hash TEXT NOT NULL,
    verdict TEXT NOT NULL,
    evidence JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (workspace_id, id),
    FOREIGN KEY (workspace_id, campaign_id)
        REFERENCES evaluation_campaigns (workspace_id, id) ON DELETE RESTRICT,
    FOREIGN KEY (workspace_id, environment_id)
        REFERENCES workspace_environments (workspace_id, id) ON DELETE RESTRICT,
    FOREIGN KEY (workspace_id, agent_id)
        REFERENCES agents (workspace_id, id) ON DELETE RESTRICT,
    CHECK (verdict IN ('passed', 'failed', 'insufficient_evidence'))
);
