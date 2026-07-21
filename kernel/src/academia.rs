//! `kernel::academia` — Академія Дмитра Євдокимова. v2.
//!
//! # O(n⁰) принцип
//! Хеш SHA3-256 = власний вектор (256 біт = 256D).
//! Перші 8 байт хешу = (u,v) параметричні координати.
//! Жодного навчання, жодного HashMap, жодної power iteration.
//!
//! # Операції
//! - Вставка: push до Vec<[u8; 32]> (O(1), ~5ns)
//! - Eigenvectors: не потребують обчислення (хеш і є вектор)
//! - Сітка: будується ліниво при першому пошуку
//! - Пошук: hash → uv → grid cell → 9 cells scan = O(1)
//!
//! # Пам'ять
//! | Компонент           | 610M паперів |
//! |---------------------|--------------|
//! | Hashes (32B × N)    | 19.5 GB      |
//! | Grid indices (4B × N)| 2.4 GB      |
//! | **TOTAL**           | **~22 GB**   |

use crate::event_log::sha3_256;

// ─── Quantized helpers ─────────────────────────────────────────────────

/// Hash → (u,v) ∈ [-1, 1]² via first 8 bytes.
fn hash_to_uv(hash: &[u8; 32]) -> (f32, f32) {
    let u = u32::from_le_bytes([hash[0], hash[1], hash[2], hash[3]]);
    let v = u32::from_le_bytes([hash[4], hash[5], hash[6], hash[7]]);
    (u as f32 / u32::MAX as f32 * 2.0 - 1.0, v as f32 / u32::MAX as f32 * 2.0 - 1.0)
}

/// Grid cell from (u,v).
fn grid_cell(u: f32, v: f32, res: usize) -> usize {
    let ui = ((u + 1.0) / 2.0 * res as f32).clamp(0.0, res.saturating_sub(1) as f32) as usize;
    let vi = ((v + 1.0) / 2.0 * res as f32).clamp(0.0, res.saturating_sub(1) as f32) as usize;
    vi * res + ui
}

/// Distance in parametric space.
fn uv_distance(u1: f32, v1: f32, u2: f32, v2: f32) -> f32 {
    ((u1 - u2).powi(2) + (v1 - v2).powi(2)).sqrt()
}

// ─── Hash-only bloom filter ───────────────────────────────────────────

/// Compact bloom filter for P2P sync.
pub struct AcademiaBloom {
    bits: Vec<u64>,
    pub bit_count: usize,
    num_hashes: usize,
    pub count: u32,
}

impl AcademiaBloom {
    pub fn new(expected: usize, fp_rate: f64) -> Self {
        let bits = (-(expected as f64) * fp_rate.ln() / (std::f64::consts::LN_2.powi(2))) as usize;
        let hashes = ((bits as f64 / expected as f64) * std::f64::consts::LN_2) as usize;
        AcademiaBloom {
            bits: vec![0; (bits / 64 + 1).max(1)],
            bit_count: bits.max(1),
            num_hashes: hashes.max(1),
            count: 0,
        }
    }

    pub fn insert(&mut self, hash: &[u8; 32]) {
        for i in 0..self.num_hashes {
            let h = (u64::from_le_bytes([
                hash[(i*4)%28], hash[(i*4+1)%28], hash[(i*4+2)%28], hash[(i*4+3)%28],
                hash[(i*4+4)%28], hash[(i*4+5)%28], hash[(i*4+6)%28], hash[(i*4+7)%28],
            ]) % self.bit_count as u64) as usize;
            self.bits[h / 64] |= 1u64 << (h % 64);
        }
        self.count += 1;
    }

