DROP INDEX IF EXISTS gateway_routes_enforcement_profile_idx;
DROP INDEX IF EXISTS gateway_routes_provider_connection_idx;
DROP INDEX IF EXISTS gateway_routes_active_idx;
DROP INDEX IF EXISTS enforcement_profiles_active_idx;
DROP INDEX IF EXISTS gateway_provider_connections_active_idx;

DROP TABLE IF EXISTS gateway_routes;
DROP TABLE IF EXISTS enforcement_profiles;
DROP TABLE IF EXISTS gateway_provider_connections;
