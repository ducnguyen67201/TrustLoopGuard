//! Gateway/proxy integration surface.
//!
//! SDK callers receive a `Decision` and handle it in their code. Gateway
//! callers route provider traffic through Featherlane AI, so this module
//! resolves dashboard config and applies the decision before returning a
//! provider-compatible response.

mod activation;
pub mod api;
mod budget;
mod crypto;
mod errors;
mod normalization;
mod provider;
mod service;
mod session_worker;
mod store;

pub use activation::{
    __path_create_gateway_activation, __path_gateway_production_readiness,
    create_gateway_activation, gateway_production_readiness,
};
pub use api::{
    __path_create_gateway_provider_connection, __path_create_gateway_route,
    __path_delete_gateway_provider_connection, __path_list_gateway_provider_connections,
    __path_list_gateway_routes, __path_patch_gateway_provider_connection,
    __path_patch_gateway_route, __path_proxy_anthropic_messages,
    __path_proxy_openai_chat_completions, create_gateway_provider_connection, create_gateway_route,
    delete_gateway_provider_connection, list_gateway_provider_connections, list_gateway_routes,
    patch_gateway_provider_connection, patch_gateway_route, proxy_anthropic_messages,
    proxy_openai_chat_completions, GatewayState,
};
pub use crypto::build_seal_key;
pub(crate) use crypto::seal_provider_key;
pub(crate) use crypto::unseal_provider_key;
#[cfg(feature = "postgres")]
pub(crate) use normalization::provider_kind_storage_text;
pub(crate) use provider::forward_payment;
pub(crate) use session_worker::spawn_gateway_session_worker;
pub use store::{
    GatewayRoutePatch, GatewayStore, GatewayStoreError, MemoryGatewayStore,
    NewGatewayProviderConnection, NewGatewayRoute, ProviderConnectionPatch,
    ProviderConnectionSecret, ResolvedGatewayRoute,
};
