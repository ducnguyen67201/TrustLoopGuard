use tracing::instrument;

use crate::{
    Client, CreateRunEventRequest, CreateRunRequest, RunDetail, RunEventListResponse,
    RunEventSummary, RunStatus, RunSummary, SdkError, TraceListResponse, UpdateRunRequest,
};

impl Client {
    /// Create a run that groups related guardrail checks.
    #[instrument(
        name = "tl_sdk_rust::start_run",
        skip_all,
        fields(agent_id = %req.agent_id, attempt = tracing::field::Empty),
    )]
    pub async fn start_run(&self, req: CreateRunRequest) -> Result<RunSummary, SdkError> {
        self.retry_loop("/v1/runs", || self.send_post_json("/v1/runs", &req))
            .await
    }

    /// Fetch a run with recent events and traces.
    #[instrument(
        name = "tl_sdk_rust::get_run",
        skip_all,
        fields(run_id = %run_id, attempt = tracing::field::Empty),
    )]
    pub async fn get_run(&self, run_id: &str) -> Result<RunDetail, SdkError> {
        let path = format!("/v1/runs/{}", urlencoding::encode(run_id));
        self.retry_loop(&path, || self.send_get(&path)).await
    }

    /// Update run status or metadata.
    #[instrument(
        name = "tl_sdk_rust::update_run",
        skip_all,
        fields(run_id = %run_id, attempt = tracing::field::Empty),
    )]
    pub async fn update_run(
        &self,
        run_id: &str,
        req: UpdateRunRequest,
    ) -> Result<RunSummary, SdkError> {
        let path = format!("/v1/runs/{}", urlencoding::encode(run_id));
        self.retry_loop(&path, || self.send_patch_json(&path, &req))
            .await
    }

    /// Mark a run completed.
    #[instrument(
        name = "tl_sdk_rust::finish_run",
        skip_all,
        fields(run_id = %run_id, attempt = tracing::field::Empty),
    )]
    pub async fn finish_run(&self, run_id: &str) -> Result<RunSummary, SdkError> {
        self.update_run(
            run_id,
            UpdateRunRequest {
                status: Some(RunStatus::Completed),
                metadata: None,
                ended_at: None,
            },
        )
        .await
    }

    /// Append an event to a run timeline.
    #[instrument(
        name = "tl_sdk_rust::create_run_event",
        skip_all,
        fields(run_id = %run_id, attempt = tracing::field::Empty),
    )]
    pub async fn create_run_event(
        &self,
        run_id: &str,
        req: CreateRunEventRequest,
    ) -> Result<RunEventSummary, SdkError> {
        let path = format!("/v1/runs/{}/events", urlencoding::encode(run_id));
        self.retry_loop(&path, || self.send_post_json(&path, &req))
            .await
    }

    /// List run timeline events.
    #[instrument(
        name = "tl_sdk_rust::list_run_events",
        skip_all,
        fields(run_id = %run_id, attempt = tracing::field::Empty),
    )]
    pub async fn list_run_events(&self, run_id: &str) -> Result<RunEventListResponse, SdkError> {
        let path = format!("/v1/runs/{}/events", urlencoding::encode(run_id));
        self.retry_loop(&path, || self.send_get(&path)).await
    }

    /// List traces grouped under a run.
    #[instrument(
        name = "tl_sdk_rust::list_run_traces",
        skip_all,
        fields(run_id = %run_id, attempt = tracing::field::Empty),
    )]
    pub async fn list_run_traces(&self, run_id: &str) -> Result<TraceListResponse, SdkError> {
        let path = format!("/v1/runs/{}/traces", urlencoding::encode(run_id));
        self.retry_loop(&path, || self.send_get(&path)).await
    }
}
