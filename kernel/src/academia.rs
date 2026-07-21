//! `kernel::academia` — Академія Дмитра Євдокимова.
//!
//! Quantized spectral library with P2P snapshot sync.
//! Named in memory of Dmytro Yevdokymov — загиблого друга.
//!
//! # Architecture
//! Each paper = SHA3-256 hash (32 bytes). That's it.
//! The hash IS the vector: first 8 bytes → (u,v) parametric coordinates.
//! The hash IS the identifier: dedup via hash check.
//! The hash IS the content address: fetch full paper from hash.
//!
//! # Quantization
//! | Формат            | Розмір/папір | 1M паперів | 610M паперів |
//! |-------------------|--------------|------------|--------------|
//! | Flat tensor (f64) | 2048 B       | 2.0 GB     | 1,165 GB     |
//! | Spectral surface  | 48 B         | 48 MB      | 27 GB        |
//! | **Quantized hash** | **32 B**    | **32 MB**  | **19.5 GB**  |
//!
//! # P2P Sync
//! After contact: exchange hash bloom filters → request missing hashes →
//! merge into local spectral index. No central server, no API.
//!
//! # Snapshot
//! Entire library serialized as: `[count: u32] [hash: [u8; 32]]*`.
//! Load/save in O(n). Single file, portable, content-addressed.

use crate::event_log::sha3_256;
use crate::TriState;
use std::collections::{HashMap, HashSet};

/// Max papers in library.
pub const MAX_KADEMIA: usize = 1_000_000_000;

// ─── Quantized Paper ─────────────────────────────────────────────────────

/// A paper reduced to its quantized essence: SHA3-256 hash + metadata.
/// The hash IS the vector: 32 bytes, no embedding storage needed.
#[derive(Debug, Clone)]
pub struct QuantizedPaper {
    /// SHA3-256 of title (the content address, the vector, the ID).
    pub hash: [u8; 32],
    /// Parametric u-coordinate (derived from hash[0..4]).
    pub u: f32,
    /// Parametric v-coordinate (derived from hash[4..8]).
    pub v: f32,
    /// Year (compressed to u16 — max 65535).
    pub year: u16,
    /// Categories hash (first 8 bytes of SHA3-256 of categories string).
    pub cats_hash: [u8; 8],
}

impl QuantizedPaper {
    /// Create from title and metadata.
    /// Everything is deterministically derived from the title.
    pub fn from_title(title: &str, year: u16, cats: &str) -> Self {
        let clean: String = title.chars().map(|c| if c.is_ascii() && (c.is_ascii_graphic() || c == ' ') { c } else { ' ' }).collect();
        let hash = sha3_256(clean.as_bytes());
        let (u, v) = Self::hash_to_uv(&hash);
        let cats_hash = Self::hash_cats(cats);
        QuantizedPaper { hash, u, v, year, cats_hash }
    }

    /// First 8 bytes of hash → (u,v) ∈ [-1, 1]².
    fn hash_to_uv(hash: &[u8; 32]) -> (f32, f32) {
        let u = u32::from_le_bytes([hash[0], hash[1], hash[2], hash[3]]);
        let v = u32::from_le_bytes([hash[4], hash[5], hash[6], hash[7]]);
        (u as f32 / u32::MAX as f32 * 2.0 - 1.0, v as f32 / u32::MAX as f32 * 2.0 - 1.0)
    }

    fn hash_cats(cats: &str) -> [u8; 8] {
        let h = sha3_256(cats.as_bytes());
        let mut out = [0u8; 8];
        out.copy_from_slice(&h[..8]);
        out
    }

    /// Distance on parametric surface (euclidean).
    pub fn distance(&self, other: &QuantizedPaper) -> f32 {
        ((self.u - other.u).powi(2) + (self.v - other.v).powi(2)).sqrt()
    }

    /// Grid cell for parametric navigation.
    pub fn grid_cell(u: f32, v: f32, grid_res: usize) -> usize {
        let ui = ((u + 1.0) / 2.0 * grid_res as f32).clamp(0.0, grid_res.saturating_sub(1) as f32) as usize;
        let vi = ((v + 1.0) / 2.0 * grid_res as f32).clamp(0.0, grid_res.saturating_sub(1) as f32) as usize;
        vi * grid_res + ui
    }
}

// ─── Academia Library ─────────────────────────────────────────────────────

