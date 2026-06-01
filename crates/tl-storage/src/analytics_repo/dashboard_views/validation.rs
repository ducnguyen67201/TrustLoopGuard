use tl_core::{AnalyticsDashboardViewConfig, AnalyticsWidgetLayout};

use crate::StorageError;

pub(super) fn validate_view_name(name: &str) -> Result<(), StorageError> {
    if name.trim().is_empty() {
        return Err(StorageError::Internal(
            "analytics view name is required".into(),
        ));
    }
    Ok(())
}

pub(super) fn validate_view_config(
    config: &AnalyticsDashboardViewConfig,
) -> Result<(), StorageError> {
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
