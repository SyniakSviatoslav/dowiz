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

    /// Full dispatch chain for a command: the primary name followed by its
    /// alternatives in fallback order. This is the sole routing path — every
    /// tool/command invocation resolves through living memory and tries each
    /// entry in the chain until one succeeds. Unknown names pass through as-is.
    pub fn dispatch_chain(&self, name: &str) -> Vec<String> {
        match self.command(name) {
            Some(c) => {
                let mut v = Vec::with_capacity(1 + c.alternatives.len());
                v.push(c.name.clone());
                v.extend(c.alternatives.iter().cloned());
                v
            }
            None => alloc::vec![String::from(name)],
        }
    }

    pub fn commands(&self) -> &[CommandEntry] {
        &self.commands
    }

    /// Serialize the whole palace to a line-based text format (one record per
    /// line, tab-separated, tab/newline/backslash escaped). Round-trips through
    /// [`LivingMemory::from_lines`] so the palace persists across sessions.
    pub fn to_lines(&self) -> String {
        let mut out = String::new();
        for r in &self.records {
            let (ms, me) = r
                .mentioned
                .map_or((-1i64, -1i64), |(s, e)| (s as i64, e as i64));
            out.push_str(&kind_idx(r.kind).to_string());
            out.push('\t');
            out.push_str(&esc(&r.wing));
            out.push('\t');
            out.push_str(&esc(&r.room));
            out.push('\t');
            out.push_str(&esc(&r.key));
            out.push('\t');
            out.push_str(&esc(&r.summary));
            out.push('\t');
            out.push_str(&esc(&r.content));
            out.push('\t');
            out.push_str(&r.stamp.to_string());
            out.push('\t');
            out.push_str(&ms.to_string());
            out.push('\t');
            out.push_str(&me.to_string());
            out.push('\t');
            let facets: Vec<String> = r
                .facets
                .iter()
                .map(|f| {
                    format!(
                        "{}|{}|{}|{}",
                        facet_idx(f.facet_type),
                        esc(&f.search_text),
                        f.aliases.join(","),
                        esc(&f.description)
                    )
                })
                .collect();
            out.push_str(&facets.join(";"));
            out.push('\n');
        }
        out
    }

    /// Load a palace from [`LivingMemory::to_lines`] output.
    pub fn from_lines(text: &str) -> Self {
        let mut m = Self::new();
        for line in text.lines() {
            if line.is_empty() {
                continue;
            }
            let f: Vec<&str> = line.split('\t').collect();
            if f.len() < 10 {
                continue;
            }
            let kind = kind_from_idx(f[0].parse().unwrap_or(0));
            let stamp: u64 = f[7].parse().unwrap_or(0);
            let ms: i64 = f[8].parse().unwrap_or(-1);
            let me: i64 = f[9].parse().unwrap_or(-1);
            let mentioned = if ms < 0 || me < 0 {
                None
            } else {
                Some((ms as u64, me as u64))
            };
            let id = m.remember_full(
                kind,
                &unesc(f[1]),
                &unesc(f[2]),
                &unesc(f[3]),
                &unesc(f[4]),
                &unesc(f[5]),
                mentioned,
            );
            m.records[id].stamp = stamp;
            // Restore facets.
            for fs in f[10..].join("\t").split(';').filter(|s| !s.is_empty()) {
                let parts: Vec<&str> = fs.split('|').collect();
                if parts.len() >= 2 {
                    let ft = facet_from_idx(parts[0].parse().unwrap_or(0));
                    let rec = &mut m.records[id];
                    rec.facets.push(Facet {
                        facet_type: ft,
                        search_text: unesc(parts[1]),
                        aliases: if parts.len() > 2 {
                            parts[2].split(',').map(String::from).collect()
                        } else {
                            Vec::new()
                        },
                        description: if parts.len() > 3 {
                            unesc(parts[3])
                        } else {
                            String::new()
                        },
                    });
                }
            }
        }
        m
    }
}

fn kind_idx(k: MemoryKind) -> u8 {
    match k {
        MemoryKind::Episodic => 0,
        MemoryKind::Semantic => 1,
        MemoryKind::Procedural => 2,
        MemoryKind::ShortTerm => 3,
        MemoryKind::LongTerm => 4,
    }
}

fn kind_from_idx(i: u8) -> MemoryKind {
    match i {
        1 => MemoryKind::Semantic,
        2 => MemoryKind::Procedural,
        3 => MemoryKind::ShortTerm,
        4 => MemoryKind::LongTerm,
        _ => MemoryKind::Episodic,
    }
}

fn facet_idx(t: FacetType) -> u8 {
    match t {
        FacetType::Decision => 0,
        FacetType::Risk => 1,
        FacetType::Outcome => 2,
        FacetType::Metric => 3,
        FacetType::Issue => 4,
        FacetType::Plan => 5,
        FacetType::Constraint => 6,
        FacetType::Cause => 7,
    }
}

fn facet_from_idx(i: u8) -> FacetType {
    match i {
        1 => FacetType::Risk,
        2 => FacetType::Outcome,
        3 => FacetType::Metric,
        4 => FacetType::Issue,
        5 => FacetType::Plan,
        6 => FacetType::Constraint,
        7 => FacetType::Cause,
        _ => FacetType::Decision,
    }
}

/// Escape tab/newline/backslash so a field stays on one line.
fn esc(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('\t', "\\t")
        .replace('\n', "\\n")
}

/// Reverse of [`esc`].
fn unesc(s: &str) -> String {
    let mut out = String::new();
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.next() {
                Some('t') => out.push('\t'),
                Some('n') => out.push('\n'),
                Some('\\') => out.push('\\'),
                Some(o) => {
                    out.push('\\');
                    out.push(o);
                }
                None => out.push('\\'),
            }
        } else {
            out.push(c);
        }
    }
    out
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

    #[test]
    fn dispatch_chain_orders_primary_then_alternatives() {
        let mut m = LivingMemory::new();
        m.register_command("grep", "search files", &["rg", "ag", "git grep"]);
        assert_eq!(
            m.dispatch_chain("grep"),
            vec![
                String::from("grep"),
                String::from("rg"),
                String::from("ag"),
                String::from("git grep"),
            ]
        );
        // Unknown command passes through unchanged.
        assert_eq!(
            m.dispatch_chain("mystery-cmd"),
            vec![String::from("mystery-cmd")]
        );
    }

    #[test]
    fn persistence_round_trips() {
        let mut m = LivingMemory::new();
        let a = m.remember_full(
            MemoryKind::Semantic,
            "project",
            "day-1",
            "auth",
            "auth\tuses\npq",
            "verbatim content",
            Some((100, 200)),
        );
        m.add_facet(a, FacetType::Decision, "retire serde_json");
        m.add_facet(a, FacetType::Metric, "3463 tests");
        let serialized = m.to_lines();
        let loaded = LivingMemory::from_lines(&serialized);
        assert_eq!(loaded.by_kind(MemoryKind::Semantic).len(), 1);
        let r = loaded.recall_by_key("auth").unwrap();
        assert_eq!(r.wing, "project");
        assert_eq!(r.room, "day-1");
        assert_eq!(r.summary, "auth\tuses\npq"); // escaped round-trip
        assert_eq!(r.content, "verbatim content");
        assert_eq!(r.mentioned, Some((100, 200)));
        assert_eq!(r.facets.len(), 2);
        assert_eq!(r.facets[0].facet_type, FacetType::Decision);
        assert_eq!(r.facets[0].search_text, "retire serde_json");
    }
}
