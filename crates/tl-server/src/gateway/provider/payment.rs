//! Generic HTTP payment forward — the "act" half of the pay gate.
//!
//! Deliberately NOT a [`super::GatewayProvider`] impl: that trait is
//! chat-shaped (streaming, output extraction, rewrites, safe responses). A
//! payment forward is one POST with the vaulted credential injected and an
//! idempotency key so a retried forward can never double-charge on providers
//! that honor the header.
//
// ponytail: one generic bearer+JSON forward covers demos and any provider
// with a compatible endpoint; add PSP-specific request shaping (Stripe form
// encoding etc.) as separate adapters when a real provider is chosen.

use reqwest::header;
use serde_json::Value;
use tl_core::GatewayProviderConnection;

use super::{provider_json_response, provider_url};

/// Forward an approved payment to the connection's endpoint.
///
/// `idempotency_key` must be the decision's trace id: the same decision can
/// then never execute twice, no matter how the call is retried.
pub(crate) async fn forward_payment(
    http: &reqwest::Client,
    connection: &GatewayProviderConnection,
    api_key: &str,
    idempotency_key: &str,
    body: &Value,
) -> Result<Value, String> {
    // Validated at connection-create for payment_http; fail closed rather
    // than invent a host if an old row slips through.
    if connection.base_url.is_none() {
        return Err("payment connection has no base_url".to_string());
    }
    let url = provider_url(connection, "", "/payments");
    let response = http
        .post(url)
        .header(header::AUTHORIZATION, format!("Bearer {api_key}"))
        .header("Idempotency-Key", idempotency_key)
        .header(header::CONTENT_TYPE, "application/json")
        .json(body)
        .send()
        .await
        .map_err(|e| format!("payment provider request failed: {e}"))?;
    provider_json_response(response).await
}
