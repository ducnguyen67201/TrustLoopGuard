use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use base64::{engine::general_purpose::STANDARD, Engine as _};
use tl_storage::{KnowledgeRepo, NewKnowledgeFile, NewKnowledgeSource};

use crate::knowledge_sources::KnowledgeStore;

pub struct PostgresKnowledgeAdapter(pub Arc<KnowledgeRepo>);

impl PostgresKnowledgeAdapter {
    pub fn new(repo: Arc<KnowledgeRepo>) -> Arc<Self> {
        Arc::new(Self(repo))
    }
}

#[async_trait]
impl KnowledgeStore for PostgresKnowledgeAdapter {
    async fn list(
        &self,
        workspace_id: &str,
    ) -> Result<Vec<tl_core::KnowledgeSourceDocument>, crate::knowledge_sources::KnowledgeStoreError>
    {
        self.0
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
            .0
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
        knowledge_row_to_document(row)
    }

    async fn get_file(
        &self,
        workspace_id: &str,
        source_id: &str,
    ) -> Result<tl_core::KnowledgeSourceFileResponse, crate::knowledge_sources::KnowledgeStoreError>
    {
        let row = self
            .0
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
