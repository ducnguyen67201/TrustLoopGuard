use chrono::{DateTime, Utc};
use diesel::prelude::*;
use diesel::upsert::excluded;
use diesel_async::RunQueryDsl;
use sha2::{Digest, Sha256};

use crate::postgres::{DbConnection, DbPool};
use crate::schema::{knowledge_source_files, knowledge_sources};
use crate::StorageError;

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

        Ok(row)
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

impl std::fmt::Debug for KnowledgeRepo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("KnowledgeRepo").finish_non_exhaustive()
    }
}
