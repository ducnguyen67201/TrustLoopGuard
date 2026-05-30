use std::collections::{BTreeSet, HashMap};

use chrono::{DateTime, Utc};
use diesel::prelude::*;
use diesel_async::{AsyncConnection, RunQueryDsl};
use tl_core::{
    AnalyticsCatalogDimension, AnalyticsCatalogMetric, AnalyticsChartType, AnalyticsDashboardView,
    AnalyticsDashboardViewConfig, AnalyticsDimension, AnalyticsFacet,
    AnalyticsFacetCatalogResponse, AnalyticsFilter, AnalyticsMetric, AnalyticsQueryPoint,
    AnalyticsQueryRequest, AnalyticsQueryResponse, AnalyticsWidgetLayout,
    CreateAnalyticsDashboardViewRequest, UpdateAnalyticsDashboardViewRequest,
};
use uuid::Uuid;

use crate::postgres::{DbConnection, DbPool};
use crate::schema::{analytics_dashboard_views, human_review_events, run_events, runs, traces};
use crate::StorageError;

#[derive(Clone)]
pub struct AnalyticsRepo {
    pool: DbPool,
}

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

#[derive(Clone)]
struct AnalyticsFact {
    environment_id: String,
    decision: String,
    elapsed_ms: i32,
    agent_id: String,
    run_kind: String,
    run_status: String,
    external_id: String,
    workflow_step: String,
    review_outcome: String,
    policy_ids: Vec<String>,
}

