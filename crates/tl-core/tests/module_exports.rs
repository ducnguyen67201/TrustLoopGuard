use tl_core::analytics::AnalyticsMetric;
use tl_core::dashboard::DashboardApiKey;
use tl_core::error::ApiErrorCode;
use tl_core::guard::{CheckRequest, Decision, Verdict};
use tl_core::human_review::HumanReviewOutcome;
use tl_core::knowledge::DashboardKnowledgeSourceKind;
use tl_core::run::RunKind;
use tl_core::trace::TraceListResponse;

#[test]
fn core_contracts_are_available_from_named_modules() {
    let request = CheckRequest::default();
    let decision = Decision::allow("trace-1");

    assert_eq!(request.agent_id, "");
    assert_eq!(decision.verdict, Verdict::Allow);
    assert_eq!(ApiErrorCode::from_http_status(404), ApiErrorCode::NotFound);

    let _: Option<TraceListResponse> = None;
    let _: Option<HumanReviewOutcome> = Some(HumanReviewOutcome::Accepted);
    let _: Option<RunKind> = Some(RunKind::Workflow);
    let _: Option<DashboardApiKey> = None;
    let _: Option<DashboardKnowledgeSourceKind> = Some(DashboardKnowledgeSourceKind::Url);
    let _: Option<AnalyticsMetric> = Some(AnalyticsMetric::TraceCount);
}
