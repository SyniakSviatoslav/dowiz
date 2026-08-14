//! reconstruction_memory.rs — MemHarness reimplementation: memory as reconstruction, not replay.
//!
//! # What this is
//! A kernel-native reconstruction memory engine. Instead of replaying stored
//! experience verbatim (which causes negative transfer), this engine:
//! 1. Records experience as structured memory entries (HarvestLedger-backed)
//! 2. On recall, critiques the stored entry against current context
//! 3. Reconstructs an adapted version that fits the present situation
//! 4. Uses trigram pattern surface to find recurring experience patterns
//!
//! # MemHarness mapping
//! - "policy model critiques and reconstructs" → `reconstruct()` method
//! - "contextually-grounded prompt before action" → `recall_adapted()` method
//! - "GRPO group-relative" → `group_compare()` compares multiple memory entries
//! - "negative transfer avoidance" → critique rejects irrelevant memories
//!
//! # Design
//! - Pure Rust, zero external dependencies
//! - Uses existing kernel primitives: telemetry_harvest, trigram, markov
//! - Deterministic reconstruction (no RNG in the reconstruction path)

use crate::telemetry_harvest::{HarvestLedger, HarvestRecord};
use crate::telemetry::surface_recurring_patterns;
use std::collections::HashMap;

/// A memory entry — stored experience that can be reconstructed.
#[derive(Debug, Clone)]
pub struct MemoryEntry {
    /// Unique ID for this memory.
    pub id: u64,
    /// What the memory is about (topic tag).
    pub topic: String,
    /// The stored experience (original form).
    pub content: String,
    /// Context tags that describe when this memory applies.
    pub context_tags: Vec<String>,
    /// Success/failure outcome when this memory was created.
    pub outcome: f64, // 0.0 = failure, 1.0 = success
    /// Timestamp of creation.
    pub timestamp_us: u64,
    /// SHA3-256 of the canonical bytes (integrity).
    pub hash: [u8; 32],
}

/// A reconstructed memory — the adapted version after critique.
#[derive(Debug, Clone)]
pub struct ReconstructedMemory {
    /// The original entry ID.
    pub original_id: u64,
    /// Whether the memory passed critique (relevant to current context).
    pub passed_critique: bool,
    /// The reconstructed/adapted content.
    pub reconstructed_content: String,
    /// Confidence in the reconstruction (0.0-1.0).
    pub confidence: f64,
    /// Why this memory was selected (critique reasoning).
    pub critique_reason: String,
}

/// Critique verdict on a memory entry against current context.
#[derive(Debug, Clone, PartialEq)]
pub enum CritiqueVerdict {
    /// Memory is relevant and can be used as-is.
    Relevant,
    /// Memory is partially relevant — needs reconstruction.
    PartialReconstruction {
        /// Which context tags matched.
        matched_tags: Vec<String>,
        /// Which context tags are missing.
        missing_tags: Vec<String>,
    },
    /// Memory is irrelevant to current context — negative transfer risk.
    Irrelevant {
        /// Why it was rejected.
        reason: String,
    },
}

/// The reconstruction memory engine.
pub struct ReconstructionMemory {
    /// Underlying storage for memory entries.
    entries: Vec<MemoryEntry>,
    /// Monotonic ID counter.
    next_id: u64,
    /// Current context tags (what the agent is working on now).
    current_context: Vec<String>,
    /// Pattern surface cache (trigram-based recurring patterns).
    pattern_cache: Option<crate::telemetry::PatternSurface>,
}

impl ReconstructionMemory {
    /// Create a new reconstruction memory engine.
    pub fn new() -> Self {
        ReconstructionMemory {
            entries: Vec::new(),
            next_id: 0,
            current_context: Vec::new(),
            pattern_cache: None,
        }
    }

    /// Set the current context tags (what the agent is working on).
    pub fn set_context(&mut self, tags: Vec<String>) {
        self.current_context = tags;
        self.pattern_cache = None; // invalidate cache
    }

