-- Migration 51 intentionally removed the legacy approval/mandate state while
-- preserving financial history. Actions from that history have no unified
-- authorization intent and cannot be approved or executed through the current
-- pipeline. Remove them before the dashboard can present their fallback
-- `defer/evaluating` projection as current state.
--
-- Dependent legacy events, outcomes, ledger entries, payment reservations, and
-- receipts are removed by their existing ON DELETE CASCADE foreign keys.
DELETE FROM financial_actions
WHERE authorization_intent_id IS NULL;
