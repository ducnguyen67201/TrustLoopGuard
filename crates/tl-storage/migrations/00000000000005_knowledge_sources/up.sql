DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_type WHERE typname = 'knowledge_source_kind') THEN
        CREATE TYPE knowledge_source_kind AS ENUM ('url', 'file', 'note');
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_type WHERE typname = 'knowledge_source_status') THEN
        CREATE TYPE knowledge_source_status AS ENUM ('draft', 'indexing', 'ready', 'failed');
    END IF;
END $$;

CREATE TABLE IF NOT EXISTS knowledge_sources (
    id TEXT PRIMARY KEY,
    workspace_id TEXT NOT NULL,
    title TEXT NOT NULL,
    kind knowledge_source_kind NOT NULL,
    location TEXT,
    status knowledge_source_status NOT NULL DEFAULT 'draft',
    metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    last_indexed_at TIMESTAMPTZ,
    deleted_at TIMESTAMPTZ
);

CREATE INDEX IF NOT EXISTS knowledge_sources_workspace_status_idx
    ON knowledge_sources (workspace_id, status);

CREATE TABLE IF NOT EXISTS knowledge_source_files (
    knowledge_source_id TEXT PRIMARY KEY REFERENCES knowledge_sources(id) ON DELETE CASCADE,
    file_name TEXT NOT NULL,
    media_type TEXT NOT NULL,
    byte_size INTEGER NOT NULL,
    checksum_sha256 TEXT NOT NULL,
    data BYTEA NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
