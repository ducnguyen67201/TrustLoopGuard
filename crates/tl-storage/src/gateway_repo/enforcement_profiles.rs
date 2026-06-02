use diesel::dsl::now;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use tl_core::EnforcementProfile;

use super::{mapping::profile_record_to_wire, EnforcementProfilePatch, GatewayRepo};
use crate::{
    models::{EnforcementProfileRecord, NewEnforcementProfile},
    schema::enforcement_profiles,
    StorageError,
};

impl GatewayRepo {
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
        if let Some(value) = patch.response_mode {
            current.response_mode = value;
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
            enforcement_profiles::response_mode.eq(current.response_mode),
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
}
