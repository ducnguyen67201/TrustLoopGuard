use super::{validation::clean_string, HumanReviewAnalyticsFilter};

pub(super) fn read_filter(query: Option<&str>) -> HumanReviewAnalyticsFilter {
    let mut filter = HumanReviewAnalyticsFilter::default();
    for (key, value) in query_parts(query) {
        match key.as_str() {
            "agent_id" => filter.agent_id = clean_string(&value),
            "policy_id" => filter.policy_id = clean_string(&value),
            "run_kind" => filter.run_kind = clean_string(&value),
            "workflow_step" => filter.workflow_step = clean_string(&value),
            _ => {}
        }
    }
    filter
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
    query.into_iter().flat_map(|query| {
        query.split('&').filter_map(|part| {
            let (key, value) = part.split_once('=')?;
            let key = url::form_urlencoded::parse(key.as_bytes()).next()?.0;
            let value = url::form_urlencoded::parse(value.as_bytes()).next()?.0;
            Some((key.into_owned(), value.into_owned()))
        })
    })
}
