use tl_core::{AnalyticsDashboardViewConfig, AnalyticsWidgetLayout};

use super::AnalyticsStoreError;

pub(super) fn validate_view_request(
    name: &str,
    config: &AnalyticsDashboardViewConfig,
) -> Result<(), AnalyticsStoreError> {
    validate_name(name)?;
    validate_config(config)
}

pub(super) fn validate_name(name: &str) -> Result<(), AnalyticsStoreError> {
    if name.trim().is_empty() {
        Err(AnalyticsStoreError::Validation(
            "analytics view name is required".into(),
        ))
    } else {
        Ok(())
    }
}

pub(super) fn validate_config(
    config: &AnalyticsDashboardViewConfig,
) -> Result<(), AnalyticsStoreError> {
    if config.widgets.is_empty() {
        return Err(AnalyticsStoreError::Validation(
            "analytics view must include at least one widget".into(),
        ));
    }
    for widget in &config.widgets {
        validate_layout(&widget.layout)?;
    }
    Ok(())
}

fn validate_layout(layout: &AnalyticsWidgetLayout) -> Result<(), AnalyticsStoreError> {
    if layout.w == 0 || layout.w > 12 || layout.h == 0 || layout.h > 4 {
        return Err(AnalyticsStoreError::Validation(
            "analytics widget layout must use width 1-12 and height 1-4".into(),
        ));
    }
    if layout.x >= 12 || layout.x + layout.w > 12 {
        return Err(AnalyticsStoreError::Validation(
            "analytics widget layout must fit within the 12-column grid".into(),
        ));
    }
    Ok(())
}
