use sha2::{Digest, Sha256};

pub const MAX_RISK_STATEMENT_CHARS: usize = 1_200;
pub const MAX_PROPOSED_FILES: usize = 10;
pub const MAX_GENERATED_BYTES: usize = 1024 * 1024;
pub const MAX_FILE_BYTES: usize = 100 * 1024;
pub const MAX_TOTAL_CONTEXT_BYTES: usize = 750 * 1024;
pub const MAX_CONTEXT_FILES: usize = 30;

const FORBIDDEN_EXACT_FILES: &[&str] = &[
    "package-lock.json",
    "pnpm-lock.yaml",
    "yarn.lock",
    "bun.lockb",
];
const FORBIDDEN_PREFIXES: &[&str] = &[
    ".github/workflows/",
    "node_modules/",
    "vendor/",
    "dist/",
    "build/",
];

pub fn normalize_root_path(value: &str) -> Result<String, String> {
    let trimmed = value.trim().trim_matches('/');
    if trimmed.is_empty() {
        return Ok(String::new());
    }
    normalize_relative_path(trimmed)
}

pub fn normalize_relative_path(value: &str) -> Result<String, String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err("path cannot be empty".into());
    }
    if trimmed.starts_with('/')
        || trimmed.contains('\\')
        || trimmed.contains('\0')
        || trimmed.contains('%')
        || trimmed
            .split('/')
            .any(|part| part == "." || part == ".." || part.is_empty())
    {
        return Err("path must be a normalized repository-relative path".into());
    }
    Ok(trimmed.to_string())
}

pub fn validate_risk_statement(value: &str) -> Result<String, String> {
    let trimmed = value.trim();
    if trimmed.len() < 20 {
        return Err(
            "risk_statement must describe the high-stakes action in at least 20 characters".into(),
        );
    }
    if trimmed.chars().count() > MAX_RISK_STATEMENT_CHARS {
        return Err(format!(
            "risk_statement must be {MAX_RISK_STATEMENT_CHARS} characters or fewer"
        ));
    }
    Ok(trimmed.to_string())
}

pub fn validate_candidate_path(path: &str) -> Result<String, String> {
    let normalized = normalize_relative_path(path)?;
    let lowered = normalized.to_ascii_lowercase();
    if lowered.starts_with(".env") || lowered.contains("/.env") {
        return Err("environment files cannot be read or edited".into());
    }
    if FORBIDDEN_EXACT_FILES
        .iter()
        .any(|forbidden| lowered.ends_with(forbidden))
    {
        return Err("lockfiles cannot be read or edited by the remote GitHub integration".into());
    }
    if FORBIDDEN_PREFIXES
        .iter()
        .any(|prefix| lowered.starts_with(prefix))
    {
        return Err("generated, workflow, or vendored files cannot be read or edited".into());
    }
    if lowered.contains("secret")
        || lowered.contains("private_key")
        || lowered.ends_with(".pem")
        || lowered.ends_with(".key")
        || lowered.ends_with(".p12")
    {
        return Err("secret-bearing files cannot be read or edited".into());
    }
    Ok(normalized)
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

pub fn state_hash(raw: &str) -> Vec<u8> {
    Sha256::digest(raw.as_bytes()).to_vec()
}

pub fn is_probably_binary(bytes: &[u8]) -> bool {
    bytes.iter().take(1024).any(|byte| *byte == 0)
}

pub fn contains_required_marker(content: &str, connection_id: &str) -> bool {
    content.contains("featherlane_ai_integration_id")
        && content.contains(connection_id)
        && content.contains("FEATHERLANE_AI_API_KEY")
        && !content.contains("sk_featherlane_ai_")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn root_path_normalization_rejects_escape_paths() {
        assert_eq!(normalize_root_path(" /apps/web/ ").unwrap(), "apps/web");
        assert_eq!(normalize_root_path("/").unwrap(), "");
        assert!(normalize_root_path("../apps/web").is_err());
        assert!(normalize_root_path("apps//web").is_err());
        assert!(normalize_root_path("apps\\web").is_err());
        assert!(normalize_root_path("apps/%2e%2e/web").is_err());
    }

    #[test]
    fn candidate_path_rejects_sensitive_or_unsupported_files() {
        assert_eq!(
            validate_candidate_path("apps/web/app/api/route.ts").unwrap(),
            "apps/web/app/api/route.ts"
        );
        assert!(validate_candidate_path(".env").is_err());
        assert!(validate_candidate_path("apps/web/.env.production").is_err());
        assert!(validate_candidate_path(".github/workflows/ci.yml").is_err());
        assert!(validate_candidate_path("pnpm-lock.yaml").is_err());
        assert!(validate_candidate_path("config/private_key.pem").is_err());
    }

    #[test]
    fn required_marker_requires_connection_env_key_and_no_plain_secret() {
        let content = r#"guard({
            context: { featherlane_ai_integration_id: "conn_123" },
            apiKey: process.env.FEATHERLANE_AI_API_KEY
        })"#;
        assert!(contains_required_marker(content, "conn_123"));
        assert!(!contains_required_marker(content, "conn_456"));
        assert!(!contains_required_marker(
            "featherlane_ai_integration_id conn_123 sk_featherlane_ai_live",
            "conn_123"
        ));
    }
}
