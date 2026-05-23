use chrono::{DateTime, Utc};
use diesel::dsl::now;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use tl_core::{
    EnforcementProfile, FailMode, GatewayCredentialStatus, GatewayInputAction, GatewayOutputAction,
    GatewayProviderConnection, GatewayProviderKind, GatewayRoute, RetentionMode,
};

use crate::models::{
    EnforcementProfileRecord, GatewayProviderConnectionRecord, GatewayRouteRecord,
    NewEnforcementProfile, NewGatewayProviderConnection, NewGatewayRoute,
};
use crate::postgres::{DbConnection, DbPool};
use crate::schema::{enforcement_profiles, gateway_provider_connections, gateway_routes};
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
        let count = diesel::update(
            gateway_provider_connections::table
                .filter(gateway_provider_connections::workspace_id.eq(workspace_id))
                .filter(gateway_provider_connections::id.eq(id))
                .filter(gateway_provider_connections::deleted_at.is_null()),
        )
        .set((
            gateway_provider_connections::deleted_at.eq(now),
            gateway_provider_connections::updated_at.eq(now),
        ))
        .execute(&mut conn)
        .await?;
        if count == 0 {
            return Err(StorageError::NotFound);
        }
        Ok(())
    }

    pub async fn list_enforcement_profiles(
        &self,
        workspace_id: &str,
    ) -> Result<Vec<EnforcementProfile>, StorageError> {
        let mut conn = self.connection().await?;
        let rows = enforcement_profiles::table
            .filter(enforcement_profiles::workspace_id.eq(workspace_id))
            .filter(enforcement_profiles::deleted_at.is_null())
            .order(enforcement_profiles::created_at.desc())
            .select(EnforcementProfileRecord::as_select())
            .load::<EnforcementProfileRecord>(&mut conn)
            .await?;
        rows.into_iter().map(profile_record_to_wire).collect()
    }

    pub async fn create_enforcement_profile(
        &self,
        input: NewEnforcementProfile,
    ) -> Result<EnforcementProfile, StorageError> {
        let mut conn = self.connection().await?;
        let row = diesel::insert_into(enforcement_profiles::table)
            .values(input)
            .returning(EnforcementProfileRecord::as_returning())
            .get_result::<EnforcementProfileRecord>(&mut conn)
            .await?;
        profile_record_to_wire(row)
    }

    pub async fn update_enforcement_profile(
        &self,
        workspace_id: &str,
        id: &str,
        patch: EnforcementProfilePatch,
    ) -> Result<EnforcementProfile, StorageError> {
        let mut conn = self.connection().await?;
        let mut current = enforcement_profiles::table
            .filter(enforcement_profiles::workspace_id.eq(workspace_id))
            .filter(enforcement_profiles::id.eq(id))
            .filter(enforcement_profiles::deleted_at.is_null())
            .select(EnforcementProfileRecord::as_select())
            .first::<EnforcementProfileRecord>(&mut conn)
            .await?;

        if let Some(value) = patch.display_name {
            current.display_name = value;
        }
        if let Some(value) = patch.input_action {
            current.input_action = value;
        }
        if let Some(value) = patch.output_action {
            current.output_action = value;
        }
        if let Some(value) = patch.fail_mode {
            current.fail_mode = value;
        }
        if let Some(value) = patch.retention_mode {
            current.retention_mode = value;
        }
        if let Some(value) = patch.fallback_message {
            current.fallback_message = value;
        }
        if let Some(value) = patch.max_regenerations {
            current.max_regenerations = value;
        }

        let row = diesel::update(
            enforcement_profiles::table
                .filter(enforcement_profiles::workspace_id.eq(workspace_id))
                .filter(enforcement_profiles::id.eq(id))
                .filter(enforcement_profiles::deleted_at.is_null()),
        )
        .set((
            enforcement_profiles::display_name.eq(current.display_name),
            enforcement_profiles::input_action.eq(current.input_action),
            enforcement_profiles::output_action.eq(current.output_action),
            enforcement_profiles::fail_mode.eq(current.fail_mode),
            enforcement_profiles::retention_mode.eq(current.retention_mode),
            enforcement_profiles::fallback_message.eq(current.fallback_message),
            enforcement_profiles::max_regenerations.eq(current.max_regenerations),
            enforcement_profiles::updated_at.eq(now),
        ))
        .returning(EnforcementProfileRecord::as_returning())
        .get_result::<EnforcementProfileRecord>(&mut conn)
        .await?;
        profile_record_to_wire(row)
    }

    pub async fn delete_enforcement_profile(
        &self,
        workspace_id: &str,
        id: &str,
    ) -> Result<(), StorageError> {
        let mut conn = self.connection().await?;
        let count = diesel::update(
            enforcement_profiles::table
                .filter(enforcement_profiles::workspace_id.eq(workspace_id))
                .filter(enforcement_profiles::id.eq(id))
                .filter(enforcement_profiles::deleted_at.is_null()),
        )
        .set((
            enforcement_profiles::deleted_at.eq(now),
            enforcement_profiles::updated_at.eq(now),
        ))
        .execute(&mut conn)
        .await?;
        if count == 0 {
            return Err(StorageError::NotFound);
        }
        Ok(())
    }

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

