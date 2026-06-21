use chrono::{DateTime, Utc};
use diesel::prelude::*;
use diesel::upsert::excluded;
use diesel_async::RunQueryDsl;
use sha2::{Digest, Sha256};

use crate::postgres::{DbConnection, DbPool};
use crate::schema::{
    knowledge_chunk_embeddings, knowledge_source_chunks, knowledge_source_files, knowledge_sources,
};
use crate::StorageError;

const DEFAULT_CHUNK_MAX_CHARS: usize = 1_500;

#[derive(Debug, Clone)]
pub struct NewKnowledgeFile {
    pub file_name: String,
    pub media_type: String,
    pub data: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct NewKnowledgeSource {
    pub title: String,
    pub kind: String,
    pub location: Option<String>,
    pub notes: Option<String>,
    pub file: Option<NewKnowledgeFile>,
}

#[derive(Debug, Clone, Queryable, Selectable)]
#[diesel(table_name = knowledge_sources)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct KnowledgeSourceRow {
    pub id: String,
    pub title: String,
    pub kind: String,
    pub location: Option<String>,
    pub status: String,
    pub metadata: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub last_indexed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Queryable)]
pub struct KnowledgeFileRow {
    pub file_name: String,
    pub media_type: String,
    pub byte_size: i32,
    pub data: Vec<u8>,
}

#[derive(Debug, Clone, Queryable, Selectable)]
#[diesel(table_name = knowledge_source_chunks)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct KnowledgeChunkRow {
    pub id: String,
    pub workspace_id: String,
    pub knowledge_source_id: String,
    pub chunk_index: i32,
    pub text: String,
    pub checksum_sha256: String,
    pub char_count: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Queryable, Selectable)]
#[diesel(table_name = knowledge_chunk_embeddings)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct KnowledgeEmbeddingRow {
    pub chunk_id: String,
    pub model: String,
    pub dimension: i32,
    pub vector: serde_json::Value,
    pub checksum_sha256: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct NewKnowledgeEmbedding {
    pub chunk_id: String,
    pub model: String,
    pub dimension: i32,
    pub vector: Vec<f32>,
    pub checksum_sha256: String,
}

#[derive(Clone)]
pub struct KnowledgeRepo {
    pool: DbPool,
}

impl KnowledgeRepo {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    pub async fn list(&self, workspace_id: &str) -> Result<Vec<KnowledgeSourceRow>, StorageError> {
        let mut conn = self.connection().await?;
        knowledge_sources::table
            .filter(knowledge_sources::workspace_id.eq(workspace_id))
            .filter(knowledge_sources::deleted_at.is_null())
            .select(KnowledgeSourceRow::as_select())
            .order(knowledge_sources::title.asc())
            .load::<KnowledgeSourceRow>(&mut conn)
            .await
            .map_err(|e| StorageError::Internal(format!("knowledge source list: {e}")))
    }

