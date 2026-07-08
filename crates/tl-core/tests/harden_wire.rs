use tl_core::{
    ApprovalRule, EventVerifyResult, HardenCandidate, HardenCandidateOperation,
    HardenEventCandidate, HardenRejection, HardenRejectionReason, HardenRequest, HardenResponse,
    PolicyDocument, PolicyMatchType, RegressionCaseSource, RegressionCaseSummary,
    RegressionExpectedOutcome, Severity, SideEffectClass, ToolMetadata, VerifyResult,
};

#[test]
fn policy_match_type_semantic_serializes_lowercase() {
    assert_eq!(
        serde_json::to_value(PolicyMatchType::Semantic).expect("match type serializes"),
        "semantic"
    );
}

#[test]
fn harden_wire_types_serialize_as_snake_case_contract() {
    let candidate = HardenCandidate {
        policy: PolicyDocument {
            id: "block-refund-bypass".into(),
            family: tl_core::PolicyFamily::Content,
            description: Some("Blocks unreviewed refund approvals".into()),
            severity: Severity::High,
            enabled: false,
            source_yaml: "id: block-refund-bypass\n".into(),
        },
        operation: HardenCandidateOperation::Tighten,
        existing_policy_id: Some("block-refund-bypass".into()),
        substrate: "semantic_output".into(),
        evidence_seqs: vec![2],
        source: "deterministic".into(),
        verify: VerifyResult {
            blocked_landed: 1,
            landed_total: 1,
            blocked_variants: 9,
            variant_total: 10,
            false_blocks: 0,
            control_total: 1,
            passed: true,
        },
    };
    let rejection = HardenRejection {
        reason: HardenRejectionReason::MissedVariant,
        substrate: "semantic_output".into(),
        evidence_seqs: vec![4],
        verify: Some(VerifyResult {
            blocked_landed: 1,
            landed_total: 1,
            blocked_variants: 1,
            variant_total: 2,
            false_blocks: 0,
            control_total: 0,
            passed: false,
        }),
        message: "candidate missed a reworded landed reply".into(),
    };
    let response = HardenResponse {
        candidates: vec![candidate],
        event_candidates: vec![HardenEventCandidate {
            tool_metadata: ToolMetadata {
                tool: "issue_refund".into(),
                side_effect: SideEffectClass::ApiMutation,
                reversible: false,
                params: vec![],
                approval: Some(ApprovalRule {
                    required: true,
                    approver_roles: vec!["admin".into()],
                    reason: Some("Refunds require approval.".into()),
                }),
                sandbox_hint: None,
            },
            operation: HardenCandidateOperation::Create,
            existing_tool_id: None,
            substrate: "approval".into(),
            evidence_seqs: vec![2],
            source: "deterministic".into(),
            verify: EventVerifyResult {
                escalated_landed: 1,
                landed_total: 1,
                false_blocks: 0,
                control_total: 1,
                passed: true,
            },
        }],
        label_policy_candidates: vec![],
        rejections: vec![rejection],
        unreachable: vec!["approval".into()],
        regression_cases: vec![RegressionCaseSummary {
            id: "case-1".into(),
            case_key: "harden:job-1:semantic_output:block-refund-bypass:2".into(),
            environment_id: "production".into(),
            agent_id: Some("agent-1".into()),
            source: RegressionCaseSource::Harden,
            source_job_id: Some("job-1".into()),
            source_session_seqs: vec![2],
            substrate: "semantic_output".into(),
            artifact_id: "block-refund-bypass".into(),
            expected_outcome: RegressionExpectedOutcome::Block,
            attack: "refund bypass".into(),
            goal: "approve refund without review".into(),
            created_at: "2026-06-14T00:00:00Z".into(),
            updated_at: "2026-06-14T00:00:00Z".into(),
        }],
        generated_at: "2026-06-14T00:00:00Z".into(),
    };

    let json = serde_json::to_value(&response).expect("response serializes");
    assert_eq!(json["candidates"][0]["substrate"], "semantic_output");
    assert_eq!(json["candidates"][0]["policy"]["enabled"], false);
    assert_eq!(json["candidates"][0]["policy"]["id"], "block-refund-bypass");
    assert_eq!(json["candidates"][0]["operation"], "tighten");
    assert_eq!(
        json["candidates"][0]["existing_policy_id"],
        "block-refund-bypass"
    );
    assert_eq!(json["candidates"][0]["evidence_seqs"][0], 2);
    assert_eq!(json["candidates"][0]["source"], "deterministic");
    assert_eq!(json["candidates"][0]["verify"]["blocked_landed"], 1);
    assert_eq!(json["candidates"][0]["verify"]["variant_total"], 10);
    assert_eq!(json["candidates"][0]["verify"]["passed"], true);
    assert_eq!(json["event_candidates"][0]["substrate"], "approval");
    assert_eq!(
        json["event_candidates"][0]["tool_metadata"]["tool"],
        "issue_refund"
    );
    assert_eq!(
        json["event_candidates"][0]["tool_metadata"]["approval"]["required"],
        true
    );
    assert_eq!(json["event_candidates"][0]["verify"]["escalated_landed"], 1);
    assert_eq!(json["rejections"][0]["reason"], "missed_variant");
    assert_eq!(json["rejections"][0]["verify"]["passed"], false);
    assert_eq!(json["unreachable"][0], "approval");
    assert_eq!(json["regression_cases"][0]["source"], "harden");
    assert_eq!(json["regression_cases"][0]["expected_outcome"], "block");
    assert_eq!(json["generated_at"], "2026-06-14T00:00:00Z");
}

#[test]
fn harden_request_persist_defaults_false() {
    let req: HardenRequest =
        serde_json::from_value(serde_json::json!({})).expect("default request");
    assert!(!req.persist);
    assert!(!req.promote_regression);
}