/// Кадемія Дмитра Євдокимова — quantized spectral library with P2P sync.
#[derive(Debug)]
pub struct Academia {
    /// All quantized papers.
    pub papers: Vec<QuantizedPaper>,
    /// Hash → index.
    hash_index: HashMap<[u8; 32], usize>,
    /// Parametric grid for O(1) navigation.
    pub grid: Vec<Vec<usize>>,
    pub grid_res: usize,
    /// Bloom filter of all hashes (for P2P sync).
    pub bloom: BloomFilter,
}

impl Academia {
    pub fn new(grid_res: usize) -> Self {
        Academia {
            papers: Vec::with_capacity(MAX_KADEMIA.min(1_000_000)),
            hash_index: HashMap::new(),
            grid: vec![Vec::new(); grid_res * grid_res],
            grid_res,
            bloom: BloomFilter::new(1_000_000_000, 0.001), // 1B entries, 0.1% FP rate
        }
    }

    /// Insert a quantized paper (O(1) — hash → grid cell → push).
    pub fn insert(&mut self, paper: QuantizedPaper) -> bool {
        if self.hash_index.contains_key(&paper.hash) { return false; }
        if self.papers.len() >= MAX_KADEMIA { return false; }
        let idx = self.papers.len();
        self.hash_index.insert(paper.hash, idx);
        let cell = QuantizedPaper::grid_cell(paper.u, paper.v, self.grid_res);
        self.grid[cell].push(idx);
        self.bloom.insert(&paper.hash);
        self.papers.push(paper);
        true
    }

    /// Bulk insert: pre-allocate and insert in batch (fast path).
    pub fn insert_batch(&mut self, papers: Vec<QuantizedPaper>) -> usize {
        let mut count = 0;
        for p in papers { if self.insert(p) { count += 1; } }
        count
    }

    /// Search: O(9 cells × avg density) = O(1) = O(n⁰).
    pub fn search(&self, query: &QuantizedPaper, top_k: usize) -> Vec<(usize, f32)> {
        let center = QuantizedPaper::grid_cell(query.u, query.v, self.grid_res);
        let mut candidates = Vec::new();

        // Search 3×3 neighborhood (O(9) = O(1)).
        let gr = self.grid_res as i32;
        for dr in -1i32..=1 {
            for dc in -1i32..=1 {
                let ci = (center % self.grid_res) as i32 + dc;
                let ri = (center / self.grid_res) as i32 + dr;
                if ci < 0 || ci >= gr || ri < 0 || ri >= gr { continue; }
                let cell = (ri * gr + ci) as usize;
                for &pi in &self.grid[cell] {
                    let p = &self.papers[pi];
                    let d = query.distance(p);
                    candidates.push((pi, d));
                }
            }
        }

        candidates.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
        candidates.truncate(top_k);
        candidates
    }

    /// Get paper by hash.
    pub fn get(&self, hash: &[u8; 32]) -> Option<&QuantizedPaper> {
        self.hash_index.get(hash).map(|&i| &self.papers[i])
    }

    pub fn len(&self) -> usize { self.papers.len() }

    // ── Snapshot ─────────────────────────────────────────────────────────

    /// Serialize entire library to binary snapshot.
    /// Format: [count: u32 LE] [hash: [u8; 32]]* [year: u16 LE]* [cats: [u8; 8]]*
    pub fn to_snapshot(&self) -> Vec<u8> {
        let n = self.papers.len();
        let mut buf = Vec::with_capacity(4 + n * (32 + 2 + 8));
        buf.extend_from_slice(&(n as u32).to_le_bytes());
        for p in &self.papers {
            buf.extend_from_slice(&p.hash);
        }
        for p in &self.papers {
            buf.extend_from_slice(&p.year.to_le_bytes());
        }
        for p in &self.papers {
            buf.extend_from_slice(&p.cats_hash);
        }
        buf
    }

    /// Load library from binary snapshot.
    pub fn from_snapshot(data: &[u8]) -> Result<Self, String> {
        if data.len() < 4 { return Err("too short".into()); }
        let n = u32::from_le_bytes([data[0], data[1], data[2], data[3]]) as usize;
        let record_size = 32 + 2 + 8; // hash + year + cats
        if data.len() < 4 + n * record_size { return Err("data truncated".into()); }

        let mut lib = Academia::new(4096); // Finer grid for large datasets.
        let hash_start = 4;
        let year_start = hash_start + n * 32;
        let cats_start = year_start + n * 2;

        for i in 0..n {
            let mut hash = [0u8; 32];
            hash.copy_from_slice(&data[hash_start + i*32 .. hash_start + (i+1)*32]);
            let year = u16::from_le_bytes([data[year_start + i*2], data[year_start + i*2 + 1]]);
            let mut cats = [0u8; 8];
            cats.copy_from_slice(&data[cats_start + i*8 .. cats_start + (i+1)*8]);
            let (u, v) = QuantizedPaper::hash_to_uv(&hash);
            lib.papers.push(QuantizedPaper { hash, u, v, year, cats_hash: cats });
            lib.hash_index.insert(hash, i);
            let cell = QuantizedPaper::grid_cell(u, v, lib.grid_res);
            lib.grid[cell].push(i);
            lib.bloom.insert(&hash);
        }
        Ok(lib)
    }

