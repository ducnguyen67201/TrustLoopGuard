-- Per-owner payment policy: the spend caps a `set_policy` call writes.
CREATE TABLE IF NOT EXISTS pay_policy (
    workspace_id           TEXT        NOT NULL,
    owner                  TEXT        NOT NULL,
    per_transaction_minor  BIGINT,
    daily_minor            BIGINT,
    monthly_minor          BIGINT,
    hold_above_minor       BIGINT,
    updated_at             TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (workspace_id, owner)
);

-- One row per gated spend decision (allow/block/hold). The audit log,
-- the hold registry (resolve_hold flips `resolution`), and the source of
-- the windowed spend totals that enforce daily/monthly caps:
--   spend counted toward caps = status='allow' OR (status='hold' AND resolution).
CREATE TABLE IF NOT EXISTS pay_decision (
    id            UUID        NOT NULL,
    workspace_id  TEXT        NOT NULL,
    owner         TEXT        NOT NULL,
    decision_id   TEXT        NOT NULL,
    amount_minor  BIGINT      NOT NULL,
    merchant      TEXT        NOT NULL,
    category      TEXT        NOT NULL DEFAULT '',
    status        TEXT        NOT NULL,
    resolution    BOOLEAN,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (id)
);

CREATE INDEX IF NOT EXISTS pay_decision_owner_created_idx
    ON pay_decision (workspace_id, owner, created_at DESC);
CREATE UNIQUE INDEX IF NOT EXISTS pay_decision_decision_id_idx
    ON pay_decision (workspace_id, decision_id);