fn provider_record_to_wire(
    row: GatewayProviderConnectionRecord,
) -> Result<GatewayProviderConnection, StorageError> {
    Ok(GatewayProviderConnection {
        id: row.id,
        display_name: row.display_name,
        kind: parse_provider_kind(&row.kind)?,
        base_url: row.base_url,
        default_model: row.default_model,
        credential_status: GatewayCredentialStatus::Configured,
        created_at: to_rfc3339(row.created_at),
        updated_at: to_rfc3339(row.updated_at),
    })
}

fn profile_record_to_wire(
    row: EnforcementProfileRecord,
) -> Result<EnforcementProfile, StorageError> {
    Ok(EnforcementProfile {
        id: row.id,
        display_name: row.display_name,
        input_action: parse_input_action(&row.input_action)?,
        output_action: parse_output_action(&row.output_action)?,
        fail_mode: parse_fail_mode(&row.fail_mode)?,
        retention_mode: parse_retention_mode(&row.retention_mode)?,
        fallback_message: row.fallback_message,
        max_regenerations: row.max_regenerations.max(0) as u32,
        created_at: to_rfc3339(row.created_at),
        updated_at: to_rfc3339(row.updated_at),
    })
}

fn route_record_to_wire(row: GatewayRouteRecord) -> GatewayRoute {
    GatewayRoute {
        id: row.id,
        display_name: row.display_name,
        provider_connection_id: row.provider_connection_id,
        agent_id: row.agent_id,
        enforcement_profile_id: row.enforcement_profile_id,
        created_at: to_rfc3339(row.created_at),
        updated_at: to_rfc3339(row.updated_at),
    }
}

fn to_rfc3339(value: DateTime<Utc>) -> String {
    value.to_rfc3339()
}

fn parse_provider_kind(value: &str) -> Result<GatewayProviderKind, StorageError> {
    match value {
        "openai_compatible" => Ok(GatewayProviderKind::OpenaiCompatible),
        "anthropic" => Ok(GatewayProviderKind::Anthropic),
        other => Err(StorageError::Internal(format!(
            "unknown provider kind: {other}"
        ))),
    }
}

fn parse_input_action(value: &str) -> Result<GatewayInputAction, StorageError> {
    match value {
        "allow" => Ok(GatewayInputAction::Allow),
        "block" => Ok(GatewayInputAction::Block),
        "redact" => Ok(GatewayInputAction::Redact),
        other => Err(StorageError::Internal(format!(
            "unknown input action: {other}"
        ))),
    }
}

fn parse_output_action(value: &str) -> Result<GatewayOutputAction, StorageError> {
    match value {
        "allow" => Ok(GatewayOutputAction::Allow),
        "block" => Ok(GatewayOutputAction::Block),
        "rewrite" => Ok(GatewayOutputAction::Rewrite),
        "escalate" => Ok(GatewayOutputAction::Escalate),
        other => Err(StorageError::Internal(format!(
            "unknown output action: {other}"
        ))),
    }
}

fn parse_fail_mode(value: &str) -> Result<FailMode, StorageError> {
    match value {
        "open" => Ok(FailMode::Open),
        "closed" => Ok(FailMode::Closed),
        other => Err(StorageError::Internal(format!(
            "unknown fail mode: {other}"
        ))),
    }
}

fn parse_retention_mode(value: &str) -> Result<RetentionMode, StorageError> {
    match value {
        "metadata_only" => Ok(RetentionMode::MetadataOnly),
        "redacted_body" => Ok(RetentionMode::RedactedBody),
        "full_body" => Ok(RetentionMode::FullBody),
        other => Err(StorageError::Internal(format!(
            "unknown retention mode: {other}"
        ))),
    }
}