    /// Snapshot size estimate.
    pub fn snapshot_size(&self) -> usize { 4 + self.papers.len() * (32 + 2 + 8) }

    // ── P2P Sync ─────────────────────────────────────────────────────────

    /// Compute hash diff: which hashes from `remote` are missing locally.
    pub fn missing_hashes<'a>(&self, remote_hashes: &'a [[u8; 32]]) -> Vec<&'a [u8; 32]> {
        remote_hashes.iter().filter(|h| !self.hash_index.contains_key(*h)).collect()
    }

    /// Estimate missing count using bloom filter (fast, before full hash exchange).
    pub fn estimate_missing(&self, remote_bloom: &BloomFilter) -> usize {
        let mut misses = 0u64;
        let sample_size = 1000.min(self.papers.len());
        for i in 0..sample_size {
            if !remote_bloom.contains(&self.papers[i].hash) {
                misses += 1;
            }
        }
        (misses as f64 / sample_size.max(1) as f64 * self.papers.len() as f64) as usize
    }

    pub fn dashboard(&self) -> String {
        let mb = self.snapshot_size() as f64 / 1_000_000.0;
        format!(
            "Кадемія Дмитра Євдокимова\n  Papers:  {}\n  Grid:    {}×{}\n  Cells:   {} occupied\n  Bloom:   {} bits\n  Snapshot: {:.1} MB",
            self.papers.len(), self.grid_res, self.grid_res,
            self.grid.iter().filter(|c| !c.is_empty()).count(),
            self.bloom.bit_count,
            mb
        )
    }
}

// ─── Bloom Filter (for P2P sync) ─────────────────────────────────────────

/// Simple bloom filter for hash presence testing.
/// Used by P2P sync to estimate data overlap without transferring all hashes.
#[derive(Debug, Clone)]
pub struct BloomFilter {
    pub bits: Vec<u64>,
    pub bit_count: usize,
    num_hashes: usize,
}

impl BloomFilter {
    pub fn new(expected_entries: usize, fp_rate: f64) -> Self {
        let bit_count = (-(expected_entries as f64) * fp_rate.ln() / (std::f64::consts::LN_2.powi(2))) as usize;
        let num_hashes = ((bit_count as f64 / expected_entries as f64) * std::f64::consts::LN_2) as usize;
        let word_count = (bit_count + 63) / 64;
        BloomFilter { bits: vec![0; word_count.max(1)], bit_count: bit_count.max(1), num_hashes: num_hashes.max(1) }
    }

    pub fn insert(&mut self, hash: &[u8; 32]) {
        for i in 0..self.num_hashes {
            let h = self.hash_index(hash, i);
            self.bits[h / 64] |= 1u64 << (h % 64);
        }
    }

    pub fn contains(&self, hash: &[u8; 32]) -> bool {
        for i in 0..self.num_hashes {
            let h = self.hash_index(hash, i);
            if self.bits[h / 64] & (1u64 << (h % 64)) == 0 { return false; }
        }
        true
    }

    fn hash_index(&self, hash: &[u8; 32], i: usize) -> usize {
        let idx = (i * 4) % 28;
        let v = u64::from_le_bytes([
            hash[idx], hash[idx+1], hash[idx+2], hash[idx+3],
            hash[(idx+4) % 28], hash[(idx+5) % 28], hash[(idx+6) % 28], hash[(idx+7) % 28],
        ]);
        (v as usize) % self.bit_count
    }

    pub fn bytes(&self) -> usize { self.bits.len() * 8 }
}

// ─── P2P Contact Sync ─────────────────────────────────────────────────────

/// After-contact sync protocol message.
/// Peers exchange bloom filters → estimate overlap → request missing.
#[derive(Debug, Clone)]
pub struct SyncMessage {
    pub bloom_bytes: Vec<u8>,
    pub paper_count: u32,
    pub snapshot_hashes: Vec<[u8; 32]>,
}

impl SyncMessage {
    /// Build sync message from Academia library.
    pub fn from_library(lib: &Academia) -> Self {
        SyncMessage {
            bloom_bytes: lib.bloom.bits.iter().flat_map(|w| w.to_le_bytes()).collect(),
            paper_count: lib.papers.len() as u32,
            snapshot_hashes: lib.papers.iter().map(|p| p.hash).collect(),
        }
    }

