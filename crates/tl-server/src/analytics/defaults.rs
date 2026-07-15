use tl_core::{
    AnalyticsCatalogDimension, AnalyticsCatalogMetric, AnalyticsChartType, AnalyticsDashboardView,
    AnalyticsDashboardViewConfig, AnalyticsDashboardWidget, AnalyticsDimension,
    AnalyticsFacetCatalogResponse, AnalyticsMetric, AnalyticsWidgetLayout,
};

pub(super) fn empty_catalog() -> AnalyticsFacetCatalogResponse {
    AnalyticsFacetCatalogResponse {
        metrics: vec![
            AnalyticsCatalogMetric {
                metric: AnalyticsMetric::TraceCount,
                label: "Traces".into(),
                default_chart_type: AnalyticsChartType::Bar,
            },
            AnalyticsCatalogMetric {
                metric: AnalyticsMetric::InterventionRate,
                label: "Intervention rate".into(),
                default_chart_type: AnalyticsChartType::Line,
            },
            AnalyticsCatalogMetric {
                metric: AnalyticsMetric::P95LatencyMs,
                label: "p95 latency".into(),
                default_chart_type: AnalyticsChartType::Line,
            },
        ],
        dimensions: vec![
            AnalyticsCatalogDimension {
                dimension: AnalyticsDimension::AgentId,
                label: "Agent".into(),
            },
            AnalyticsCatalogDimension {
                dimension: AnalyticsDimension::AuthorizationEffect,
                label: "Authorization effect".into(),
            },
        ],
        chart_types: vec![
            AnalyticsChartType::BigNumber,
            AnalyticsChartType::Bar,
            AnalyticsChartType::Line,
            AnalyticsChartType::Donut,
        ],
        facets: vec![],
    }
}

pub(super) fn default_views() -> Vec<AnalyticsDashboardView> {
    let now = chrono::Utc::now().to_rfc3339();
    vec![AnalyticsDashboardView {
        id: "default".into(),
        name: "Default analytics".into(),
        is_default: true,
        config: AnalyticsDashboardViewConfig {
            filters: vec![],
            widgets: vec![
                AnalyticsDashboardWidget {
                    id: "trace-volume".into(),
                    title: "Trace volume".into(),
                    metric: AnalyticsMetric::TraceCount,
                    chart_type: AnalyticsChartType::Bar,
                    group_by: Some(AnalyticsDimension::AuthorizationEffect),
                    layout: AnalyticsWidgetLayout {
                        x: 0,
                        y: 0,
                        w: 6,
                        h: 1,
                    },
                },
                AnalyticsDashboardWidget {
                    id: "intervention-rate".into(),
                    title: "Intervention rate".into(),
                    metric: AnalyticsMetric::InterventionRate,
                    chart_type: AnalyticsChartType::BigNumber,
                    group_by: None,
                    layout: AnalyticsWidgetLayout {
                        x: 6,
                        y: 0,
                        w: 3,
                        h: 1,
                    },
                },
                AnalyticsDashboardWidget {
                    id: "p95-latency".into(),
                    title: "p95 latency".into(),
                    metric: AnalyticsMetric::P95LatencyMs,
                    chart_type: AnalyticsChartType::BigNumber,
                    group_by: None,
                    layout: AnalyticsWidgetLayout {
                        x: 9,
                        y: 0,
                        w: 3,
                        h: 1,
                    },
                },
            ],
        },
        created_at: now.clone(),
        updated_at: now,
    }]
}
