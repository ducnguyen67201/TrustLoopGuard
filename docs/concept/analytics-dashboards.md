# Analytics Dashboards

Analytics dashboards let workspace users choose the guardrail data they want to monitor without making the web app a second backend.

## Ownership

Rust owns analytics data, query semantics, and saved dashboard views.

```text
Browser analytics UI
  -> Next.js same-origin proxy
  -> Rust /v1/analytics/*
  -> tl-storage traces, runs, human_review_events, analytics_dashboard_views
```

The dashboard may render filters, widget controls, and charts. It must not persist saved analytics views or compute durable analytics semantics in a web-owned database.

## Access

Analytics endpoints are workspace-scoped. User-session calls use the Rust `UserContext`; internal-service calls from the web dashboard's OAuth fallback must forward `X-TLG-User-Id`. Rust verifies that an ordinary user is a member of the requested `X-TLG-Workspace-Id` before returning analytics data or saved views. A [platform administrator](web-dashboard-authentication.md) may access any active workspace through the same Rust authorization gate without receiving an inserted membership row. Workspace runtime keys remain forbidden from dashboard analytics endpoints.

## Template Variables

Dashboard filters follow the same product idea as Datadog template variables: choose a workspace-level variable, then apply it to every widget query.

Supported filter dimensions:

- `agent_id`
- `environment`
- `run_kind`
- `run_status`
- `decision`
- `policy_id`
- `workflow_step`
- `review_outcome`
- `external_id`

`GET /v1/analytics/catalog` returns supported metrics, dimensions, chart types, and current facet values for the workspace. Environment is a first-class dimension so dashboards can compare or filter dev, staging, and production traffic.

## Queries

`POST /v1/analytics/query` accepts one metric, optional `group_by`, filters, and a result limit. Rust computes the result from persisted traces, linked runs, run events, and latest human review outcomes.

Supported metrics:

- `trace_count`
- `allow_count`
- `block_count`
- `rewrite_count`
- `escalate_count`
- `intervention_rate`
- `p95_latency_ms`
- `human_review_count`
- `human_intervention_rate`
- `false_positive_rate`

## Widget Layout

Dashboard widget order and grid placement live in the Rust-owned saved view config. Each widget stores a 12-column grid layout with `x`, `y`, `w`, and `h` values.

The web UI renders the saved layout as a responsive grid. Users can reorder widgets and choose width/height presets; saving the view persists those layout values back through the Rust analytics API. Because the persisted contract already carries grid coordinates and dimensions, a later direct drag-resize editor can update the same fields without changing the dashboard storage shape.

## Saved Views

Saved dashboard views are workspace-scoped records in `analytics_dashboard_views`. Each view stores a name, default flag, and typed config containing filters and widgets.

Routes:

- `GET /v1/analytics/views`
- `POST /v1/analytics/views`
- `PATCH /v1/analytics/views/{id}`
- `DELETE /v1/analytics/views/{id}`

Only one view can be default per workspace. Views are shared by the workspace, not private per user.
