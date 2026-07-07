ALTER TABLE financial_actions
    DROP CONSTRAINT IF EXISTS financial_actions_operation_check;

ALTER TABLE financial_actions
    DROP COLUMN operation;
