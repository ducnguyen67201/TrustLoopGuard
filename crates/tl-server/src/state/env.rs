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
    use super::{hosted_user_approval_required_from_values, password_auth_enabled_from_values};

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
}
