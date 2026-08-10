use std::collections::BTreeMap;
use std::future::Future;
use std::pin::Pin;

use tracing::instrument;

use crate::{
    AuthorizationDecision, Client, CreateRunEventRequest, CreateRunRequest, FinalizeRunRequest,
    FinalizeRunResponse, GuardEvent, RunBoundarySource, RunDetail, RunEventListResponse,
    RunEventSummary, RunStatus, RunSummary, SdkError, TraceListResponse, UpdateRunRequest,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RunCorrelationContext {
    pub run_id: String,
    pub agent_id: String,
    pub run_event_id: Option<String>,
    pub attributes: BTreeMap<String, String>,
    pub headers: BTreeMap<String, String>,
}

impl RunCorrelationContext {
    pub fn new(run_id: impl Into<String>, agent_id: impl Into<String>) -> Self {
        let run_id = run_id.into();
        let agent_id = agent_id.into();
        let mut attributes = BTreeMap::new();
        attributes.insert("featherlane.run.id".into(), run_id.clone());
        attributes.insert("featherlane.agent.id".into(), agent_id.clone());
        let mut headers = BTreeMap::new();
        headers.insert("x-featherlane-run-id".into(), run_id.clone());
        headers.insert("x-featherlane-agent-id".into(), agent_id.clone());
        Self {
            run_id,
            agent_id,
            run_event_id: None,
            attributes,
            headers,
        }
    }

    fn with_flush_id(&self, flush_id: &str) -> Self {
        let mut context = self.clone();
        context
            .attributes
            .insert("featherlane.flush.id".into(), flush_id.into());
        context
            .headers
            .insert("x-featherlane-flush-id".into(), flush_id.into());
        context
    }
}

pub trait RunTelemetryHook: Send + Sync {
    fn bind_run(&self, _context: &RunCorrelationContext) {}

    fn force_flush<'a>(
        &'a self,
        context: &'a RunCorrelationContext,
    ) -> Pin<Box<dyn Future<Output = Result<(), String>> + Send + 'a>>;
}

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
        let finish = self.finish_run_with_status(&run.id, status).await;
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
        let agent_id = req.agent_id.clone();
        let run: RunSummary = self
            .retry_loop("/v1/runs", || self.send_post_json("/v1/runs", &req))
            .await?;
        let correlation = RunCorrelationContext::new(&run.id, agent_id);
        if let Some(hook) = self.run_telemetry.as_ref() {
            hook.bind_run(&correlation);
        }
        self.run_correlations
            .lock()
            .expect("run correlation lock")
            .insert(run.id.clone(), correlation);
        Ok(run)
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
        self.finish_run_with_status(run_id, RunStatus::Completed)
            .await
    }

    pub async fn finish_run_with_status(
        &self,
        run_id: &str,
        status: RunStatus,
    ) -> Result<RunSummary, SdkError> {
        let correlation = self
            .run_correlations
            .lock()
            .expect("run correlation lock")
            .get(run_id)
            .cloned();
        let mut expected_flush_id = None;
        if let (Some(hook), Some(correlation)) = (self.run_telemetry.as_ref(), correlation) {
            let flush_id = uuid::Uuid::now_v7().to_string();
            let flush_context = correlation.with_flush_id(&flush_id);
            match tokio::time::timeout(
                self.telemetry_flush_timeout,
                hook.force_flush(&flush_context),
            )
            .await
            {
                Ok(Ok(())) => expected_flush_id = Some(flush_id),
                Ok(Err(error)) => tracing::warn!(run_id, error, "run telemetry flush failed"),
                Err(_) => tracing::warn!(run_id, "run telemetry flush timed out"),
            }
        }
        let request = FinalizeRunRequest {
            status,
            ended_at: None,
            boundary_source: RunBoundarySource::ExplicitSdk,
            expected_flush_id,
            last_event_sequence: None,
        };
        let path = format!("/v1/runs/{}/finalize", urlencoding::encode(run_id));
        let response: FinalizeRunResponse = self
            .retry_loop(&path, || self.send_post_json(&path, &request))
            .await?;
        self.run_correlations
            .lock()
            .expect("run correlation lock")
            .remove(run_id);
        Ok(response.run)
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
