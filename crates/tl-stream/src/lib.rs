//! Streaming check for voice / token-by-token text. Emits `Interrupt` the
//! moment a block fires; otherwise emits `Continue` and accumulates.

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

    pub fn push(&mut self, chunk: &str) -> StreamDecision {
        self.buffer.push_str(chunk);
        if self.buffer.len() > self.window {
            let drop_to = self.buffer.len() - self.window;
            self.buffer.drain(..drop_to);
        }
        // Static engine integration lands in Phase 3 of the plan. For now
        // streaming checker is structurally correct but always Continue.
        StreamDecision::Continue
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
}
