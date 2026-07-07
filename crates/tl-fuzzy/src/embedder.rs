//! Text embedder abstraction.
//!
//! `Embedder` is the trait every backend implements. v0 ships:
//! - `MockEmbedder` — deterministic word-bag-style vectors. No network,
//!   no model download. Used by HNSW unit tests and as a placeholder in
//!   environments where `fastembed` is disabled.
//! - `FastEmbedder` (`fastembed` feature) — real semantic embeddings via
//!   `fastembed-rs` running BGE-small (384-dim) on ONNX. Downloads the
//!   model on first use; cached locally afterwards.

use async_trait::async_trait;

#[derive(Debug, thiserror::Error)]
pub enum EmbedError {
    #[error("embedder init failed: {0}")]
    Init(String),
    #[error("embed call failed: {0}")]
    Embed(String),
}

/// Implementations must be deterministic for a given input — the HNSW
/// index assumes that re-embedding the same text yields a vector that
/// satisfies the configured distance threshold against the stored copy.
#[async_trait]
pub trait Embedder: Send + Sync {
    /// Embed a batch of texts. Returns one vector per input, in order.
    async fn embed(&self, texts: &[String]) -> Result<Vec<Vec<f32>>, EmbedError>;
    /// Dimensionality of the produced vectors. Must be constant across calls.
    fn dimension(&self) -> usize;
}

// -------- MockEmbedder --------

/// Deterministic embedder used in HNSW unit tests and any environment
/// without `fastembed`. It is **not** a semantic model — vectors are
/// produced by hashing tokens into a fixed-size float array. Texts that
/// share many tokens score high; otherwise low. That's enough to verify
/// HNSW behaviour without pulling in 100MB of ONNX weights.
pub struct MockEmbedder {
    dim: usize,
}

impl MockEmbedder {
    pub fn new(dim: usize) -> Self {
        assert!(dim >= 32, "dim must be >= 32 for hash distribution");
        Self { dim }
    }
}

impl Default for MockEmbedder {
    fn default() -> Self {
        Self::new(64)
    }
}

#[async_trait]
impl Embedder for MockEmbedder {
    async fn embed(&self, texts: &[String]) -> Result<Vec<Vec<f32>>, EmbedError> {
        Ok(texts.iter().map(|t| word_bag_embed(t, self.dim)).collect())
    }
    fn dimension(&self) -> usize {
        self.dim
    }
}

/// Word-bag embedding: lowercase, split on non-alphanumeric, hash each
/// distinct token into a bucket, then L2-normalise. Bigram-padded so a
/// shared substring of length >= 2 still contributes to similarity.
pub(crate) fn word_bag_embed(text: &str, dim: usize) -> Vec<f32> {
    let mut v = vec![0f32; dim];
    let lower = text.to_ascii_lowercase();
    let mut tokens: Vec<&str> = lower
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|s| !s.is_empty())
        .collect();
    tokens.sort();
    tokens.dedup();
    for tok in &tokens {
        let h = fnv1a(tok.as_bytes()) as usize;
        v[h % dim] += 1.0;
        // Add bigrams so "refund" and "refunddd" share weight.
        for w in tok.as_bytes().windows(2) {
            let bh = fnv1a(w) as usize;
            v[bh % dim] += 0.5;
        }
    }
    let norm = v.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm > 0.0 {
        for x in &mut v {
            *x /= norm;
        }
    }
    v
}

/// FNV-1a — fast, low-quality hash, fine for a non-cryptographic bucketer.
fn fnv1a(bytes: &[u8]) -> u32 {
    let mut h: u32 = 0x811c9dc5;
    for &b in bytes {
        h ^= b as u32;
        h = h.wrapping_mul(0x01000193);
    }
    h
}

// -------- FastEmbedder --------

#[cfg(feature = "fastembed")]
pub use real::FastEmbedder;

#[cfg(feature = "fastembed")]
mod real {
    use super::{EmbedError, Embedder};
    use async_trait::async_trait;
    use fastembed::{EmbeddingModel, InitOptions, TextEmbedding};
    use std::sync::Mutex;

    pub struct FastEmbedder {
        inner: Mutex<TextEmbedding>,
        dim: usize,
    }

    impl FastEmbedder {
        /// Initialise BGE-small (384-dim). Downloads the model from
        /// HuggingFace on first call; cached locally afterwards.
        pub fn new() -> Result<Self, EmbedError> {
            let inner = TextEmbedding::try_new(
                InitOptions::new(EmbeddingModel::BGESmallENV15).with_show_download_progress(false),
            )
            .map_err(|e| EmbedError::Init(e.to_string()))?;
            Ok(Self {
                inner: Mutex::new(inner),
                dim: 384,
            })
        }
    }

    #[async_trait]
    impl Embedder for FastEmbedder {
        async fn embed(&self, texts: &[String]) -> Result<Vec<Vec<f32>>, EmbedError> {
            let owned: Vec<&str> = texts.iter().map(String::as_str).collect();
            self.inner
                .lock()
                .map_err(|_| EmbedError::Embed("embedder mutex poisoned".to_string()))?
                .embed(owned, None)
                .map_err(|e| EmbedError::Embed(e.to_string()))
        }
        fn dimension(&self) -> usize {
            self.dim
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn mock_embedder_is_deterministic() {
        let e = MockEmbedder::new(64);
        let a = e.embed(&["hello world".into()]).await.unwrap();
        let b = e.embed(&["hello world".into()]).await.unwrap();
        assert_eq!(a, b);
    }

    #[tokio::test]
    async fn mock_embedder_normalises_to_unit() {
        let e = MockEmbedder::new(64);
        let v = e.embed(&["hello world".into()]).await.unwrap();
        let norm: f32 = v[0].iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((norm - 1.0).abs() < 1e-5, "norm was {norm}");
    }

    #[tokio::test]
    async fn similar_texts_score_higher_than_dissimilar() {
        // Sanity check: shared-token texts have higher cosine than
        // orthogonal-token texts, even with the simple word-bag embedder.
        let e = MockEmbedder::new(128);
        let v = e
            .embed(&[
                "i promise full refund".into(),
                "i promise complete refund now".into(),
                "the weather is sunny in tokyo".into(),
            ])
            .await
            .unwrap();
        let close = cosine(&v[0], &v[1]);
        let far = cosine(&v[0], &v[2]);
        assert!(close > far, "close={close} far={far}");
        assert!(close > 0.4, "close={close}");
        assert!(far < 0.3, "far={far}");
    }

    fn cosine(a: &[f32], b: &[f32]) -> f32 {
        a.iter().zip(b.iter()).map(|(x, y)| x * y).sum()
    }
}
