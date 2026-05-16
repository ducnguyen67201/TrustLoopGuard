CREATE TABLE IF NOT EXISTS knowledge_sources (
    id TEXT PRIMARY KEY,
    workspace_id TEXT NOT NULL,
    title TEXT NOT NULL,
    kind TEXT NOT NULL,
    location TEXT,
    status TEXT NOT NULL DEFAULT 'draft',
    metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    last_indexed_at TIMESTAMPTZ,
    deleted_at TIMESTAMPTZ,
    CONSTRAINT knowledge_sources_kind_check CHECK (kind IN ('url', 'file', 'note')),
    CONSTRAINT knowledge_sources_status_check CHECK (status IN ('draft', 'indexing', 'ready', 'failed'))
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