#[derive(Default)]
struct MetricAccumulator {
    traces: i64,
    allow: i64,
    block: i64,
    rewrite: i64,
    escalate: i64,
    human_reviews: i64,
    human_interventions: i64,
    false_positives: i64,
    latencies: Vec<i32>,
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
                let mut grouped = HashMap::<String, Vec<&AnalyticsFact>>::new();
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
            let exists = analytics_dashboard_views::table
                .filter(analytics_dashboard_views::workspace_id.eq(workspace_id))
                .filter(analytics_dashboard_views::id.eq(view_id))
                .select(analytics_dashboard_views::id)
                .first::<String>(conn)
                .await
                .optional()?
                .is_some();
            if !exists {
                return Err(StorageError::NotFound);
            }
            if request.is_default == Some(true) {
                clear_default(conn, workspace_id).await?;
            }
            if let Some(name) = request.name.as_deref() {
                diesel::update(
                    analytics_dashboard_views::table
                        .filter(analytics_dashboard_views::workspace_id.eq(workspace_id))
                        .filter(analytics_dashboard_views::id.eq(view_id)),
                )
                .set(analytics_dashboard_views::name.eq(name.trim()))
                .execute(conn)
                .await?;
            }
            if let Some(is_default) = request.is_default {
                diesel::update(
                    analytics_dashboard_views::table
                        .filter(analytics_dashboard_views::workspace_id.eq(workspace_id))
                        .filter(analytics_dashboard_views::id.eq(view_id)),
                )
                .set(analytics_dashboard_views::is_default.eq(is_default))
                .execute(conn)
                .await?;
            }
            if let Some(config) = config {
                diesel::update(
                    analytics_dashboard_views::table
                        .filter(analytics_dashboard_views::workspace_id.eq(workspace_id))
                        .filter(analytics_dashboard_views::id.eq(view_id)),
                )
                .set(analytics_dashboard_views::config.eq(config))
                .execute(conn)
                .await?;
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

    async fn facts(&self, workspace_id: &str) -> Result<Vec<AnalyticsFact>, StorageError> {
        let mut conn = self.connection().await?;
        let trace_rows = traces::table
            .filter(traces::workspace_id.eq(workspace_id))
            .select((
                traces::trace_id,
                traces::run_id,
                traces::run_event_id,
                traces::environment_id,
                traces::decision,
                traces::elapsed_ms,
                traces::payload,
            ))
            .order(traces::created_at.desc())
            .limit(5_000)
            .load::<(
                Uuid,
                Option<Uuid>,
                Option<Uuid>,
                String,
                String,
                i32,
                serde_json::Value,
            )>(&mut conn)
            .await
            .map_err(|e| StorageError::Internal(format!("analytics traces: {e}")))?;
        let run_rows = runs::table
            .filter(runs::workspace_id.eq(workspace_id))
            .select((
                runs::id,
                runs::agent_id,
                runs::kind,
                runs::status,
                runs::external_id,
            ))
            .load::<(Uuid, String, String, String, Option<String>)>(&mut conn)
            .await
            .map_err(|e| StorageError::Internal(format!("analytics runs: {e}")))?;
        let event_rows = run_events::table
            .filter(run_events::workspace_id.eq(workspace_id))
            .select((
                run_events::id,
                run_events::kind,
                run_events::label,
                run_events::metadata,
            ))
            .load::<(Uuid, String, Option<String>, serde_json::Value)>(&mut conn)
            .await
            .map_err(|e| StorageError::Internal(format!("analytics run events: {e}")))?;
        let review_rows = human_review_events::table
            .filter(human_review_events::workspace_id.eq(workspace_id))
            .select((
                human_review_events::trace_id,
                human_review_events::outcome,
                human_review_events::created_at,
            ))
            .order(human_review_events::created_at.desc())
            .load::<(Uuid, String, DateTime<Utc>)>(&mut conn)
            .await
            .map_err(|e| StorageError::Internal(format!("analytics reviews: {e}")))?;

        let runs_by_id = run_rows
            .into_iter()
            .map(|(id, agent_id, kind, status, external_id)| {
                (
                    id,
                    (
                        agent_id,
                        kind,
                        status,
                        external_id.unwrap_or_else(|| "none".into()),
                    ),
                )
            })
            .collect::<HashMap<_, _>>();
        let events_by_id = event_rows
            .into_iter()
            .map(|(id, kind, label, metadata)| (id, (kind, label, metadata)))
            .collect::<HashMap<_, _>>();
        let mut latest_reviews = HashMap::<Uuid, String>::new();
        for (trace_id, outcome, _) in review_rows {
            latest_reviews.entry(trace_id).or_insert(outcome);
        }

        Ok(trace_rows
            .into_iter()
            .map(
                |(
                    trace_id,
                    run_id,
                    run_event_id,
                    environment_id,
                    decision,
                    elapsed_ms,
                    payload,
                )| {
                    let run = run_id.and_then(|id| runs_by_id.get(&id));
                    let event = run_event_id.and_then(|id| events_by_id.get(&id));
                    AnalyticsFact {
                        environment_id,
                        decision,
                        elapsed_ms,
                        agent_id: run
                            .map(|row| row.0.clone())
                            .or_else(|| payload_string(&payload, "agent_id"))
                            .unwrap_or_else(|| "unknown".into()),
                        run_kind: run
                            .map(|row| row.1.clone())
                            .unwrap_or_else(|| "ungrouped".into()),
                        run_status: run
                            .map(|row| row.2.clone())
                            .unwrap_or_else(|| "unknown".into()),
                        external_id: run
                            .map(|row| row.3.clone())
                            .unwrap_or_else(|| "none".into()),
                        workflow_step: event
                            .and_then(|row| workflow_step(&row.0, row.1.as_deref(), &row.2))
                            .unwrap_or_else(|| "unlabeled".into()),
                        review_outcome: latest_reviews
                            .get(&trace_id)
                            .cloned()
                            .unwrap_or_else(|| "not_reviewed".into()),
                        policy_ids: policy_ids(&payload),
                    }
                },
            )
            .collect())
    }

    async fn connection(&self) -> Result<DbConnection<'_>, StorageError> {
        self.pool
            .get()
            .await
            .map_err(|e| StorageError::Internal(format!("db pool: {e}")))
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

fn validate_query(request: &AnalyticsQueryRequest) -> Result<(), StorageError> {
    for filter in &request.filters {
        if filter.values.iter().any(|value| value.trim().is_empty()) {
            return Err(StorageError::Internal(
                "analytics filters must not contain empty values".into(),
            ));
        }
    }
    Ok(())
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

fn matches_filters(fact: &AnalyticsFact, filters: &[AnalyticsFilter]) -> bool {
    filters.iter().all(|filter| {
        let wanted = filter.values.iter().collect::<BTreeSet<_>>();
        values_for_dimension(fact, filter.dimension)
            .iter()
            .any(|value| wanted.contains(value))
    })
}

fn metric_value<'a>(
    metric: AnalyticsMetric,
    facts: impl Iterator<Item = &'a AnalyticsFact>,
) -> f64 {
    let acc = facts.fold(MetricAccumulator::default(), |mut acc, fact| {
        acc.traces += 1;
        acc.latencies.push(fact.elapsed_ms);
        match fact.decision.as_str() {
            "allow" => acc.allow += 1,
            "block" => acc.block += 1,
            "rewrite" => acc.rewrite += 1,
            "escalate" => acc.escalate += 1,
            _ => {}
        }
        if fact.review_outcome != "not_reviewed" {
            acc.human_reviews += 1;
        }
        if matches!(
            fact.review_outcome.as_str(),
            "corrected" | "rejected" | "missed_issue"
        ) {
            acc.human_interventions += 1;
        }
        if fact.review_outcome == "false_positive" {
            acc.false_positives += 1;
        }
        acc
    });
    match metric {
        AnalyticsMetric::TraceCount => acc.traces as f64,
        AnalyticsMetric::AllowCount => acc.allow as f64,
        AnalyticsMetric::BlockCount => acc.block as f64,
        AnalyticsMetric::RewriteCount => acc.rewrite as f64,
        AnalyticsMetric::EscalateCount => acc.escalate as f64,
        AnalyticsMetric::InterventionRate => {
            percentage(acc.block + acc.rewrite + acc.escalate, acc.traces)
        }
        AnalyticsMetric::P95LatencyMs => p95(acc.latencies).unwrap_or_default() as f64,
        AnalyticsMetric::HumanReviewCount => acc.human_reviews as f64,
        AnalyticsMetric::HumanInterventionRate => percentage(acc.human_interventions, acc.traces),
        AnalyticsMetric::FalsePositiveRate => percentage(acc.false_positives, acc.traces),
    }
}

fn values_for_dimension(fact: &AnalyticsFact, dimension: AnalyticsDimension) -> Vec<String> {
    match dimension {
        AnalyticsDimension::AgentId => vec![fact.agent_id.clone()],
        AnalyticsDimension::Environment => vec![fact.environment_id.clone()],
        AnalyticsDimension::RunKind => vec![fact.run_kind.clone()],
        AnalyticsDimension::RunStatus => vec![fact.run_status.clone()],
        AnalyticsDimension::Decision => vec![fact.decision.clone()],
        AnalyticsDimension::PolicyId => fact.policy_ids.clone(),
        AnalyticsDimension::WorkflowStep => vec![fact.workflow_step.clone()],
        AnalyticsDimension::ReviewOutcome => vec![fact.review_outcome.clone()],
        AnalyticsDimension::ExternalId => vec![fact.external_id.clone()],
    }
}

fn fact_values(facts: &[AnalyticsFact], dimension: AnalyticsDimension) -> BTreeSet<String> {
    facts
        .iter()
        .flat_map(|fact| values_for_dimension(fact, dimension))
        .collect()
}

fn supported_metrics() -> Vec<AnalyticsMetric> {
    vec![
        AnalyticsMetric::TraceCount,
        AnalyticsMetric::AllowCount,
        AnalyticsMetric::BlockCount,
        AnalyticsMetric::RewriteCount,
        AnalyticsMetric::EscalateCount,
        AnalyticsMetric::InterventionRate,
        AnalyticsMetric::P95LatencyMs,
        AnalyticsMetric::HumanReviewCount,
        AnalyticsMetric::HumanInterventionRate,
        AnalyticsMetric::FalsePositiveRate,
    ]
}

fn supported_dimensions() -> Vec<AnalyticsDimension> {
    vec![
        AnalyticsDimension::AgentId,
        AnalyticsDimension::Environment,
        AnalyticsDimension::RunKind,
        AnalyticsDimension::RunStatus,
        AnalyticsDimension::Decision,
        AnalyticsDimension::PolicyId,
        AnalyticsDimension::WorkflowStep,
        AnalyticsDimension::ReviewOutcome,
        AnalyticsDimension::ExternalId,
    ]
}

fn metric_label(metric: AnalyticsMetric) -> &'static str {
    match metric {
        AnalyticsMetric::TraceCount => "Traces",
        AnalyticsMetric::AllowCount => "Allowed",
        AnalyticsMetric::BlockCount => "Blocked",
        AnalyticsMetric::RewriteCount => "Rewritten",
        AnalyticsMetric::EscalateCount => "Escalated",
        AnalyticsMetric::InterventionRate => "Intervention rate",
        AnalyticsMetric::P95LatencyMs => "p95 latency",
        AnalyticsMetric::HumanReviewCount => "Human reviews",
        AnalyticsMetric::HumanInterventionRate => "Human intervention rate",
        AnalyticsMetric::FalsePositiveRate => "False positive rate",
    }
}

fn dimension_label(dimension: AnalyticsDimension) -> &'static str {
    match dimension {
        AnalyticsDimension::AgentId => "Agent",
        AnalyticsDimension::Environment => "Environment",
        AnalyticsDimension::RunKind => "Run kind",
        AnalyticsDimension::RunStatus => "Run status",
        AnalyticsDimension::Decision => "Verdict",
        AnalyticsDimension::PolicyId => "Policy",
        AnalyticsDimension::WorkflowStep => "Workflow step",
        AnalyticsDimension::ReviewOutcome => "Review outcome",
        AnalyticsDimension::ExternalId => "External id",
    }
}

