use tl_core::RedteamDispatchRequest;
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
    Ok(())
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
