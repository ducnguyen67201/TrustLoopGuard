use std::future::Future;

use tracing::instrument;

use crate::{
    AuthorizationDecision, Client, CreateRunEventRequest, CreateRunRequest, GuardEvent, RunDetail,
    RunEventListResponse, RunEventSummary, RunStatus, RunSummary, SdkError, TraceListResponse,
    UpdateRunRequest,
};

#[derive(Clone, Debug)]
pub struct RunClient {
    client: Client,
    run_id: String,
    run_event_id: Option<String>,
}

impl RunClient {
    pub fn run_id(&self) -> &str {
        &self.run_id
    }

    pub fn run_event_id(&self) -> Option<&str> {
        self.run_event_id.as_deref()
    }

    pub async fn submit_event(
        &self,
        event: &GuardEvent,
    ) -> Result<AuthorizationDecision, SdkError> {
        let mut event = event.clone();
        if event.principal.run_id.is_none() {
            event.principal.run_id = Some(self.run_id.clone());
        }
        if event.principal.run_event_id.is_none()
            && event.principal.run_id.as_deref() == Some(self.run_id.as_str())
        {
            event.principal.run_event_id = self.run_event_id.clone();
        }
        self.client.submit_event(&event).await
    }

    pub async fn with_event<T, F, Fut>(
        &self,
        req: CreateRunEventRequest,
        f: F,
    ) -> Result<T, SdkError>
    where
        F: FnOnce(RunClient) -> Fut,
        Fut: Future<Output = Result<T, SdkError>>,
    {
        let event = self.client.create_run_event(&self.run_id, req).await?;
        f(RunClient {
            client: self.client.clone(),
            run_id: self.run_id.clone(),
            run_event_id: Some(event.id),
        })
        .await
    }
}

impl Client {
    pub async fn with_run<T, F, Fut>(&self, req: CreateRunRequest, f: F) -> Result<T, SdkError>
    where
        F: FnOnce(RunClient) -> Fut,
        Fut: Future<Output = Result<T, SdkError>>,
    {
        let run = self.start_run(req).await?;
        let result = f(RunClient {
            client: self.clone(),
            run_id: run.id.clone(),
            run_event_id: None,
        })
        .await;
        let status = if result.is_ok() {
            RunStatus::Completed
        } else {
            RunStatus::Failed
        };
        let finish = self
            .update_run(
                &run.id,
                UpdateRunRequest {
                    status: Some(status),
                    metadata: None,
                    ended_at: None,
                },
            )
            .await;
        match (result, finish) {
            (Ok(value), Ok(_)) => Ok(value),
            (Ok(_), Err(err)) | (Err(err), _) => Err(err),
        }
    }

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
