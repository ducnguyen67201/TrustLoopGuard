use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use anyhow::Result;
use async_trait::async_trait;
use base64::{engine::general_purpose::STANDARD, Engine as _};
use tl_engine::{KnowledgeRetrievalRequest, KnowledgeRetriever, KnowledgeSnippet};
use tl_fuzzy::{Embedder, MockEmbedder};
use tl_storage::{
    KnowledgeChunkRow, KnowledgeEmbeddingRow, KnowledgeRepo, NewKnowledgeEmbedding,
    NewKnowledgeFile, NewKnowledgeSource,
};

use crate::feature_flags::{FeatureFlagStore, KNOWLEDGE_GROUNDING_FLAG};
use crate::knowledge_sources::KnowledgeStore;

use super::super::env::{KnowledgeGroundingConfig, KnowledgeGroundingMode};

const FEATURE_FLAG_CACHE_TTL: Duration = Duration::from_secs(5);
const CANDIDATE_MULTIPLIER: usize = 8;

pub struct PostgresKnowledgeAdapter {
    repo: Arc<KnowledgeRepo>,
    config: KnowledgeGroundingConfig,
    feature_flags: Arc<dyn FeatureFlagStore>,
    feature_flag_cache: Mutex<Option<CachedFeatureFlag>>,
    embedder: Arc<dyn Embedder>,
}

#[derive(Debug, Clone)]
struct CachedFeatureFlag {
    enabled: bool,
    refreshed_at: Instant,
}

impl PostgresKnowledgeAdapter {
    pub fn new(
        repo: Arc<KnowledgeRepo>,
        config: KnowledgeGroundingConfig,
        feature_flags: Arc<dyn FeatureFlagStore>,
    ) -> Arc<Self> {
        Arc::new(Self {
            repo,
            config,
            feature_flags,
            feature_flag_cache: Mutex::new(None),
            embedder: Arc::new(MockEmbedder::default()),
        })
    }

    fn vector_mode_enabled(&self) -> bool {
        matches!(
            self.config.mode,
            KnowledgeGroundingMode::Vector | KnowledgeGroundingMode::Hybrid
        )
    }

    fn cached_feature_flag(&self) -> Option<bool> {
        let cache = self
            .feature_flag_cache
            .lock()
            .expect("knowledge feature flag cache");
        cache.as_ref().and_then(|cached| {
            (cached.refreshed_at.elapsed() < FEATURE_FLAG_CACHE_TTL).then_some(cached.enabled)
        })
    }

    async fn knowledge_grounding_enabled(&self) -> bool {
        if self.config.mode == KnowledgeGroundingMode::Off {
            return false;
        }
        if let Some(enabled) = self.cached_feature_flag() {
            return enabled;
        }

        match self
            .feature_flags
            .is_enabled(KNOWLEDGE_GROUNDING_FLAG)
            .await
        {
            Ok(enabled) => {
                *self
                    .feature_flag_cache
                    .lock()
                    .expect("knowledge feature flag cache") = Some(CachedFeatureFlag {
                    enabled,
                    refreshed_at: Instant::now(),
                });
                enabled
            }
            Err(error) => {
                tracing::warn!(
                    error = %error,
                    flag = KNOWLEDGE_GROUNDING_FLAG,
                    "knowledge grounding feature flag read failed"
                );
                false
            }
        }
    }

    async fn index_embeddings_for_source(&self, workspace_id: &str, source_id: &str) {
        if !self.vector_mode_enabled() || !self.knowledge_grounding_enabled().await {
            return;
        }

        match self
            .repo
            .list_ready_chunks_for_sources(workspace_id, &[source_id.to_string()])
            .await
        {
            Ok(chunks) => {
                if let Err(error) = self.ensure_embeddings(&chunks).await {
                    tracing::warn!(
                        workspace_id,
                        source_id,
                        error = %error,
                        "knowledge embedding index failed"
                    );
                }
            }
            Err(error) => {
                tracing::warn!(
                    workspace_id,
                    source_id,
                    error = %error,
                    "knowledge chunk load failed during indexing"
                );
            }
        }
    }

    fn max_candidate_chunks(&self) -> usize {
        self.config
            .max_chunks
            .saturating_mul(CANDIDATE_MULTIPLIER)
            .max(1)
    }