fn default_chart_type(metric: AnalyticsMetric) -> AnalyticsChartType {
    match metric {
        AnalyticsMetric::TraceCount
        | AnalyticsMetric::AllowCount
        | AnalyticsMetric::BlockCount
        | AnalyticsMetric::RewriteCount
        | AnalyticsMetric::EscalateCount
        | AnalyticsMetric::HumanReviewCount => AnalyticsChartType::Bar,
        AnalyticsMetric::InterventionRate
        | AnalyticsMetric::P95LatencyMs
        | AnalyticsMetric::HumanInterventionRate
        | AnalyticsMetric::FalsePositiveRate => AnalyticsChartType::Line,
    }
}

fn payload_string(payload: &serde_json::Value, key: &str) -> Option<String> {
    payload
        .get(key)
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn policy_ids(payload: &serde_json::Value) -> Vec<String> {
    let ids = payload
        .get("triggered_policies")
        .and_then(serde_json::Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|policy| payload_string(policy, "id"))
        .collect::<Vec<_>>();
    if ids.is_empty() {
        vec!["baseline".to_string()]
    } else {
        ids
    }
}

fn workflow_step(
    event_kind: &str,
    event_label: Option<&str>,
    metadata: &serde_json::Value,
) -> Option<String> {
    metadata
        .get("workflow_step")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .or_else(|| {
            event_label
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_string)
        })
        .or_else(|| {
            let trimmed = event_kind.trim();
            (!trimmed.is_empty()).then(|| trimmed.to_string())
        })
}

fn percentage(numerator: i64, denominator: i64) -> f64 {
    if denominator == 0 {
        0.0
    } else {
        ((numerator as f64 / denominator as f64) * 10_000.0).round() / 100.0
    }
}

fn p95(mut latencies: Vec<i32>) -> Option<i32> {
    if latencies.is_empty() {
        return None;
    }
    latencies.sort_unstable();
    let index = ((latencies.len() as f64) * 0.95).ceil() as usize;
    latencies.get(index.saturating_sub(1)).copied()
}

impl std::fmt::Debug for AnalyticsRepo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AnalyticsRepo").finish_non_exhaustive()
    }
}
