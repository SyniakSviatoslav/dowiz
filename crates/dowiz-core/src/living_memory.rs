//! living_memory.rs — the unified living-memory graph (priority #1).
//!
//! One no_std layer co-locating memory, context, and plans WITH the codebase,
//! natively reproducing the cognitive-memory model of `m_flow` (episode anchor →
//! typed facets, coarse-to-fine retrieval) and the palace hierarchy of
//! `mempalace` (wing → room → drawer, verbatim content). Navigation goes through
//! an embedded [`crate::code_graph::CodeGraph`]; a command registry records
//! alternatives for every tool. Vector navigation (hypervector) and quantum
//! prediction (QState) attach as follow-on steps.

use crate::code_graph::{CodeGraph, NodeKind};
use crate::hypervector_index::HypervectorIndex;
use crate::retrieval::bm25::tokenize;
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

/// m_flow facet type — a typed angle on an episode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FacetType {
    Decision,
    Risk,
    Outcome,
    Metric,
    Issue,
    Plan,
    Constraint,
    Cause,
}

/// m_flow Facet: the precise detail handle of an episode anchor.
#[derive(Debug, Clone)]
pub struct Facet {
    pub facet_type: FacetType,
    /// Short, sharp retrieval handle (participates in vectorization).
    pub search_text: String,
    /// Synonymous fallback handles (2–5 short expressions).
    pub aliases: Vec<String>,
    /// Expansion field — not indexed, used for context expansion.
    pub description: String,
}

/// A single memory record = an episode anchor (m_flow) placed in a palace
/// wing/room (mempalace), optionally linked to a code entity.
#[derive(Debug, Clone)]
pub struct MemoryRecord {
    pub id: usize,
    pub kind: MemoryKind,
    pub key: String,
    /// mempalace wing — broad category (person/project/topic).
    pub wing: String,
    /// mempalace room — grouping (day/session/subtopic).
    pub room: String,
    /// m_flow episode summary — the coarse retrieval carrier.
    pub summary: String,
    /// Verbatim content (mempalace: never summarize the actual words).
    pub content: String,
    /// Typed facets (m_flow details).
    pub facets: Vec<Facet>,
    pub tags: Vec<String>,
    /// Monotonic recency stamp (higher = more recent).
    pub stamp: u64,
    /// Mentioned time window (ms) — temporal validity (m_flow + mempalace KG).
    pub mentioned: Option<(u64, u64)>,
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
    /// Vector-navigation index (hypervector similarity over summary+content).
    hv_index: HypervectorIndex,
    stamp: u64,
}

impl LivingMemory {
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a memory record (episode anchor); returns its id.
    pub fn remember(&mut self, kind: MemoryKind, key: &str, content: &str) -> usize {
        self.remember_full(kind, "", "", key, content, content, None)
    }

    /// Add a memory with full palace placement (wing/room), a summary, and a
    /// mentioned-time window.
    pub fn remember_full(
        &mut self,
        kind: MemoryKind,
        wing: &str,
        room: &str,
        key: &str,
        summary: &str,
        content: &str,
        mentioned: Option<(u64, u64)>,
    ) -> usize {
        self.stamp += 1;
        let id = self.records.len();
        self.records.push(MemoryRecord {
            id,
            kind,
            key: String::from(key),
            wing: String::from(wing),
            room: String::from(room),
            summary: String::from(summary),
            content: String::from(content),
            facets: Vec::new(),
            tags: Vec::new(),
            stamp: self.stamp,
            mentioned,
            code_node: None,
        });
        // Vector navigation: index summary+content so `vector_search` can rank
        // by hypervector similarity (hv doc id == record id, appended in order).
        let mut terms = tokenize(summary);
        terms.extend(tokenize(content));
        self.hv_index.insert(terms);
        id
    }

