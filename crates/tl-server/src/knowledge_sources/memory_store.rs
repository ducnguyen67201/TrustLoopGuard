use async_trait::async_trait;
use base64::{engine::general_purpose::STANDARD, Engine as _};
use serde_json::json;
use tl_core::{
    CreateKnowledgeSourceRequest, KnowledgeSourceDocument, KnowledgeSourceFileResponse,
    KnowledgeSourceStatus,
};
use tokio::sync::RwLock;

use super::{
    validation::{decode_file_data, validate_create_request},
    KnowledgeStore, KnowledgeStoreError,
};

#[derive(Debug, Default)]
pub struct MemoryKnowledgeStore {
    sources: RwLock<Vec<KnowledgeSourceDocument>>,
    files: RwLock<std::collections::HashMap<String, KnowledgeSourceFileResponse>>,
}

impl MemoryKnowledgeStore {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl KnowledgeStore for MemoryKnowledgeStore {
    async fn list(
        &self,
        _workspace_id: &str,
    ) -> Result<Vec<KnowledgeSourceDocument>, KnowledgeStoreError> {
        Ok(self.sources.read().await.clone())
    }

    async fn create(
        &self,
        _workspace_id: &str,
        input: CreateKnowledgeSourceRequest,
    ) -> Result<KnowledgeSourceDocument, KnowledgeStoreError> {
        validate_create_request(&input)?;

        let now = chrono::Utc::now().to_rfc3339();
        let id = uuid::Uuid::new_v4().to_string();
        let mut metadata = serde_json::Map::new();

        if let Some(notes) = input
            .notes
            .as_ref()
            .filter(|value| !value.trim().is_empty())
        {
            metadata.insert("notes".into(), serde_json::Value::String(notes.clone()));
        }

        if let Some(file) = input.file {
            let bytes = decode_file_data(&file.data_base64)?;
            metadata.insert(
                "file".into(),
                json!({
                    "fileName": file.file_name,
                    "mediaType": file.media_type,
                    "byteSize": bytes.len(),
                }),
            );
            self.files.write().await.insert(
                id.clone(),
                KnowledgeSourceFileResponse {
                    file_name: file.file_name,
                    media_type: file.media_type,
                    byte_size: bytes.len() as i32,
                    data_base64: STANDARD.encode(bytes),
                },
            );
        }

        let source = KnowledgeSourceDocument {
            id,
            title: input.title,
            kind: input.kind,
            location: input.location,
            status: KnowledgeSourceStatus::Ready,
            metadata: serde_json::Value::Object(metadata),
            created_at: now.clone(),
            updated_at: now.clone(),
            last_indexed_at: Some(now),
        };
        self.sources.write().await.push(source.clone());
        Ok(source)
    }

    async fn get_file(
        &self,
        _workspace_id: &str,
        source_id: &str,
    ) -> Result<KnowledgeSourceFileResponse, KnowledgeStoreError> {
        self.files
            .read()
            .await
            .get(source_id)
            .cloned()
            .ok_or(KnowledgeStoreError::NotFound)
    }
}
