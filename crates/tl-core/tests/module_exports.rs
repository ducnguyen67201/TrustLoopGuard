use tl_core::analytics::AnalyticsMetric;
use tl_core::dashboard::DashboardApiKey;
use tl_core::error::ApiErrorCode;
use tl_core::event::{EventKind, GuardEvent};
use tl_core::guard::{CheckRequest, Decision};
use tl_core::human_review::HumanReviewOutcome;
use tl_core::knowledge::DashboardKnowledgeSourceKind;
use tl_core::label::{Labels, Origin};
use tl_core::provenance::ProvenanceMap;
use tl_core::run::RunKind;
use tl_core::tool::{ParamRole, ToolMetadata};
use tl_core::trace::TraceListResponse;
use tl_core::AuthorizationEffect;

#[test]
fn core_contracts_are_available_from_named_modules() {
    let request = CheckRequest::default();
    let decision = Decision::allow("trace-1");

    assert_eq!(request.agent_id, "");
    assert_eq!(decision.effect, AuthorizationEffect::Permit);
    assert_eq!(ApiErrorCode::from_http_status(404), ApiErrorCode::NotFound);

    let _: Option<TraceListResponse> = None;
    let _: Option<HumanReviewOutcome> = Some(HumanReviewOutcome::Accepted);
    let _: Option<RunKind> = Some(RunKind::Workflow);
    let _: Option<DashboardApiKey> = None;
    let _: Option<DashboardKnowledgeSourceKind> = Some(DashboardKnowledgeSourceKind::Url);
    let _: Option<AnalyticsMetric> = Some(AnalyticsMetric::TraceCount);
    let _: Option<GuardEvent> = None;
    let _: Option<EventKind> = Some(EventKind::OutputProposed);
    let _: Option<Labels> = Some(Labels::default());
    let _: Option<Origin> = Some(Origin::Unknown);
    let _: Option<ProvenanceMap> = Some(ProvenanceMap::default());
    let _: Option<ToolMetadata> = None;
    let _: Option<ParamRole> = Some(ParamRole::AuthorityBearing);
}