    pub async fn create(
        &self,
        workspace_id: &str,
        input: NewKnowledgeSource,
    ) -> Result<KnowledgeSourceRow, StorageError> {
        let id = uuid::Uuid::new_v4().to_string();
        let now = Utc::now();
        let chunks = chunks_for_source(&input);
        let mut metadata = serde_json::Map::new();
        if let Some(notes) = input
            .notes
            .as_ref()
            .filter(|value| !value.trim().is_empty())
        {
            metadata.insert("notes".into(), serde_json::Value::String(notes.clone()));
        }
        let file_metadata = input.file.as_ref().map(|file| {
            let checksum_sha256 = sha256_hex(&file.data);
            let byte_size = file.data.len().min(i32::MAX as usize) as i32;
            serde_json::json!({
                "fileName": file.file_name,
                "mediaType": file.media_type,
                "byteSize": byte_size,
                "checksumSha256": checksum_sha256,
            })
        });
        if let Some(file) = file_metadata {
            metadata.insert("file".into(), file);
        }
        metadata.insert(
            "indexing".into(),
            serde_json::json!({
                "chunkCount": chunks.len(),
                "chunkMaxChars": DEFAULT_CHUNK_MAX_CHARS,
            }),
        );

        let metadata = serde_json::Value::Object(metadata);
        let status = "ready";
        let row = {
            let mut conn = self.connection().await?;
            diesel::insert_into(knowledge_sources::table)
                .values((
                    knowledge_sources::id.eq(&id),
                    knowledge_sources::workspace_id.eq(workspace_id),
                    knowledge_sources::title.eq(&input.title),
                    knowledge_sources::kind.eq(&input.kind),
                    knowledge_sources::location.eq(input.location.as_deref()),
                    knowledge_sources::status.eq(status),
                    knowledge_sources::metadata.eq(&metadata),
                    knowledge_sources::created_at.eq(now),
                    knowledge_sources::updated_at.eq(now),
                    knowledge_sources::last_indexed_at.eq(Some(now)),
                    knowledge_sources::deleted_at.eq(None::<DateTime<Utc>>),
                ))
                .returning(KnowledgeSourceRow::as_returning())
                .get_result::<KnowledgeSourceRow>(&mut conn)
                .await
                .map_err(|e| StorageError::Internal(format!("knowledge source insert: {e}")))?
        };

        if let Some(file) = input.file {
            let byte_size = i32::try_from(file.data.len())
                .map_err(|_| StorageError::Internal("knowledge file too large".into()))?;
            let checksum_sha256 = sha256_hex(&file.data);
            let mut conn = self.connection().await?;
            diesel::insert_into(knowledge_source_files::table)
                .values((
                    knowledge_source_files::knowledge_source_id.eq(&id),
                    knowledge_source_files::file_name.eq(&file.file_name),
                    knowledge_source_files::media_type.eq(&file.media_type),
                    knowledge_source_files::byte_size.eq(byte_size),
                    knowledge_source_files::checksum_sha256.eq(&checksum_sha256),
                    knowledge_source_files::data.eq(&file.data),
                    knowledge_source_files::created_at.eq(now),
                    knowledge_source_files::updated_at.eq(now),
                ))
                .on_conflict(knowledge_source_files::knowledge_source_id)
                .do_update()
                .set((
                    knowledge_source_files::file_name
                        .eq(excluded(knowledge_source_files::file_name)),
                    knowledge_source_files::media_type
                        .eq(excluded(knowledge_source_files::media_type)),
                    knowledge_source_files::byte_size
                        .eq(excluded(knowledge_source_files::byte_size)),
                    knowledge_source_files::checksum_sha256
                        .eq(excluded(knowledge_source_files::checksum_sha256)),
                    knowledge_source_files::data.eq(excluded(knowledge_source_files::data)),
                    knowledge_source_files::updated_at
                        .eq(excluded(knowledge_source_files::updated_at)),
                ))
                .execute(&mut conn)
                .await
                .map_err(|e| StorageError::Internal(format!("knowledge file insert: {e}")))?;
        }

        self.replace_chunks(workspace_id, &id, &chunks).await?;

        Ok(row)
    }

    pub async fn replace_chunks(
        &self,
        workspace_id: &str,
        source_id: &str,
        chunks: &[String],
    ) -> Result<Vec<KnowledgeChunkRow>, StorageError> {
        let now = Utc::now();
        let mut conn = self.connection().await?;

        diesel::delete(
            knowledge_source_chunks::table
                .filter(knowledge_source_chunks::workspace_id.eq(workspace_id))
                .filter(knowledge_source_chunks::knowledge_source_id.eq(source_id)),
        )
        .execute(&mut conn)
        .await
        .map_err(|e| StorageError::Internal(format!("knowledge chunks delete: {e}")))?;

        if chunks.is_empty() {
            return Ok(vec![]);
        }

        let rows = chunks
            .iter()
            .enumerate()
            .map(|(index, text)| {
                let id = format!("{source_id}:{index}");
                let checksum = sha256_hex(text.as_bytes());
                let char_count = text.chars().count().min(i32::MAX as usize) as i32;
                (
                    knowledge_source_chunks::id.eq(id),
                    knowledge_source_chunks::workspace_id.eq(workspace_id),
                    knowledge_source_chunks::knowledge_source_id.eq(source_id),
                    knowledge_source_chunks::chunk_index.eq(index as i32),
                    knowledge_source_chunks::text.eq(text.as_str()),
                    knowledge_source_chunks::checksum_sha256.eq(checksum),
                    knowledge_source_chunks::char_count.eq(char_count),
                    knowledge_source_chunks::created_at.eq(now),
                    knowledge_source_chunks::updated_at.eq(now),
                )
            })
            .collect::<Vec<_>>();

        diesel::insert_into(knowledge_source_chunks::table)
            .values(rows)
            .returning(KnowledgeChunkRow::as_returning())
            .get_results::<KnowledgeChunkRow>(&mut conn)
            .await
            .map_err(|e| StorageError::Internal(format!("knowledge chunks insert: {e}")))
    }

    pub async fn list_ready_chunks_for_sources(
        &self,
        workspace_id: &str,
        source_ids: &[String],
    ) -> Result<Vec<KnowledgeChunkRow>, StorageError> {
        if source_ids.is_empty() {
            return Ok(vec![]);
        }

        let mut conn = self.connection().await?;
        knowledge_source_chunks::table
            .inner_join(knowledge_sources::table)
            .filter(knowledge_source_chunks::workspace_id.eq(workspace_id))
            .filter(knowledge_source_chunks::knowledge_source_id.eq_any(source_ids))
            .filter(knowledge_sources::status.eq("ready"))
            .filter(knowledge_sources::deleted_at.is_null())
            .select(KnowledgeChunkRow::as_select())
            .order((
                knowledge_source_chunks::knowledge_source_id.asc(),
                knowledge_source_chunks::chunk_index.asc(),
            ))
            .load::<KnowledgeChunkRow>(&mut conn)
            .await
            .map_err(|e| StorageError::Internal(format!("knowledge chunks list: {e}")))
    }

