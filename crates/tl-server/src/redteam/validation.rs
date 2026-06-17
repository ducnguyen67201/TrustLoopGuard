use base64::Engine as _;
use tl_core::{RedteamAttackSurface, RedteamDispatchRequest, RedteamDocumentTemplate};
use url::Url;

use super::RedteamJobStoreError;

/// Valid attack profiles understood by the runner.
const PROFILES: [&str; 3] = ["fast", "full", "max"];

/// Loopback hosts the orchestrator is allowed to drive an attack against.
/// Mirrors the web edge allowlist (`apps/web/lib/arena-redteam.ts`). This is
/// the authoritative SSRF gate: the worker ultimately causes `target_url` to be
/// fetched, and a direct API caller (workspace key) bypasses the web edge, so
/// the allowlist must live here too — deny-by-default.
const ALLOWED_TARGET_HOSTS: [&str; 3] = ["127.0.0.1", "localhost", "::1"];
const MAX_DOCUMENT_TEMPLATE_BYTES: usize = 10 * 1024 * 1024;

pub(super) fn validate_dispatch(
    input: &RedteamDispatchRequest,
) -> Result<(), RedteamJobStoreError> {
    let target = input.target_url.trim();
    if target.is_empty() {
        return Err(RedteamJobStoreError::Validation(
            "target_url is required".into(),
        ));
    }
    if !is_loopback_target(target) {
        return Err(RedteamJobStoreError::Validation(
            "target_url must be an http(s) loopback agent (127.0.0.1, localhost, or ::1)".into(),
        ));
    }
    if !PROFILES.contains(&input.profile.trim()) {
        return Err(RedteamJobStoreError::Validation(
            "profile must be one of: fast, full, max".into(),
        ));
    }
    validate_document_template(input)?;
    Ok(())
}

fn validate_document_template(input: &RedteamDispatchRequest) -> Result<(), RedteamJobStoreError> {
    let Some(template) = &input.document_template else {
        return Ok(());
    };
    if input.attack_surface != RedteamAttackSurface::DocumentWorkflow {
        return Err(RedteamJobStoreError::Validation(
            "document_template is only valid for document_workflow".into(),
        ));
    }
    validate_template_fields(template)?;
    let looks_like_pdf = template
        .file_name
        .trim()
        .to_ascii_lowercase()
        .ends_with(".pdf")
        || template
            .media_type
            .trim()
            .eq_ignore_ascii_case("application/pdf");
    if !looks_like_pdf {
        return Err(RedteamJobStoreError::Validation(
            "document_template must be a PDF".into(),
        ));
    }
    let decoded = base64::engine::general_purpose::STANDARD
        .decode(template.data_base64.trim())
        .map_err(|_| RedteamJobStoreError::Validation("document_template is not base64".into()))?;
    if decoded.len() > MAX_DOCUMENT_TEMPLATE_BYTES {
        return Err(RedteamJobStoreError::Validation(
            "document_template must be 10 MB or smaller".into(),
        ));
    }
    if !decoded.starts_with(b"%PDF-") {
        return Err(RedteamJobStoreError::Validation(
            "document_template must contain PDF bytes".into(),
        ));
    }
    Ok(())
}

fn validate_template_fields(
    template: &RedteamDocumentTemplate,
) -> Result<(), RedteamJobStoreError> {
    let Some(fields) = &template.fields else {
        return Ok(());
    };
    if !fields.is_empty()
        && !fields
            .iter()
            .any(|(field, value)| field.trim().is_empty() || value.trim().is_empty())
    {
        return Ok(());
    }
    Err(RedteamJobStoreError::Validation(
        "document_template.fields must be a non-empty object".into(),
    ))
}

/// True only for an http(s) URL whose host is a loopback agent.
fn is_loopback_target(raw: &str) -> bool {
    let Ok(url) = Url::parse(raw) else {
        return false;
    };
    if url.scheme() != "http" && url.scheme() != "https" {
        return false;
    }
    match url.host_str() {
        Some(host) => {
            let host = host
                .trim_start_matches('[')
                .trim_end_matches(']')
                .to_ascii_lowercase();
            ALLOWED_TARGET_HOSTS.contains(&host.as_str())
        }
        None => false,
    }
}

pub(super) fn clean_optional(value: Option<String>) -> Option<String> {
    value.and_then(|value| {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    })
}
