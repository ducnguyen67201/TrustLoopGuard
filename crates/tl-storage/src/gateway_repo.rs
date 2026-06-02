mod enforcement_profiles;
mod mapping;
mod provider_connections;
mod routes;

use tl_core::{EnforcementProfile, GatewayProviderConnection, GatewayRoute};

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
    pub enforcement_profile: EnforcementProfile,
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
pub struct EnforcementProfilePatch {
    pub display_name: Option<String>,
    pub input_action: Option<String>,
    pub output_action: Option<String>,
    pub fail_mode: Option<String>,
    pub retention_mode: Option<String>,
    pub response_mode: Option<String>,
    pub fallback_message: Option<String>,
    pub max_regenerations: Option<i32>,
}

#[derive(Default)]
pub struct GatewayRoutePatch {
    pub display_name: Option<String>,
    pub provider_connection_id: Option<String>,
    pub agent_id: Option<String>,
    pub enforcement_profile_id: Option<String>,
}
