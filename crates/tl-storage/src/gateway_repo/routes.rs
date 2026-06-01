use diesel::dsl::now;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use tl_core::GatewayRoute;

use super::{
    mapping::{profile_record_to_wire, provider_record_to_wire, route_record_to_wire},
    GatewayRepo, GatewayRoutePatch, ResolvedGatewayRoute,
};
use crate::{
    models::{
        EnforcementProfileRecord, GatewayProviderConnectionRecord, GatewayRouteRecord,
        NewGatewayRoute,
    },
    schema::{enforcement_profiles, gateway_provider_connections, gateway_routes},
    StorageError,
};

impl GatewayRepo {
    pub async fn list_gateway_routes(
        &self,
        workspace_id: &str,
    ) -> Result<Vec<GatewayRoute>, StorageError> {
        let mut conn = self.connection().await?;
        let rows = gateway_routes::table
            .filter(gateway_routes::workspace_id.eq(workspace_id))
            .filter(gateway_routes::deleted_at.is_null())
            .order(gateway_routes::created_at.desc())
            .select(GatewayRouteRecord::as_select())
            .load::<GatewayRouteRecord>(&mut conn)
            .await?;
        Ok(rows.into_iter().map(route_record_to_wire).collect())
    }

    pub async fn create_gateway_route(
        &self,
        input: NewGatewayRoute,
    ) -> Result<GatewayRoute, StorageError> {
        let mut conn = self.connection().await?;
        let row = diesel::insert_into(gateway_routes::table)
            .values(input)
            .returning(GatewayRouteRecord::as_returning())
            .get_result::<GatewayRouteRecord>(&mut conn)
            .await?;
        Ok(route_record_to_wire(row))
    }

    pub async fn update_gateway_route(
        &self,
        workspace_id: &str,
        id: &str,
        patch: GatewayRoutePatch,
    ) -> Result<GatewayRoute, StorageError> {
        let mut conn = self.connection().await?;
        let mut current = gateway_routes::table
            .filter(gateway_routes::workspace_id.eq(workspace_id))
            .filter(gateway_routes::id.eq(id))
            .filter(gateway_routes::deleted_at.is_null())
            .select(GatewayRouteRecord::as_select())
            .first::<GatewayRouteRecord>(&mut conn)
            .await?;

        if let Some(value) = patch.display_name {
            current.display_name = value;
        }
        if let Some(value) = patch.provider_connection_id {
            current.provider_connection_id = value;
        }
        if let Some(value) = patch.agent_id {
            current.agent_id = value;
        }
        if let Some(value) = patch.enforcement_profile_id {
            current.enforcement_profile_id = value;
        }

        let row = diesel::update(
            gateway_routes::table
                .filter(gateway_routes::workspace_id.eq(workspace_id))
                .filter(gateway_routes::id.eq(id))
                .filter(gateway_routes::deleted_at.is_null()),
        )
        .set((
            gateway_routes::display_name.eq(current.display_name),
            gateway_routes::provider_connection_id.eq(current.provider_connection_id),
            gateway_routes::agent_id.eq(current.agent_id),
            gateway_routes::enforcement_profile_id.eq(current.enforcement_profile_id),
            gateway_routes::updated_at.eq(now),
        ))
        .returning(GatewayRouteRecord::as_returning())
        .get_result::<GatewayRouteRecord>(&mut conn)
        .await?;
        Ok(route_record_to_wire(row))
    }

    pub async fn delete_gateway_route(
        &self,
        workspace_id: &str,
        id: &str,
    ) -> Result<(), StorageError> {
        let mut conn = self.connection().await?;
        let count = diesel::update(
            gateway_routes::table
                .filter(gateway_routes::workspace_id.eq(workspace_id))
                .filter(gateway_routes::id.eq(id))
                .filter(gateway_routes::deleted_at.is_null()),
        )
        .set((
            gateway_routes::deleted_at.eq(now),
            gateway_routes::updated_at.eq(now),
        ))
        .execute(&mut conn)
        .await?;
        if count == 0 {
            return Err(StorageError::NotFound);
        }
        Ok(())
    }

    pub async fn resolve_gateway_route(
        &self,
        workspace_id: &str,
        route_id: &str,
    ) -> Result<ResolvedGatewayRoute, StorageError> {
        let mut conn = self.connection().await?;
        let route = gateway_routes::table
            .filter(gateway_routes::workspace_id.eq(workspace_id))
            .filter(gateway_routes::id.eq(route_id))
            .filter(gateway_routes::deleted_at.is_null())
            .select(GatewayRouteRecord::as_select())
            .first::<GatewayRouteRecord>(&mut conn)
            .await?;

        let provider = gateway_provider_connections::table
            .filter(gateway_provider_connections::workspace_id.eq(workspace_id))
            .filter(gateway_provider_connections::id.eq(&route.provider_connection_id))
            .filter(gateway_provider_connections::deleted_at.is_null())
            .select(GatewayProviderConnectionRecord::as_select())
            .first::<GatewayProviderConnectionRecord>(&mut conn)
            .await?;

        let profile = enforcement_profiles::table
            .filter(enforcement_profiles::workspace_id.eq(workspace_id))
            .filter(enforcement_profiles::id.eq(&route.enforcement_profile_id))
            .filter(enforcement_profiles::deleted_at.is_null())
            .select(EnforcementProfileRecord::as_select())
            .first::<EnforcementProfileRecord>(&mut conn)
            .await?;

        Ok(ResolvedGatewayRoute {
            route: route_record_to_wire(route),
            encrypted_api_key: provider.encrypted_api_key.clone(),
            provider_connection: provider_record_to_wire(provider)?,
            enforcement_profile: profile_record_to_wire(profile)?,
        })
    }
}