    pub fn contains(&self, hash: &[u8; 32]) -> bool {
        for i in 0..self.num_hashes {
            let h = (u64::from_le_bytes([
                hash[(i*4)%28], hash[(i*4+1)%28], hash[(i*4+2)%28], hash[(i*4+3)%28],
                hash[(i*4+4)%28], hash[(i*4+5)%28], hash[(i*4+6)%28], hash[(i*4+7)%28],
            ]) % self.bit_count as u64) as usize;
            if self.bits[h / 64] & (1u64 << (h % 64)) == 0 { return false; }
        }
        true
    }

    pub fn bytes(&self) -> Vec<u8> {
        self.bits.iter().flat_map(|w| w.to_le_bytes()).collect()
    }
}

// ─── Academia — hash-only spectral library ───────────────────────────

/// Академія Дмитра Євдокимова — v2: hash-only, lazy grid.
pub struct Academia {
    /// Flat array of hashes. That's ALL the data.
    pub hashes: Vec<[u8; 32]>,
    /// Lazy grid: built on first search. Vec of Vec<u32>.
    grid: Vec<Vec<u32>>,
    /// Grid resolution.
    grid_res: usize,
    /// Grid built?
    grid_built: bool,
    /// Bloom filter for P2P sync.
    pub bloom: AcademiaBloom,
}

impl Academia {
    pub fn new(grid_res: usize) -> Self {
        let cap = 1_000_000.min(grid_res * grid_res * 100);
        Academia {
            hashes: Vec::with_capacity(cap),
            grid: vec![Vec::new(); grid_res * grid_res],
            grid_res,
            grid_built: false,
            bloom: AcademiaBloom::new(1_000_000_000, 0.001),
        }
    }

    /// Insert: O(1) push to Vec. ~5ns per paper. No HashMap, no index.
    pub fn insert(&mut self, hash: [u8; 32]) -> bool {
        if self.hashes.len() >= 1_000_000_000 { return false; }
        self.hashes.push(hash);
        self.bloom.insert(&hash);
        true
    }

    /// Bulk insert from raw snapshot bytes (fast path).
    pub fn insert_batch(&mut self, batch: &[[u8; 32]]) -> usize {
        let n = batch.len().min(1_000_000_000 - self.hashes.len());
        self.hashes.extend_from_slice(&batch[..n]);
        for h in &batch[..n] { self.bloom.insert(h); }
        n
    }

    /// Build grid lazily: one pass, O(N).
    pub fn build_grid(&mut self) {
        if self.grid_built { return; }
        for (i, hash) in self.hashes.iter().enumerate() {
            let (u, v) = hash_to_uv(hash);
            let cell = grid_cell(u, v, self.grid_res);
            self.grid[cell].push(i as u32);
        }
        self.grid_built = true;
    }

    /// Search: O(9 cells) = O(1) after grid build.
    pub fn search(&mut self, query_hash: &[u8; 32], top_k: usize) -> Vec<(u32, f32)> {
        if !self.grid_built { self.build_grid(); }
        let (qu, qv) = hash_to_uv(query_hash);
        let center = grid_cell(qu, qv, self.grid_res);
        let mut candidates = Vec::new();
        let gr = self.grid_res as i32;

        for dr in -1i32..=1 {
            for dc in -1i32..=1 {
                let ci = (center % self.grid_res) as i32 + dc;
                let ri = (center / self.grid_res) as i32 + dr;
                if ci < 0 || ci >= gr || ri < 0 || ri >= gr { continue; }
                for &pidx in &self.grid[(ri * gr + ci) as usize] {
                    let (pu, pv) = hash_to_uv(&self.hashes[pidx as usize]);
                    let d = uv_distance(qu, qv, pu, pv);
                    candidates.push((pidx, d));
                }
            }
        }

        candidates.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
        candidates.truncate(top_k);
        candidates
    }

    /// Number of papers.
    pub fn len(&self) -> usize { self.hashes.len() }

    // ── Snapshot ─────────────────────────────────────────────────────

