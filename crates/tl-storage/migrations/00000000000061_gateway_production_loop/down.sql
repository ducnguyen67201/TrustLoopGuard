DROP INDEX IF EXISTS runs_gateway_active_activity_idx;
DROP INDEX IF EXISTS runs_gateway_active_session_unique;
ALTER TABLE gateway_routes DROP CONSTRAINT IF EXISTS gateway_routes_fallback_provider_connection_fk;
ALTER TABLE gateway_routes DROP CONSTRAINT IF EXISTS gateway_routes_reliability_mode_check;
ALTER TABLE gateway_routes DROP COLUMN IF EXISTS fallback_provider_connection_id;
ALTER TABLE gateway_routes DROP COLUMN IF EXISTS reliability_mode;