    /// Add a memory entry.
    pub fn remember(&mut self, topic: &str, content: &str, context_tags: Vec<String>, outcome: f64) -> u64 {
        let id = self.next_id;
        self.next_id += 1;

        let timestamp_us = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_micros() as u64;

        let mut entry = MemoryEntry {
            id,
            topic: topic.to_string(),
            content: content.to_string(),
            context_tags,
            outcome,
            timestamp_us,
            hash: [0u8; 32],
        };

        // Compute hash.
        let mut buf = Vec::with_capacity(64);
        buf.extend_from_slice(&id.to_le_bytes());
        buf.extend_from_slice(topic.as_bytes());
        buf.extend_from_slice(content.as_bytes());
        entry.hash = crate::event_log::sha3_256(&buf);

        self.entries.push(entry);
        id
    }

    /// Critique a memory entry against the current context.
    pub fn critique(&self, entry: &MemoryEntry) -> CritiqueVerdict {
        if self.current_context.is_empty() {
            return CritiqueVerdict::Relevant; // no context = accept all
        }

        let mut matched = Vec::new();
        let mut missing = Vec::new();

        for tag in &self.current_context {
            if entry.context_tags.contains(tag) {
                matched.push(tag.clone());
            } else {
                missing.push(tag.clone());
            }
        }

        if matched.is_empty() && missing.len() == self.current_context.len() {
            CritiqueVerdict::Irrelevant {
                reason: format!("no context tags matched (needed {:?}, had {:?})",
                    self.current_context, entry.context_tags)
            }
        } else if missing.is_empty() {
            CritiqueVerdict::Relevant
        } else {
            CritiqueVerdict::PartialReconstruction { matched_tags: matched, missing_tags: missing }
        }
    }

    /// Reconstruct a memory entry for the current context.
    ///
    /// Returns `ReconstructedMemory` with adapted content based on critique.
    pub fn reconstruct(&self, entry: &MemoryEntry) -> ReconstructedMemory {
        let critique = self.critique(entry);

        match critique {
            CritiqueVerdict::Relevant => {
                ReconstructedMemory {
                    original_id: entry.id,
                    passed_critique: true,
                    reconstructed_content: entry.content.clone(),
                    confidence: 1.0,
                    critique_reason: "fully relevant — used as-is".to_string(),
                }
            }
            CritiqueVerdict::PartialReconstruction { matched_tags, missing_tags } => {
                // Reconstruct: keep matched parts, note missing context
                let reconstructed = format!(
                    "[RECONSTRUCTED from memory #{}] {}\n\
                     Matched context: {}\n\
                     Missing context: {}\n\
                     Original: {}",
                    entry.id,
                    entry.topic,
                    matched_tags.join(", "),
                    missing_tags.join(", "),
                    entry.content
                );

                ReconstructedMemory {
                    original_id: entry.id,
                    passed_critique: true,
                    reconstructed_content: reconstructed,
                    confidence: 0.7, // partial match = lower confidence
                    critique_reason: format!("partial match: {} tags matched, {} missing",
                        matched_tags.len(), missing_tags.len()),
                }
            }
            CritiqueVerdict::Irrelevant { reason } => {
                ReconstructedMemory {
                    original_id: entry.id,
                    passed_critique: false,
                    reconstructed_content: String::new(),
                    confidence: 0.0,
                    critique_reason: reason,
                }
            }
        }
    }

    /// Recall memories adapted to the current context.
    ///
    /// Returns reconstructed memories that passed critique, sorted by relevance.
    pub fn recall_adapted(&self, limit: usize) -> Vec<ReconstructedMemory> {
        let mut results = Vec::new();

        for entry in &self.entries {
            let reconstructed = self.reconstruct(entry);
            if reconstructed.passed_critique {
                results.push(reconstructed);
            }
        }

        // Sort by confidence descending.
        results.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap_or(core::cmp::Ordering::Equal));

