//! scenario.rs — deterministic scenario record / replay / resume + secret
//! redaction.
//!
//! Item #2 of screenshot-batch-2. The referenced tool wrapped browser-account
//! automation with ToS-violating intent (account farming, Cloudflare bypass,
//! API-key harvesting). Those capabilities are **deliberately NOT here** — they
//! are abuse. What IS extracted is the legitimate, reusable technical
//! substrate: (1) a deterministic step recorder with checkpoints/resume, and
//! (2) "censor mode" — masking secrets in output (a real security win).
//!
//! Zero-dep, deterministic.

use alloc::string::String;
use alloc::string::ToString;
use alloc::vec::Vec;

/// One recorded step.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Step {
    /// Monotonic step id.
    pub id: u64,
    /// Opaque action tag (what the step does).
    pub action: String,
    /// Opaque payload.
    pub payload: Vec<u8>,
}

/// A scenario: an append-only step list with a resumable cursor.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Scenario {
    steps: Vec<Step>,
    cursor: usize,
}

impl Scenario {
    pub fn new() -> Self {
        Self { steps: Vec::new(), cursor: 0 }
    }

    /// Record a step (assigns the next id).
    pub fn record(&mut self, action: &str, payload: Vec<u8>) -> u64 {
        let id = self.steps.len() as u64;
        self.steps.push(Step { id, action: action.to_string(), payload });
        id
    }

    /// Number of steps.
    pub fn len(&self) -> usize {
        self.steps.len()
    }

    pub fn is_empty(&self) -> bool {
        self.steps.is_empty()
    }

    /// Peek the step at the cursor (does not advance).
    pub fn peek(&self) -> Option<&Step> {
        self.steps.get(self.cursor)
    }

    /// Advance the cursor by one, returning the step consumed.
    pub fn next(&mut self) -> Option<&Step> {
        let s = self.steps.get(self.cursor)?;
        self.cursor += 1;
        Some(s)
    }

    /// Current cursor position.
    pub fn cursor(&self) -> usize {
        self.cursor
    }

    /// Checkpoint: current cursor position (for later resume).
    pub fn checkpoint(&self) -> usize {
        self.cursor
    }

    /// Resume from a saved checkpoint (clamped to valid range).
    pub fn resume(&mut self, checkpoint: usize) {
        self.cursor = checkpoint.min(self.steps.len());
    }

    /// Replay from the start (reset cursor, return all steps in order).
    pub fn replay(&mut self) -> Vec<Step> {
        self.cursor = 0;
        self.steps.clone()
    }

    /// Deterministic serialization (id, action-len, action, payload-len, payload).
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::new();
        for s in &self.steps {
            out.extend_from_slice(&s.id.to_le_bytes());
            out.extend_from_slice(&(s.action.len() as u64).to_le_bytes());
            out.extend_from_slice(s.action.as_bytes());
            out.extend_from_slice(&(s.payload.len() as u64).to_le_bytes());
            out.extend_from_slice(&s.payload);
        }
        out
    }

    /// Deserialize (inverse of `to_bytes`). Returns `None` on truncation.
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        let mut s = Scenario::new();
        let mut i = 0usize;
        let rd_u64 = |i: &mut usize| -> Option<u64> {
            let b = bytes.get(*i..*i + 8)?;
            *i += 8;
            Some(u64::from_le_bytes(b.try_into().ok()?))
        };
        while i < bytes.len() {
            let id = rd_u64(&mut i)?;
            let alen = rd_u64(&mut i)? as usize;
            let action = String::from_utf8_lossy(bytes.get(i..i + alen)?).into_owned();
            i += alen;
            let plen = rd_u64(&mut i)? as usize;
            let payload = bytes.get(i..i + plen)?.to_vec();
            i += plen;
            s.steps.push(Step { id, action, payload });
        }
        Some(s)
    }
}

/// Censor mode: mask every secret with `***`, longest-first so overlapping
/// secrets are fully covered. Deterministic.
pub fn redact(input: &str, secrets: &[&str]) -> String {
    if secrets.is_empty() {
        return input.to_string();
    }
    let mut sorted: Vec<&str> = secrets.to_vec();
    sorted.sort_by_key(|s| core::cmp::Reverse(s.len()));
    let mut out = input.to_string();
    for secret in sorted {
        if secret.is_empty() {
            continue;
        }
        out = out.replace(secret, "***");
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn record_and_replay() {
        let mut s = Scenario::new();
        let a = s.record("open", b"url".to_vec());
        let b = s.record("click", b"btn".to_vec());
        assert_eq!(a, 0);
        assert_eq!(b, 1);
        assert_eq!(s.len(), 2);

        let steps = s.replay();
        assert_eq!(steps[0].action, "open");
        assert_eq!(steps[1].action, "click");
        assert_eq!(s.cursor(), 0);
    }

    #[test]
    fn checkpoint_and_resume() {
        let mut s = Scenario::new();
        for i in 0..5 {
            s.record("step", vec![i as u8]);
        }
        // Consume 3 steps.
        s.next();
        s.next();
        s.next();
        let cp = s.checkpoint();
        assert_eq!(cp, 3);
        // Resume from cp: cursor is restored.
        s.resume(cp);
        assert_eq!(s.cursor(), 3);
        let nxt = s.next().unwrap();
        assert_eq!(nxt.id, 3);
    }

    #[test]
    fn serialization_roundtrip() {
        let mut s = Scenario::new();
        s.record("open", b"https://x".to_vec());
        s.record("type", b"hello world".to_vec());
        let bytes = s.to_bytes();
        let back = Scenario::from_bytes(&bytes).unwrap();
        assert_eq!(back.steps, s.steps);
    }

    #[test]
    fn serialization_rejects_truncation() {
        assert_eq!(Scenario::from_bytes(&[1, 2, 3]), None);
    }

    #[test]
    fn redact_masks_secrets_longest_first() {
        let out = redact("token=abcdef and api_key=abc", &["abc", "abcdef"]);
        assert_eq!(out, "token=*** and api_key=***");
    }

    #[test]
    fn redact_empty_secrets_identity() {
        assert_eq!(redact("hello", &[]), "hello");
        assert_eq!(redact("hello", &[""]), "hello");
    }
}
