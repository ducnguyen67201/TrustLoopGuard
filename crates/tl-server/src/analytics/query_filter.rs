use tl_core::{AnalyticsDimension, AnalyticsFilter, AnalyticsQueryRequest};

pub(super) fn with_default_environment_filter(
    mut request: AnalyticsQueryRequest,
    environment_id: &str,
) -> AnalyticsQueryRequest {
    request
        .filters
        .retain(|filter| filter.dimension != AnalyticsDimension::Environment);
    request.filters.push(AnalyticsFilter {
        dimension: AnalyticsDimension::Environment,
        values: vec![environment_id.to_string()],
    });
    request
}
