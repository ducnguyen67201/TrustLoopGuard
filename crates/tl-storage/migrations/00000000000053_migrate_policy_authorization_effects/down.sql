-- This policy conversion is intentionally irreversible: legacy `escalate`
-- represented both approval and uncertainty, so reconstructing it would lose
-- the distinction established by AuthorizationEffect.
SELECT 1;
