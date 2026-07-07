use tl_core::{
    HardenCandidate, HardenCandidateOperation, HardenRejection, HardenRejectionReason,
    HardenRequest, HardenResponse, PolicyDocument, PolicyMatchType, Severity, VerifyResult,
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
        rejections: vec![rejection],
        unreachable: vec!["approval".into()],
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
    assert_eq!(json["rejections"][0]["reason"], "missed_variant");
    assert_eq!(json["rejections"][0]["verify"]["passed"], false);
    assert_eq!(json["unreachable"][0], "approval");
    assert_eq!(json["generated_at"], "2026-06-14T00:00:00Z");
}

#[test]
fn harden_request_persist_defaults_false() {
    let req: HardenRequest =
        serde_json::from_value(serde_json::json!({})).expect("default request");
    assert!(!req.persist);
}