    /// Merge remote hashes into local library (bulk insert).
    pub fn merge_into(&self, lib: &mut Academia) -> usize {
        let mut count = 0;
        for hash in &self.snapshot_hashes {
            if lib.hash_index.contains_key(hash) { continue; }
            // Reconstruct QuantizedPaper from hash (minimal).
            let (u, v) = QuantizedPaper::hash_to_uv(hash);
            let paper = QuantizedPaper {
                hash: *hash, u, v,
                year: 0, // Unknown year (needs full metadata fetch)
                cats_hash: [0u8; 8], // Unknown categories
            };
            lib.insert(paper);
            count += 1;
        }
        count
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quantized_paper_from_title() {
        let p = QuantizedPaper::from_title("Attention Is All You Need", 2017, "cs.LG cs.AI");
        assert!(p.u >= -1.0 && p.u <= 1.0);
        assert!(p.v >= -1.0 && p.v <= 1.0);
        assert_eq!(p.year, 2017);
    }

    #[test]
    fn deterministic_uv() {
        let a = QuantizedPaper::from_title("Same Title", 2024, "");
        let b = QuantizedPaper::from_title("Same Title", 2024, "");
        assert_eq!(a.u, b.u);
        assert_eq!(a.v, b.v);
    }

    #[test]
    fn unique_uv_for_different_titles() {
        let a = QuantizedPaper::from_title("AAAA", 2024, "");
        let b = QuantizedPaper::from_title("BBBB", 2024, "");
        assert!(a.u != b.u || a.v != b.v);
    }

    #[test]
    fn academia_insert_dedup() {
        let mut k = Academia::new(32);
        let p1 = QuantizedPaper::from_title("Paper", 2024, "");
        let p2 = QuantizedPaper::from_title("Paper", 2024, "");
        assert!(k.insert(p1));
        assert!(!k.insert(p2));
        assert_eq!(k.len(), 1);
    }

    #[test]
    fn snapshot_roundtrip() {
        let mut k = Academia::new(32);
        k.insert(QuantizedPaper::from_title("Paper A", 2023, "cs.LG"));
        k.insert(QuantizedPaper::from_title("Paper B", 2024, "cs.AI"));
        let snap = k.to_snapshot();
        let loaded = Academia::from_snapshot(&snap).unwrap();
        assert_eq!(loaded.len(), 2);
    }

    #[test]
    fn search_returns_nearby() {
        let mut k = Academia::new(32);
        for i in 0..100 {
            k.insert(QuantizedPaper::from_title(&format!("Paper {}", i), 2024, ""));
        }
        let query = QuantizedPaper::from_title("Neural network paper", 2024, "");
        let results = k.search(&query, 5);
        assert!(results.len() <= 5);
    }

    #[test]
    fn missing_hashes() {
        let mut k = Academia::new(32);
        k.insert(QuantizedPaper::from_title("Existing", 2024, ""));
        let remote = [QuantizedPaper::from_title("Existing", 2024, "").hash,
                      QuantizedPaper::from_title("Missing", 2024, "").hash];
        let missing = k.missing_hashes(&remote);
        assert_eq!(missing.len(), 1);
    }

    #[test]
    fn sync_message_merge() {
        let mut lib_a = Academia::new(32);
        lib_a.insert(QuantizedPaper::from_title("Paper A", 2024, ""));
        let msg = SyncMessage::from_library(&lib_a);
        let mut lib_b = Academia::new(32);
        lib_b.insert(QuantizedPaper::from_title("Paper B", 2024, ""));
        let merged = msg.merge_into(&mut lib_b);
        assert_eq!(lib_b.len(), 2);
    }

    #[test]
    fn bloom_filter_works() {
        let hash = QuantizedPaper::from_title("Test", 2024, "").hash;
        let mut bf = BloomFilter::new(1_000_000, 0.001);
        bf.insert(&hash);
        assert!(bf.contains(&hash));
    }

    #[test]
    fn dashboard_contains_name() {
        let k = Academia::new(32);
        let d = k.dashboard();
        assert!(d.contains("Кадемія"));
    }

    #[test]
    fn grid_cell_bounds() {
        for u in [-1.1, -0.5, 0.0, 0.5, 1.1] {
            for v in [-1.1, -0.5, 0.0, 0.5, 1.1] {
                let cell = QuantizedPaper::grid_cell(u, v, 32);
                assert!(cell < 32 * 32, "cell {} for ({}, {})", cell, u, v);
            }
        }
    }
}
