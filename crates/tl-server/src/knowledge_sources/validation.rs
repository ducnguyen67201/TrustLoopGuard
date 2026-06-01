use base64::{engine::general_purpose::STANDARD, Engine as _};
use tl_core::{CreateKnowledgeSourceRequest, DashboardKnowledgeSourceKind};

use super::KnowledgeStoreError;

const MAX_KNOWLEDGE_FILE_BYTES: usize = 10 * 1024 * 1024;

pub(super) fn validate_create_request(
    input: &CreateKnowledgeSourceRequest,
) -> Result<(), KnowledgeStoreError> {
    if input.title.trim().is_empty() {
        return Err(KnowledgeStoreError::Validation("title is required".into()));
    }

    match input.kind {
        DashboardKnowledgeSourceKind::File => validate_file_input(input),
        DashboardKnowledgeSourceKind::Url | DashboardKnowledgeSourceKind::Note => {
            validate_non_file_input(input)
        }
    }
}

pub(crate) fn decode_file_data(data_base64: &str) -> Result<Vec<u8>, KnowledgeStoreError> {
    STANDARD
        .decode(data_base64)
        .map_err(|e| KnowledgeStoreError::Validation(format!("file data is not base64: {e}")))
}

fn validate_file_input(input: &CreateKnowledgeSourceRequest) -> Result<(), KnowledgeStoreError> {
    let file = input
        .file
        .as_ref()
        .ok_or_else(|| KnowledgeStoreError::Validation("file is required".into()))?;

    if file.file_name.trim().is_empty() {
        return Err(KnowledgeStoreError::Validation(
            "file_name is required".into(),
        ));
    }

    let bytes = decode_file_data(&file.data_base64)?;
    if bytes.len() > MAX_KNOWLEDGE_FILE_BYTES {
        return Err(KnowledgeStoreError::Validation(
            "file must be 10 MB or smaller".into(),
        ));
    }

    Ok(())
}

fn validate_non_file_input(
    input: &CreateKnowledgeSourceRequest,
) -> Result<(), KnowledgeStoreError> {
    if input.file.is_some() {
        return Err(KnowledgeStoreError::Validation(
            "file is only valid for file knowledge sources".into(),
        ));
    }

    Ok(())
}
