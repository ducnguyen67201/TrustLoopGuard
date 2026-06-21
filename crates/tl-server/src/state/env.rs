pub(super) fn hosted_user_approval_required_from_env() -> bool {
    hosted_user_approval_required_from_values(
        std::env::var("TL_HOSTED_DEPLOYMENT").ok().as_deref(),
        std::env::var("TL_APP_ENV").ok().as_deref(),
        std::env::var("APP_ENV").ok().as_deref(),
        std::env::var("NEXT_PUBLIC_APP_ENV").ok().as_deref(),
        std::env::var("VERCEL_ENV").ok().as_deref(),
    )
}

fn hosted_user_approval_required_from_values(
    hosted_deployment: Option<&str>,
    tl_app_env: Option<&str>,
    app_env: Option<&str>,
    next_public_app_env: Option<&str>,
    vercel_env: Option<&str>,
) -> bool {
    let hosted = hosted_deployment
        .map(|value| {
            matches!(
                value.trim().to_ascii_lowercase().as_str(),
                "1" | "true" | "yes" | "hosted"
            )
        })
        .unwrap_or(false);
    if !hosted {
        return false;
    }

    [tl_app_env, app_env, next_public_app_env, vercel_env]
        .into_iter()
        .flatten()
        .any(|value| {
            matches!(
                value.trim().to_ascii_lowercase().as_str(),
                "staging" | "stage" | "preview" | "prod" | "production"
            )
        })
}

#[cfg(any(feature = "postgres", test))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum KnowledgeGroundingMode {
    Off,
    Lexical,
    Vector,
    Hybrid,
}

#[cfg(any(feature = "postgres", test))]
#[derive(Debug, Clone, PartialEq)]
pub(super) struct KnowledgeGroundingConfig {
    pub mode: KnowledgeGroundingMode,
    pub max_chunks: usize,
    pub max_snippet_chars: usize,
    pub max_chunk_chars: usize,
    pub min_similarity: f32,
    pub retrieval_timeout_ms: u64,
    pub embedding_model: String,
}

#[cfg(any(feature = "postgres", test))]
impl Default for KnowledgeGroundingConfig {
    fn default() -> Self {
        Self {
            mode: KnowledgeGroundingMode::Hybrid,
            max_chunks: 5,
            max_snippet_chars: 8_000,
            max_chunk_chars: 1_500,
            min_similarity: 0.65,
            retrieval_timeout_ms: 50,
            embedding_model: "mock-word-bag-64".into(),
        }
    }
}

#[cfg(feature = "postgres")]
pub(super) fn knowledge_grounding_config_from_env() -> KnowledgeGroundingConfig {
    if std::env::var("TL_KNOWLEDGE_GROUNDING_ENABLED").is_ok() {
        tracing::warn!(
            "TL_KNOWLEDGE_GROUNDING_ENABLED is ignored; use global_feature_flags.knowledge_grounding"
        );
    }

    knowledge_grounding_config_from_values(
        std::env::var("TL_KNOWLEDGE_GROUNDING_MODE").ok().as_deref(),
        std::env::var("TL_KNOWLEDGE_MAX_CHUNKS").ok().as_deref(),
        std::env::var("TL_KNOWLEDGE_MAX_SNIPPET_CHARS")
            .ok()
            .as_deref(),
        std::env::var("TL_KNOWLEDGE_MAX_CHUNK_CHARS")
            .ok()
            .as_deref(),
        std::env::var("TL_KNOWLEDGE_MIN_SIMILARITY").ok().as_deref(),
        std::env::var("TL_KNOWLEDGE_RETRIEVAL_TIMEOUT_MS")
            .ok()
            .as_deref(),
        std::env::var("TL_KNOWLEDGE_EMBEDDING_MODEL")
            .ok()
            .as_deref(),
    )
}

#[cfg(any(feature = "postgres", test))]
fn knowledge_grounding_config_from_values(
    mode: Option<&str>,
    max_chunks: Option<&str>,
    max_snippet_chars: Option<&str>,
    max_chunk_chars: Option<&str>,
    min_similarity: Option<&str>,
    retrieval_timeout_ms: Option<&str>,
    embedding_model: Option<&str>,
) -> KnowledgeGroundingConfig {
    let mode = parse_knowledge_mode(mode).unwrap_or(KnowledgeGroundingMode::Hybrid);

    KnowledgeGroundingConfig {
        mode,
        max_chunks: parse_usize(max_chunks, 5, 1, 20),
        max_snippet_chars: parse_usize(max_snippet_chars, 8_000, 500, 32_000),
        max_chunk_chars: parse_usize(max_chunk_chars, 1_500, 250, 8_000),
        min_similarity: parse_f32(min_similarity, 0.65, 0.0, 1.0),
        retrieval_timeout_ms: parse_u64(retrieval_timeout_ms, 50, 1, 2_000),
        embedding_model: embedding_model
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .unwrap_or("mock-word-bag-64")
            .to_string(),
    }
}

#[cfg(any(feature = "postgres", test))]
fn parse_knowledge_mode(value: Option<&str>) -> Option<KnowledgeGroundingMode> {
    match value?.trim().to_ascii_lowercase().as_str() {
        "off" => Some(KnowledgeGroundingMode::Off),
        "lexical" => Some(KnowledgeGroundingMode::Lexical),
        "vector" => Some(KnowledgeGroundingMode::Vector),
        "hybrid" => Some(KnowledgeGroundingMode::Hybrid),
        _ => None,
    }
}

