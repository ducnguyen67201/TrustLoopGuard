//! Streaming check for voice / token-by-token text. Accumulates a sliding
//! window of streamed output and, when evaluated, emits `Interrupt` the moment
//! a guard verdict turns non-`Allow`; otherwise emits `Continue`.
//!
//! The checker is engine-agnostic: callers supply an evaluator closure
//! (typically wrapping `tl_engine::Engine::check`) so this crate stays free of
//! engine/storage dependencies. A typical loop pushes each token, then calls
//! [`StreamingChecker::check`] to decide whether to keep streaming.

use serde::{Deserialize, Serialize};
use tl_core::Verdict;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StreamDecision {
    Continue,
    Interrupt { verdict: Verdict, reason: String },
}

pub struct StreamingChecker {
    buffer: String,
    window: usize,
}

impl StreamingChecker {
    pub fn new(window: usize) -> Self {
        Self {
            buffer: String::new(),
            window,
        }
    }

    /// Append a streamed chunk to the sliding window and return the current
    /// accumulated text. The window is bounded to `window` bytes.
    pub fn push(&mut self, chunk: &str) -> &str {
        self.buffer.push_str(chunk);
        if self.buffer.len() > self.window {
            let drop_to = self.buffer.len() - self.window;
            self.buffer.drain(..drop_to);
        }
        &self.buffer
    }

    /// Evaluate the accumulated window with `evaluate` (e.g. the guard engine).
    /// Any non-`Allow` verdict produces an `Interrupt` so the caller can stop
    /// streaming before unsafe text is delivered.
    pub fn check<F>(&self, mut evaluate: F) -> StreamDecision
    where
        F: FnMut(&str) -> Verdict,
    {
        match evaluate(&self.buffer) {
            Verdict::Allow => StreamDecision::Continue,
            verdict => StreamDecision::Interrupt {
                verdict,
                reason: format!("streaming output interrupted by {verdict:?} verdict"),
            },
        }
    }

    pub fn current(&self) -> &str {
        &self.buffer
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn buffer_truncates_to_window() {
        let mut c = StreamingChecker::new(8);
        let _ = c.push("abcdefgh");
        let _ = c.push("ij");
        assert!(c.current().len() <= 8);
    }

    #[test]
    fn continues_when_evaluator_allows() {
        let mut c = StreamingChecker::new(64);
        c.push("all good");
        let decision = c.check(|_| Verdict::Allow);
        assert!(matches!(decision, StreamDecision::Continue));
    }

    #[test]
    fn interrupts_when_evaluator_flags_window() {
        let mut c = StreamingChecker::new(64);
        c.push("here is the ");
        c.push("secret");
        let decision = c.check(|text| {
            if text.contains("secret") {
                Verdict::Block
            } else {
                Verdict::Allow
            }
        });
        match decision {
            StreamDecision::Interrupt { verdict, .. } => assert_eq!(verdict, Verdict::Block),
            StreamDecision::Continue => panic!("expected interrupt"),
        }
    }
}
