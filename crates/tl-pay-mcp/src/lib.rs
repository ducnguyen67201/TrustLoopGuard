//! TrustLoopGuard payment-gate MCP surface.
//!
//! A thin doorway over the product's gate: agents call `set_policy`, `pay`,
//! `resolve_hold`, and `export_audit`; the durable policy + spend ledger live
//! in `tl-storage` and the decision logic in [`evaluate`]. No separate process,
//! no separate database — this crate is mounted into `tl-server`.
//!
//! Per-owner caps (per-transaction, daily, monthly, hold band) are evaluated
//! here because tool-metadata limits in the engine are per-*tool*, not
//! per-*owner*. The engine stays the source of truth for per-call guardrails;
//! this is the per-owner spend gate that needs durable windowed state.

pub mod backend;
pub mod evaluate;
pub mod tools;

pub use backend::{PayBackend, PayError};

// `transport` (the rmcp mount) is added in tl-server, which owns the
// PayBackend impl over its repos.
