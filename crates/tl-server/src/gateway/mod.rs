//! Gateway/proxy integration surface.
//!
//! SDK callers receive a `Decision` and handle it in their code. Gateway
//! callers route provider traffic through TrustLoopGuard, so this module
//! resolves dashboard config and applies the decision before returning a
//! provider-compatible response.

pub mod api;
mod crypto;
mod errors;
mod normalization;
mod provider;
mod service;
mod store;

pub use api::{
    __path_create_enforcement_profile, __path_create_gateway_provider_connection,
    __path_create_gateway_route, __path_list_enforcement_profiles,
    __path_list_gateway_provider_connections, __path_list_gateway_routes,
    __path_patch_enforcement_profile, __path_patch_gateway_provider_connection,
    __path_patch_gateway_route, __path_proxy_anthropic_messages,
    __path_proxy_openai_chat_completions, create_enforcement_profile,
    create_gateway_provider_connection, create_gateway_route, list_enforcement_profiles,
    list_gateway_provider_connections, list_gateway_routes, patch_enforcement_profile,
    patch_gateway_provider_connection, patch_gateway_route, proxy_anthropic_messages,
    proxy_openai_chat_completions, GatewayState,
};
pub use crypto::build_seal_key;
#[cfg(feature = "postgres")]
pub(crate) use normalization::{
    fail_mode_storage_text, input_action_storage_text, output_action_storage_text,
    provider_kind_storage_text, response_mode_storage_text, retention_mode_storage_text,
};
pub use store::{
    EnforcementProfilePatch, GatewayRoutePatch, GatewayStore, GatewayStoreError,
    MemoryGatewayStore, NewEnforcementProfile, NewGatewayProviderConnection, NewGatewayRoute,
    ProviderConnectionPatch, ProviderConnectionSecret, ResolvedGatewayRoute,
};
