//! Fuzzy similarity index. Stores `(label, vector)` pairs and answers
//! "which labels are within cosine threshold of this query?".
//!
//! Backed by `hnsw_rs` for sub-millisecond approximate nearest-neighbour
//! lookup at scale. Distance is cosine; we negate it during scoring so a
//! similarity of 1.0 means "identical" and 0.0 means "orthogonal".
//!
//! The index does not own the embedder. Callers embed query text and
//! pass the vector in. This keeps the index reusable across embedder
//! choices (Mock vs Fast vs custom) and avoids holding async state in a
//! sync data structure.

use hnsw_rs::prelude::{DistCosine, Hnsw};

#[derive(Debug, Clone)]
pub struct IndexHit {
    pub label: String,
    pub similarity: f32,
}

/// Cosine-similarity HNSW index. Each entry has a label (e.g. the
/// originating pattern text) and a vector. Insertions are O(log n);
/// queries are sub-millisecond for indexes up to ~10K entries.
pub struct HnswIndex {
    inner: Hnsw<'static, f32, DistCosine>,
    labels: Vec<String>,
    dim: usize,
}

impl HnswIndex {
    pub fn new(dim: usize, expected_capacity: usize) -> Self {
        // Parameters tuned for small-to-medium pattern sets (a few
        // hundred entries per tenant). HNSW paper defaults: M=16
        // connections per layer, ef_construction=200, max_layer=16.
        let max_nb_connection = 16;
        let max_elements = expected_capacity.max(16);
        let max_layer = 16;
        let ef_construction = 200;
        let inner = Hnsw::<f32, DistCosine>::new(
            max_nb_connection,
            max_elements,
            max_layer,
            ef_construction,
            DistCosine {},
        );
        Self {
            inner,
            labels: Vec::with_capacity(expected_capacity),
            dim,
        }
    }

    pub fn len(&self) -> usize {
        self.labels.len()
    }

    pub fn is_empty(&self) -> bool {
        self.labels.is_empty()
    }

    pub fn dimension(&self) -> usize {
        self.dim
    }

    /// Insert a labelled vector. The label is what callers receive in
    /// `IndexHit` — typically the original pattern text or its policy id.
    pub fn insert(&mut self, label: impl Into<String>, vector: Vec<f32>) {
        assert_eq!(
            vector.len(),
            self.dim,
            "vector dim {} doesn't match index dim {}",
            vector.len(),
            self.dim
        );
        let id = self.labels.len();
        self.labels.push(label.into());
        self.inner.insert((vector.as_slice(), id));
    }

    /// Query the index. Returns up to `top_k` hits with similarity above
    /// `min_similarity` (cosine, in `[0.0, 1.0]`). Hits are sorted by
    /// descending similarity.
    pub fn query(&self, vector: &[f32], top_k: usize, min_similarity: f32) -> Vec<IndexHit> {
        if self.labels.is_empty() || vector.len() != self.dim {
            return vec![];
        }
        // ef parameter trades query latency for recall. A factor of 4x
        // top_k is the common starting point.
        let ef = (top_k * 4).max(16);
        let neighbours = self.inner.search(vector, top_k, ef);
        neighbours
            .into_iter()
            .filter_map(|n| {
                // hnsw_rs reports cosine *distance* in [0, 2]; convert to
                // similarity in [-1, 1] via 1 - distance, clamped to [0, 1]
                // for the public API (negative cosine is "more orthogonal
                // than orthogonal" and rare for normalised text vectors).
                let sim: f32 = (1.0_f32 - n.distance).max(0.0);
                if sim < min_similarity {
                    return None;
                }
                let label = self.labels.get(n.d_id)?.clone();
                Some(IndexHit {
                    label,
                    similarity: sim,
                })
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unit(values: &[f32]) -> Vec<f32> {
        let n = values.iter().map(|x| x * x).sum::<f32>().sqrt();
        values.iter().map(|x| x / n).collect()
    }

    #[test]
    fn empty_index_returns_empty_query() {
        let idx = HnswIndex::new(8, 16);
        let hits = idx.query(&unit(&[1.0; 8]), 5, 0.0);
        assert!(hits.is_empty());
    }

    #[test]
    fn identical_vector_scores_one() {
        let mut idx = HnswIndex::new(4, 16);
        let v = unit(&[1.0, 0.0, 0.0, 0.0]);
        idx.insert("alpha", v.clone());
        let hits = idx.query(&v, 1, 0.0);
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].label, "alpha");
        assert!((hits[0].similarity - 1.0).abs() < 1e-3);
    }

    #[test]
    fn orthogonal_vector_below_threshold() {
        let mut idx = HnswIndex::new(4, 16);
        idx.insert("alpha", unit(&[1.0, 0.0, 0.0, 0.0]));
        let hits = idx.query(&unit(&[0.0, 1.0, 0.0, 0.0]), 5, 0.5);
        assert!(hits.is_empty(), "orthogonal should be filtered");
    }

    #[test]
    fn ranks_by_similarity_descending() {
        let mut idx = HnswIndex::new(4, 16);
        idx.insert("near", unit(&[1.0, 0.05, 0.0, 0.0]));
        idx.insert("far", unit(&[0.7, 0.7, 0.0, 0.0]));
        let hits = idx.query(&unit(&[1.0, 0.0, 0.0, 0.0]), 2, 0.0);
        assert_eq!(hits.len(), 2);
        assert_eq!(hits[0].label, "near");
        assert_eq!(hits[1].label, "far");
        assert!(hits[0].similarity > hits[1].similarity);
    }

    #[test]
    fn dim_mismatch_yields_empty_query() {
        let mut idx = HnswIndex::new(4, 16);
        idx.insert("alpha", unit(&[1.0, 0.0, 0.0, 0.0]));
        let hits = idx.query(&[1.0, 0.0], 1, 0.0);
        assert!(hits.is_empty());
    }

    #[test]
    fn mock_embedder_round_trip_through_index() {
        // Bridge test: word-bag mock vectors recover their pattern when
        // queried with a near-duplicate string. Validates the wiring
        // (embed -> insert -> embed -> query) end-to-end.
        use crate::embedder::word_bag_embed;
        let dim = 128;
        let mut idx = HnswIndex::new(dim, 16);
        idx.insert("refund-promise", word_bag_embed("i promise full refund", dim));
        idx.insert("greeting", word_bag_embed("hello and welcome", dim));
        let q = word_bag_embed("i promise complete refund now", dim);
        let hits = idx.query(&q, 5, 0.0);
        assert!(!hits.is_empty());
        assert_eq!(hits[0].label, "refund-promise");
    }
}
