use std::collections::HashMap;
use std::sync::Arc;

use crate::budget::TokenBudget;
use crate::client::LlmClient;
use crate::config::RouterConfig;
use crate::{OpenAiClient, OpenRouterClient};

use super::{JudgeKind, LlmRouter, ResolvedRoute, RouterBuildError};

impl LlmRouter {
    /// Build a router from parsed TOML config. Reads API keys from the
    /// env vars named in each provider's `api_key_env` at this point -
    /// missing keys produce a `RouterBuildError::MissingEnv`.
    pub fn from_config(config: &RouterConfig) -> Result<Self, RouterBuildError> {
        let providers = build_providers(config)?;
        let routes = build_routes(config, &providers)?;
        let budget = build_budget(config);

        Ok(Self::new(providers, routes, Arc::new(budget)))
    }
}

fn build_providers(
    config: &RouterConfig,
) -> Result<HashMap<String, Arc<dyn LlmClient>>, RouterBuildError> {
    let mut providers: HashMap<String, Arc<dyn LlmClient>> = HashMap::new();
    for (id, provider) in &config.providers {
        providers.insert(id.clone(), build_provider(provider)?);
    }
    Ok(providers)
}

fn build_provider(
    provider: &crate::config::ProviderConfig,
) -> Result<Arc<dyn LlmClient>, RouterBuildError> {
    let key = std::env::var(&provider.api_key_env)
        .map_err(|_| RouterBuildError::MissingEnv(provider.api_key_env.clone()))?;
    match provider.kind.as_str() {
        "openai" => {
            let mut client =
                OpenAiClient::new(key).map_err(|e| RouterBuildError::Provider(e.to_string()))?;
            if let Some(base) = &provider.base_url {
                client = client.with_base_url(base);
            }
            Ok(Arc::new(client))
        }
        "openrouter" => {
            let mut client = OpenRouterClient::new(key)
                .map_err(|e| RouterBuildError::Provider(e.to_string()))?;
            if let Some(base) = &provider.base_url {
                client = client.with_base_url(base);
            }
            Ok(Arc::new(client))
        }
        other => Err(RouterBuildError::UnknownProviderKind(other.into())),
    }
}

fn build_routes(
    config: &RouterConfig,
    providers: &HashMap<String, Arc<dyn LlmClient>>,
) -> Result<HashMap<JudgeKind, ResolvedRoute>, RouterBuildError> {
    let mut routes = HashMap::new();
    for (name, route) in &config.routes {
        let kind = judge_kind(name)?;
        // Validate referenced providers now so misconfigs fail at boot, not on
        // the first request.
        ensure_provider_exists(providers, &route.primary.provider)?;
        if let Some(fallback) = &route.fallback {
            ensure_provider_exists(providers, &fallback.provider)?;
        }
        routes.insert(
            kind,
            ResolvedRoute {
                primary: route.primary.clone(),
                fallback: route.fallback.clone(),
            },
        );
    }
    Ok(routes)
}

fn judge_kind(name: &str) -> Result<JudgeKind, RouterBuildError> {
    match name {
        "hallucination" => Ok(JudgeKind::Hallucination),
        "tone" => Ok(JudgeKind::Tone),
        "authority" => Ok(JudgeKind::Authority),
        "semantic_policy" => Ok(JudgeKind::SemanticPolicy),
        other => Err(RouterBuildError::UnknownJudgeKind(other.into())),
    }
}

fn ensure_provider_exists(
    providers: &HashMap<String, Arc<dyn LlmClient>>,
    provider: &str,
) -> Result<(), RouterBuildError> {
    if providers.contains_key(provider) {
        Ok(())
    } else {
        Err(RouterBuildError::UnknownProvider(provider.into()))
    }
}

fn build_budget(config: &RouterConfig) -> TokenBudget {
    let budget = TokenBudget::new(config.budgets.default_monthly_tokens);
    for (tenant, limit) in &config.budgets.tenants {
        budget.set_tenant_limit(tenant.clone(), *limit);
    }
    budget
}
