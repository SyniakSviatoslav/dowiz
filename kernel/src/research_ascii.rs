//! `kernel::research_ascii` — Compact ASCII library for research papers.
//!
//! Converts Papers to compact ASCII format for memory/disk efficiency.
//! Every paper is stored as a single line using ASCII unit separator (0x1F)
//! between fields. Non-ASCII characters are stripped.
//!
//! # ASCII Library Format
//! Each line: `hash|id|title|year|cite|cat|abstract(truncated)`
//! Field separator: 0x1F (ASCII Unit Separator)
//! Within-field: 0x1F → 0x1E (Record Separator) escape
//! Content-addressed: stored by SHA3-256 hash for dedup
//!
//! # Size
//! ~200 bytes per paper in ASCII vs ~2000 bytes in JSON → 10x compression
//! 100K papers ≈ 20MB in ASCII vs 200MB in JSON
//! After dedup: ~15-18MB for 100K unique papers

use crate::event_log::sha3_256;
use crate::research::Paper;
use crate::TriState;

/// ASCII Unit Separator (0x1F) — field delimiter.
const US: char = '\x1F';
/// ASCII Record Separator (0x1E) — escape within fields.
const RS: char = '\x1E';
/// Max abstract length in ASCII library (chars).
const MAX_ABSTRACT_LEN: usize = 400;

// ─── ASCII Conversion ─────────────────────────────────────────────────────

/// Strip non-ASCII characters and escape separator characters.
fn to_ascii(s: &str) -> String {
    s.chars()
        .map(|c| {
            if c == US { RS }          // Escape unit separator
            else if c == RS { RS }     // Escape record separator too
            else if c.is_ascii_graphic() || c == ' ' { c }
            else if c == '\n' || c == '\t' { ' ' }
            else { ' ' }               // Replace non-ASCII with space
        })
        .collect()
}

/// Convert a Paper to a compact ASCII line.
pub fn paper_to_ascii(paper: &Paper) -> String {
    let hash_hex = hex::encode(&paper.paper_hash);
    let id = to_ascii(&paper.id);
    let title = to_ascii(&paper.title);
    let year = paper.year.to_string();
    let cite = paper.citation_count.to_string();
    let cats = to_ascii(&paper.categories.join(","));
    let abstract_short = to_ascii(&paper.abstract_text.chars().take(MAX_ABSTRACT_LEN).collect::<String>());

    format!("{}{US}{}{US}{}{US}{}{US}{}{US}{}{US}{}",
        hash_hex, id, title, year, cite, cats, abstract_short,
        US = US)
}

/// Parse an ASCII library line back into a Paper.
pub fn ascii_to_paper(line: &str) -> Option<Paper> {
    let parts: Vec<&str> = line.split(US).collect();
    if parts.len() < 7 { return None; }

    let hash_hex = parts[0].trim();
    let id_str = parts[1].to_string();
    let title = parts[2].to_string();
    let year: u32 = parts[3].trim().parse().unwrap_or(0);
    let cite: u32 = parts[4].trim().parse().unwrap_or(0);
    let _cats = parts[5].to_string();

    let mut hash = [0u8; 32];
    if hash_hex.len() == 64 {
        for i in 0..32 {
            hash[i] = u8::from_str_radix(&hash_hex[2*i..2*i+2], 16).unwrap_or(0);
        }
    } else {
        hash = sha3_256(title.as_bytes());
    }

    Some(Paper {
        id: id_str.clone(),
        title,
        authors: vec![],
        abstract_text: parts[6..].join(&US.to_string()),
        categories: vec![],
        year, citation_count: cite,
        arxiv_id: Some(id_str),
        doi: None,
        paper_hash: hash,
        full_text_accessible: TriState::True,
        embedding: vec![],
    })
}

// ─── ASCII Library ─────────────────────────────────────────────────────────

/// Content-addressed ASCII library for research papers.
/// Deduplicates by SHA3-256 hash: identical papers map to one storage slot.
#[derive(Debug, Clone)]
pub struct AsciiLibrary {
    /// Papers stored as ASCII lines.
    pub entries: Vec<String>,
    /// Hash → index map for dedup.
    hash_index: std::collections::HashMap<[u8; 32], usize>,
    /// Max entries (memory cap).
    max_entries: usize,
}

impl AsciiLibrary {
    pub fn new(max_entries: usize) -> Self {
        AsciiLibrary {
            entries: Vec::with_capacity(max_entries.min(1000)),
            hash_index: std::collections::HashMap::new(),
            max_entries,
        }
    }

    /// Add a paper to the library (deduplicated by SHA3-256 hash).
    /// Returns true if the paper was new (inserted), false if duplicate.
    pub fn add(&mut self, paper: &Paper) -> bool {
        if self.hash_index.contains_key(&paper.paper_hash) {
            return false; // Duplicate.
        }
        if self.entries.len() >= self.max_entries {
            return false; // Library full.
        }

        let ascii_line = paper_to_ascii(paper);
        let idx = self.entries.len();
        self.entries.push(ascii_line);
        self.hash_index.insert(paper.paper_hash, idx);
        true
    }

    /// Add many papers at once.
    pub fn add_batch(&mut self, papers: &[Paper]) -> usize {
        let mut new_count = 0;
        for p in papers {
            if self.add(p) { new_count += 1; }
        }
        new_count
    }

