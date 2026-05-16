-- Remove the old web-owned Auth.js tables and repair pre-existing dashboard
-- admin tables so their user references point at Rust-owned users(id).
--
-- Fresh databases created by prior Rust migrations already have the desired
-- UUID foreign keys. This migration is mainly for databases that had the old
-- apps/web Drizzle schema before the Rust ownership move.

DO $$
DECLARE
    constraint_row RECORD;
BEGIN
    FOR constraint_row IN
        SELECT conrelid::regclass::text AS table_name, conname AS constraint_name
        FROM pg_constraint
        WHERE confrelid = 'auth_users'::regclass
    LOOP
        EXECUTE format(
            'ALTER TABLE %s DROP CONSTRAINT IF EXISTS %I',
            constraint_row.table_name,
            constraint_row.constraint_name
        );
    END LOOP;
EXCEPTION
    WHEN undefined_table THEN
        NULL;
END $$;

DELETE FROM organization_members
WHERE CASE
    WHEN user_id::text ~* '^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$'
        THEN NOT EXISTS (SELECT 1 FROM users WHERE users.id = organization_members.user_id::uuid)
    ELSE TRUE
END;

DELETE FROM workspace_members
WHERE CASE
    WHEN user_id::text ~* '^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$'
        THEN NOT EXISTS (SELECT 1 FROM users WHERE users.id = workspace_members.user_id::uuid)
    ELSE TRUE
END;

UPDATE workspace_invites
SET invited_by_user_id = NULL
WHERE invited_by_user_id IS NOT NULL
  AND CASE
    WHEN invited_by_user_id::text ~* '^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$'
        THEN NOT EXISTS (SELECT 1 FROM users WHERE users.id = workspace_invites.invited_by_user_id::uuid)
    ELSE TRUE
END;

UPDATE workspace_api_keys
SET created_by_user_id = NULL
WHERE created_by_user_id IS NOT NULL
  AND CASE
    WHEN created_by_user_id::text ~* '^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$'
        THEN NOT EXISTS (SELECT 1 FROM users WHERE users.id = workspace_api_keys.created_by_user_id::uuid)
    ELSE TRUE
END;

DO $$
BEGIN
    IF EXISTS (
        SELECT 1 FROM information_schema.columns
        WHERE table_schema = 'public'
          AND table_name = 'organization_members'
          AND column_name = 'user_id'
          AND udt_name <> 'uuid'
    ) THEN
        ALTER TABLE organization_members
            ALTER COLUMN user_id TYPE UUID USING user_id::uuid;
    END IF;

    IF EXISTS (
        SELECT 1 FROM information_schema.columns
        WHERE table_schema = 'public'
          AND table_name = 'workspace_members'
          AND column_name = 'user_id'
          AND udt_name <> 'uuid'
    ) THEN
        ALTER TABLE workspace_members
            ALTER COLUMN user_id TYPE UUID USING user_id::uuid;
    END IF;

    IF EXISTS (
        SELECT 1 FROM information_schema.columns
        WHERE table_schema = 'public'
          AND table_name = 'workspace_invites'
          AND column_name = 'invited_by_user_id'
          AND udt_name <> 'uuid'
    ) THEN
        ALTER TABLE workspace_invites
            ALTER COLUMN invited_by_user_id TYPE UUID USING invited_by_user_id::uuid;
    END IF;

    IF EXISTS (
        SELECT 1 FROM information_schema.columns
        WHERE table_schema = 'public'
          AND table_name = 'workspace_api_keys'
          AND column_name = 'created_by_user_id'
          AND udt_name <> 'uuid'
    ) THEN
        ALTER TABLE workspace_api_keys
            ALTER COLUMN created_by_user_id TYPE UUID USING created_by_user_id::uuid;
    END IF;
END $$;

DO $$
DECLARE
    target RECORD;
BEGIN
    FOR target IN
        SELECT * FROM (VALUES
            ('organizations', 'created_at'),
            ('organizations', 'updated_at'),
            ('organization_members', 'created_at'),
            ('workspaces', 'created_at'),
            ('workspaces', 'updated_at'),
            ('workspaces', 'deleted_at'),
            ('workspace_members', 'created_at'),
            ('workspace_invites', 'created_at'),
            ('workspace_invites', 'expires_at'),
            ('workspace_settings', 'updated_at'),
            ('workspace_api_keys', 'created_at'),
            ('workspace_api_keys', 'last_used_at'),
            ('workspace_api_keys', 'revoked_at'),
            ('knowledge_sources', 'created_at'),
            ('knowledge_sources', 'updated_at'),
            ('knowledge_sources', 'last_indexed_at'),
            ('knowledge_sources', 'deleted_at')
        ) AS columns_to_convert(table_name, column_name)
    LOOP
        IF EXISTS (
            SELECT 1 FROM information_schema.columns
            WHERE table_schema = 'public'
              AND table_name = target.table_name
              AND column_name = target.column_name
              AND data_type = 'timestamp without time zone'
        ) THEN
            EXECUTE format(
                'ALTER TABLE %I ALTER COLUMN %I TYPE TIMESTAMPTZ USING %I AT TIME ZONE ''UTC''',
                target.table_name,
                target.column_name,
                target.column_name
            );
        END IF;
    END LOOP;
END $$;

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_constraint WHERE conname = 'organization_members_user_id_users_id_fk'
    ) THEN
        ALTER TABLE organization_members
            ADD CONSTRAINT organization_members_user_id_users_id_fk
            FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE;
    END IF;

    IF NOT EXISTS (
        SELECT 1 FROM pg_constraint WHERE conname = 'workspace_members_user_id_users_id_fk'
    ) THEN
        ALTER TABLE workspace_members
            ADD CONSTRAINT workspace_members_user_id_users_id_fk
            FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE;
    END IF;

    IF NOT EXISTS (
        SELECT 1 FROM pg_constraint WHERE conname = 'workspace_invites_invited_by_user_id_users_id_fk'
    ) THEN
        ALTER TABLE workspace_invites
            ADD CONSTRAINT workspace_invites_invited_by_user_id_users_id_fk
            FOREIGN KEY (invited_by_user_id) REFERENCES users(id) ON DELETE SET NULL;
    END IF;

    IF NOT EXISTS (
        SELECT 1 FROM pg_constraint WHERE conname = 'workspace_api_keys_created_by_user_id_users_id_fk'
    ) THEN
        ALTER TABLE workspace_api_keys
            ADD CONSTRAINT workspace_api_keys_created_by_user_id_users_id_fk
            FOREIGN KEY (created_by_user_id) REFERENCES users(id) ON DELETE SET NULL;
    END IF;
END $$;

DROP TABLE IF EXISTS auth_accounts;
DROP TABLE IF EXISTS auth_sessions;
DROP TABLE IF EXISTS auth_verification_tokens;
DROP TABLE IF EXISTS auth_users;
