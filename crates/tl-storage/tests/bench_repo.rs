#![cfg(feature = "postgres")]

use tl_storage::{BenchRunArmRowInput, BenchRunFilter, BenchRunRepo};

#[test]
fn bench_repo_public_types_compile() {
    let _ = std::mem::size_of::<BenchRunRepo>();
    let filter = BenchRunFilter { limit: 25 };
    assert_eq!(filter.limit, 25);

    let arm = BenchRunArmRowInput {
        arm: "raw".into(),
        label: "raw".into(),
        target: "http://127.0.0.1:9101".into(),
        redteam_job_id: Some("018f0000-0000-7000-8000-000000000000".into()),
        checker_config: Some("off".into()),
    };
    assert_eq!(arm.arm, "raw");
}
