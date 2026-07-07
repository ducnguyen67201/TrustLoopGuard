use tracing::instrument;

use crate::{Client, PolicyDocument, PolicyFamily, PolicyListResponse, SdkError};

impl Client {
    /// List policies visible to the authenticated workspace.
    #[instrument(
        name = "tl_sdk_rust::list_policies",
        skip_all,
        fields(family = ?family, attempt = tracing::field::Empty),
    )]
    pub async fn list_policies(
        &self,
        family: Option<PolicyFamily>,
    ) -> Result<PolicyListResponse, SdkError> {
        let path = match family {
            Some(family) => format!("/v1/policies?family={}", family.as_str()),
            None => "/v1/policies".to_string(),
        };
        self.retry_loop(&path, || self.send_get(&path)).await
    }

    /// Fetch one policy document by id.
    #[instrument(
        name = "tl_sdk_rust::get_policy",
        skip_all,
        fields(policy_id = %policy_id, attempt = tracing::field::Empty),
    )]
    pub async fn get_policy(&self, policy_id: &str) -> Result<PolicyDocument, SdkError> {
        let path = format!("/v1/policies/{}", urlencoding::encode(policy_id));
        self.retry_loop(&path, || self.send_get(&path)).await
    }

    /// Create or update a policy from YAML.
    #[instrument(
        name = "tl_sdk_rust::upsert_policy",
        skip_all,
        fields(attempt = tracing::field::Empty),
    )]
    pub async fn upsert_policy(&self, source_yaml: &str) -> Result<PolicyDocument, SdkError> {
        self.retry_loop("/v1/policies", || {
            self.send_post_text("/v1/policies", source_yaml, "application/yaml")
        })
        .await
    }
}