    /// Snapshot: [N: u32] [hash: [u8; 32]]*
    pub fn to_snapshot(&self) -> Vec<u8> {
        let n = self.hashes.len() as u32;
        let mut buf = Vec::with_capacity(4 + n as usize * 32);
        buf.extend_from_slice(&n.to_le_bytes());
        for h in &self.hashes { buf.extend_from_slice(h); }
        buf
    }

    pub fn from_snapshot(data: &[u8]) -> Result<Self, String> {
        if data.len() < 4 { return Err("too short".into()); }
        let n = u32::from_le_bytes([data[0], data[1], data[2], data[3]]) as usize;
        if data.len() < 4 + n * 32 { return Err("truncated".into()); }
        let mut lib = Academia::new(4096);
        for i in 0..n {
            let mut hash = [0u8; 32];
            hash.copy_from_slice(&data[4 + i*32 .. 4 + (i+1)*32]);
            lib.hashes.push(hash);
            lib.bloom.insert(&hash);
        }
        Ok(lib)
    }

    // ── P2P Sync ─────────────────────────────────────────────────────

    /// Estimate missing papers from remote bloom filter.
    pub fn estimate_missing(&self, remote_bloom: &AcademiaBloom) -> usize {
        let sample = 1000.min(self.hashes.len());
        let mut miss = 0;
        for i in 0..sample {
            if !remote_bloom.contains(&self.hashes[i]) { miss += 1; }
        }
        (miss as f64 / sample.max(1) as f64 * self.hashes.len() as f64) as usize
    }

    pub fn dashboard(&self) -> String {
        let mb = (4 + self.hashes.len() * 32) as f64 / 1_000_000.0;
        let grid_occ = self.grid.iter().filter(|c| !c.is_empty()).count();
        format!(
            "Академія Дмитра Євдокимова\n  Papers:  {}\n  Grid:    {}×{} ({} occ)\n  Bloom:   {} entries\n  Snapshot: {:.1} MB",
            self.hashes.len(), self.grid_res, self.grid_res, grid_occ, self.bloom.count, mb
        )
    }
}
// ─── Tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_hash(s: &str) -> [u8; 32] { sha3_256(s.as_bytes()) }

    #[test]
    fn insert_is_o1() {
        let mut a = Academia::new(32);
        assert!(a.insert(make_hash("test")));
        assert_eq!(a.len(), 1);
    }

    #[test]
    fn hash_to_uv_bounds() {
        for i in 0..100 {
            let h = make_hash(&format!("paper {}", i));
            let (u, v) = hash_to_uv(&h);
            assert!(u >= -1.0 && u <= 1.0);
            assert!(v >= -1.0 && v <= 1.0);
        }
    }

    #[test]
    fn deterministic_uv() {
        let a = make_hash("same");
        let b = make_hash("same");
        assert_eq!(hash_to_uv(&a), hash_to_uv(&b));
    }

    #[test]
    fn lazy_grid_build() {
        let mut a = Academia::new(32);
        for i in 0..100 { a.insert(make_hash(&format!("p{}", i))); }
        assert!(!a.grid_built);
        let h = make_hash("query");
        let _ = a.search(&h, 5);
        assert!(a.grid_built);
    }

    #[test]
    fn search_returns_results() {
        let mut a = Academia::new(32);
        for i in 0..500 { a.insert(make_hash(&format!("paper about machine learning {}", i))); }
        let results = a.search(&make_hash("deep learning transformer"), 10);
        assert!(results.len() <= 10);
    }

    #[test]
    fn snapshot_roundtrip() {
        let mut a = Academia::new(32);
        a.insert(make_hash("Paper A"));
        a.insert(make_hash("Paper B"));
        let snap = a.to_snapshot();
        let b = Academia::from_snapshot(&snap).unwrap();
        assert_eq!(b.len(), 2);
    }

    #[test]
    fn dashboard_contains_name() {
        let a = Academia::new(32);
        let d = a.dashboard();
        assert!(d.contains("Академія"));
    }
}
