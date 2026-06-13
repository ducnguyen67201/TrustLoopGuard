use tl_core::RedteamDispatchRequest;

use super::RedteamJobStoreError;

/// Valid attack profiles understood by the runner.
const PROFILES: [&str; 3] = ["fast", "full", "max"];

pub(super) fn validate_dispatch(
    input: &RedteamDispatchRequest,
) -> Result<(), RedteamJobStoreError> {
    let target = input.target_url.trim();
    if target.is_empty() {
        return Err(RedteamJobStoreError::Validation(
            "target_url is required".into(),
        ));
    }
    if !(target.starts_with("http://") || target.starts_with("https://")) {
        return Err(RedteamJobStoreError::Validation(
            "target_url must be an http(s) URL".into(),
        ));
    }
    if !PROFILES.contains(&input.profile.trim()) {
        return Err(RedteamJobStoreError::Validation(
            "profile must be one of: fast, full, max".into(),
        ));
    }
    Ok(())
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
