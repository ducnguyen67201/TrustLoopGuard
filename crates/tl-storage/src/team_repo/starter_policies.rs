use diesel::prelude::*;
use diesel_async::RunQueryDsl;

use crate::schema::{entity_versions, policies, policy_environment_deployments};
use crate::StorageError;

const STARTER_POLICY_YAMLS: &[&str] = &[
    r#"
id: starter-pii-email
description: Blocks drafts that expose an email address.
match:
  regex: "(?i)[a-z0-9._%+-]+@[a-z0-9.-]+\\.[a-z]{2,}"
action: deny
severity: high
"#,
    r#"
id: starter-pii-phone
description: Blocks drafts that expose a US-style phone number.
match:
  regex: "\\b(?:\\+?1[-.\\s]?)?(?:\\(?\\d{3}\\)?[-.\\s]?)\\d{3}[-.\\s]?\\d{4}\\b"
action: deny
severity: high
"#,
    r#"
id: starter-pii-ssn
description: Blocks drafts that expose a US Social Security number.
match:
  regex: "\\b\\d{3}-\\d{2}-\\d{4}\\b"
action: deny
severity: critical
"#,
    r#"
id: starter-pii-credit-card
description: Blocks drafts that expose a likely payment card number.
match:
  regex: "\\b(?:\\d[ -]*?){13,19}\\b"
action: deny
severity: critical
"#,
    r#"
id: starter-pii-ipv4
description: Blocks drafts that expose an IPv4 address.
match:
  regex: "\\b(?:(?:25[0-5]|2[0-4]\\d|1?\\d?\\d)\\.){3}(?:25[0-5]|2[0-4]\\d|1?\\d?\\d)\\b"
action: deny
severity: medium
"#,
    r#"
id: starter-prompt-injection
description: Escalates drafts that contain common prompt-injection phrases.
match:
  regex: "(?i)(ignore previous instructions|ignore all previous instructions|ignore the above|disregard the above|reveal (the )?(system )?prompt|developer message)"
action: require_approval
severity: high
"#,
];

pub(super) async fn seed_starter_policies(
    conn: &mut crate::postgres::DbConnection<'_>,
    workspace_id: &str,
    environment_id: &str,
) -> Result<(), StorageError> {
    for source_yaml in STARTER_POLICY_YAMLS {
        let policy = tl_policy::load_str(source_yaml)
            .map_err(|e| StorageError::Internal(format!("starter policy parse: {e}")))?;
        let parsed_policy = serde_json::to_value(&policy)
            .map_err(|e| StorageError::Internal(format!("starter policy serialize: {e}")))?;

        diesel::insert_into(policies::table)
            .values((
                policies::workspace_id.eq(workspace_id),
                policies::id.eq(&policy.id),
                policies::policy_yaml.eq(source_yaml),
                policies::parsed_policy.eq(parsed_policy),
                policies::enabled.eq(true),
                policies::owner_agent_id.eq(None::<String>),
            ))
            .on_conflict((policies::workspace_id, policies::id))
            .do_nothing()
            .execute(conn)
            .await
            .map_err(|e| StorageError::Internal(format!("starter policy insert: {e}")))?;

        diesel::insert_into(policy_environment_deployments::table)
            .values((
                policy_environment_deployments::workspace_id.eq(workspace_id),
                policy_environment_deployments::environment_id.eq(environment_id),
                policy_environment_deployments::policy_id.eq(&policy.id),
                policy_environment_deployments::enabled.eq(true),
                policy_environment_deployments::deployed_version.eq(None::<i32>),
            ))
            .on_conflict((
                policy_environment_deployments::workspace_id,
                policy_environment_deployments::environment_id,
                policy_environment_deployments::policy_id,
            ))
            .do_nothing()
            .execute(conn)
            .await
            .map_err(|e| {
                StorageError::Internal(format!("starter policy deployment insert: {e}"))
            })?;

        diesel::insert_into(entity_versions::table)
            .values((
                entity_versions::workspace_id.eq(workspace_id),
                entity_versions::entity_type.eq("policy"),
                entity_versions::entity_id.eq(&policy.id),
                entity_versions::version.eq(1),
                entity_versions::content.eq(source_yaml),
            ))
            .on_conflict((
                entity_versions::workspace_id,
                entity_versions::entity_type,
                entity_versions::entity_id,
                entity_versions::version,
            ))
            .do_nothing()
            .execute(conn)
            .await
            .map_err(|e| StorageError::Internal(format!("starter policy version insert: {e}")))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn starter_policies_are_valid_yaml() {
        let ids: Vec<_> = super::STARTER_POLICY_YAMLS
            .iter()
            .map(|source| tl_policy::load_str(source).expect("starter policy").id)
            .collect();

        assert_eq!(
            ids,
            vec![
                "starter-pii-email",
                "starter-pii-phone",
                "starter-pii-ssn",
                "starter-pii-credit-card",
                "starter-pii-ipv4",
                "starter-prompt-injection",
            ]
        );
    }
}