    pub async fn list_embeddings_for_chunks(
        &self,
        chunk_ids: &[String],
        model: &str,
    ) -> Result<Vec<KnowledgeEmbeddingRow>, StorageError> {
        if chunk_ids.is_empty() {
            return Ok(vec![]);
        }

        let mut conn = self.connection().await?;
        knowledge_chunk_embeddings::table
            .filter(knowledge_chunk_embeddings::chunk_id.eq_any(chunk_ids))
            .filter(knowledge_chunk_embeddings::model.eq(model))
            .select(KnowledgeEmbeddingRow::as_select())
            .load::<KnowledgeEmbeddingRow>(&mut conn)
            .await
            .map_err(|e| StorageError::Internal(format!("knowledge embeddings list: {e}")))
    }

    pub async fn upsert_embeddings(
        &self,
        embeddings: &[NewKnowledgeEmbedding],
    ) -> Result<(), StorageError> {
        if embeddings.is_empty() {
            return Ok(());
        }

        let now = Utc::now();
        let values = embeddings
            .iter()
            .map(|embedding| {
                (
                    knowledge_chunk_embeddings::chunk_id.eq(embedding.chunk_id.as_str()),
                    knowledge_chunk_embeddings::model.eq(embedding.model.as_str()),
                    knowledge_chunk_embeddings::dimension.eq(embedding.dimension),
                    knowledge_chunk_embeddings::vector.eq(serde_json::json!(embedding.vector)),
                    knowledge_chunk_embeddings::checksum_sha256
                        .eq(embedding.checksum_sha256.as_str()),
                    knowledge_chunk_embeddings::created_at.eq(now),
                    knowledge_chunk_embeddings::updated_at.eq(now),
                )
            })
            .collect::<Vec<_>>();

        let mut conn = self.connection().await?;
        diesel::insert_into(knowledge_chunk_embeddings::table)
            .values(values)
            .on_conflict(knowledge_chunk_embeddings::chunk_id)
            .do_update()
            .set((
                knowledge_chunk_embeddings::model.eq(excluded(knowledge_chunk_embeddings::model)),
                knowledge_chunk_embeddings::dimension
                    .eq(excluded(knowledge_chunk_embeddings::dimension)),
                knowledge_chunk_embeddings::vector.eq(excluded(knowledge_chunk_embeddings::vector)),
                knowledge_chunk_embeddings::checksum_sha256
                    .eq(excluded(knowledge_chunk_embeddings::checksum_sha256)),
                knowledge_chunk_embeddings::updated_at
                    .eq(excluded(knowledge_chunk_embeddings::updated_at)),
            ))
            .execute(&mut conn)
            .await
            .map_err(|e| StorageError::Internal(format!("knowledge embeddings upsert: {e}")))?;

        Ok(())
    }

    pub async fn get_file(
        &self,
        workspace_id: &str,
        source_id: &str,
    ) -> Result<KnowledgeFileRow, StorageError> {
        let mut conn = self.connection().await?;
        knowledge_source_files::table
            .inner_join(knowledge_sources::table)
            .filter(knowledge_sources::workspace_id.eq(workspace_id))
            .filter(knowledge_sources::id.eq(source_id))
            .filter(knowledge_sources::kind.eq("file"))
            .filter(knowledge_sources::deleted_at.is_null())
            .select((
                knowledge_source_files::file_name,
                knowledge_source_files::media_type,
                knowledge_source_files::byte_size,
                knowledge_source_files::data,
            ))
            .first::<KnowledgeFileRow>(&mut conn)
            .await
            .optional()
            .map_err(|e| StorageError::Internal(format!("knowledge file get: {e}")))?
            .ok_or(StorageError::NotFound)
    }

    async fn connection(&self) -> Result<DbConnection<'_>, StorageError> {
        self.pool
            .get()
            .await
            .map_err(|e| StorageError::Internal(format!("db pool: {e}")))
    }
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut out = String::with_capacity(digest.len() * 2);
    for byte in digest {
        out.push_str(&format!("{byte:02x}"));
    }
    out
}

fn chunks_for_source(input: &NewKnowledgeSource) -> Vec<String> {
    let text = match input.kind.as_str() {
        "note" => input.notes.as_deref(),
        "file" => input.file.as_ref().and_then(text_from_file),
        _ => None,
    };

    text.map(|value| chunk_text(value, DEFAULT_CHUNK_MAX_CHARS))
        .unwrap_or_default()
}