    async fn retrieve_inner(
        &self,
        request: KnowledgeRetrievalRequest,
    ) -> Result<Vec<KnowledgeSnippet>, String> {
        if !self.knowledge_grounding_enabled().await || request.source_ids.is_empty() {
            return Ok(vec![]);
        }

        let mut chunks = self
            .repo
            .list_ready_chunks_for_sources(&request.workspace_id, &request.source_ids)
            .await
            .map_err(|error| error.to_string())?;
        if chunks.is_empty() {
            return Ok(vec![]);
        }
        chunks.truncate(self.max_candidate_chunks());

        let query = format!("{}\n\n{}", request.input, request.proposed_output);
        let mut scored = match self.config.mode {
            KnowledgeGroundingMode::Off => vec![],
            KnowledgeGroundingMode::Lexical => lexical_scores(&query, &chunks),
            KnowledgeGroundingMode::Vector => self.vector_scores(&query, &chunks).await?,
            KnowledgeGroundingMode::Hybrid => {
                let lexical = lexical_scores(&query, &chunks);
                let vector = self.vector_scores(&query, &chunks).await?;
                hybrid_scores(lexical, vector)
            }
        };

        scored.retain(|candidate| candidate.score >= self.config.min_similarity);
        scored.sort_by(|left, right| {
            right
                .score
                .partial_cmp(&left.score)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| left.chunk.id.cmp(&right.chunk.id))
        });

        Ok(cap_snippets(
            scored,
            self.config.max_chunks,
            self.config.max_chunk_chars,
            self.config.max_snippet_chars,
        ))
    }

    async fn vector_scores(
        &self,
        query: &str,
        chunks: &[KnowledgeChunkRow],
    ) -> Result<Vec<ScoredChunk>, String> {
        self.ensure_embeddings(chunks).await?;

        let chunk_ids = chunks
            .iter()
            .map(|chunk| chunk.id.clone())
            .collect::<Vec<_>>();
        let embeddings = self
            .repo
            .list_embeddings_for_chunks(&chunk_ids, &self.config.embedding_model)
            .await
            .map_err(|error| error.to_string())?;
        let embedding_by_chunk = embeddings
            .into_iter()
            .filter_map(|embedding| {
                let vector = vector_from_row(&embedding)?;
                Some((embedding.chunk_id, vector))
            })
            .collect::<std::collections::HashMap<_, _>>();

        let query_embedding = self
            .embedder
            .embed(&[query.to_string()])
            .await
            .map_err(|error| error.to_string())?
            .into_iter()
            .next()
            .ok_or_else(|| "embedder returned no query vector".to_string())?;

        Ok(chunks
            .iter()
            .filter_map(|chunk| {
                let vector = embedding_by_chunk.get(&chunk.id)?;
                Some(ScoredChunk {
                    chunk: chunk.clone(),
                    score: cosine_similarity(&query_embedding, vector),
                })
            })
            .collect())
    }

    async fn ensure_embeddings(&self, chunks: &[KnowledgeChunkRow]) -> Result<(), String> {
        if chunks.is_empty() {
            return Ok(());
        }

        let chunk_ids = chunks
            .iter()
            .map(|chunk| chunk.id.clone())
            .collect::<Vec<_>>();
        let existing = self
            .repo
            .list_embeddings_for_chunks(&chunk_ids, &self.config.embedding_model)
            .await
            .map_err(|error| error.to_string())?;
        let existing_checksums = existing
            .into_iter()
            .map(|row| (row.chunk_id, row.checksum_sha256))
            .collect::<std::collections::HashMap<_, _>>();
        let missing = chunks
            .iter()
            .filter(|chunk| existing_checksums.get(&chunk.id) != Some(&chunk.checksum_sha256))
            .cloned()
            .collect::<Vec<_>>();

        if missing.is_empty() {
            return Ok(());
        }

        let texts = missing
            .iter()
            .map(|chunk| chunk.text.clone())
            .collect::<Vec<_>>();
        let vectors = self
            .embedder
            .embed(&texts)
            .await
            .map_err(|error| error.to_string())?;
        let embeddings = missing
            .iter()
            .zip(vectors)
            .map(|(chunk, vector)| NewKnowledgeEmbedding {
                chunk_id: chunk.id.clone(),
                model: self.config.embedding_model.clone(),
                dimension: self.embedder.dimension() as i32,
                vector,
                checksum_sha256: chunk.checksum_sha256.clone(),
            })
            .collect::<Vec<_>>();

        self.repo
            .upsert_embeddings(&embeddings)
            .await
            .map_err(|error| error.to_string())
    }
}

#[async_trait]
impl KnowledgeStore for PostgresKnowledgeAdapter {
    async fn list(
        &self,
        workspace_id: &str,
    ) -> Result<Vec<tl_core::KnowledgeSourceDocument>, crate::knowledge_sources::KnowledgeStoreError>
    {
        self.repo
            .list(workspace_id)
            .await
            .map_err(|e| crate::knowledge_sources::KnowledgeStoreError::Internal(e.to_string()))?
            .into_iter()
            .map(knowledge_row_to_document)
            .collect()
    }

