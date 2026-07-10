mod operations;

use std::sync::RwLock;

use tl_core::{GatewayProviderConnection, GatewayRoute};

use super::GatewayStoreError;

#[derive(Debug, Default)]
pub struct MemoryGatewayStore {
    pub(super) provider_connections: RwLock<Vec<MemoryProviderConnection>>,
    pub(super) gateway_routes: RwLock<Vec<MemoryGatewayRoute>>,
}

#[derive(Debug, Clone)]
pub(super) struct MemoryProviderConnection {
    pub(super) workspace_id: String,
    pub(super) connection: GatewayProviderConnection,
    pub(super) encrypted_api_key: String,
}

#[derive(Debug, Clone)]
pub(super) struct MemoryGatewayRoute {
    pub(super) workspace_id: String,
    pub(super) route: GatewayRoute,
}

impl MemoryGatewayStore {
    pub fn new() -> Self {
        Self::default()
    }
}

pub(super) fn lock_error<T>(_error: std::sync::PoisonError<T>) -> GatewayStoreError {
    GatewayStoreError::Internal("gateway store lock poisoned".into())
}
