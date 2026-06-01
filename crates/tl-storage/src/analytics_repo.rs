mod dashboard_views;
mod facts;
mod metrics;

use std::collections::HashMap;

use tl_core::{
    AnalyticsCatalogDimension, AnalyticsCatalogMetric, AnalyticsChartType, AnalyticsFacet,
    AnalyticsFacetCatalogResponse, AnalyticsQueryPoint, AnalyticsQueryRequest,
    AnalyticsQueryResponse,
};

use crate::postgres::{DbConnection, DbPool};
use crate::StorageError;

use metrics::{
    default_chart_type, dimension_label, fact_values, matches_filters, metric_label, metric_value,
    supported_dimensions, supported_metrics, validate_query, values_for_dimension,
};

#[derive(Clone)]
pub struct AnalyticsRepo {
    pool: DbPool,
}

impl AnalyticsRepo {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    pub async fn catalog(
        &self,
        workspace_id: &str,
    ) -> Result<AnalyticsFacetCatalogResponse, StorageError> {
        let facts = self.facts(workspace_id).await?;
        let mut facets = Vec::new();
        for dimension in supported_dimensions() {
            let values = fact_values(&facts, dimension)
                .into_iter()
                .take(100)
                .collect::<Vec<_>>();
            facets.push(AnalyticsFacet {
                dimension,
                label: dimension_label(dimension).to_string(),
                values,
            });
        }
        Ok(AnalyticsFacetCatalogResponse {
            metrics: supported_metrics()
                .into_iter()
                .map(|metric| AnalyticsCatalogMetric {
                    metric,
                    label: metric_label(metric).to_string(),
                    default_chart_type: default_chart_type(metric),
                })
                .collect(),
            dimensions: supported_dimensions()
                .into_iter()
                .map(|dimension| AnalyticsCatalogDimension {
                    dimension,
                    label: dimension_label(dimension).to_string(),
                })
                .collect(),
            chart_types: vec![
                AnalyticsChartType::BigNumber,
                AnalyticsChartType::Bar,
                AnalyticsChartType::Line,
                AnalyticsChartType::Area,
                AnalyticsChartType::Donut,
                AnalyticsChartType::Table,
            ],
            facets,
        })
    }

    pub async fn query(
        &self,
        workspace_id: &str,
        request: AnalyticsQueryRequest,
    ) -> Result<AnalyticsQueryResponse, StorageError> {
        validate_query(&request)?;
        let facts = self.facts(workspace_id).await?;
        let filtered = facts
            .iter()
            .filter(|fact| matches_filters(fact, &request.filters))
            .collect::<Vec<_>>();
        let total = metric_value(request.metric, filtered.iter().copied());

        let mut points = match request.group_by {
            Some(dimension) => {
                let mut grouped: HashMap<String, Vec<_>> = HashMap::new();
                for fact in &filtered {
                    for value in values_for_dimension(fact, dimension) {
                        grouped.entry(value).or_default().push(*fact);
                    }
                }
                grouped
                    .into_iter()
                    .map(|(label, rows)| AnalyticsQueryPoint {
                        label,
                        value: metric_value(request.metric, rows.into_iter()),
                    })
                    .collect::<Vec<_>>()
            }
            None => vec![AnalyticsQueryPoint {
                label: metric_label(request.metric).to_string(),
                value: total,
            }],
        };
        points.sort_by(|a, b| {
            b.value
                .partial_cmp(&a.value)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.label.cmp(&b.label))
        });
        points.truncate(request.limit.unwrap_or(12).clamp(1, 100));

        Ok(AnalyticsQueryResponse {
            metric: request.metric,
            group_by: request.group_by,
            total,
            points,
        })
    }

    async fn connection(&self) -> Result<DbConnection<'_>, StorageError> {
        self.pool
            .get()
            .await
            .map_err(|e| StorageError::Internal(format!("db pool: {e}")))
    }
}

impl std::fmt::Debug for AnalyticsRepo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AnalyticsRepo").finish_non_exhaustive()
    }
}
