use std::time::Instant;

use axum::{extract::Request, http::HeaderMap, middleware::Next, response::Response};

pub(crate) async fn log_http_response(request: Request, next: Next) -> Response {
    let method = request.method().clone();
    let path = request
        .uri()
        .path_and_query()
        .map(|value| value.as_str().to_string())
        .unwrap_or_else(|| request.uri().path().to_string());
    let content_type = header_value(request.headers(), "content-type");
    let content_length = header_value(request.headers(), "content-length");
    let user_agent = header_value(request.headers(), "user-agent");
    let workspace_id = header_value(request.headers(), "x-featherlane-ai-workspace-id");
    let user_id = header_value(request.headers(), "x-featherlane-ai-user-id");
    let has_authorization = request.headers().contains_key("authorization");

    tracing::debug!(
        method = %method,
        path = %path,
        content_type = content_type.as_deref().unwrap_or(""),
        content_length = content_length.as_deref().unwrap_or(""),
        user_agent = user_agent.as_deref().unwrap_or(""),
        workspace_id = workspace_id.as_deref().unwrap_or(""),
        user_id = user_id.as_deref().unwrap_or(""),
        has_authorization,
        "http request"
    );

    let started = Instant::now();
    let response = next.run(request).await;
    let status = response.status();
    let latency_ms = started.elapsed().as_millis();

    match status.as_u16() {
        500..=599 => tracing::error!(
            method = %method,
            path = %path,
            status = status.as_u16(),
            latency_ms,
            outcome = "error",
            "http response"
        ),
        400..=499 => tracing::warn!(
            method = %method,
            path = %path,
            status = status.as_u16(),
            latency_ms,
            outcome = "warn",
            "http response"
        ),
        _ => tracing::info!(
            method = %method,
            path = %path,
            status = status.as_u16(),
            latency_ms,
            outcome = "ok",
            "http response"
        ),
    }

    response
}

fn header_value(headers: &HeaderMap, name: &'static str) -> Option<String> {
    headers
        .get(name)
        .and_then(|value| value.to_str().ok())
        .map(str::to_string)
}
