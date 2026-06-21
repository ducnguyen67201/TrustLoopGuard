# Knowledge sources

Knowledge sources are Rust-owned trusted references used by guardrails to ground
LLM-facing checks. They are stored in `tl-storage`, surfaced through the Rust
knowledge-source API, and consumed by the Tier 3 hallucination judge through the
engine's `KnowledgeRetriever` seam.

## Ownership

The dashboard may create, list, and download knowledge sources, but it does not
own indexing or runtime retrieval. The authoritative data lives in Postgres:

- `knowledge_sources` stores source metadata and lifecycle status.
- `knowledge_source_files` stores uploaded file bytes.
- `knowledge_source_chunks` stores bounded text chunks extracted from notes and
  text-like files.
- `knowledge_chunk_embeddings` stores optional vector embeddings for those
  chunks.

Agent profiles reference approved sources by `knowledge_sources[].kb_id`. The
profile reference is a catalog pointer; it is not copied into every
`GuardEvent`, and it is not enough by itself to ground an answer.

## Ingestion

Knowledge-source ingestion is a cold-path operation. On create, TrustLoopGuard
stores source metadata, extracts text from note sources and text-like files,
splits that text into deterministic chunks, and stores the chunks separately.
URL sources and unsupported binary files can be stored as metadata before a
fetcher or parser exists for them.

When vector grounding is enabled, the Postgres adapter embeds missing or stale
chunks and writes their vectors to `knowledge_chunk_embeddings`. The MVP uses
the deterministic `MockEmbedder` from `tl-fuzzy`, so local tests and default
builds do not need a model download.

## Runtime retrieval

Runtime checks do not read the full knowledge base. Tier 3 only asks the
retriever for snippets when the hallucination judge has a configured route. The
retriever builds a query from the user input and proposed output, filters chunks
to the current workspace and the agent profile's approved source ids, ranks
them with lexical, vector, or hybrid scoring, and returns a capped set of
snippets.

Those snippets are appended to request-provided `context.docs` before the
hallucination prompt is built. If retrieval fails or times out, the guardrail
fails open to the existing behavior and uses only `context.docs`.

## Operator flag

Knowledge grounding is globally operator-controlled, not a workspace setting.
The on/off switch lives in Postgres so operators can change behavior without
server restart:

```sql
UPDATE global_feature_flags
SET enabled = true, updated_at = now(), updated_by = 'operator'
WHERE key = 'knowledge_grounding';
```

Disable it the same way:

```sql
UPDATE global_feature_flags
SET enabled = false, updated_at = now(), updated_by = 'operator'
WHERE key = 'knowledge_grounding';
```

The server reads this flag at retrieval time with a short cache, so changes
take effect without redeploying or restarting. The default seeded value is
`false`.

Environment variables only tune retrieval behavior:

```text
TL_KNOWLEDGE_GROUNDING_MODE=off|lexical|vector|hybrid
TL_KNOWLEDGE_MAX_CHUNKS=5
TL_KNOWLEDGE_MAX_SNIPPET_CHARS=8000
TL_KNOWLEDGE_MAX_CHUNK_CHARS=1500
TL_KNOWLEDGE_MIN_SIMILARITY=0.65
TL_KNOWLEDGE_RETRIEVAL_TIMEOUT_MS=50
TL_KNOWLEDGE_EMBEDDING_MODEL=mock-word-bag-64
```

When disabled, source creation still works and chunks can still be stored, but
runtime Tier 3 does not retrieve managed snippets.

## Cost controls

The cost boundary is the retrieval cap, not the source size. TrustLoopGuard
keeps cost bounded by embedding chunks off the hot path when possible, embedding
the runtime query once, limiting top-K snippets, limiting total snippet
characters, and enforcing a short retrieval timeout. The LLM judge sees only the
selected snippets, never the whole corpus.