    /// Attach a typed facet (m_flow detail) to a record.
    pub fn add_facet(&mut self, record_id: usize, facet_type: FacetType, search_text: &str) {
        if let Some(rec) = self.records.get_mut(record_id) {
            rec.facets.push(Facet {
                facet_type,
                search_text: String::from(search_text),
                aliases: Vec::new(),
                description: String::new(),
            });
        }
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

    /// All records in a mempalace wing.
    pub fn by_wing(&self, wing: &str) -> Vec<&MemoryRecord> {
        self.records.iter().filter(|r| r.wing == wing).collect()
    }

    /// All records in a wing + room.
    pub fn by_room(&self, wing: &str, room: &str) -> Vec<&MemoryRecord> {
        self.records
            .iter()
            .filter(|r| r.wing == wing && r.room == room)
            .collect()
    }

    /// All facets of a given type across the whole palace.
    pub fn facets_of(&self, facet_type: FacetType) -> Vec<(&MemoryRecord, &Facet)> {
        let mut out = Vec::new();
        for r in &self.records {
            for f in &r.facets {
                if f.facet_type == facet_type {
                    out.push((r, f));
                }
            }
        }
        out
    }

    /// The `n` most recent records (short-term view).
    pub fn recent(&self, n: usize) -> Vec<&MemoryRecord> {
        let mut v: Vec<&MemoryRecord> = self.records.iter().collect();
        v.sort_by_key(|r| core::cmp::Reverse(r.stamp));
        v.truncate(n);
        v
    }

    /// Keyword search across key + summary + content + tags + facet search_text
    /// (case-insensitive substring).
    pub fn search(&self, query: &str) -> Vec<usize> {
        let q = query.to_ascii_lowercase();
        self.records
            .iter()
            .filter(|r| {
                r.key.to_ascii_lowercase().contains(&q)
                    || r.summary.to_ascii_lowercase().contains(&q)
                    || r.content.to_ascii_lowercase().contains(&q)
                    || r.tags.iter().any(|t| t.to_ascii_lowercase().contains(&q))
                    || r.facets
                        .iter()
                        .any(|f| f.search_text.to_ascii_lowercase().contains(&q))
            })
            .map(|r| r.id)
            .collect()
    }

    /// Semantic vector navigation: rank records by hypervector similarity to the
    /// query, returning (record_id, score) pairs (most similar first).
    pub fn vector_search(&self, query: &str, k: usize) -> Vec<(usize, f64)> {
        let terms = tokenize(query);
        let q = self.hv_index.encode_query(&terms);
        self.hv_index.top_k(&q, k)
    }

    /// Anchor a memory record to a code node (memory co-located with code).
    pub fn link_to_code(&mut self, record_id: usize, node_name: &str) {
        if let Some(rec) = self.records.get_mut(record_id) {
            let node = self.code.add_node(node_name, NodeKind::Concept);
            rec.code_node = Some(node);
        }
    }

    pub fn code_graph(&self) -> &CodeGraph {
        &self.code
    }
    pub fn code_graph_mut(&mut self) -> &mut CodeGraph {
        &mut self.code
    }

    // — Command registry —

    pub fn register_command(&mut self, name: &str, description: &str, alternatives: &[&str]) {
        self.commands.push(CommandEntry {
            name: String::from(name),
            description: String::from(description),
            alternatives: alternatives.iter().map(|s| String::from(*s)).collect(),
        });
    }

    pub fn command(&self, name: &str) -> Option<&CommandEntry> {
        self.commands.iter().find(|c| c.name == name)
    }

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
    }

    #[test]
    fn palace_wing_room_partitions() {
        let mut m = LivingMemory::new();
        m.remember_full(
            MemoryKind::Semantic,
            "project",
            "day-1",
            "a",
            "auth",
            "uses pq",
            None,
        );
        m.remember_full(
            MemoryKind::Semantic,
            "project",
            "day-2",
            "b",
            "crypto",
            "uses kem",
            None,
        );
        m.remember_full(
            MemoryKind::Episodic,
            "people",
            "day-1",
            "c",
            "meeting",
            "notes",
            None,
        );
        assert_eq!(m.by_wing("project").len(), 2);
        assert_eq!(m.by_room("project", "day-1").len(), 1);
        assert_eq!(m.by_wing("people").len(), 1);
    }

    #[test]
    fn facets_are_typed_and_retrievable() {
        let mut m = LivingMemory::new();
        let id = m.remember(MemoryKind::Episodic, "migration", "wave-59 done");
        m.add_facet(id, FacetType::Decision, "retire serde_json");
        m.add_facet(id, FacetType::Metric, "3463 tests green");
        assert_eq!(m.facets_of(FacetType::Decision).len(), 1);
        assert_eq!(m.facets_of(FacetType::Metric).len(), 1);
        assert_eq!(m.facets_of(FacetType::Risk).len(), 0);
        // facet search_text participates in search.
        assert!(m.search("serde_json").contains(&id));
    }

    #[test]
    fn recent_orders_by_recency() {
        let mut m = LivingMemory::new();
        m.remember(MemoryKind::ShortTerm, "k1", "first");
        m.remember(MemoryKind::ShortTerm, "k2", "second");
        assert_eq!(m.recent(1)[0].key, "k2");
    }

    #[test]
    fn search_matches_key_content_and_facets() {
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
    fn vector_search_ranks_by_similarity() {
        let mut m = LivingMemory::new();
        let a = m.remember(MemoryKind::Semantic, "quantum", "superposition and oracles");
        let b = m.remember(MemoryKind::Semantic, "crypto", "post quantum keygen");
        let c = m.remember(MemoryKind::Episodic, "meeting", "daily standup notes");
        // "quantum superposition oracle" should rank the quantum record first.
        let hits = m.vector_search("quantum superposition oracle", 1);
        assert_eq!(hits[0].0, a);
        // "keygen crypto" should rank the crypto record first.
        let hits2 = m.vector_search("keygen crypto", 1);
        assert_eq!(hits2[0].0, b);
        let _ = c;
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
        assert!(m.command("nonexistent").is_none());
    }
}
