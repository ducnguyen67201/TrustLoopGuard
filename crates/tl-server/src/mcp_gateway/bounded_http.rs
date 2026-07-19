use std::borrow::Cow;
use std::collections::HashMap;
use std::sync::Arc;

use axum::http::{HeaderName, HeaderValue};
use bytes::{Bytes, BytesMut};
use futures::{stream::BoxStream, StreamExt};
use reqwest::header::{ACCEPT, CONTENT_TYPE};
use rmcp::model::{ClientJsonRpcMessage, ServerJsonRpcMessage};
use rmcp::transport::streamable_http_client::{
    SseError, StreamableHttpClient, StreamableHttpError, StreamableHttpPostResponse,
};
use sse_stream::{Sse, SseStream};

const EVENT_STREAM_MIME_TYPE: &str = "text/event-stream";
const JSON_MIME_TYPE: &str = "application/json";
const HEADER_SESSION_ID: &str = "mcp-session-id";
const HEADER_LAST_EVENT_ID: &str = "last-event-id";

#[derive(Clone)]
pub(super) struct BoundedHttpClient {
    inner: reqwest::Client,
    max_response_bytes: usize,
}

#[derive(Debug, thiserror::Error)]
pub(super) enum BoundedHttpError {
    #[error("HTTP client failed: {0}")]
    Reqwest(#[from] reqwest::Error),
    #[error("upstream response exceeded the configured byte limit")]
    ResponseTooLarge,
    #[error("invalid JSON response: {0}")]
    Json(#[from] serde_json::Error),
}

impl BoundedHttpClient {
    pub(super) fn new(inner: reqwest::Client, max_response_bytes: usize) -> Self {
        Self {
            inner,
            max_response_bytes,
        }
    }

    fn apply_custom_headers(
        &self,
        mut request: reqwest::RequestBuilder,
        custom_headers: HashMap<HeaderName, HeaderValue>,
    ) -> Result<reqwest::RequestBuilder, StreamableHttpError<BoundedHttpError>> {
        for (name, value) in custom_headers {
            if matches!(
                name.as_str(),
                "accept"
                    | "authorization"
                    | "content-type"
                    | HEADER_SESSION_ID
                    | HEADER_LAST_EVENT_ID
            ) {
                return Err(StreamableHttpError::ReservedHeaderConflict(
                    name.to_string(),
                ));
            }
            request = request.header(name, value);
        }
        Ok(request)
    }

    fn bounded_sse(
        &self,
        response: reqwest::Response,
    ) -> BoxStream<'static, Result<Sse, SseError>> {
        let max = self.max_response_bytes;
        let mut received = 0usize;
        let bytes = response.bytes_stream().map(move |chunk| match chunk {
            Ok(chunk) => {
                received = received.saturating_add(chunk.len());
                if received > max {
                    Err(BoundedHttpError::ResponseTooLarge)
                } else {
                    Ok(chunk)
                }
            }
            Err(error) => Err(BoundedHttpError::Reqwest(error)),
        });
        SseStream::from_byte_stream(bytes).boxed()
    }

    async fn bounded_bytes(
        &self,
        response: reqwest::Response,
    ) -> Result<Bytes, StreamableHttpError<BoundedHttpError>> {
        if response
            .content_length()
            .is_some_and(|length| length > self.max_response_bytes as u64)
        {
            return Err(StreamableHttpError::Client(
                BoundedHttpError::ResponseTooLarge,
            ));
        }
        let mut body = BytesMut::new();
        let mut stream = response.bytes_stream();
        while let Some(chunk) = stream.next().await {
            let chunk = chunk
                .map_err(|error| StreamableHttpError::Client(BoundedHttpError::Reqwest(error)))?;
            if body
                .len()
                .checked_add(chunk.len())
                .is_none_or(|length| length > self.max_response_bytes)
            {
                return Err(StreamableHttpError::Client(
                    BoundedHttpError::ResponseTooLarge,
                ));
            }
            body.extend_from_slice(&chunk);
        }
        Ok(body.freeze())
    }

    fn validate_content_type(
        response: &reqwest::Response,
    ) -> Result<Option<String>, StreamableHttpError<BoundedHttpError>> {
        let content_type = response
            .headers()
            .get(CONTENT_TYPE)
            .map(|value| String::from_utf8_lossy(value.as_bytes()).to_string());
        if content_type.as_deref().is_some_and(|value| {
            !value.starts_with(EVENT_STREAM_MIME_TYPE) && !value.starts_with(JSON_MIME_TYPE)
        }) {
            return Err(StreamableHttpError::UnexpectedContentType(content_type));
        }
        Ok(content_type)
    }
}

impl StreamableHttpClient for BoundedHttpClient {
    type Error = BoundedHttpError;

