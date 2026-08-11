DROP INDEX IF EXISTS runs_gateway_active_activity_idx;
DROP INDEX IF EXISTS runs_gateway_active_session_unique;
DROP TABLE IF EXISTS gateway_route_fallbacks;
ALTER TABLE gateway_routes DROP CONSTRAINT IF EXISTS gateway_routes_reliability_mode_check;
ALTER TABLE gateway_routes DROP COLUMN IF EXISTS reliability_mode;
