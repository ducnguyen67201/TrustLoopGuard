mod mapping;
mod provider_connections;
mod routes;

use tl_core::{GatewayProviderConnection, GatewayRoute};

use crate::postgres::{DbConnection, DbPool};
use crate::StorageError;

#[derive(Clone)]
pub struct GatewayRepo {
    pool: DbPool,
}

#[derive(Debug, Clone)]
pub struct GatewayProviderConnectionSecret {
    pub connection: GatewayProviderConnection,
    pub encrypted_api_key: String,
}

#[derive(Debug, Clone)]
pub struct ResolvedGatewayRoute {
    pub route: GatewayRoute,
    pub provider_connection: GatewayProviderConnection,
    pub encrypted_api_key: String,
}

impl GatewayRepo {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    async fn connection(&self) -> Result<DbConnection<'_>, StorageError> {
        self.pool
            .get()
            .await
            .map_err(|e| StorageError::Internal(format!("db pool: {e}")))
    }
}

#[derive(Default)]
pub struct GatewayRoutePatch {
    pub display_name: Option<String>,
    pub provider_connection_id: Option<String>,
    pub agent_id: Option<String>,
}
