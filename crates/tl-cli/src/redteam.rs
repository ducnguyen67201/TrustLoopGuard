use anyhow::{bail, Context};
use tl_core::{
    RegressionResultSnapshotSummary, RegressionResultStatus, RegressionResultSummaryResponse,
    RegressionResultTrendResponse,
};

use crate::{http, RedteamCmd, RegressionCmd};

pub(super) async fn run_redteam(cmd: RedteamCmd) -> anyhow::Result<()> {
    match cmd {
        RedteamCmd::Regressions { cmd } => run_regressions(cmd).await,
    }
}

async fn run_regressions(cmd: RegressionCmd) -> anyhow::Result<()> {
    match cmd {
        RegressionCmd::Check {
            job_id,
            source_job_id,
            case_keys,
            limit,
            max_failed,
            max_missing,
            max_inconclusive,
            json,
            url,
            api_key,
        } => {
            let summary = fetch_regression_summary(
                &http::server_url(url),
                api_key,
                &job_id,
                &source_job_id,
                &case_keys,
                limit,
            )
            .await?;
            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&summary).context("encode regression summary")?
                );
            } else {
                print_summary(&summary);
            }
            enforce_thresholds(
                &summary,
                RegressionThresholds {
                    max_failed,
                    max_missing,
                    max_inconclusive,
                },
            )
        }
        RegressionCmd::History {
            source_job_id,
            job_id,
            agent_id,
            limit,
            json,
            url,
            api_key,
        } => {
            let history = fetch_regression_history(
                &http::server_url(url),
                api_key,
                source_job_id.as_deref(),
                job_id.as_deref(),
                agent_id.as_deref(),
                limit,
            )
            .await?;
            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&history).context("encode regression history")?
                );
            } else {
                print_history(&history.snapshots);
            }
            Ok(())
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct RegressionThresholds {
    max_failed: u32,
    max_missing: u32,
    max_inconclusive: u32,
}

async fn fetch_regression_summary(
    base_url: &str,
    api_key: Option<String>,
    job_id: &str,
    source_job_id: &str,
    case_keys: &[String],
    limit: Option<usize>,
) -> anyhow::Result<RegressionResultSummaryResponse> {
    let mut url = format!(
        "{}/v1/redteam/regressions/results/{}",
        base_url.trim_end_matches('/'),
        http::urlencode_id(job_id),
    );
    append_query_param(&mut url, "source_job_id", source_job_id);
    for case_key in case_keys {
        append_query_param(&mut url, "case_key", case_key);
    }
    if let Some(limit) = limit {
        append_query_param(&mut url, "limit", &limit.to_string());
    }
    let mut req = reqwest::Client::new().get(url);
    if let Some(key) = http::resolve_api_key(api_key) {
        req = req.bearer_auth(key);
    }
    let response = req.send().await.context("send regression result check")?;
    http::decode_typed_response(response, "regression result summary").await
}

async fn fetch_regression_history(
    base_url: &str,
    api_key: Option<String>,
    source_job_id: Option<&str>,
    job_id: Option<&str>,
    agent_id: Option<&str>,
    limit: Option<usize>,
) -> anyhow::Result<RegressionResultTrendResponse> {
    let mut url = format!(
        "{}/v1/redteam/regressions/results",
        base_url.trim_end_matches('/')
    );
    if let Some(source_job_id) = source_job_id {
        append_query_param(&mut url, "source_job_id", source_job_id);
    }
    if let Some(job_id) = job_id {
        append_query_param(&mut url, "job_id", job_id);
    }
    if let Some(agent_id) = agent_id {
        append_query_param(&mut url, "agent_id", agent_id);
    }
    if let Some(limit) = limit {
        append_query_param(&mut url, "limit", &limit.to_string());
    }
    let mut req = reqwest::Client::new().get(url);
    if let Some(key) = http::resolve_api_key(api_key) {
        req = req.bearer_auth(key);
    }
    let response = req.send().await.context("send regression result history")?;
    http::decode_typed_response(response, "regression result history").await
}

fn append_query_param(url: &mut String, key: &str, value: &str) {
    let sep = if url.contains('?') { '&' } else { '?' };
    url.push(sep);
    url.push_str(key);
    url.push('=');
    url.push_str(&http::urlencode_id(value));
}

fn print_summary(summary: &RegressionResultSummaryResponse) {
    println!(
        "regression job `{}`: {}/{} passed (failed={} missing={} inconclusive={})",
        summary.job.id,
        summary.passed,
        summary.total,
        summary.failed,
        summary.missing,
        summary.inconclusive
    );
    if summary.failed == 0 && summary.missing == 0 && summary.inconclusive == 0 {
        println!("ok: regression check passed");
        return;
    }
    for result in summary
        .results
        .iter()
        .filter(|result| result.status != RegressionResultStatus::Passed)
    {
        let reason = result.reason.as_deref().unwrap_or("no reason provided");
        println!(
            "  - {} [{}]: {reason}",
            result.case_key,
            result_status_label(result.status)
        );
    }
}

fn print_history(snapshots: &[RegressionResultSnapshotSummary]) {
    if snapshots.is_empty() {
        println!("no regression result snapshots found");
        return;
    }
    println!("regression result snapshots:");
    for snapshot in snapshots {
        println!(
            "  - job={} source={} updated={} passed={}/{} failed={} missing={} inconclusive={}",
            snapshot.job_id,
            snapshot.source_job_id,
            snapshot.updated_at,
            snapshot.passed,
            snapshot.total,
            snapshot.failed,
            snapshot.missing,
            snapshot.inconclusive
        );
    }
}

fn enforce_thresholds(
    summary: &RegressionResultSummaryResponse,
    thresholds: RegressionThresholds,
) -> anyhow::Result<()> {
    if summary.failed <= thresholds.max_failed
        && summary.missing <= thresholds.max_missing
        && summary.inconclusive <= thresholds.max_inconclusive
    {
        return Ok(());
    }
    bail!(
        "regression check failed: failed={} (max {}), missing={} (max {}), inconclusive={} (max {})",
        summary.failed,
        thresholds.max_failed,
        summary.missing,
        thresholds.max_missing,
        summary.inconclusive,
        thresholds.max_inconclusive
    )
}

fn result_status_label(status: RegressionResultStatus) -> &'static str {
    match status {
        RegressionResultStatus::Passed => "passed",
        RegressionResultStatus::Failed => "failed",
        RegressionResultStatus::Missing => "missing",
        RegressionResultStatus::Inconclusive => "inconclusive",
    }
}
