//! living_memory.rs — the unified living-memory graph (priority #1).
//!
//! One no_std layer that co-locates memory, context, and plans WITH the codebase:
//! a typed memory store (episodic / semantic / procedural / short-term /
//! long-term) plus an embedded [`crate::code_graph::CodeGraph`] for code
//! navigation, and a command registry that records alternatives for every
//! tool/command. This is the layer Hermes and every agent query *instead of
//! grepping*. Vector navigation (hypervector), pixel snapshoting, and quantum
//! prediction (QState) attach to it as separate steps.

use crate::code_graph::{CodeGraph, EdgeKind, NodeKind};
use alloc::string::String;
use alloc::vec::Vec;

/// Memory taxonomy — every kind the system records.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryKind {
    /// "What happened" — events, sessions, interactions.
    Episodic,
    /// "What is" — durable facts and concepts.
    Semantic,
    /// "How" — skills, procedures, commands.
    Procedural,
    /// Working / short-term memory (recent, high-churn).
    ShortTerm,
    /// Long-term memory (durable, low-churn).
    LongTerm,
}

/// A single memory record, optionally linked to a code entity.
#[derive(Debug, Clone)]
pub struct MemoryRecord {
    pub id: usize,
    pub kind: MemoryKind,
    pub key: String,
    pub content: String,
    pub tags: Vec<String>,
    /// Monotonic recency stamp (higher = more recent).
    pub stamp: u64,
    /// Code node this memory is anchored to (if any).
    pub code_node: Option<usize>,
}

/// A registered command/tool, with known alternatives.
#[derive(Debug, Clone)]
pub struct CommandEntry {
    pub name: String,
    pub description: String,
    pub alternatives: Vec<String>,
}

/// The unified living memory.
#[derive(Debug, Clone, Default)]
pub struct LivingMemory {
    records: Vec<MemoryRecord>,
    commands: Vec<CommandEntry>,
    code: CodeGraph,
    stamp: u64,
}

impl LivingMemory {
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a memory record; returns its id.
    pub fn remember(&mut self, kind: MemoryKind, key: &str, content: &str) -> usize {
        self.stamp += 1;
        let id = self.records.len();
        self.records.push(MemoryRecord {
            id,
            kind,
            key: String::from(key),
            content: String::from(content),
            tags: Vec::new(),
            stamp: self.stamp,
            code_node: None,
        });
        id
    }

    /// Recall a record by id.
    pub fn recall(&self, id: usize) -> Option<&MemoryRecord> {
        self.records.get(id)
    }

    /// Recall the most recent record with the given key (any kind).
    pub fn recall_by_key(&self, key: &str) -> Option<&MemoryRecord> {
        self.records
            .iter()
            .filter(|r| r.key == key)
            .max_by_key(|r| r.stamp)
    }

    /// All records of one kind.
    pub fn by_kind(&self, kind: MemoryKind) -> Vec<&MemoryRecord> {
        self.records.iter().filter(|r| r.kind == kind).collect()
    }

    /// The `n` most recent records (short-term view).
    pub fn recent(&self, n: usize) -> Vec<&MemoryRecord> {
        let mut v: Vec<&MemoryRecord> = self.records.iter().collect();
        v.sort_by_key(|r| core::cmp::Reverse(r.stamp));
        v.truncate(n);
        v
    }

    /// Keyword search across key + content + tags (case-insensitive substring).
    pub fn search(&self, query: &str) -> Vec<usize> {
        let q = query.to_ascii_lowercase();
        self.records
            .iter()
            .filter(|r| {
                r.key.to_ascii_lowercase().contains(&q)
                    || r.content.to_ascii_lowercase().contains(&q)
                    || r.tags.iter().any(|t| t.to_ascii_lowercase().contains(&q))
            })
            .map(|r| r.id)
            .collect()
    }

    /// Anchor a memory record to a code node (so memory is co-located with code).
    pub fn link_to_code(&mut self, record_id: usize, node_name: &str) {
        if let Some(rec) = self.records.get_mut(record_id) {
            let node = self.code.add_node(node_name, NodeKind::Concept);
            rec.code_node = Some(node);
        }
    }

    /// Expose the embedded code graph for navigation queries.
    pub fn code_graph(&self) -> &CodeGraph {
        &self.code
    }
    pub fn code_graph_mut(&mut self) -> &mut CodeGraph {
        &mut self.code
    }

    // — Command registry —

    /// Register a command/tool with its alternatives.
    pub fn register_command(&mut self, name: &str, description: &str, alternatives: &[&str]) {
        self.commands.push(CommandEntry {
            name: String::from(name),
            description: String::from(description),
            alternatives: alternatives.iter().map(|s| String::from(*s)).collect(),
        });
    }

    /// Look up a command by name.
    pub fn command(&self, name: &str) -> Option<&CommandEntry> {
        self.commands.iter().find(|c| c.name == name)
    }

    /// All registered commands.
    pub fn commands(&self) -> &[CommandEntry] {
        &self.commands
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn memory_is_typed_and_recallable() {
        let mut m = LivingMemory::new();
        let a = m.remember(MemoryKind::Semantic, "auth", "auth uses pq::dsa");
        let b = m.remember(MemoryKind::Episodic, "session-1", "migrated wave-59");
        assert_eq!(m.recall(a).unwrap().kind, MemoryKind::Semantic);
        assert_eq!(m.recall(b).unwrap().content, "migrated wave-59");
        assert_eq!(m.by_kind(MemoryKind::Episodic).len(), 1);
        assert_eq!(m.by_kind(MemoryKind::Semantic).len(), 1);
    }

    #[test]
    fn recent_orders_by_recency() {
        let mut m = LivingMemory::new();
        m.remember(MemoryKind::ShortTerm, "k1", "first");
        m.remember(MemoryKind::ShortTerm, "k2", "second");
        let recent = m.recent(1);
        assert_eq!(recent[0].key, "k2");
    }

    #[test]
    fn search_matches_key_content_and_tags() {
        let mut m = LivingMemory::new();
        let id = m.remember(
            MemoryKind::Semantic,
            "quantum",
            "superposition with oracles",
        );
        assert!(m.search("quantum").contains(&id));
        assert!(m.search("oracle").contains(&id));
        assert!(!m.search("nonexistent").contains(&id));
    }

    #[test]
    fn memory_links_to_code_node() {
        let mut m = LivingMemory::new();
        let id = m.remember(MemoryKind::Procedural, "how-keygen", "call pq::kem keygen");
        m.link_to_code(id, "kem::keygen");
        assert!(m.recall(id).unwrap().code_node.is_some());
        assert_eq!(m.code_graph().node_count(), 1);
    }

    #[test]
    fn command_registry_tracks_alternatives() {
        let mut m = LivingMemory::new();
        m.register_command("grep", "search files", &["rg", "ag", "git grep"]);
        assert_eq!(m.command("grep").unwrap().alternatives.len(), 3);
        assert!(m
            .command("grep")
            .unwrap()
            .alternatives
            .contains(&String::from("rg")));
        assert!(m.command("nonexistent").is_none());
    }
}
