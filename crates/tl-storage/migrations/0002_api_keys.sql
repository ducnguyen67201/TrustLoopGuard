-- 0002_api_keys.sql — per-user API keys minted by the dashboard.
--
-- The dashboard never sees a stored key after creation; we hold only
-- the SHA-256(plaintext). `prefix` is the first ~5 chars of the
-- plaintext so list views can show a recognisable handle without
-- needing the plaintext.
--
-- Soft delete via `revoked_at`; the partial index on `revoked_at IS NULL`
-- keeps the /v1/check hot-path lookup small.

CREATE TABLE "ApiKey" (
    id           UUID         PRIMARY KEY,
    user_id      TEXT         NOT NULL,
    name         TEXT         NOT NULL,
    prefix       TEXT         NOT NULL,
    hash         BYTEA        NOT NULL UNIQUE,
    last_used_at TIMESTAMPTZ,
    created_at   TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
    revoked_at   TIMESTAMPTZ
);

CREATE INDEX "ApiKey_active_hash_idx" ON "ApiKey" (hash) WHERE revoked_at IS NULL;
CREATE INDEX "ApiKey_user_idx" ON "ApiKey" (user_id, created_at DESC);
