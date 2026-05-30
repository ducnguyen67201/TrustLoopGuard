-- Add the per-route response mode (regular buffered JSON vs. streaming SSE) to
-- enforcement profiles. Existing rows default to regular (today's behavior).
ALTER TABLE enforcement_profiles
    ADD COLUMN response_mode TEXT NOT NULL DEFAULT 'regular';
