use diesel::dsl::now;
use diesel::prelude::*;
use diesel_async::{AsyncConnection, RunQueryDsl};
use std::collections::HashMap;
use tl_core::GatewayRoute;

use super::{
    mapping::{provider_record_to_wire, reliability_mode_text, route_record_to_wire},
    GatewayRepo, GatewayRoutePatch, ResolvedGatewayRoute,
};
use crate::{
    models::{
        GatewayProviderConnectionRecord, GatewayRouteFallbackRecord, GatewayRouteRecord,
        NewGatewayRoute, NewGatewayRouteFallback,
    },
    schema::{gateway_provider_connections, gateway_route_fallbacks, gateway_routes},
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
        let fallback_rows = gateway_route_fallbacks::table
            .filter(gateway_route_fallbacks::workspace_id.eq(workspace_id))
            .order((
                gateway_route_fallbacks::route_id.asc(),
                gateway_route_fallbacks::position.asc(),
            ))
            .select(GatewayRouteFallbackRecord::as_select())
            .load::<GatewayRouteFallbackRecord>(&mut conn)
            .await?;
        let mut fallbacks: HashMap<String, Vec<String>> = HashMap::new();
        for fallback in fallback_rows {
            fallbacks
                .entry(fallback.route_id)
                .or_default()
                .push(fallback.provider_connection_id);
        }
        rows.into_iter()
            .map(|row| {
                let route_fallbacks = fallbacks.remove(&row.id).unwrap_or_default();
                route_record_to_wire(row, route_fallbacks)
            })
            .collect()
    }

    pub async fn create_gateway_route(
        &self,
        input: NewGatewayRoute,
        fallback_provider_connection_ids: Vec<String>,
    ) -> Result<GatewayRoute, StorageError> {
        let mut conn = self.connection().await?;
        conn.transaction::<GatewayRoute, StorageError, _>(async move |conn| {
            let workspace_id = input.workspace_id.clone();
            let route_id = input.id.clone();
            let row = diesel::insert_into(gateway_routes::table)
                .values(input)
                .returning(GatewayRouteRecord::as_returning())
                .get_result::<GatewayRouteRecord>(conn)
                .await?;
            replace_fallbacks(
                conn,
                &workspace_id,
                &route_id,
                &fallback_provider_connection_ids,
            )
            .await?;
            route_record_to_wire(row, fallback_provider_connection_ids)
        })
        .await
    }

    pub async fn update_gateway_route(
        &self,
        workspace_id: &str,
        id: &str,
        patch: GatewayRoutePatch,
    ) -> Result<GatewayRoute, StorageError> {
        let mut conn = self.connection().await?;
        conn.transaction::<GatewayRoute, StorageError, _>(async move |conn| {
            let mut current = gateway_routes::table
                .filter(gateway_routes::workspace_id.eq(workspace_id))
                .filter(gateway_routes::id.eq(id))
                .filter(gateway_routes::deleted_at.is_null())
                .select(GatewayRouteRecord::as_select())
                .first::<GatewayRouteRecord>(conn)
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
            if let Some(value) = patch.reliability_mode {
                current.reliability_mode = reliability_mode_text(value).to_string();
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
                gateway_routes::reliability_mode.eq(current.reliability_mode),
                gateway_routes::updated_at.eq(now),
            ))
            .returning(GatewayRouteRecord::as_returning())
            .get_result::<GatewayRouteRecord>(conn)
            .await?;

            let fallback_ids = match patch.fallback_provider_connection_ids {
                Some(values) => {
                    replace_fallbacks(conn, workspace_id, id, &values).await?;
                    values
                }
                None => load_fallback_ids(conn, workspace_id, id).await?,
            };
            route_record_to_wire(row, fallback_ids)
        })
        .await
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

        let fallback_ids = load_fallback_ids(&mut conn, workspace_id, route_id).await?;
        let provider = gateway_provider_connections::table
            .filter(gateway_provider_connections::workspace_id.eq(workspace_id))
            .filter(gateway_provider_connections::id.eq(&route.provider_connection_id))
            .filter(gateway_provider_connections::deleted_at.is_null())
            .select(GatewayProviderConnectionRecord::as_select())
            .first::<GatewayProviderConnectionRecord>(&mut conn)
            .await?;

        let mut fallback_provider_connections = Vec::with_capacity(fallback_ids.len());
        for fallback_id in &fallback_ids {
            let fallback = gateway_provider_connections::table
                .filter(gateway_provider_connections::workspace_id.eq(workspace_id))
                .filter(gateway_provider_connections::id.eq(fallback_id))
                .filter(gateway_provider_connections::deleted_at.is_null())
                .select(GatewayProviderConnectionRecord::as_select())
                .first::<GatewayProviderConnectionRecord>(&mut conn)
                .await?;
            let encrypted_api_key = fallback.encrypted_api_key.clone();
            fallback_provider_connections.push(super::GatewayProviderConnectionSecret {
                connection: provider_record_to_wire(fallback)?,
                encrypted_api_key,
            });
        }
        Ok(ResolvedGatewayRoute {
            route: route_record_to_wire(route, fallback_ids)?,
            encrypted_api_key: provider.encrypted_api_key.clone(),
            provider_connection: provider_record_to_wire(provider)?,
            fallback_provider_connections,
        })
    }
}

async fn replace_fallbacks(
    conn: &mut diesel_async::AsyncPgConnection,
    workspace_id: &str,
    route_id: &str,
    provider_connection_ids: &[String],
) -> Result<(), StorageError> {
    diesel::delete(
        gateway_route_fallbacks::table
            .filter(gateway_route_fallbacks::workspace_id.eq(workspace_id))
            .filter(gateway_route_fallbacks::route_id.eq(route_id)),
    )
    .execute(conn)
    .await?;
    if provider_connection_ids.is_empty() {
        return Ok(());
    }
    let rows = provider_connection_ids
        .iter()
        .enumerate()
        .map(
            |(position, provider_connection_id)| NewGatewayRouteFallback {
                workspace_id: workspace_id.to_string(),
                route_id: route_id.to_string(),
                position: position as i32 + 1,
                provider_connection_id: provider_connection_id.clone(),
            },
        )
        .collect::<Vec<_>>();
    diesel::insert_into(gateway_route_fallbacks::table)
        .values(rows)
        .execute(conn)
        .await?;
    Ok(())
}

async fn load_fallback_ids(
    conn: &mut diesel_async::AsyncPgConnection,
    workspace_id: &str,
    route_id: &str,
) -> Result<Vec<String>, StorageError> {
    Ok(gateway_route_fallbacks::table
        .filter(gateway_route_fallbacks::workspace_id.eq(workspace_id))
        .filter(gateway_route_fallbacks::route_id.eq(route_id))
        .order(gateway_route_fallbacks::position.asc())
        .select(gateway_route_fallbacks::provider_connection_id)
        .load::<String>(conn)
        .await?)
}