    async fn create(
        &self,
        workspace_id: &str,
        input: tl_core::CreateKnowledgeSourceRequest,
    ) -> Result<tl_core::KnowledgeSourceDocument, crate::knowledge_sources::KnowledgeStoreError>
    {
        let file = match input.file {
            Some(file) => {
                let data = crate::knowledge_sources::decode_file_data(&file.data_base64)?;
                Some(NewKnowledgeFile {
                    file_name: file.file_name,
                    media_type: file.media_type,
                    data,
                })
            }
            None => None,
        };
        let row = self
            .repo
            .create(
                workspace_id,
                NewKnowledgeSource {
                    title: input.title,
                    kind: knowledge_kind_text(input.kind).to_string(),
                    location: input.location,
                    notes: input.notes,
                    file,
                },
            )
            .await
            .map_err(|e| crate::knowledge_sources::KnowledgeStoreError::Internal(e.to_string()))?;
        let timeout = Duration::from_millis(self.config.retrieval_timeout_ms);
        if tokio::time::timeout(
            timeout,
            self.index_embeddings_for_source(workspace_id, &row.id),
        )
        .await
        .is_err()
        {
            tracing::warn!(
                workspace_id,
                source_id = %row.id,
                timeout_ms = self.config.retrieval_timeout_ms,
                "knowledge embedding indexing timed out"
            );
        }
        knowledge_row_to_document(row)
    }

    async fn get_file(
        &self,
        workspace_id: &str,
        source_id: &str,
    ) -> Result<tl_core::KnowledgeSourceFileResponse, crate::knowledge_sources::KnowledgeStoreError>
    {
        let row = self
            .repo
            .get_file(workspace_id, source_id)
            .await
            .map_err(|e| match e {
                tl_storage::StorageError::NotFound => {
                    crate::knowledge_sources::KnowledgeStoreError::NotFound
                }
                other => crate::knowledge_sources::KnowledgeStoreError::Internal(other.to_string()),
            })?;
        Ok(tl_core::KnowledgeSourceFileResponse {
            file_name: row.file_name,
            media_type: row.media_type,
            byte_size: row.byte_size,
            data_base64: STANDARD.encode(row.data),
        })
    }
}

#[async_trait]
impl KnowledgeRetriever for PostgresKnowledgeAdapter {
    async fn retrieve(&self, request: KnowledgeRetrievalRequest) -> Vec<KnowledgeSnippet> {
        let timeout = Duration::from_millis(self.config.retrieval_timeout_ms);
        match tokio::time::timeout(timeout, self.retrieve_inner(request)).await {
            Ok(Ok(snippets)) => snippets,
            Ok(Err(error)) => {
                tracing::warn!(error = %error, "knowledge retrieval failed");
                vec![]
            }
            Err(_) => {
                tracing::warn!(
                    timeout_ms = self.config.retrieval_timeout_ms,
                    "knowledge retrieval timed out"
                );
                vec![]
            }
        }
    }
}

#[derive(Debug, Clone)]
struct ScoredChunk {
    chunk: KnowledgeChunkRow,
    score: f32,
}

fn lexical_scores(query: &str, chunks: &[KnowledgeChunkRow]) -> Vec<ScoredChunk> {
    let query_tokens = tokens(query);
    if query_tokens.is_empty() {
        return vec![];
    }

    chunks
        .iter()
        .filter_map(|chunk| {
            let chunk_tokens = tokens(&chunk.text);
            if chunk_tokens.is_empty() {
                return None;
            }
            let overlap = chunk_tokens.intersection(&query_tokens).count() as f32;
            let denominator = query_tokens.len().max(chunk_tokens.len()) as f32;
            Some(ScoredChunk {
                chunk: chunk.clone(),
                score: overlap / denominator,
            })
        })
        .collect()
}

fn hybrid_scores(left: Vec<ScoredChunk>, right: Vec<ScoredChunk>) -> Vec<ScoredChunk> {
    let mut by_chunk = std::collections::HashMap::<String, ScoredChunk>::new();
    for candidate in left {
        by_chunk.insert(
            candidate.chunk.id.clone(),
            ScoredChunk {
                score: candidate.score * 0.3,
                ..candidate
            },
        );
    }
    for candidate in right {
        by_chunk
            .entry(candidate.chunk.id.clone())
            .and_modify(|current| current.score += candidate.score * 0.7)
            .or_insert_with(|| ScoredChunk {
                score: candidate.score * 0.7,
                ..candidate
            });
    }
    by_chunk.into_values().collect()
}

