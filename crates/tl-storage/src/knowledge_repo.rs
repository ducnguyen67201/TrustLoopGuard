use chrono::{DateTime, Utc};
use diesel::deserialize::QueryableByName;
use diesel::prelude::*;
use diesel::sql_types::{Binary, Integer, Jsonb, Nullable, Text, Timestamptz};
use diesel_async::RunQueryDsl;
use sha2::{Digest, Sha256};

use crate::postgres::{DbConnection, DbPool};
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

#[derive(Debug, Clone, QueryableByName)]
pub struct KnowledgeSourceRow {
    #[diesel(sql_type = Text)]
    pub id: String,
    #[diesel(sql_type = Text)]
    pub title: String,
    #[diesel(sql_type = Text)]
    pub kind: String,
    #[diesel(sql_type = Nullable<Text>)]
    pub location: Option<String>,
    #[diesel(sql_type = Text)]
    pub status: String,
    #[diesel(sql_type = Jsonb)]
    pub metadata: serde_json::Value,
    #[diesel(sql_type = Timestamptz)]
    pub created_at: DateTime<Utc>,
    #[diesel(sql_type = Timestamptz)]
    pub updated_at: DateTime<Utc>,
    #[diesel(sql_type = Nullable<Timestamptz>)]
    pub last_indexed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, QueryableByName)]
pub struct KnowledgeFileRow {
    #[diesel(sql_type = Text)]
    pub file_name: String,
    #[diesel(sql_type = Text)]
    pub media_type: String,
    #[diesel(sql_type = Integer)]
    pub byte_size: i32,
    #[diesel(sql_type = Binary)]
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
        diesel::sql_query(
            r#"
            SELECT
                id,
                title,
                kind::text AS kind,
                location,
                status::text AS status,
                metadata,
                created_at,
                updated_at,
                last_indexed_at
            FROM knowledge_sources
            WHERE workspace_id = $1 AND deleted_at IS NULL
            ORDER BY title ASC
            "#,
        )
        .bind::<Text, _>(workspace_id)
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
            diesel::sql_query(
                r#"
                INSERT INTO knowledge_sources (
                    id,
                    workspace_id,
                    title,
                    kind,
                    location,
                    status,
                    metadata,
                    created_at,
                    updated_at,
                    last_indexed_at,
                    deleted_at
                )
                VALUES (
                    $1,
                    $2,
                    $3,
                    $4::knowledge_source_kind,
                    $5,
                    $6::knowledge_source_status,
                    $7,
                    $8,
                    $8,
                    $8,
                    NULL
                )
                RETURNING
                    id,
                    title,
                    kind::text AS kind,
                    location,
                    status::text AS status,
                    metadata,
                    created_at,
                    updated_at,
                    last_indexed_at
                "#,
            )
            .bind::<Text, _>(&id)
            .bind::<Text, _>(workspace_id)
            .bind::<Text, _>(&input.title)
            .bind::<Text, _>(&input.kind)
            .bind::<Nullable<Text>, _>(input.location.as_deref())
            .bind::<Text, _>(status)
            .bind::<Jsonb, _>(&metadata)
            .bind::<Timestamptz, _>(now)
            .get_result::<KnowledgeSourceRow>(&mut conn)
            .await
            .map_err(|e| StorageError::Internal(format!("knowledge source insert: {e}")))?
        };

        if let Some(file) = input.file {
            let byte_size = i32::try_from(file.data.len())
                .map_err(|_| StorageError::Internal("knowledge file too large".into()))?;
            let checksum_sha256 = sha256_hex(&file.data);
            let mut conn = self.connection().await?;
            diesel::sql_query(
                r#"
                INSERT INTO knowledge_source_files (
                    knowledge_source_id,
                    file_name,
                    media_type,
                    byte_size,
                    checksum_sha256,
                    data,
                    created_at,
                    updated_at
                )
                VALUES ($1, $2, $3, $4, $5, $6, $7, $7)
                ON CONFLICT (knowledge_source_id) DO UPDATE SET
                    file_name = EXCLUDED.file_name,
                    media_type = EXCLUDED.media_type,
                    byte_size = EXCLUDED.byte_size,
                    checksum_sha256 = EXCLUDED.checksum_sha256,
                    data = EXCLUDED.data,
                    updated_at = EXCLUDED.updated_at
                "#,
            )
            .bind::<Text, _>(&id)
            .bind::<Text, _>(&file.file_name)
            .bind::<Text, _>(&file.media_type)
            .bind::<Integer, _>(byte_size)
            .bind::<Text, _>(&checksum_sha256)
            .bind::<Binary, _>(&file.data)
            .bind::<Timestamptz, _>(now)
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
        diesel::sql_query(
            r#"
            SELECT
                f.file_name,
                f.media_type,
                f.byte_size,
                f.data
            FROM knowledge_source_files f
            INNER JOIN knowledge_sources s ON s.id = f.knowledge_source_id
            WHERE
                s.workspace_id = $1
                AND s.id = $2
                AND s.kind = 'file'::knowledge_source_kind
                AND s.deleted_at IS NULL
            "#,
        )
        .bind::<Text, _>(workspace_id)
        .bind::<Text, _>(source_id)
        .get_result::<KnowledgeFileRow>(&mut conn)
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