#[cfg(any(feature = "postgres", test))]
fn parse_usize(value: Option<&str>, default: usize, min: usize, max: usize) -> usize {
    value
        .and_then(|value| value.trim().parse::<usize>().ok())
        .unwrap_or(default)
        .clamp(min, max)
}

#[cfg(any(feature = "postgres", test))]
fn parse_u64(value: Option<&str>, default: u64, min: u64, max: u64) -> u64 {
    value
        .and_then(|value| value.trim().parse::<u64>().ok())
        .unwrap_or(default)
        .clamp(min, max)
}

#[cfg(any(feature = "postgres", test))]
fn parse_f32(value: Option<&str>, default: f32, min: f32, max: f32) -> f32 {
    value
        .and_then(|value| value.trim().parse::<f32>().ok())
        .unwrap_or(default)
        .clamp(min, max)
}

pub(super) fn password_auth_enabled_from_env() -> bool {
    password_auth_enabled_from_values(
        std::env::var("TL_APP_ENV").ok().as_deref(),
        std::env::var("APP_ENV").ok().as_deref(),
        std::env::var("NEXT_PUBLIC_APP_ENV").ok().as_deref(),
        std::env::var("VERCEL_ENV").ok().as_deref(),
        std::env::var("NODE_ENV").ok().as_deref(),
        std::env::var("DATABASE_URL").ok().as_deref(),
        std::env::var("TL_API_KEY").ok().as_deref(),
    )
}

fn password_auth_enabled_from_values(
    tl_app_env: Option<&str>,
    app_env: Option<&str>,
    next_public_app_env: Option<&str>,
    vercel_env: Option<&str>,
    node_env: Option<&str>,
    database_url: Option<&str>,
    tl_api_key: Option<&str>,
) -> bool {
    for value in [
        tl_app_env,
        app_env,
        next_public_app_env,
        vercel_env,
        node_env,
    ]
    .into_iter()
    .flatten()
    {
        match value.trim().to_ascii_lowercase().as_str() {
            "dev" | "development" | "local" | "test" => return true,
            "staging" | "stage" | "preview" | "prod" | "production" => return false,
            _ => {}
        }
    }

    database_url.is_none() && tl_api_key.is_none()
}

#[cfg(test)]
mod tests {
    use super::{
        hosted_user_approval_required_from_values, knowledge_grounding_config_from_values,
        password_auth_enabled_from_values, KnowledgeGroundingMode,
    };

    #[test]
    fn password_auth_env_gate_allows_local_dev() {
        assert!(password_auth_enabled_from_values(
            Some("dev"),
            None,
            None,
            None,
            None,
            Some("postgres://example"),
            Some("secret")
        ));
    }

    #[test]
    fn password_auth_env_gate_blocks_staging_and_prod() {
        assert!(!password_auth_enabled_from_values(
            None,
            Some("staging"),
            None,
            None,
            None,
            None,
            None
        ));
        assert!(!password_auth_enabled_from_values(
            None,
            None,
            Some("prod"),
            None,
            None,
            None,
            None
        ));
    }

    #[test]
    fn password_auth_env_gate_defaults_to_off_for_configured_server() {
        assert!(!password_auth_enabled_from_values(
            None,
            None,
            None,
            None,
            None,
            Some("postgres://example"),
            Some("secret")
        ));
    }

    #[test]
    fn password_auth_env_gate_defaults_to_on_for_unconfigured_local_server() {
        assert!(password_auth_enabled_from_values(
            None, None, None, None, None, None, None
        ));
    }

    #[test]
    fn hosted_user_approval_gate_requires_hosted_stage_or_prod() {
        assert!(hosted_user_approval_required_from_values(
            Some("true"),
            None,
            Some("staging"),
            None,
            None
        ));
        assert!(hosted_user_approval_required_from_values(
            Some("hosted"),
            None,
            None,
            Some("prod"),
            None
        ));
        assert!(!hosted_user_approval_required_from_values(
            None,
            None,
            Some("prod"),
            None,
            None
        ));
        assert!(!hosted_user_approval_required_from_values(
            Some("true"),
            Some("dev"),
            None,
            None,
            None
        ));
    }

    #[test]
    fn knowledge_grounding_config_defaults_to_hybrid_tuning() {
        let config =
            knowledge_grounding_config_from_values(None, None, None, None, None, None, None);

        assert_eq!(config.mode, KnowledgeGroundingMode::Hybrid);
        assert_eq!(config.max_chunks, 5);
    }

    #[test]
    fn knowledge_grounding_config_bounds_values() {
        let config = knowledge_grounding_config_from_values(
            None,
            Some("100"),
            Some("1"),
            Some("999999"),
            Some("1.5"),
            Some("0"),
            Some("custom"),
        );

        assert_eq!(config.mode, KnowledgeGroundingMode::Hybrid);
        assert_eq!(config.max_chunks, 20);
        assert_eq!(config.max_snippet_chars, 500);
        assert_eq!(config.max_chunk_chars, 8_000);
        assert_eq!(config.min_similarity, 1.0);
        assert_eq!(config.retrieval_timeout_ms, 1);
        assert_eq!(config.embedding_model, "custom");
    }

    #[test]
    fn knowledge_grounding_honors_explicit_lexical_mode() {
        let config = knowledge_grounding_config_from_values(
            Some("lexical"),
            None,
            None,
            None,
            None,
            None,
            None,
        );

        assert_eq!(config.mode, KnowledgeGroundingMode::Lexical);
    }
}
