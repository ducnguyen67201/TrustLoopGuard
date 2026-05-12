//! TrustLoopGuard decision cache.
//!
//! Two pieces:
//! - [`MokaCache`] — in-process Moka-backed cache holding `Decision`
//!   keyed by an opaque string. Default TTL 5 min, capacity 10K. Use
//!   [`MokaCache::disabled()`] to opt out.
//! - [`for_check_request`] — derive a stable cache key from a
//!   `CheckRequest` via canonical JSON + BLAKE3.
//!
//! No engine wiring lands in this crate; see `tl-engine` for the
//! lookup-before-orchestrate path.

pub mod key;
pub mod moka;

pub use key::{for_check_request, for_check_request_with_policy_scope};
pub use moka::{MokaCache, DEFAULT_CAPACITY, DEFAULT_TTL};