        results.truncate(limit);
        results
    }

    /// Group-relative comparison: compare multiple memory entries and rank them.
    ///
    /// This is the GRPO-inspired part: memories are compared within a group
    /// (same topic) and ranked by outcome + relevance to current context.
    pub fn group_compare(&self, topic: &str) -> Vec<(u64, f64, String)> {
        // Collect entries matching the topic.
        let mut group: Vec<&MemoryEntry> = self.entries.iter()
            .filter(|e| e.topic == topic)
            .collect();

        if group.is_empty() {
            return Vec::new();
        }

        // Score each entry: outcome * relevance_to_current_context
        let mut scored: Vec<(u64, f64, String)> = group.iter().map(|e| {
            let relevance = if self.current_context.is_empty() {
                1.0
            } else {
                let matches = self.current_context.iter()
                    .filter(|ctx| e.context_tags.contains(ctx))
                    .count();
                matches as f64 / self.current_context.len() as f64
            };
            let score = e.outcome * relevance;
            (e.id, score, e.content.clone())
        }).collect();

        // Sort by score descending (group-relative ranking).
        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(core::cmp::Ordering::Equal));

        scored
    }

    /// Find recurring patterns in memory content using trigram surface.
    pub fn find_patterns(&mut self, k: usize) -> Option<crate::telemetry::PatternSurface> {
        if self.entries.is_empty() {
            return None;
        }

        // Tokenize memory content into outcome tokens.
        let tokens: Vec<&str> = self.entries.iter()
            .flat_map(|e| e.content.split_whitespace())
            .collect();

        if tokens.len() < 3 {
            return None;
        }

        let surface = surface_recurring_patterns(&tokens, k);
        self.pattern_cache = Some(surface.clone());
        Some(surface)
    }

    /// Get the number of stored memories.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Check if there are no memories.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Get a memory entry by ID.
    pub fn get(&self, id: u64) -> Option<&MemoryEntry> {
        self.entries.iter().find(|e| e.id == id)
    }

    /// Clear all memories (for reset/testing).
    pub fn clear(&mut self) {
        self.entries.clear();
        self.next_id = 0;
        self.pattern_cache = None;
    }

    /// ASCII report of current memory state.
    pub fn ascii_report(&self) -> String {
        let mut out = String::from("=== Reconstruction Memory Report ===\n");
        out.push_str(&format!("Total memories: {}\n", self.len()));
        out.push_str(&format!("Current context: {:?}\n", self.current_context));

        if !self.entries.is_empty() {
            out.push_str("\nRecent memories:\n");
            for entry in self.entries.iter().rev().take(5) {
                out.push_str(&format!(
                    "  #{}: {} [{}] outcome={:.2}\n",
                    entry.id, entry.topic, entry.context_tags.join(","),
                    entry.outcome
                ));
            }
        }

        if let Some(ref surface) = self.pattern_cache {
            if !surface.top.is_empty() {
                out.push_str("\nRecurring patterns:\n");
                for (tri, count) in &surface.top {
                    out.push_str(&format!("  {:?} × {}\n", tri, count));
                }
            }
        }

        out.push_str("\n=== End Report ===\n");
        out
    }
}

impl Default for ReconstructionMemory {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_memory_is_empty() {
        let mem = ReconstructionMemory::new();
        assert!(mem.is_empty());
        assert_eq!(mem.len(), 0);
    }

    #[test]
    fn remember_adds_entry() {
        let mut mem = ReconstructionMemory::new();
        let id = mem.remember("test", "some content", vec!["ctx1".to_string()], 1.0);
        assert_eq!(id, 0);
        assert_eq!(mem.len(), 1);
        assert!(!mem.is_empty());
    }

    #[test]
    fn remember_multiple_gets_incrementing_ids() {
        let mut mem = ReconstructionMemory::new();
        let id1 = mem.remember("a", "c1", vec![], 0.5);
        let id2 = mem.remember("b", "c2", vec![], 0.8);
        assert_eq!(id1, 0);
        assert_eq!(id2, 1);
        assert_eq!(mem.len(), 2);
    }

