DROP INDEX IF EXISTS authorization_receipts_principal_created_idx;
DROP INDEX IF EXISTS authorization_receipts_created_idx;

ALTER TABLE authorization_receipts
    DROP COLUMN IF EXISTS run_id,
    DROP COLUMN IF EXISTS operation,
    DROP COLUMN IF EXISTS principal_id;

ALTER TABLE mcp_oauth_refresh_tokens
    DROP CONSTRAINT IF EXISTS mcp_oauth_refresh_tokens_agent_fk;
ALTER TABLE mcp_oauth_authorization_codes
    DROP CONSTRAINT IF EXISTS mcp_oauth_authorization_codes_agent_fk;

ALTER TABLE mcp_oauth_refresh_tokens
    DROP COLUMN IF EXISTS agent_id;
ALTER TABLE mcp_oauth_authorization_codes
    DROP COLUMN IF EXISTS agent_id;

DROP TABLE IF EXISTS mcp_agent_tool_assignments;
