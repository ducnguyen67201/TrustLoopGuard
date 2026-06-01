use chrono::{DateTime, Utc};
use diesel::prelude::*;
use diesel_async::{AsyncConnection, RunQueryDsl};
use tl_core::{
    AnalyticsDashboardView, AnalyticsDashboardViewConfig, AnalyticsWidgetLayout,
    CreateAnalyticsDashboardViewRequest, UpdateAnalyticsDashboardViewRequest,
};
use uuid::Uuid;

use crate::postgres::DbConnection;
use crate::schema::analytics_dashboard_views;
use crate::StorageError;

use super::AnalyticsRepo;

#[derive(Debug, Queryable, Selectable)]
#[diesel(table_name = analytics_dashboard_views)]
#[diesel(check_for_backend(diesel::pg::Pg))]
struct ViewRecord {
    id: String,
    name: String,
    is_default: bool,
    config: serde_json::Value,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[derive(Insertable)]
#[diesel(table_name = analytics_dashboard_views)]
struct NewViewRecord<'a> {
    workspace_id: &'a str,
    id: &'a str,
    name: &'a str,
    is_default: bool,
    config: serde_json::Value,
}

impl AnalyticsRepo {
    pub async fn list_views(
        &self,
        workspace_id: &str,
    ) -> Result<Vec<AnalyticsDashboardView>, StorageError> {
        let mut conn = self.connection().await?;
        let rows = analytics_dashboard_views::table
            .filter(analytics_dashboard_views::workspace_id.eq(workspace_id))
            .select(ViewRecord::as_select())
            .order((
                analytics_dashboard_views::is_default.desc(),
                analytics_dashboard_views::created_at.asc(),
            ))
            .load::<ViewRecord>(&mut conn)
            .await
            .map_err(|e| StorageError::Internal(format!("analytics views list: {e}")))?;
        rows.into_iter().map(view_from_record).collect()
    }

    pub async fn create_view(
        &self,
        workspace_id: &str,
        request: CreateAnalyticsDashboardViewRequest,
    ) -> Result<AnalyticsDashboardView, StorageError> {
        validate_view_name(&request.name)?;
        validate_view_config(&request.config)?;
        let id = Uuid::now_v7().to_string();
        let config = serde_json::to_value(request.config)
            .map_err(|e| StorageError::Internal(format!("analytics view config: {e}")))?;

        let mut conn = self.connection().await?;
        conn.transaction::<_, StorageError, _>(async |conn| {
            if request.is_default {
                clear_default(conn, workspace_id).await?;
            }
            diesel::insert_into(analytics_dashboard_views::table)
                .values(NewViewRecord {
                    workspace_id,
                    id: &id,
                    name: request.name.trim(),
                    is_default: request.is_default,
                    config,
                })
                .execute(conn)
                .await
                .map_err(|e| StorageError::Internal(format!("analytics view create: {e}")))?;
            Ok(())
        })
        .await?;

        drop(conn);
        self.get_view(workspace_id, &id).await
    }

    pub async fn update_view(
        &self,
        workspace_id: &str,
        view_id: &str,
        request: UpdateAnalyticsDashboardViewRequest,
    ) -> Result<AnalyticsDashboardView, StorageError> {
        if let Some(name) = request.name.as_deref() {
            validate_view_name(name)?;
        }
        if let Some(config) = request.config.as_ref() {
            validate_view_config(config)?;
        }
        let config = request
            .config
            .map(serde_json::to_value)
            .transpose()
            .map_err(|e| StorageError::Internal(format!("analytics view config: {e}")))?;

        let mut conn = self.connection().await?;
        conn.transaction::<_, StorageError, _>(async |conn| {
            ensure_view_exists(conn, workspace_id, view_id).await?;
            if request.is_default == Some(true) {
                clear_default(conn, workspace_id).await?;
            }
            if let Some(name) = request.name.as_deref() {
                update_view_name(conn, workspace_id, view_id, name.trim()).await?;
            }
            if let Some(is_default) = request.is_default {
                update_view_default(conn, workspace_id, view_id, is_default).await?;
            }
            if let Some(config) = config {
                update_view_config(conn, workspace_id, view_id, config).await?;
            }
            Ok(())
        })
        .await?;

        drop(conn);
        self.get_view(workspace_id, view_id).await
    }

    pub async fn delete_view(&self, workspace_id: &str, view_id: &str) -> Result<(), StorageError> {
        let mut conn = self.connection().await?;
        let rows = diesel::delete(
            analytics_dashboard_views::table
                .filter(analytics_dashboard_views::workspace_id.eq(workspace_id))
                .filter(analytics_dashboard_views::id.eq(view_id)),
        )
        .execute(&mut conn)
        .await
        .map_err(|e| StorageError::Internal(format!("analytics view delete: {e}")))?;
        if rows == 0 {
            Err(StorageError::NotFound)
        } else {
            Ok(())
        }
    }