    #[test]
    fn critique_rejects_irrelevant() {
        let mut mem = ReconstructionMemory::new();
        mem.set_context(vec!["ctx1".to_string(), "ctx2".to_string()]);
        mem.remember("topic", "content", vec!["ctx3".to_string()], 1.0);

        let entry = mem.get(0).unwrap();
        let verdict = mem.critique(entry);
        match verdict {
            CritiqueVerdict::Irrelevant { reason } => {
                assert!(reason.contains("no context tags matched"));
            }
            _ => panic!("expected Irrelevant, got {:?}", verdict),
        }
    }

    #[test]
    fn critique_accepts_relevant() {
        let mut mem = ReconstructionMemory::new();
        mem.set_context(vec!["ctx1".to_string()]);
        mem.remember("topic", "content", vec!["ctx1".to_string()], 1.0);

        let entry = mem.get(0).unwrap();
        let verdict = mem.critique(entry);
        assert_eq!(verdict, CritiqueVerdict::Relevant);
    }

    #[test]
    fn reconstruct_passes_critique_for_relevant() {
        let mut mem = ReconstructionMemory::new();
        mem.set_context(vec!["ctx1".to_string()]);
        mem.remember("topic", "original content", vec!["ctx1".to_string()], 1.0);

        let entry = mem.get(0).unwrap();
        let recon = mem.reconstruct(entry);
        assert!(recon.passed_critique);
        assert_eq!(recon.confidence, 1.0);
        assert_eq!(recon.reconstructed_content, "original content");
    }

    #[test]
    fn reconstruct_fails_critique_for_irrelevant() {
        let mut mem = ReconstructionMemory::new();
        mem.set_context(vec!["ctx1".to_string()]);
        mem.remember("topic", "content", vec!["ctx2".to_string()], 1.0);

        let entry = mem.get(0).unwrap();
        let recon = mem.reconstruct(entry);
        assert!(!recon.passed_critique);
        assert_eq!(recon.confidence, 0.0);
    }

    #[test]
    fn recall_adapted_returns_only_passed() {
        let mut mem = ReconstructionMemory::new();
        mem.set_context(vec!["ctx1".to_string()]);
        mem.remember("a", "content_a", vec!["ctx1".to_string()], 1.0); // relevant
        mem.remember("b", "content_b", vec!["ctx2".to_string()], 1.0); // irrelevant

        let results = mem.recall_adapted(10);
        assert_eq!(results.len(), 1);
        assert!(results[0].passed_critique);
    }

    #[test]
    fn group_compare_ranks_by_outcome() {
        let mut mem = ReconstructionMemory::new();
        mem.remember("topic", "low", vec![], 0.3);
        mem.remember("topic", "high", vec![], 0.9);
        mem.remember("other", "skip", vec![], 1.0);

        let ranked = mem.group_compare("topic");
        assert_eq!(ranked.len(), 2);
        // High outcome should rank first.
        assert!(ranked[0].1 >= ranked[1].1);
    }

    #[test]
    fn find_patterns_returns_none_for_empty() {
        let mut mem = ReconstructionMemory::new();
        assert!(mem.find_patterns(5).is_none());
    }

    #[test]
    fn clear_resets_state() {
        let mut mem = ReconstructionMemory::new();
        mem.remember("a", "c", vec![], 1.0);
        mem.remember("b", "c", vec![], 1.0);
        assert_eq!(mem.len(), 2);

        mem.clear();
        assert_eq!(mem.len(), 0);
        assert!(mem.is_empty());
    }

    #[test]
    fn ascii_report_format() {
        let mem = ReconstructionMemory::new();
        let report = mem.ascii_report();
        assert!(report.contains("Reconstruction Memory Report"));
        assert!(report.contains("Total memories: 0"));
    }

    #[test]
    fn memory_hash_is_computed() {
        let mut mem = ReconstructionMemory::new();
        mem.remember("topic", "content", vec![], 1.0);
        let entry = mem.get(0).unwrap();
        assert_eq!(entry.hash.len(), 32);
        assert!(!entry.hash.iter().all(|&b| b == 0));
    }
}
