use tl_core::{BenchArm, BenchRunCreateRequest, BenchRunStatus};

#[test]
fn bench_wire_types_serialize_as_snake_case_contract() {
    let request = BenchRunCreateRequest {
        raw_target_url: "http://127.0.0.1:9101".into(),
        guarded_target_url: "http://127.0.0.1:9102".into(),
        profile: "fast".into(),
        agent_id: Some("agent-1".into()),
        seed: Some("seed-1".into()),
    };

    let json = serde_json::to_value(request).expect("request serializes");
    assert_eq!(json["raw_target_url"], "http://127.0.0.1:9101");
    assert_eq!(json["guarded_target_url"], "http://127.0.0.1:9102");
    assert!(json.get("generator").is_none());

    assert_eq!(
        serde_json::to_value(BenchArm::Raw).expect("arm serializes"),
        "raw"
    );
    assert_eq!(
        serde_json::to_value(BenchRunStatus::Queued).expect("status serializes"),
        "queued"
    );
}
