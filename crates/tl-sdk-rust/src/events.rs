use tracing::instrument;

use crate::{Client, Decision, GuardEvent, SdkError};

impl Client {
    /// Submit a full `GuardEvent` (sources + provenance) for evidence
    /// collection.
    ///
    /// The `checks` and `signals` fields are server-populated;
    /// client-supplied values are ignored. The returned decision starts
    /// as an observe-only `allow` and only changes verdict when an
    /// enforce-mode checker fires.
    #[instrument(
        name = "tl_sdk_rust::submit_event",
        skip_all,
        fields(
            agent_id = %event.principal.agent_id,
            operation = %event.action.operation,
            attempt = tracing::field::Empty,
        ),
    )]
    pub async fn submit_event(&self, event: &GuardEvent) -> Result<Decision, SdkError> {
        self.retry_loop("/v1/events", || self.send_post_json("/v1/events", event))
            .await
    }
}