    async fn get_stream(
        &self,
        uri: Arc<str>,
        session_id: Arc<str>,
        last_event_id: Option<String>,
        auth_token: Option<String>,
        custom_headers: HashMap<HeaderName, HeaderValue>,
    ) -> Result<BoxStream<'static, Result<Sse, SseError>>, StreamableHttpError<Self::Error>> {
        let mut request = self
            .inner
            .get(uri.as_ref())
            .header(ACCEPT, [EVENT_STREAM_MIME_TYPE, JSON_MIME_TYPE].join(", "))
            .header(HEADER_SESSION_ID, session_id.as_ref());
        if let Some(last_event_id) = last_event_id {
            request = request.header(HEADER_LAST_EVENT_ID, last_event_id);
        }
        if let Some(auth_token) = auth_token {
            request = request.bearer_auth(auth_token);
        }
        let response = self
            .apply_custom_headers(request, custom_headers)?
            .send()
            .await
            .map_err(|error| StreamableHttpError::Client(BoundedHttpError::Reqwest(error)))?;
        if response.status() == reqwest::StatusCode::METHOD_NOT_ALLOWED {
            return Err(StreamableHttpError::ServerDoesNotSupportSse);
        }
        let response = response
            .error_for_status()
            .map_err(|error| StreamableHttpError::Client(BoundedHttpError::Reqwest(error)))?;
        let content_type = Self::validate_content_type(&response)?;
        if content_type.is_none() {
            return Err(StreamableHttpError::UnexpectedContentType(None));
        }
        Ok(self.bounded_sse(response))
    }

    async fn delete_session(
        &self,
        uri: Arc<str>,
        session_id: Arc<str>,
        auth_token: Option<String>,
        custom_headers: HashMap<HeaderName, HeaderValue>,
    ) -> Result<(), StreamableHttpError<Self::Error>> {
        let mut request = self
            .inner
            .delete(uri.as_ref())
            .header(HEADER_SESSION_ID, session_id.as_ref());
        if let Some(auth_token) = auth_token {
            request = request.bearer_auth(auth_token);
        }
        let response = self
            .apply_custom_headers(request, custom_headers)?
            .send()
            .await
            .map_err(|error| StreamableHttpError::Client(BoundedHttpError::Reqwest(error)))?;
        if response.status() == reqwest::StatusCode::METHOD_NOT_ALLOWED {
            return Ok(());
        }
        response
            .error_for_status()
            .map_err(|error| StreamableHttpError::Client(BoundedHttpError::Reqwest(error)))?;
        Ok(())
    }

    async fn post_message(
        &self,
        uri: Arc<str>,
        message: ClientJsonRpcMessage,
        session_id: Option<Arc<str>>,
        auth_token: Option<String>,
        custom_headers: HashMap<HeaderName, HeaderValue>,
    ) -> Result<StreamableHttpPostResponse, StreamableHttpError<Self::Error>> {
        let mut request = self
            .inner
            .post(uri.as_ref())
            .header(ACCEPT, [EVENT_STREAM_MIME_TYPE, JSON_MIME_TYPE].join(", "));
        if let Some(auth_token) = auth_token {
            request = request.bearer_auth(auth_token);
        }
        let session_was_attached = session_id.is_some();
        if let Some(session_id) = session_id {
            request = request.header(HEADER_SESSION_ID, session_id.as_ref());
        }
        let response = self
            .apply_custom_headers(request, custom_headers)?
            .json(&message)
            .send()
            .await
            .map_err(|error| StreamableHttpError::Client(BoundedHttpError::Reqwest(error)))?;
        let status = response.status();
        if matches!(
            status,
            reqwest::StatusCode::ACCEPTED | reqwest::StatusCode::NO_CONTENT
        ) {
            return Ok(StreamableHttpPostResponse::Accepted);
        }
        if status == reqwest::StatusCode::NOT_FOUND && session_was_attached {
            return Err(StreamableHttpError::SessionExpired);
        }
        if !status.is_success() {
            return Err(StreamableHttpError::UnexpectedServerResponse(Cow::Owned(
                format!("HTTP {status}"),
            )));
        }
        let content_length = response.content_length();
        let content_type = Self::validate_content_type(&response)?;
        let session_id = response
            .headers()
            .get(HEADER_SESSION_ID)
            .and_then(|value| value.to_str().ok())
            .map(str::to_string);
        if content_length == Some(0) {
            return Ok(StreamableHttpPostResponse::Accepted);
        }
        match content_type.as_deref() {
            Some(value) if value.starts_with(EVENT_STREAM_MIME_TYPE) => Ok(
                StreamableHttpPostResponse::Sse(self.bounded_sse(response), session_id),
            ),
            Some(value) if value.starts_with(JSON_MIME_TYPE) => {
                let body = self.bounded_bytes(response).await?;
                let message = serde_json::from_slice::<ServerJsonRpcMessage>(&body)?;
                Ok(StreamableHttpPostResponse::Json(message, session_id))
            }
            _ => Err(StreamableHttpError::UnexpectedContentType(content_type)),
        }
    }
}
