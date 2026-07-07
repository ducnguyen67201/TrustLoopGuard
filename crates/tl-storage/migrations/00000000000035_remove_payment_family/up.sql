-- `family: payment` has been removed in favor of typed `family: financial`
-- policies. Soft-delete old rows before Rust deserializes enabled family
-- policies into the narrower FamilyPolicy enum.
UPDATE policies
SET deleted_at = COALESCE(deleted_at, now()),
    enabled = false,
    updated_at = now()
WHERE family = 'payment'
  AND deleted_at IS NULL;
