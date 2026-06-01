use chrono::{DateTime, Utc};
use diesel::prelude::*;
use tl_core::AnalyticsDashboardView;

use crate::schema::analytics_dashboard_views;
use crate::StorageError;

#[derive(Debug, Queryable, Selectable)]
#[diesel(table_name = analytics_dashboard_views)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub(super) struct ViewRecord {
    id: String,
    name: String,
    is_default: bool,
    config: serde_json::Value,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[derive(Insertable)]
#[diesel(table_name = analytics_dashboard_views)]
pub(super) struct NewViewRecord<'a> {
    pub(super) workspace_id: &'a str,
    pub(super) id: &'a str,
    pub(super) name: &'a str,
    pub(super) is_default: bool,
    pub(super) config: serde_json::Value,
}

pub(super) fn view_from_record(row: ViewRecord) -> Result<AnalyticsDashboardView, StorageError> {
    Ok(AnalyticsDashboardView {
        id: row.id,
        name: row.name,
        is_default: row.is_default,
        config: serde_json::from_value(row.config)
            .map_err(|e| StorageError::Internal(format!("analytics view parse: {e}")))?,
        created_at: row.created_at.to_rfc3339(),
        updated_at: row.updated_at.to_rfc3339(),
    })
}
