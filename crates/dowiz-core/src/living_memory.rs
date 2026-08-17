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
use crate::hypervector::Hypervector;
use crate::hypervector_index::HypervectorIndex;
use crate::retrieval::bm25::tokenize;
use alloc::string::String;
use alloc::string::ToString;
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
    /// Pre-lowercased search blob (key + summary + content) — the fast path for
    /// [`LivingMemory::search`]. Derived once at insert/load; never re-lowered
    /// per query.
    lower: String,
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
        self.remember_full_inner(kind, wing, room, key, summary, content, mentioned, None)
    }

    /// [`remember_full`] with an optional pre-computed hypervector code. The
    /// persistence path (`from_lines`) passes `Some(code)` so the record is
    /// indexed by a 128-byte copy instead of re-hashing + re-bundling its terms
    /// — the cold-start killer. The live path passes `None` to compute it.
    fn remember_full_inner(
        &mut self,
        kind: MemoryKind,
        wing: &str,
        room: &str,
        key: &str,
        summary: &str,
        content: &str,
        mentioned: Option<(u64, u64)>,
        hv_code: Option<Hypervector>,
    ) -> usize {
        self.stamp += 1;
        let id = self.records.len();
        // Pre-lowercased search blob: key + summary + content, joined with the
        // control char \x1f so a query can't span two fields.
        let mut lower = String::with_capacity(key.len() + summary.len() + content.len() + 2);
        lower.push_str(&key.to_ascii_lowercase());
        lower.push('\x1f');
        lower.push_str(&summary.to_ascii_lowercase());
        lower.push('\x1f');
        lower.push_str(&content.to_ascii_lowercase());
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
            lower,
            stamp: self.stamp,
            mentioned,
            code_node: None,
        });
        // Vector navigation: index summary+content so `vector_search` can rank
        // by hypervector similarity (hv doc id == record id, appended in order).
        match hv_code {
            Some(code) => {
                self.hv_index.insert_with_code(code);
            }
            None => {
                let mut terms = tokenize(summary);
                terms.extend(tokenize(content));
                self.hv_index.insert(terms);
            }
        }
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
    /// (case-insensitive substring). The hot path reads the pre-lowercased
    /// `lower` blob (one `contains` per record, no per-query allocations);
    /// tags/facets are checked live (they are sparse).
    pub fn search(&self, query: &str) -> Vec<usize> {
        let q = query.to_ascii_lowercase();
        self.records
            .iter()
            .filter(|r| {
                r.lower.contains(&q)
                    || (!r.tags.is_empty()
                        && r.tags.iter().any(|t| t.to_ascii_lowercase().contains(&q)))
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

    /// Shift-invariant vector navigation: re-rank the top Hamming candidates by
    /// the maximum Hamming similarity over **all cyclic alignments**, computed
    /// with the exact-integer NTT (circular cross-correlation, O(D log D) per
    /// pair). This finds a query pattern embedded in a record at any offset —
    /// something plain Hamming ranking (alignment 0 only) cannot.
    ///
    /// Two-stage: fast O(D) Hamming prefilter to the top `CAND`, then the NTT
    /// re-rank over that small candidate set (full-corpus NTT would be
    /// O(n · D log D) and is not needed — the prefilter prunes the space).
    pub fn convolution_search(&self, query: &str, k: usize) -> Vec<(usize, f64)> {
        const CAND: usize = 64;
        let terms = tokenize(query);
        let q = self.hv_index.encode_query(&terms);
        let candidates = self.hv_index.top_k(&q, CAND);
        let mut scored: Vec<(usize, f64)> = candidates
            .iter()
            .filter_map(|(id, _)| {
                self.hv_index
                    .code_of(*id)
                    .map(|c| (*id, q.shift_invariant_similarity(&c)))
            })
            .collect();
        scored.sort_by(|a, b| {
            b.1.partial_cmp(&a.1)
                .unwrap_or(core::cmp::Ordering::Equal)
                .then_with(|| a.0.cmp(&b.0))
        });
        scored.truncate(k);
        scored
    }

    /// Serialize the hypervector index to a compact binary blob (codes only —
    /// the fast, re-hash-free persistence path; see `HypervectorIndex`).
    pub fn vector_index_binary(&self) -> Vec<u8> {
        self.hv_index.to_binary()
    }

    /// Replace the hypervector index from a binary blob produced by
    /// [`Self::vector_index_binary`]. Returns `false` (and leaves the index
    /// untouched) if the blob is corrupt — the caller falls back to a full
    /// rebuild from the text WAL.
    pub fn load_vector_index_binary(&mut self, bytes: &[u8]) -> bool {
        match HypervectorIndex::from_binary(bytes) {
            Some(idx) => {
                self.hv_index = idx;
                true
            }
            None => false,
        }
    }

    /// Serialize the full palace (records + facets + hypervector codes) to a
    /// binary blob — the fast-load `.idx` sidecar. Reading this back re-hashes
    /// and re-bundles nothing: codes are stored as raw words and the `lower`
    /// search blob is recomputed from the (already-loaded) fields. This is the
    /// mmap-equivalent zero-copy fast path; the Markdown file remains the
    /// durable, human-readable source of truth.
    pub fn to_binary_full(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(16 + self.records.len() * 160);
        out.extend_from_slice(b"LMFB");
        put_u32(&mut out, 1); // version
        put_u32(&mut out, self.records.len() as u32);
        for r in &self.records {
            let code = self.hv_index.code_of(r.id).unwrap_or_else(Hypervector::zero);
            put_u32(&mut out, r.id as u32);
            out.push(kind_idx(r.kind));
            put_u64(&mut out, r.stamp);
            let (ms, me) = r.mentioned.map_or((-1i64, -1i64), |(s, e)| (s as i64, e as i64));
            put_u64(&mut out, ms as u64);
            put_u64(&mut out, me as u64);
            for w in code.as_words() {
                put_u64(&mut out, *w);
            }
            put_str(&mut out, &r.wing);
            put_str(&mut out, &r.room);
            put_str(&mut out, &r.key);
            put_str(&mut out, &r.summary);
            put_str(&mut out, &r.content);
            put_str(&mut out, &r.lower);
            put_u32(&mut out, r.facets.len() as u32);
            for f in &r.facets {
                out.push(facet_idx(f.facet_type));
                put_str(&mut out, &f.search_text);
                put_u32(&mut out, f.aliases.len() as u32);
                for a in &f.aliases {
                    put_str(&mut out, a);
                }
                put_str(&mut out, &f.description);
            }
        }
        out
    }

    /// Deserialize [`Self::to_binary_full`]. Returns `None` (fail-closed) on a
    /// bad magic, short buffer, or length mismatch — the caller falls back to
    /// the Markdown source of truth.
    pub fn from_binary_full(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < 12 || &bytes[0..4] != b"LMFB" {
            return None;
        }
        let mut r = Reader::new(bytes, 4);
        let version = r.u32()?;
        if version != 1 {
            return None;
        }
        let count = r.u32()? as usize;
        let mut m = Self::new();
        for _ in 0..count {
            let id = r.u32()? as usize;
            let kind = kind_from_idx(r.u8()?);
            let stamp = r.u64()?;
            let ms = r.u64()? as i64;
            let me = r.u64()? as i64;
            let mentioned = if ms < 0 || me < 0 {
                None
            } else {
                Some((ms as u64, me as u64))
            };
            let mut words = [0u64; crate::hypervector::WORDS];
            for w in words.iter_mut() {
                *w = r.u64()?;
            }
            let code = Hypervector::from_words(words);
            let wing = r.str()?;
            let room = r.str()?;
            let key = r.str()?;
            let summary = r.str()?;
            let content = r.str()?;
            let lower = r.str()?;
            // Direct construction: no re-hash, no re-bundle, no re-lowercase —
            // the fields and `lower` blob come straight off the wire.
            debug_assert_eq!(m.records.len(), id, "binary record id must match position");
            m.records.push(MemoryRecord {
                id,
                kind,
                wing,
                room,
                key,
                summary,
                content,
                facets: Vec::new(),
                tags: Vec::new(),
                lower,
                stamp,
                mentioned,
                code_node: None,
            });
            m.hv_index.insert_with_code(code);
            m.stamp = m.stamp.max(stamp);
            let nf = r.u32()? as usize;
            for _ in 0..nf {
                let ft = facet_from_idx(r.u8()?);
                let search_text = r.str()?;
                let na = r.u32()? as usize;
                let mut aliases = Vec::with_capacity(na);
                for _ in 0..na {
                    aliases.push(r.str()?);
                }
                let description = r.str()?;
                m.records[id].facets.push(Facet {
                    facet_type: ft,
                    search_text,
                    aliases,
                    description,
                });
            }
        }
        Some(m)
    }

    /// Serialize a single record (by id) to its persistence line, including the
    /// pre-computed hypervector code — the append-only unit for crash-safe
    /// persistence. Returns `None` if the id is out of range.
    pub fn record_to_line_by_id(&self, id: usize) -> Option<String> {
        let r = self.records.get(id)?;
        let code = self.hv_index.code_of(id).unwrap_or_else(Hypervector::zero);
        Some(record_to_line(r, &code))
    }

    /// Serialize a single record (by id) to its Markdown block — a `##` heading,
    /// a `Summary:` line for humans, and a fenced `record` block carrying the
    /// exact round-trip line (hex code included). The append-only unit for
    /// crash-safe `.md` persistence.
    pub fn record_to_md_block(&self, id: usize) -> Option<String> {
        let r = self.records.get(id)?;
        let code = self.hv_index.code_of(id).unwrap_or_else(Hypervector::zero);
        let mut s = String::new();
        s.push_str(&format!("## {id} · {} · {:?}\n\n", oneline(&r.key), r.kind));
        s.push_str("Summary: ");
        s.push_str(&oneline(&r.summary));
        s.push_str("\n\n```record\n");
        s.push_str(&record_to_line(r, &code));
        s.push_str("\n```\n\n---\n\n");
        Some(s)
    }

    /// Number of records in the palace.
    pub fn record_count(&self) -> usize {
        self.records.len()
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
    /// line, tab-separated, tab/newline/backslash escaped; the record's
    /// hypervector code is appended as a final 256-char hex field so a reload
    /// re-hashes nothing). Round-trips through [`LivingMemory::from_lines`].
    pub fn to_lines(&self) -> String {
        let mut out = String::new();
        for (id, r) in self.records.iter().enumerate() {
            let code = self.hv_index.code_of(id).unwrap_or_else(Hypervector::zero);
            out.push_str(&record_to_line(r, &code));
            out.push('\n');
        }
        out
    }

    /// Load a palace from [`LivingMemory::to_lines`] output.
    pub fn from_lines(text: &str) -> Self {
        let mut m = Self::new();
        for line in text.lines() {
            m.load_record_line(line);
        }
        m
    }

    /// Serialize the palace to a Markdown document (one `## heading` per record
    /// plus a fenced `record` block holding the exact round-trip line, hex code
    /// included). The heading + summary line are for humans; the fence is the
    /// only machine-parsed part. Round-trips through [`LivingMemory::from_md`].
    pub fn to_md(&self) -> String {
        let mut out = String::from("# dowiz living memory\n\n");
        for id in 0..self.records.len() {
            if let Some(block) = self.record_to_md_block(id) {
                out.push_str(&block);
            }
        }
        out
    }

    /// Load a palace from a Markdown document produced by [`LivingMemory::to_md`]
    /// (or a hand-edited one): parses only the fenced `record` blocks, so the
    /// human-readable headings/summaries are free to change without breaking
    /// the round-trip.
    pub fn from_md(text: &str) -> Self {
        let mut m = Self::new();
        let mut in_record = false;
        for line in text.lines() {
            let t = line.trim();
            if t == "```record" {
                in_record = true;
                continue;
            }
            if in_record {
                if t == "```" {
                    in_record = false;
                    continue;
                }
                m.load_record_line(line);
            }
        }
        m
    }

    /// Parse one record line (tab-separated, hex code in field 10) into `self`.
    /// Malformed/empty lines are skipped (fail-closed, like a torn WAL tail).
    fn load_record_line(&mut self, line: &str) {
        if line.is_empty() {
            return;
        }
        let f: Vec<&str> = line.split('\t').collect();
        if f.len() < 10 {
            return;
        }
        let kind = kind_from_idx(f[0].parse().unwrap_or(0));
        let stamp: u64 = f[6].parse().unwrap_or(0);
        let ms: i64 = f[7].parse().unwrap_or(-1);
        let me: i64 = f[8].parse().unwrap_or(-1);
        let mentioned = if ms < 0 || me < 0 {
            None
        } else {
            Some((ms as u64, me as u64))
        };
        // Pre-computed hypervector code (field 10, 256 hex chars). A
        // missing/invalid field (old-format line) falls back to a full
        // term re-hash in `remember_full_inner`.
        let hv_code = f.get(10).and_then(|h| Hypervector::from_hex(h));
        let id = self.remember_full_inner(
            kind,
            &unesc(f[1]),
            &unesc(f[2]),
            &unesc(f[3]),
            &unesc(f[4]),
            &unesc(f[5]),
            mentioned,
            hv_code,
        );
        self.records[id].stamp = stamp;
        // Restore facets (field 9 only — field 10 is the code hex).
        for fs in f[9].split(';').filter(|s| !s.is_empty()) {
            let parts: Vec<&str> = fs.split('|').collect();
            if parts.len() >= 2 {
                let ft = facet_from_idx(parts[0].parse().unwrap_or(0));
                let rec = &mut self.records[id];
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

/// Serialize a single record to one line (the append-only unit for crash-safe
/// persistence — see the kernel's `LivingMemoryStore`). The pre-computed
/// hypervector code is appended as the final 256-char hex field so a reload
/// re-hashes nothing (the cold-start fix).
pub fn record_to_line(r: &MemoryRecord, code: &Hypervector) -> String {
    let (ms, me) = r
        .mentioned
        .map_or((-1i64, -1i64), |(s, e)| (s as i64, e as i64));
    let mut out = String::new();
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
    out.push('\t');
    out.push_str(&code.to_hex());
    out
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

/// Collapse a multi-line string to one line for human-readable Markdown headings
/// (replaces newline/tab with visible markers; the fenced `record` block carries
/// the exact bytes, so this is display-only).
fn oneline(s: &str) -> String {
    s.replace('\n', "⏎").replace('\t', " ")
}

// ─── binary serialization helpers (the `.idx` full-record sidecar) ──────────

fn put_u32(out: &mut Vec<u8>, v: u32) {
    out.extend_from_slice(&v.to_le_bytes());
}

fn put_u64(out: &mut Vec<u8>, v: u64) {
    out.extend_from_slice(&v.to_le_bytes());
}

fn put_str(out: &mut Vec<u8>, s: &str) {
    put_u32(out, s.len() as u32);
    out.extend_from_slice(s.as_bytes());
}

/// Bounds-checked cursor over a byte slice. Every read returns `None` on
/// truncation, so a corrupt sidecar fails closed instead of panicking.
struct Reader<'a> {
    b: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    fn new(b: &'a [u8], pos: usize) -> Self {
        Self { b, pos }
    }

    fn take(&mut self, n: usize) -> Option<&'a [u8]> {
        if self.pos + n > self.b.len() {
            return None;
        }
        let s = &self.b[self.pos..self.pos + n];
        self.pos += n;
        Some(s)
    }

    fn u8(&mut self) -> Option<u8> {
        self.take(1).map(|s| s[0])
    }

    fn u32(&mut self) -> Option<u32> {
        let s = self.take(4)?;
        Some(u32::from_le_bytes([s[0], s[1], s[2], s[3]]))
    }

    fn u64(&mut self) -> Option<u64> {
        let s = self.take(8)?;
        let mut arr = [0u8; 8];
        arr.copy_from_slice(s);
        Some(u64::from_le_bytes(arr))
    }

    fn str(&mut self) -> Option<String> {
        let n = self.u32()? as usize;
        let s = self.take(n)?;
        String::from_utf8(s.to_vec()).ok()
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

    #[test]
    fn vector_codes_survive_persistence_bit_exact() {
        let mut m = LivingMemory::new();
        m.remember(MemoryKind::Semantic, "quantum", "superposition and oracles");
        m.remember(MemoryKind::Semantic, "crypto", "post quantum keygen");
        m.remember(MemoryKind::Episodic, "meeting", "daily standup notes");
        let serialized = m.to_lines();
        let loaded = LivingMemory::from_lines(&serialized);
        // Vector ranking must be identical: the persisted hex code is reloaded
        // bit-exact (no re-hash), so scores match to the bit.
        let q = "quantum superposition oracle";
        assert_eq!(loaded.vector_search(q, 3), m.vector_search(q, 3));
        // And the binary sidecar round-trips identically too.
        let blob = m.vector_index_binary();
        let mut m2 = LivingMemory::new();
        assert!(m2.load_vector_index_binary(&blob));
        assert_eq!(m2.vector_search(q, 3), m.vector_search(q, 3));
    }

    #[test]
    fn search_uses_prelowered_blob_case_insensitive() {
        let mut m = LivingMemory::new();
        let id = m.remember(MemoryKind::Semantic, "QUANTUM", "SuperPosition ORACLES");
        assert!(m.search("quantum").contains(&id));
        assert!(m.search("QUANTUM").contains(&id));
        assert!(m.search("superposition").contains(&id));
        assert!(m.search("oracles").contains(&id));
        assert!(!m.search("zzz").contains(&id));
    }

    #[test]
    fn convolution_search_ranks_like_vector_search_on_top() {
        let mut m = LivingMemory::new();
        let a = m.remember(MemoryKind::Semantic, "quantum", "superposition and oracles");
        let b = m.remember(MemoryKind::Semantic, "crypto", "post quantum keygen");
        let _c = m.remember(MemoryKind::Episodic, "meeting", "daily standup notes");
        let hits = m.convolution_search("quantum superposition oracle", 1);
        assert_eq!(hits[0].0, a);
        let _ = b;
    }

    #[test]
    fn binary_index_rejects_corruption_and_falls_back() {
        let mut m = LivingMemory::new();
        m.remember(MemoryKind::Semantic, "k", "v");
        let good = m.vector_index_binary();
        let mut m2 = LivingMemory::new();
        assert!(m2.load_vector_index_binary(&good));
        assert!(!m2.load_vector_index_binary(b"garbage"));
    }

    #[test]
    fn markdown_round_trips_and_preserves_codes() {
        let mut m = LivingMemory::new();
        m.remember_full(
            MemoryKind::Semantic,
            "project",
            "day-1",
            "auth",
            "auth\tuses\npq",
            "verbatim content",
            Some((100, 200)),
        );
        m.remember(MemoryKind::Semantic, "quantum", "superposition and oracles");
        let md = m.to_md();
        // Human-readable bits present.
        assert!(md.contains("# dowiz living memory"));
        assert!(md.contains("```record"));
        // Round-trip: records and vector codes survive bit-exact.
        let loaded = LivingMemory::from_md(&md);
        assert_eq!(loaded.record_count(), 2);
        assert_eq!(
            loaded.recall_by_key("auth").unwrap().summary,
            "auth\tuses\npq"
        );
        assert_eq!(
            loaded.vector_search("quantum superposition oracle", 2),
            m.vector_search("quantum superposition oracle", 2)
        );
    }

    #[test]
    fn markdown_ignores_hand_edited_headings() {
        let mut m = LivingMemory::new();
        m.remember(MemoryKind::Semantic, "quantum", "superposition and oracles");
        let mut md = m.to_md();
        // A human edits the heading/summary (not the fence).
        md = md.replace("## 0 · quantum · Semantic", "## 0 · my favorite record");
        md = md.replace("Summary: superposition and oracles", "Summary: rewritten by hand");
        let loaded = LivingMemory::from_md(&md);
        // The machine line (fence) is authoritative — data survives the edit.
        assert_eq!(
            loaded.recall_by_key("quantum").unwrap().summary,
            "superposition and oracles"
        );
        assert_eq!(loaded.record_count(), 1);
    }

    #[test]
    fn binary_full_round_trips_exactly() {
        let mut m = LivingMemory::new();
        m.remember_full(
            MemoryKind::Semantic,
            "project",
            "day-1",
            "auth",
            "auth\tuses\npq",
            "verbatim content",
            Some((100, 200)),
        );
        let id = m.remember(MemoryKind::Semantic, "quantum", "superposition and oracles");
        m.add_facet(id, FacetType::Decision, "retire serde_json");
        m.add_facet(id, FacetType::Metric, "3463 tests");
        let blob = m.to_binary_full();
        let loaded = LivingMemory::from_binary_full(&blob).expect("valid binary");
        assert_eq!(loaded.record_count(), m.record_count());
        assert_eq!(
            loaded.recall_by_key("auth").unwrap().summary,
            "auth\tuses\npq"
        );
        assert_eq!(loaded.recall_by_key("quantum").unwrap().facets.len(), 2);
        // Vector ranking is bit-identical (codes survived).
        let q = "quantum superposition oracle";
        assert_eq!(loaded.vector_search(q, 3), m.vector_search(q, 3));
        // Corruption fails closed.
        assert!(LivingMemory::from_binary_full(b"XXXX").is_none());
        assert!(LivingMemory::from_binary_full(&blob[..blob.len() - 3]).is_none());
    }
}
