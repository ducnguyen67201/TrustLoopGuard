#![cfg(feature = "postgres")]

#[cfg(feature = "postgres-it")]
use testcontainers::runners::AsyncRunner;
#[cfg(feature = "postgres-it")]
use testcontainers_modules::postgres::Postgres as PostgresImage;
#[cfg(feature = "postgres-it")]
use tl_core::{BenchRunCreateRequest, RedteamDispatchRequest};
#[cfg(feature = "postgres-it")]
use tl_storage::{connect_postgres, migrate_postgres, DbPool, RedteamJobRepo};
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

#[cfg(feature = "postgres-it")]
async fn fresh_pool() -> (DbPool, testcontainers::ContainerAsync<PostgresImage>) {
    let container = PostgresImage::default()
        .start()
        .await
        .expect("postgres container");
    let host = container.get_host().await.expect("host");
    let port = container.get_host_port_ipv4(5432).await.expect("port");
    let url = format!("postgres://postgres:postgres@{host}:{port}/postgres");
    migrate_postgres(&url).await.expect("migrate");
    let pool = connect_postgres(&url, 4).await.expect("connect");
    (pool, container)
}

#[cfg(feature = "postgres-it")]
fn bench_request() -> BenchRunCreateRequest {
    BenchRunCreateRequest {
        raw_target_url: "http://127.0.0.1:9101".into(),
        guarded_target_url: "http://127.0.0.1:9102".into(),
        profile: "fast".into(),
        agent_id: Some("agent-1".into()),
        seed: Some("seed-1".into()),
    }
}

#[cfg(feature = "postgres-it")]
fn redteam_request(target_url: &str) -> RedteamDispatchRequest {
    RedteamDispatchRequest {
        target_url: target_url.into(),
        profile: "fast".into(),
        mode: Default::default(),
        agent_id: Some("agent-1".into()),
    }
}

#[cfg(feature = "postgres-it")]
#[tokio::test]
async fn bench_repo_create_attach_list_and_detail_round_trip() {
    let (pool, _container) = fresh_pool().await;
    let bench_repo = BenchRunRepo::new(pool.clone());
    let redteam_repo = RedteamJobRepo::new(pool);

    let run = bench_repo
        .create("ws_test", "production", &bench_request())
        .await
        .expect("create bench run");
    let raw_job = redteam_repo
        .create(
            "ws_test",
            "production",
            &redteam_request("http://127.0.0.1:9101"),
        )
        .await
        .expect("create raw job");
    let guarded_job = redteam_repo
        .create(
            "ws_test",
            "production",
            &redteam_request("http://127.0.0.1:9102"),
        )
        .await
        .expect("create guarded job");

    bench_repo
        .attach_arm(
            "ws_test",
            &run.id,
            BenchRunArmRowInput {
                arm: "raw".into(),
                label: "raw".into(),
                target: "http://127.0.0.1:9101".into(),
                redteam_job_id: Some(raw_job.id.clone()),
                checker_config: Some("off".into()),
            },
        )
        .await
        .expect("attach raw arm");
    bench_repo
        .attach_arm(
            "ws_test",
            &run.id,
            BenchRunArmRowInput {
                arm: "guarded".into(),
                label: "guarded".into(),
                target: "http://127.0.0.1:9102".into(),
                redteam_job_id: Some(guarded_job.id.clone()),
                checker_config: Some("enforce".into()),
            },
        )
        .await
        .expect("attach guarded arm");

    let runs = bench_repo
        .list("ws_test", BenchRunFilter { limit: 10 })
        .await
        .expect("list runs");
    assert_eq!(runs.len(), 1);
    assert_eq!(runs[0].id, run.id);

    let detail = bench_repo
        .get_detail("ws_test", &run.id)
        .await
        .expect("get detail");
    assert_eq!(detail.arms.len(), 2);
    assert!(detail
        .arms
        .iter()
        .any(|arm| arm.arm == tl_core::BenchArm::Raw
            && arm.redteam_job_id.as_deref() == Some(raw_job.id.as_str())));
    assert!(detail
        .arms
        .iter()
        .any(|arm| arm.arm == tl_core::BenchArm::Guarded
            && arm.redteam_job_id.as_deref() == Some(guarded_job.id.as_str())));

    let invalid_arm = bench_repo
        .attach_arm(
            "ws_test",
            &run.id,
            BenchRunArmRowInput {
                arm: "shadow".into(),
                label: "shadow".into(),
                target: "http://127.0.0.1:9103".into(),
                redteam_job_id: Some(raw_job.id),
                checker_config: None,
            },
        )
        .await;
    assert!(
        invalid_arm.is_err(),
        "arm CHECK constraint should reject drift"
    );
}