fn text_from_file(file: &NewKnowledgeFile) -> Option<&str> {
    if !is_text_like_file(&file.file_name, &file.media_type) {
        return None;
    }
    std::str::from_utf8(&file.data).ok()
}

fn is_text_like_file(file_name: &str, media_type: &str) -> bool {
    let media_type = media_type.to_ascii_lowercase();
    if media_type.starts_with("text/")
        || matches!(
            media_type.as_str(),
            "application/json" | "application/yaml" | "application/x-yaml"
        )
    {
        return true;
    }

    let lower_name = file_name.to_ascii_lowercase();
    [".txt", ".md", ".markdown", ".json", ".yaml", ".yml", ".csv"]
        .iter()
        .any(|suffix| lower_name.ends_with(suffix))
}

fn chunk_text(text: &str, max_chars: usize) -> Vec<String> {
    let mut chunks = Vec::new();
    let mut current = String::new();

    for paragraph in text.split("\n\n").map(str::trim).filter(|p| !p.is_empty()) {
        let paragraph_chars = paragraph.chars().count();
        if paragraph_chars > max_chars {
            flush_chunk(&mut chunks, &mut current);
            chunks.extend(chunk_long_text(paragraph, max_chars));
            continue;
        }

        let separator_chars = usize::from(!current.is_empty()) * 2;
        if current.chars().count() + separator_chars + paragraph_chars > max_chars {
            flush_chunk(&mut chunks, &mut current);
        }

        if !current.is_empty() {
            current.push_str("\n\n");
        }
        current.push_str(paragraph);
    }

    flush_chunk(&mut chunks, &mut current);
    chunks
}

fn chunk_long_text(text: &str, max_chars: usize) -> Vec<String> {
    let mut chunks = Vec::new();
    let mut current = String::new();

    for word in text.split_whitespace() {
        let word_chars = word.chars().count();
        if word_chars > max_chars {
            flush_chunk(&mut chunks, &mut current);
            let mut piece = String::new();
            for character in word.chars() {
                if piece.chars().count() == max_chars {
                    flush_chunk(&mut chunks, &mut piece);
                }
                piece.push(character);
            }
            flush_chunk(&mut chunks, &mut piece);
            continue;
        }

        let separator_chars = usize::from(!current.is_empty());
        if current.chars().count() + separator_chars + word_chars > max_chars {
            flush_chunk(&mut chunks, &mut current);
        }
        if !current.is_empty() {
            current.push(' ');
        }
        current.push_str(word);
    }

    flush_chunk(&mut chunks, &mut current);
    chunks
}

fn flush_chunk(chunks: &mut Vec<String>, current: &mut String) {
    let trimmed = current.trim();
    if !trimmed.is_empty() {
        chunks.push(trimmed.to_string());
    }
    current.clear();
}

impl std::fmt::Debug for KnowledgeRepo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("KnowledgeRepo").finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use super::{chunk_text, chunks_for_source, NewKnowledgeFile, NewKnowledgeSource};

    #[test]
    fn chunks_notes_without_splitting_small_paragraphs() {
        let chunks = chunk_text("Refunds last 30 days.\n\nWarranty lasts 1 year.", 1_500);
        assert_eq!(
            chunks,
            vec!["Refunds last 30 days.\n\nWarranty lasts 1 year."]
        );
    }

    #[test]
    fn chunks_long_text_on_word_boundaries() {
        let chunks = chunk_text("alpha beta gamma delta", 12);
        assert_eq!(chunks, vec!["alpha beta", "gamma delta"]);
    }

    #[test]
    fn chunks_single_long_words() {
        let chunks = chunk_text("abcdefghijkl", 5);
        assert_eq!(chunks, vec!["abcde", "fghij", "kl"]);
    }

    #[test]
    fn extracts_text_like_file_chunks() {
        let source = NewKnowledgeSource {
            title: "FAQ".into(),
            kind: "file".into(),
            location: None,
            notes: None,
            file: Some(NewKnowledgeFile {
                file_name: "faq.md".into(),
                media_type: "text/markdown".into(),
                data: b"Supported content".to_vec(),
            }),
        };

        assert_eq!(chunks_for_source(&source), vec!["Supported content"]);
    }

    #[test]
    fn skips_binary_file_chunks() {
        let source = NewKnowledgeSource {
            title: "PDF".into(),
            kind: "file".into(),
            location: None,
            notes: None,
            file: Some(NewKnowledgeFile {
                file_name: "policy.pdf".into(),
                media_type: "application/pdf".into(),
                data: vec![0, 159, 146, 150],
            }),
        };

        assert!(chunks_for_source(&source).is_empty());
    }
}