    /// Open library: load from a file (one ASCII line per paper).
    pub fn open(path: &str) -> Result<Self, String> {
        let content = std::fs::read_to_string(path).map_err(|e| format!("read error: {}", e))?;
        let lines: Vec<&str> = content.lines().filter(|l| !l.trim().is_empty()).collect();
        let mut lib = AsciiLibrary::new(lines.len());
        for line in lines {
            if let Some(paper) = ascii_to_paper(line) {
                lib.hash_index.insert(paper.paper_hash, lib.entries.len());
                lib.entries.push(line.to_string());
            }
        }
        Ok(lib)
    }

    /// Save library to a file (one ASCII line per paper).
    pub fn save(&self, path: &str) -> Result<(), String> {
        let content = self.entries.join("\n");
        std::fs::write(path, &content).map_err(|e| format!("write error: {}", e))
    }

    /// Number of unique papers.
    pub fn len(&self) -> usize { self.entries.len() }
    pub fn is_empty(&self) -> bool { self.entries.is_empty() }

    /// Estimated memory usage (bytes).
    pub fn estimated_bytes(&self) -> usize {
        self.entries.iter().map(|l| l.len()).sum()
    }

    /// Compact ASCII library persistence string.
    /// Each line: `hash_hex|id|title|year|cite|cats|abstract_short`
    pub fn to_string(&self) -> String { self.entries.join("\n") }

    pub fn dashboard(&self) -> String {
        let mb = self.estimated_bytes() as f64 / 1_000_000.0;
        format!(
            "ASCII Library\n  Papers: {}\n  Size:   {:.2} MB\n  Dedup:  {} unique\n  Hash:   SHA3-256",
            self.entries.len(), mb, self.hash_index.len()
        )
    }
}

// ─── hex module (simple hex encode/decode, zero-dep) ───────────────────────

mod hex {
    pub fn encode(bytes: &[u8]) -> String {
        const HEX_CHARS: &[u8; 16] = b"0123456789abcdef";
        let mut out = vec![0u8; bytes.len() * 2];
        for (i, &b) in bytes.iter().enumerate() {
            out[2*i] = HEX_CHARS[(b >> 4) as usize];
            out[2*i + 1] = HEX_CHARS[(b & 0x0f) as usize];
        }
        unsafe { String::from_utf8_unchecked(out) }
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::research::Paper;

    fn make_paper(id: &str, title: &str, year: u32) -> Paper {
        let hash = sha3_256(title.as_bytes());
        Paper {
            id: id.to_string(),
            title: title.to_string(),
            authors: vec![],
            abstract_text: format!("Abstract for {}", title),
            categories: vec!["cs.LG".into()],
            year, citation_count: 10,
            arxiv_id: Some(id.to_string()),
            doi: None,
            paper_hash: hash,
            full_text_accessible: TriState::True,
            embedding: vec![],
        }
    }

    #[test]
    fn ascii_roundtrip() {
        let p = make_paper("1706.03762", "Attention Is All You Need", 2017);
        let ascii = paper_to_ascii(&p);
        let p2 = ascii_to_paper(&ascii).unwrap();
        assert_eq!(p2.title, "Attention Is All You Need");
        assert_eq!(p2.year, 2017);
        assert_eq!(p2.paper_hash, p.paper_hash);
    }

    #[test]
    fn non_ascii_stripped() {
        let p = make_paper("test", "Tést with ünicode 🚀 and π", 2024);
        let ascii = paper_to_ascii(&p);
        assert!(!ascii.contains('π'));
        assert!(!ascii.contains('🚀'));
        assert!(ascii.contains("T st with  nicode   and  "));
    }

    #[test]
    fn library_dedup() {
        let mut lib = AsciiLibrary::new(100);
        let p1 = make_paper("a", "Same Title", 2024);
        let p2 = make_paper("b", "Same Title", 2024); // Same title → same hash
        assert!(lib.add(&p1));
        assert!(!lib.add(&p2)); // Dedup should reject
        assert_eq!(lib.len(), 1);
    }

    #[test]
    fn library_save_load() {
        let mut lib = AsciiLibrary::new(100);
        lib.add(&make_paper("a", "Paper A", 2023));
        lib.add(&make_paper("b", "Paper B", 2024));

        let path = "/tmp/test_ascii_lib.txt";
        lib.save(path).unwrap();
        let loaded = AsciiLibrary::open(path).unwrap();
        assert_eq!(loaded.len(), 2);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn library_capacity_limit() {
        let mut lib = AsciiLibrary::new(2);
        lib.add(&make_paper("a", "A", 2020));
        lib.add(&make_paper("b", "B", 2021));
        assert!(!lib.add(&make_paper("c", "C", 2022))); // Capacity reached
        assert_eq!(lib.len(), 2);
    }

    #[test]
    fn estimated_bytes_grows() {
        let mut lib = AsciiLibrary::new(10);
        lib.add(&make_paper("a", "Hello World", 2024));
        assert!(lib.estimated_bytes() > 0);
    }

    #[test]
    fn dashboard_contains_library() {
        let lib = AsciiLibrary::new(100);
        let d = lib.dashboard();
        assert!(d.contains("ASCII Library"));
    }

    #[test]
    fn ascii_separator_escaped() {
        let p = make_paper("test", format!("Title with {} separator", US).as_str(), 2024);
        let ascii = paper_to_ascii(&p);
        // The separator within the title should be escaped.
        assert!(ascii.contains(RS));
    }

    #[test]
    fn batch_add_counts_new() {
        let mut lib = AsciiLibrary::new(100);
        let papers = vec![
            make_paper("a", "Unique A", 2024),
            make_paper("b", "Unique B", 2024),
            make_paper("a2", "Unique A", 2024), // Duplicate
        ];
        let new = lib.add_batch(&papers);
        assert_eq!(new, 2);
        assert_eq!(lib.len(), 2);
    }
}
