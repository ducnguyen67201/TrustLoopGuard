use diesel::dsl::now;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use tl_core::GatewayProviderConnection;

use super::{mapping::provider_record_to_wire, GatewayProviderConnectionSecret, GatewayRepo};
use crate::{
    models::{GatewayProviderConnectionRecord, NewGatewayProviderConnection},
    schema::{gateway_provider_connections, gateway_routes},
    StorageError,
};

impl GatewayRepo {
    pub async fn list_provider_connections(
        &self,
        workspace_id: &str,
    ) -> Result<Vec<GatewayProviderConnection>, StorageError> {
        let mut conn = self.connection().await?;
        let rows = gateway_provider_connections::table
            .filter(gateway_provider_connections::workspace_id.eq(workspace_id))
            .filter(gateway_provider_connections::deleted_at.is_null())
            .order(gateway_provider_connections::created_at.desc())
            .select(GatewayProviderConnectionRecord::as_select())
            .load::<GatewayProviderConnectionRecord>(&mut conn)
            .await?;
        rows.into_iter().map(provider_record_to_wire).collect()
    }

    pub async fn create_provider_connection(
        &self,
        input: NewGatewayProviderConnection,
    ) -> Result<GatewayProviderConnection, StorageError> {
        let mut conn = self.connection().await?;
        let row = diesel::insert_into(gateway_provider_connections::table)
            .values(input)
            .returning(GatewayProviderConnectionRecord::as_returning())
            .get_result::<GatewayProviderConnectionRecord>(&mut conn)
            .await?;
        provider_record_to_wire(row)
    }

    pub async fn update_provider_connection(
        &self,
        workspace_id: &str,
        id: &str,
        display_name: Option<&str>,
        base_url: Option<Option<&str>>,
        default_model: Option<&str>,
        encrypted_api_key: Option<&str>,
    ) -> Result<GatewayProviderConnection, StorageError> {
        let mut conn = self.connection().await?;
        let mut current = gateway_provider_connections::table
            .filter(gateway_provider_connections::workspace_id.eq(workspace_id))
            .filter(gateway_provider_connections::id.eq(id))
            .filter(gateway_provider_connections::deleted_at.is_null())
            .select(GatewayProviderConnectionRecord::as_select())
            .first::<GatewayProviderConnectionRecord>(&mut conn)
            .await?;

        if let Some(value) = display_name {
            current.display_name = value.to_string();
        }
        if let Some(value) = base_url {
            current.base_url = value.map(str::to_string);
        }
        if let Some(value) = default_model {
            current.default_model = value.to_string();
        }
        if let Some(value) = encrypted_api_key {
            current.encrypted_api_key = value.to_string();
        }

        let row = diesel::update(
            gateway_provider_connections::table
                .filter(gateway_provider_connections::workspace_id.eq(workspace_id))
                .filter(gateway_provider_connections::id.eq(id))
                .filter(gateway_provider_connections::deleted_at.is_null()),
        )
        .set((
            gateway_provider_connections::display_name.eq(current.display_name),
            gateway_provider_connections::base_url.eq(current.base_url),
            gateway_provider_connections::default_model.eq(current.default_model),
            gateway_provider_connections::encrypted_api_key.eq(current.encrypted_api_key),
            gateway_provider_connections::updated_at.eq(now),
        ))
        .returning(GatewayProviderConnectionRecord::as_returning())
        .get_result::<GatewayProviderConnectionRecord>(&mut conn)
        .await?;
        provider_record_to_wire(row)
    }

    pub async fn get_provider_connection_secret(
        &self,
        workspace_id: &str,
        id: &str,
    ) -> Result<GatewayProviderConnectionSecret, StorageError> {
        let mut conn = self.connection().await?;
        let row = gateway_provider_connections::table
            .filter(gateway_provider_connections::workspace_id.eq(workspace_id))
            .filter(gateway_provider_connections::id.eq(id))
            .filter(gateway_provider_connections::deleted_at.is_null())
            .select(GatewayProviderConnectionRecord::as_select())
            .first::<GatewayProviderConnectionRecord>(&mut conn)
            .await?;
        Ok(GatewayProviderConnectionSecret {
            encrypted_api_key: row.encrypted_api_key.clone(),
            connection: provider_record_to_wire(row)?,
        })
    }

    pub async fn delete_provider_connection(
        &self,
        workspace_id: &str,
        id: &str,
    ) -> Result<(), StorageError> {
        let mut conn = self.connection().await?;
        let route_count = gateway_routes::table
            .filter(gateway_routes::workspace_id.eq(workspace_id))
            .filter(gateway_routes::provider_connection_id.eq(id))
            .count()
            .get_result::<i64>(&mut conn)
            .await?;
        if route_count > 0 {
            return Err(StorageError::Conflict);
        }

        let count = match diesel::delete(
            gateway_provider_connections::table
                .filter(gateway_provider_connections::workspace_id.eq(workspace_id))
                .filter(gateway_provider_connections::id.eq(id))
                .filter(gateway_provider_connections::deleted_at.is_null()),
        )
        .execute(&mut conn)
        .await
        {
            Ok(count) => count,
            Err(diesel::result::Error::DatabaseError(
                diesel::result::DatabaseErrorKind::ForeignKeyViolation,
                _,
            )) => return Err(StorageError::Conflict),
            Err(error) => return Err(error.into()),
        };
        if count == 0 {
            return Err(StorageError::NotFound);
        }
        Ok(())
    }
}
