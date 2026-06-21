CREATE TABLE IF NOT EXISTS knowledge_source_chunks (
    id TEXT PRIMARY KEY,
    workspace_id TEXT NOT NULL,
    knowledge_source_id TEXT NOT NULL REFERENCES knowledge_sources(id) ON DELETE CASCADE,
    chunk_index INTEGER NOT NULL,
    text TEXT NOT NULL,
    checksum_sha256 TEXT NOT NULL,
    char_count INTEGER NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (knowledge_source_id, chunk_index)
);

CREATE INDEX IF NOT EXISTS knowledge_source_chunks_workspace_source_idx
    ON knowledge_source_chunks (workspace_id, knowledge_source_id);

CREATE TABLE IF NOT EXISTS knowledge_chunk_embeddings (
    chunk_id TEXT PRIMARY KEY REFERENCES knowledge_source_chunks(id) ON DELETE CASCADE,
    model TEXT NOT NULL,
    dimension INTEGER NOT NULL,
    vector JSONB NOT NULL,
    checksum_sha256 TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS knowledge_chunk_embeddings_model_idx
    ON knowledge_chunk_embeddings (model, dimension);
