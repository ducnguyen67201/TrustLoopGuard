//! Featherlane AI fuzzy similarity primitives.
//!
//! Three building blocks that Tier 2 (PR 6) composes into a real fuzzy
//! check:
//!
//! - [`Embedder`] — text → vector. `MockEmbedder` always available;
//!   `FastEmbedder` (real semantic embeddings via fastembed-rs / BGE-small)
//!   ships behind the `fastembed` feature so unit tests don't need a
//!   100MB model download.
//! - [`HnswIndex`] — labelled-vector store with cosine kNN. Used to ask
//!   "which patterns is this draft semantically near?".
//! - [`fuzzy_contains`] — Levenshtein-based bypass detector for catching
//!   `refund` → `refunddd` / `r3fund` typo-style evasions.
//!
//! No engine wiring lands here — that's PR 6, where Tier 2 boots an
//! embedder + HNSW per tenant and runs both checks on every draft.

pub mod embedder;
pub mod index;
pub mod levenshtein;

pub use embedder::{EmbedError, Embedder, MockEmbedder};
pub use index::{HnswIndex, IndexHit};
pub use levenshtein::{distance, fuzzy_contains};

#[cfg(feature = "fastembed")]
pub use embedder::FastEmbedder;
