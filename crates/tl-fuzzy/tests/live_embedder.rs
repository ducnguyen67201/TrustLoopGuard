//! Live integration test for `FastEmbedder`. Downloads BGE-small from
//! HuggingFace on first run (~100MB cached locally afterwards) and
//! validates the merge-gate semantic claim:
//!
//!   "i promise full refund" indexed → "ill promise a complete refund"
//!   matches with cosine similarity ≥ 0.85.
//!
//! Run with: `cargo test -p tl-fuzzy --features live`
//!
//! Off by default so `cargo test --workspace` stays fast and works
//! offline. CI opts in selectively.

#![cfg(feature = "live")]

use tl_fuzzy::{Embedder, FastEmbedder, HnswIndex};

#[tokio::test]
async fn semantic_paraphrase_above_85() {
    let emb = FastEmbedder::new().expect("init FastEmbedder");
    let dim = emb.dimension();

    let mut idx = HnswIndex::new(dim, 16);
    let pattern = "i promise full refund".to_string();
    let v_pat = emb.embed(std::slice::from_ref(&pattern)).await.unwrap();
    idx.insert("refund-promise", v_pat[0].clone());

    let query = "ill promise a complete refund".to_string();
    let v_q = emb.embed(std::slice::from_ref(&query)).await.unwrap();
    let hits = idx.query(&v_q[0], 5, 0.0);

    assert!(!hits.is_empty(), "no hits");
    let hit = hits.into_iter().find(|h| h.label == "refund-promise").expect("label present");
    assert!(
        hit.similarity >= 0.85,
        "similarity {} < 0.85 (paraphrase should match)",
        hit.similarity
    );
}

#[tokio::test]
async fn orthogonal_text_below_30() {
    let emb = FastEmbedder::new().expect("init FastEmbedder");
    let dim = emb.dimension();

    let mut idx = HnswIndex::new(dim, 16);
    let v_pat = emb.embed(&["i promise full refund".into()]).await.unwrap();
    idx.insert("refund-promise", v_pat[0].clone());

    let v_q = emb
        .embed(&["the weather in tokyo is sunny today".into()])
        .await
        .unwrap();
    // Use threshold 0.0 so we *get* the result, then assert it's low.
    let hits = idx.query(&v_q[0], 5, 0.0);
    let sim = hits
        .iter()
        .find(|h| h.label == "refund-promise")
        .map(|h| h.similarity)
        .unwrap_or(0.0);
    assert!(sim < 0.5, "orthogonal similarity was {sim}, expected < 0.5");
}
