CREATE TABLE oauth_identities (
    provider         TEXT        NOT NULL,
    provider_subject TEXT        NOT NULL,
    user_id          UUID        NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    email            TEXT        NOT NULL,
    created_at       TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at       TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (provider, provider_subject)
);

CREATE INDEX oauth_identities_user_id_idx ON oauth_identities (user_id);
CREATE INDEX oauth_identities_email_idx ON oauth_identities (LOWER(email));