    async fn get_view(
        &self,
        workspace_id: &str,
        view_id: &str,
    ) -> Result<AnalyticsDashboardView, StorageError> {
        let mut conn = self.connection().await?;
        let row = analytics_dashboard_views::table
            .filter(analytics_dashboard_views::workspace_id.eq(workspace_id))
            .filter(analytics_dashboard_views::id.eq(view_id))
            .select(ViewRecord::as_select())
            .first::<ViewRecord>(&mut conn)
            .await
            .optional()
            .map_err(|e| StorageError::Internal(format!("analytics view get: {e}")))?
            .ok_or(StorageError::NotFound)?;
        view_from_record(row)
    }
}

async fn ensure_view_exists(
    conn: &mut DbConnection<'_>,
    workspace_id: &str,
    view_id: &str,
) -> Result<(), StorageError> {
    let exists = analytics_dashboard_views::table
        .filter(analytics_dashboard_views::workspace_id.eq(workspace_id))
        .filter(analytics_dashboard_views::id.eq(view_id))
        .select(analytics_dashboard_views::id)
        .first::<String>(conn)
        .await
        .optional()?
        .is_some();
    if exists {
        Ok(())
    } else {
        Err(StorageError::NotFound)
    }
}

async fn clear_default(
    conn: &mut DbConnection<'_>,
    workspace_id: &str,
) -> Result<(), StorageError> {
    diesel::update(
        analytics_dashboard_views::table
            .filter(analytics_dashboard_views::workspace_id.eq(workspace_id))
            .filter(analytics_dashboard_views::is_default.eq(true)),
    )
    .set(analytics_dashboard_views::is_default.eq(false))
    .execute(conn)
    .await?;
    Ok(())
}

async fn update_view_name(
    conn: &mut DbConnection<'_>,
    workspace_id: &str,
    view_id: &str,
    name: &str,
) -> Result<(), StorageError> {
    diesel::update(
        analytics_dashboard_views::table
            .filter(analytics_dashboard_views::workspace_id.eq(workspace_id))
            .filter(analytics_dashboard_views::id.eq(view_id)),
    )
    .set(analytics_dashboard_views::name.eq(name))
    .execute(conn)
    .await?;
    Ok(())
}

async fn update_view_default(
    conn: &mut DbConnection<'_>,
    workspace_id: &str,
    view_id: &str,
    is_default: bool,
) -> Result<(), StorageError> {
    diesel::update(
        analytics_dashboard_views::table
            .filter(analytics_dashboard_views::workspace_id.eq(workspace_id))
            .filter(analytics_dashboard_views::id.eq(view_id)),
    )
    .set(analytics_dashboard_views::is_default.eq(is_default))
    .execute(conn)
    .await?;
    Ok(())
}

async fn update_view_config(
    conn: &mut DbConnection<'_>,
    workspace_id: &str,
    view_id: &str,
    config: serde_json::Value,
) -> Result<(), StorageError> {
    diesel::update(
        analytics_dashboard_views::table
            .filter(analytics_dashboard_views::workspace_id.eq(workspace_id))
            .filter(analytics_dashboard_views::id.eq(view_id)),
    )
    .set(analytics_dashboard_views::config.eq(config))
    .execute(conn)
    .await?;
    Ok(())
}

fn view_from_record(row: ViewRecord) -> Result<AnalyticsDashboardView, StorageError> {
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

fn validate_view_name(name: &str) -> Result<(), StorageError> {
    if name.trim().is_empty() {
        return Err(StorageError::Internal(
            "analytics view name is required".into(),
        ));
    }
    Ok(())
}

fn validate_view_config(config: &AnalyticsDashboardViewConfig) -> Result<(), StorageError> {
    if config.widgets.is_empty() {
        return Err(StorageError::Internal(
            "analytics view must include at least one widget".into(),
        ));
    }
    for widget in &config.widgets {
        validate_layout(&widget.layout)?;
    }
    Ok(())
}

fn validate_layout(layout: &AnalyticsWidgetLayout) -> Result<(), StorageError> {
    if layout.w == 0 || layout.w > 12 || layout.h == 0 || layout.h > 4 {
        return Err(StorageError::Internal(
            "analytics widget layout must use width 1-12 and height 1-4".into(),
        ));
    }
    if layout.x >= 12 || layout.x + layout.w > 12 {
        return Err(StorageError::Internal(
            "analytics widget layout must fit within the 12-column grid".into(),
        ));
    }
    Ok(())
}