fn cap_snippets(
    scored: Vec<ScoredChunk>,
    max_chunks: usize,
    max_chunk_chars: usize,
    max_snippet_chars: usize,
) -> Vec<KnowledgeSnippet> {
    let mut remaining_chars = max_snippet_chars;
    let mut snippets = Vec::new();

    for candidate in scored.into_iter().take(max_chunks) {
        if remaining_chars == 0 {
            break;
        }
        let limit = remaining_chars.min(max_chunk_chars);
        let text = truncate_chars(candidate.chunk.text.trim(), limit);
        remaining_chars = remaining_chars.saturating_sub(text.chars().count());
        snippets.push(KnowledgeSnippet {
            source_id: candidate.chunk.knowledge_source_id,
            chunk_id: candidate.chunk.id,
            score: candidate.score,
            text,
        });
    }

    snippets
}

fn truncate_chars(text: &str, max_chars: usize) -> String {
    text.chars().take(max_chars).collect()
}

fn tokens(text: &str) -> std::collections::BTreeSet<String> {
    text.to_ascii_lowercase()
        .split(|character: char| !character.is_ascii_alphanumeric())
        .filter(|token| token.len() > 1)
        .map(str::to_string)
        .collect()
}

fn vector_from_row(row: &KnowledgeEmbeddingRow) -> Option<Vec<f32>> {
    let vector = row
        .vector
        .as_array()?
        .iter()
        .map(|value| value.as_f64().map(|number| number as f32))
        .collect::<Option<Vec<_>>>()?;
    if vector.len() == row.dimension as usize {
        Some(vector)
    } else {
        None
    }
}

fn cosine_similarity(left: &[f32], right: &[f32]) -> f32 {
    if left.len() != right.len() {
        return 0.0;
    }
    let dot = left
        .iter()
        .zip(right)
        .map(|(left, right)| left * right)
        .sum::<f32>();
    let left_norm = left.iter().map(|value| value * value).sum::<f32>().sqrt();
    let right_norm = right.iter().map(|value| value * value).sum::<f32>().sqrt();
    if left_norm == 0.0 || right_norm == 0.0 {
        return 0.0;
    }
    (dot / (left_norm * right_norm)).max(0.0)
}

fn knowledge_row_to_document(
    row: tl_storage::KnowledgeSourceRow,
) -> Result<tl_core::KnowledgeSourceDocument, crate::knowledge_sources::KnowledgeStoreError> {
    Ok(tl_core::KnowledgeSourceDocument {
        id: row.id,
        title: row.title,
        kind: parse_knowledge_kind(&row.kind)?,
        location: row.location,
        status: parse_knowledge_status(&row.status)?,
        metadata: row.metadata,
        created_at: row.created_at.to_rfc3339(),
        updated_at: row.updated_at.to_rfc3339(),
        last_indexed_at: row.last_indexed_at.map(|ts| ts.to_rfc3339()),
    })
}

fn knowledge_kind_text(kind: tl_core::DashboardKnowledgeSourceKind) -> &'static str {
    match kind {
        tl_core::DashboardKnowledgeSourceKind::Url => "url",
        tl_core::DashboardKnowledgeSourceKind::File => "file",
        tl_core::DashboardKnowledgeSourceKind::Note => "note",
    }
}

fn parse_knowledge_kind(
    kind: &str,
) -> Result<tl_core::DashboardKnowledgeSourceKind, crate::knowledge_sources::KnowledgeStoreError> {
    match kind {
        "url" => Ok(tl_core::DashboardKnowledgeSourceKind::Url),
        "file" => Ok(tl_core::DashboardKnowledgeSourceKind::File),
        "note" => Ok(tl_core::DashboardKnowledgeSourceKind::Note),
        other => Err(crate::knowledge_sources::KnowledgeStoreError::Internal(
            format!("unknown knowledge source kind `{other}`"),
        )),
    }
}

fn parse_knowledge_status(
    status: &str,
) -> Result<tl_core::KnowledgeSourceStatus, crate::knowledge_sources::KnowledgeStoreError> {
    match status {
        "draft" => Ok(tl_core::KnowledgeSourceStatus::Draft),
        "indexing" => Ok(tl_core::KnowledgeSourceStatus::Indexing),
        "ready" => Ok(tl_core::KnowledgeSourceStatus::Ready),
        "failed" => Ok(tl_core::KnowledgeSourceStatus::Failed),
        other => Err(crate::knowledge_sources::KnowledgeStoreError::Internal(
            format!("unknown knowledge source status `{other}`"),
        )),
    }
}
