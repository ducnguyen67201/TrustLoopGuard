use tl_core::{RunKind, RunStatus};

use super::validation::clean_optional;
use super::{RunListFilter, RunStoreError};

pub(super) fn read_filter(query: Option<&str>) -> Result<RunListFilter, RunStoreError> {
    let mut filter = RunListFilter {
        limit: 20,
        ..RunListFilter::default()
    };
    for (key, value) in query_parts(query) {
        match key.as_str() {
            "agent_id" => filter.agent_id = clean_optional(Some(value)),
            "external_id" => filter.external_id = clean_optional(Some(value)),
            "status" => filter.status = Some(parse_status(&value)?),
            "kind" => filter.kind = Some(parse_kind(&value)?),
            "limit" => {
                filter.limit = value.parse().unwrap_or(20);
            }
            _ => {}
        }
    }
    Ok(filter)
}

pub(super) fn read_limit(query: Option<&str>) -> Option<usize> {
    query_parts(query).find_map(|(key, value)| {
        if key == "limit" {
            value.parse().ok()
        } else {
            None
        }
    })
}

fn query_parts(query: Option<&str>) -> impl Iterator<Item = (String, String)> + '_ {
    query
        .into_iter()
        .flat_map(|query| url::form_urlencoded::parse(query.as_bytes()).into_owned())
}

fn parse_kind(value: &str) -> Result<RunKind, RunStoreError> {
    match value {
        "chat_session" => Ok(RunKind::ChatSession),
        "live_call" => Ok(RunKind::LiveCall),
        "workflow" => Ok(RunKind::Workflow),
        "job" => Ok(RunKind::Job),
        "other" => Ok(RunKind::Other),
        other => Err(RunStoreError::Validation(format!(
            "unknown run kind: {other}"
        ))),
    }
}

fn parse_status(value: &str) -> Result<RunStatus, RunStoreError> {
    match value {
        "warming" => Ok(RunStatus::Warming),
        "running" => Ok(RunStatus::Running),
        "completed" => Ok(RunStatus::Completed),
        "failed" => Ok(RunStatus::Failed),
        "canceled" => Ok(RunStatus::Canceled),
        other => Err(RunStoreError::Validation(format!(
            "unknown run status: {other}"
        ))),
    }
}
