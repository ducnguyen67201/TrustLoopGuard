use serde::{Deserialize, Serialize};

#[cfg(feature = "schema")]
use schemars::JsonSchema;
#[cfg(feature = "ts-export")]
use ts_rs::TS;
#[cfg(feature = "openapi")]
use utoipa::ToSchema;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub enum AnalyticsMetric {
    TraceCount,
    AllowCount,
    BlockCount,
    RewriteCount,
    EscalateCount,
    InterventionRate,
    P95LatencyMs,
    HumanReviewCount,
    HumanInterventionRate,
    FalsePositiveRate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub enum AnalyticsDimension {
    AgentId,
    Environment,
    RunKind,
    RunStatus,
    Decision,
    PolicyId,
    WorkflowStep,
    ReviewOutcome,
    ExternalId,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub enum AnalyticsChartType {
    BigNumber,
    Bar,
    Line,
    Area,
    Donut,
    Table,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct AnalyticsFilter {
    pub dimension: AnalyticsDimension,
    pub values: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct AnalyticsQueryRequest {
    pub metric: AnalyticsMetric,
    #[serde(default)]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub group_by: Option<AnalyticsDimension>,
    #[serde(default)]
    pub filters: Vec<AnalyticsFilter>,
    #[serde(default)]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub limit: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct AnalyticsQueryPoint {
    pub label: String,
    pub value: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct AnalyticsQueryResponse {
    pub metric: AnalyticsMetric,
    #[serde(default)]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub group_by: Option<AnalyticsDimension>,
    pub total: f64,
    pub points: Vec<AnalyticsQueryPoint>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct AnalyticsCatalogMetric {
    pub metric: AnalyticsMetric,
    pub label: String,
    pub default_chart_type: AnalyticsChartType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct AnalyticsCatalogDimension {
    pub dimension: AnalyticsDimension,
    pub label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct AnalyticsFacet {
    pub dimension: AnalyticsDimension,
    pub label: String,
    pub values: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct AnalyticsFacetCatalogResponse {
    pub metrics: Vec<AnalyticsCatalogMetric>,
    pub dimensions: Vec<AnalyticsCatalogDimension>,
    pub chart_types: Vec<AnalyticsChartType>,
    pub facets: Vec<AnalyticsFacet>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct AnalyticsDashboardWidget {
    pub id: String,
    pub title: String,
    pub metric: AnalyticsMetric,
    pub chart_type: AnalyticsChartType,
    #[serde(default)]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub group_by: Option<AnalyticsDimension>,
    #[serde(default = "default_analytics_widget_layout")]
    pub layout: AnalyticsWidgetLayout,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct AnalyticsWidgetLayout {
    pub x: usize,
    pub y: usize,
    pub w: usize,
    pub h: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct AnalyticsDashboardViewConfig {
    pub filters: Vec<AnalyticsFilter>,
    pub widgets: Vec<AnalyticsDashboardWidget>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct AnalyticsDashboardView {
    pub id: String,
    pub name: String,
    pub is_default: bool,
    pub config: AnalyticsDashboardViewConfig,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct AnalyticsDashboardViewListResponse {
    pub views: Vec<AnalyticsDashboardView>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct CreateAnalyticsDashboardViewRequest {
    pub name: String,
    #[serde(default)]
    pub is_default: bool,
    pub config: AnalyticsDashboardViewConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct UpdateAnalyticsDashboardViewRequest {
    #[serde(default)]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub name: Option<String>,
    #[serde(default)]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub is_default: Option<bool>,
    #[serde(default)]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub config: Option<AnalyticsDashboardViewConfig>,
}

fn default_analytics_widget_layout() -> AnalyticsWidgetLayout {
    AnalyticsWidgetLayout {
        x: 0,
        y: 0,
        w: 6,
        h: 1,
    }
}
