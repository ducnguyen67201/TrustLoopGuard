ALTER TABLE financial_actions
    ADD COLUMN operation TEXT;

UPDATE financial_actions
SET operation = COALESCE(NULLIF(metadata ->> 'operation', ''), action_kind);

ALTER TABLE financial_actions
    ALTER COLUMN operation SET NOT NULL;

ALTER TABLE financial_actions
    ADD CONSTRAINT financial_actions_operation_check
    CHECK (
        char_length(operation) BETWEEN 1 AND 128
        AND operation ~ '^[a-z0-9_-]+$'
    );
