use tracing::{instrument, warn};

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
        let tagged = self.tag_event(event);
        let event = tagged.as_ref().unwrap_or(event);
        self.retry_loop("/v1/events", || self.send_post_json("/v1/events", event))
            .await
    }

    /// Fire-and-forget event capture for monitoring. Spawns a task,
    /// posts the event once (no retries), and logs failures via
    /// `tracing::warn!`. Never blocks and never surfaces an error to
    /// the caller. Must be called from within a tokio runtime.
    ///
    /// When monitoring is enabled the event's `principal.session_id`
    /// is filled in unless the caller already set one.
    pub fn record_event(&self, mut event: GuardEvent) {
        if event.principal.session_id.is_none() {
            event.principal.session_id = self.session_id.clone();
        }
        let client = self.clone();
        tokio::spawn(async move {
            if let Err(err) = client
                .send_post_json::<Decision, GuardEvent>("/v1/events", &event)
                .await
            {
                warn!(
                    operation = %event.action.operation,
                    error = %err,
                    "record_event failed; event dropped",
                );
            }
        });
    }

    /// When monitoring is on and the caller left `session_id` unset,
    /// return a tagged copy of the event. `None` means "send the
    /// original" — monitoring off, or the caller already set a session.
    fn tag_event(&self, event: &GuardEvent) -> Option<GuardEvent> {
        let session_id = self.session_id()?;
        if event.principal.session_id.is_some() {
            return None;
        }
        let mut tagged = event.clone();
        tagged.principal.session_id = Some(session_id.to_string());
        Some(tagged)
    }
}
