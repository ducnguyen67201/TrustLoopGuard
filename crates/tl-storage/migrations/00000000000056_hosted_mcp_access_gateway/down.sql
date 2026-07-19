DROP TABLE IF EXISTS mcp_tool_assignments;
DROP TABLE IF EXISTS mcp_tools;
DROP TABLE IF EXISTS mcp_server_connections;

ALTER TABLE workspaces
    DROP COLUMN IF EXISTS is_mcp_gateway_enabled;
