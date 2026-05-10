//! Universal baseline detectors — guardrails every tenant gets for free.
//!
//! See `docs/concept/v0-design-decisions.md §5` for the four-source layering.
//! These are Layer 1 (universal). They run inside Tier 1 alongside tenant
//! policy matchers.
//!
//! Two detector families ship in v0:
//! - `pii` — email, US SSN, US phone, IPv4, credit card (Luhn-validated).
//!   Inspects `proposed_output` (we don't want the agent to leak PII).
//! - `prompt_injection` — common jailbreak / role-override patterns.
//!   Inspects `input` (that's where injection arrives).
//!
//! Generic banned-phrase lists are intentionally NOT shipped in source.
//! Subjective content is the tenant's call; let them author that in
//! `policies/*.yaml`.

use tl_core::{CheckRequest, Severity, Verdict};

pub mod pii;
pub mod prompt_injection;

/// One detector firing. Tier 1 converts these into `TriggeredPolicy`
/// entries and (for the first non-Allow hit) a `BlockSignal`.
#[derive(Debug, Clone)]
pub struct UniversalHit {
    pub id: String,
    pub severity: Severity,
    pub message: String,
    pub verdict: Verdict,
}

/// Run every universal detector against the request. Detectors run in a
/// fixed order so reasons are deterministic across calls.
pub fn detect_all(req: &CheckRequest) -> Vec<UniversalHit> {
    let mut hits = pii::detect(&req.proposed_output);
    hits.extend(prompt_injection::detect(&req.input));
    hits
}
