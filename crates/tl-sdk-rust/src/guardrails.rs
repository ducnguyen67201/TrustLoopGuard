use tracing::instrument;

use crate::{Client, GuardrailGenerateResponse, GuardrailListResponse, SdkError};

impl Client {
    /// Derive a guardrail policy set from an agent's stored
    /// `system_prompt`. The server reads the prompt, calls its
    /// configured LLM, and persists each draft with `enabled=false`,
    /// returning what was saved.
    ///
    /// Callers must have previously registered the agent (including a
    /// non-empty `system_prompt`) via `POST /v1/agents`. The endpoint
    /// returns `404` if the agent is unknown, `422` if `system_prompt`
    /// is absent, and `503` if the deployment has no LLM configured.
    #[instrument(
        name = "tl_sdk_rust::generate_guardrails",
        skip_all,
        fields(agent_id = %agent_id, attempt = tracing::field::Empty),
    )]
    pub async fn generate_guardrails(
        &self,
        agent_id: &str,
    ) -> Result<GuardrailGenerateResponse, SdkError> {
        let path = format!(
            "/v1/agents/{}/guardrails/generate",
            urlencoding::encode(agent_id)
        );
        self.retry_loop(&path, || self.send_post_empty(&path)).await
    }

    /// List guardrail policies owned by an agent. Empty when the agent
    /// has no generated policies or doesn't exist.
    #[instrument(
        name = "tl_sdk_rust::list_guardrails",
        skip_all,
        fields(agent_id = %agent_id, attempt = tracing::field::Empty),
    )]
    pub async fn list_guardrails(&self, agent_id: &str) -> Result<GuardrailListResponse, SdkError> {
        let path = format!("/v1/agents/{}/guardrails", urlencoding::encode(agent_id));
        self.retry_loop(&path, || self.send_get(&path)).await
    }
}
