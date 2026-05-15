-- Username/password accounts for self-hosters who can't configure
-- the Google/GitHub OAuth providers in apps/web. `password_hash` is
-- the argon2id PHC-string of the SHA-256-hex the client sent — never
-- the raw password.

CREATE TABLE users (
    id              UUID        PRIMARY KEY,
    username        TEXT        NOT NULL,
    password_hash   TEXT        NOT NULL,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE UNIQUE INDEX users_username_idx ON users (LOWER(username));
