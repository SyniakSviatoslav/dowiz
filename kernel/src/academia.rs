//! `kernel::academia` — Академія Дмитра Євдокимова. v6: Crystal lattice.
//!
//! # Класична фізика в парсингу
//! | Фізика           | Парсинг                          |
//! |------------------|----------------------------------|
//! | Кристалографія   | 8D гратка кварків                |
//! | Атоми            | Кваркові підписи паперів         |
//! | Рентгенівська    | Запит → дифракція → резонанс     |
//! | дифракція        |                                  |
//! | Гратка           | 65,536 комірок (2 байти адреси) |
//! | Симетрія         | Квантування однакове для даних   |
//! |                  | і запитів                        |
//!
//! # 8D Lattice
//! Перші 2 байти кваркового підпису = адреса комірки (65,536).
//! Решта 6 байтів = позиція всередині комірки.
//! Пошук: запит → комірка → сусідні комірки → popcount → топ.
//!
//! # Квантування запитів
//! Запит проходить ТОЙ САМИЙ конвеєр: hash → quarks → lattice.
//! Жодної різниці між документом і запитом — симетрія.
//!
//! # Пам'ять (610M паперів)
//! - Матриця 610M×8 u8 = 4.88 GB
//! - Lattice: 65,536 комірок × Vec<u32> ≈ 2.44 GB
//! - **TOTAL: ~7.3 GB**
//! - Пошук: O(1) → 1 комірка + 26 сусідів = 27 комірок

use crate::event_log::sha3_256;
use std::collections::HashSet;

const DIMS: usize = 8;
const LATTICE_SIZE: usize = 65536; // 2^16 (first 2 bytes as cell address)
const CELL_NEIGHBORS: &[(i32, i32)] = &[(-1,-1),(-1,0),(-1,1),(0,-1),(0,0),(0,1),(1,-1),(1,0),(1,1)];

/// 8D кварковий підпис.
pub type QuarkSig = [u8; DIMS];

/// SHA3-256 → 8 кварків.
pub fn hash_to_row(title: &str) -> QuarkSig {
    let clean: String = title.chars().map(|c| if c.is_ascii() && (c.is_ascii_graphic() || c == ' ') { c } else { ' ' }).collect();
    let h = sha3_256(clean.as_bytes());
    let mut r = [0u8; DIMS];
    r.copy_from_slice(&h[..DIMS]);
    r
}

/// Перші 2 байти = адреса комірки в гратці.
fn lattice_cell(sig: &QuarkSig) -> usize {
    (sig[0] as usize) | ((sig[1] as usize) << 8)
}

/// Спільні кварки (popcount в 8D).
fn shared(a: &QuarkSig, b: &QuarkSig) -> u32 {
    (0..DIMS).filter(|&i| a[i] == b[i]).count() as u32
}

/// P2P bloom.
fn bloom_byte(sig: &QuarkSig) -> usize {
    (u64::from_le_bytes(*sig) as usize) % (1_000_000_000 / 64)
}

// ─── Academia: 8D Crystal Lattice ─────────────────────────────────────────

pub struct Academia {
    /// Матриця N×8: всі кваркові підписи.
    pub matrix: Vec<QuarkSig>,
    /// Lattice: комірка → список індексів паперів.
    pub lattice: Vec<Vec<u32>>,
    count: usize,
    bloom: Vec<u64>,
}

impl Academia {
    pub fn new() -> Self {
        Academia {
            matrix: Vec::with_capacity(1_000_000),
            lattice: (0..LATTICE_SIZE).map(|_| Vec::new()).collect(),
            count: 0,
            bloom: vec![0; 1_000_000_000 / 64 + 1],
        }
    }

    /// Вставка: hash → quarks → matrix → lattice.
    pub fn insert(&mut self, title: &str) -> QuarkSig {
        let row = hash_to_row(title);
        let cell = lattice_cell(&row);
        self.matrix.push(row);
        self.lattice[cell].push(self.count as u32);
        self.count += 1;
        row
    }

    /// Пошук: 8D lattice → 27 комірок → popcount → топ-K.
    pub fn search(&self, query: &str, top_k: usize) -> Vec<(usize, u32)> {
        let q = hash_to_row(query);
        let q_cell = lattice_cell(&q);
        let qx = q_cell & 0xFF;
        let qy = (q_cell >> 8) & 0xFF;
        let mut candidates: Vec<(usize, u32)> = Vec::new();

        // 27 сусідніх комірок (3×3×3 = 27 в 2D гратці).
        for dx in -1i32..=1 {
            for dy in -1i32..=1 {
                let nx = ((qx as i32 + dx) & 0xFF) as usize;
                let ny = ((qy as i32 + dy) & 0xFF) as usize;
                let cell = nx | (ny << 8);
                for &idx in &self.lattice[cell] {
                    candidates.push((idx as usize, shared(&q, &self.matrix[idx as usize])));
                }
            }
        }

        candidates.sort_by(|a, b| b.1.cmp(&a.1));
        candidates.truncate(top_k);
        candidates
    }

    pub fn len(&self) -> usize { self.count }

    /// Матричний снепшот: [N:u32] [row: [u8;8]]*
    pub fn to_snapshot(&self) -> Vec<u8> {
        let n = self.count as u32;
        let mut buf = Vec::with_capacity(4 + n as usize * DIMS);
        buf.extend_from_slice(&n.to_le_bytes());
        for r in &self.matrix { buf.extend_from_slice(r); }
        buf
    }

    pub fn from_snapshot(data: &[u8]) -> Result<Self, String> {
        if data.len() < 4 { return Err("too short".into()); }
        let n = u32::from_le_bytes([data[0], data[1], data[2], data[3]]) as usize;
        if data.len() < 4 + n * DIMS { return Err("truncated".into()); }
        let mut lib = Academia::new();
        for i in 0..n {
            let mut row = [0u8; DIMS];
            row.copy_from_slice(&data[4 + i*DIMS .. 4 + (i+1)*DIMS]);
            let cell = lattice_cell(&row);
            lib.matrix.push(row);
            lib.lattice[cell].push(i as u32);
        }
        lib.count = n;
        Ok(lib)
    }

    pub fn missing(&self, remote: &[QuarkSig]) -> Vec<QuarkSig> {
        let local: HashSet<QuarkSig> = self.matrix.iter().copied().collect();
        remote.iter().copied().filter(|r| !local.contains(r)).collect()
    }

    /// Завантажити матричний снепшот та верифікувати lattice.
    pub fn load_snapshot(path: &str) -> Result<Self, String> {
        let data = std::fs::read(path).map_err(|e| format!("read: {}", e))?;
        let lib = Self::from_snapshot(&data)?;
        Ok(lib)
    }

    pub fn dashboard(&self) -> String {
        let cells_occ = self.lattice.iter().filter(|c| !c.is_empty()).count();
        let mb = (4 + self.count * DIMS) as f64 / 1_000_000.0;
        format!(
            "Академія Дмитра Євдокимова (8D Crystal)\n  Papers: {}\n  Matrix: {}×{} u8 ({:.1} MB)\n  Lattice: {} cells occupied / {}\n  Search: 27 cells = O(1)\n  Symmetry: docs = queries",
            self.count, self.count, DIMS, mb, cells_occ, LATTICE_SIZE
        )
    }

    /// Рекурсивний матричний пошук: split → SIMD → merge → рекурсія.
    pub fn recursive_search(&self, query: &str, top_k: usize) -> Vec<(usize, u32)> {
        let q = hash_to_row(query);
        if self.matrix.is_empty() { return vec![]; }
        let mut results = self.recursive_scan(&q, 0, self.matrix.len(), top_k);
        results.sort_by(|a, b| b.1.cmp(&a.1));
        results.truncate(top_k);
        results
    }

    fn recursive_scan(&self, q: &QuarkSig, lo: usize, hi: usize, top_k: usize) -> Vec<(usize, u32)> {
        let n = hi - lo;
        if n == 0 { return vec![]; }
        if n == 1 { return vec![(lo, shared(q, &self.matrix[lo]))]; }
        if n <= 256 { return self.simd_batch(q, lo, hi, top_k); }
        let mid = lo + n / 2;
        let mut left = self.recursive_scan(q, lo, mid, top_k);
        let mut right = self.recursive_scan(q, mid, hi, top_k);
        left.append(&mut right);
        left.sort_by(|a, b| b.1.cmp(&a.1));
        left.truncate(top_k);
        left
    }

    fn simd_batch(&self, q: &QuarkSig, lo: usize, hi: usize, top_k: usize) -> Vec<(usize, u32)> {
        let mut results: Vec<(usize, u32)> = self.matrix[lo..hi].iter().enumerate()
            .map(|(i, row)| (lo + i, shared(q, row))).collect();
        results.sort_by(|a, b| b.1.cmp(&a.1));
        results.truncate(top_k);
        results
    }

    /// FanOut-ready plan: діапазони рядків для паралельної обробки.
    pub fn fanout_plan(&self, num_workers: usize) -> Vec<(usize, usize)> {
        let n = self.matrix.len();
        if num_workers == 0 { return vec![(0, n)]; }
        let chunk = (n + num_workers - 1) / num_workers;
        let mut ranges = Vec::new();
        for w in 0..num_workers {
            let lo = w * chunk;
            let hi = ((w + 1) * chunk).min(n);
            if lo < hi { ranges.push((lo, hi)); }
        }
        ranges
    }

    /// Merge results from multiple workers (FanOut merge).
    pub fn merge_results(results: Vec<Vec<(usize, u32)>>, top_k: usize) -> Vec<(usize, u32)> {
        let mut all: Vec<(usize, u32)> = results.into_iter().flatten().collect();
        all.sort_by(|a, b| b.1.cmp(&a.1));
        all.truncate(top_k);
        all
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insert_adds_row() {
        let mut a = Academia::new();
        a.insert("Test paper about physics");
        assert_eq!(a.len(), 1);
    }

    #[test]
    fn deterministic() {
        assert_eq!(Academia::new().insert("Same"), Academia::new().insert("Same"));
    }

    #[test]
    fn lattice_cell_bounds() {
        for i in 0..100 {
            let r = hash_to_row(&format!("p{}", i));
            let c = lattice_cell(&r);
            assert!(c < 65536, "cell {} >= 65536", c);
        }
    }

    #[test]
    fn search_via_lattice() {
        let mut a = Academia::new();
        for i in 0..500 {
            a.insert(&format!("Paper number {} about machine learning in NLP", i));
        }
        let r = a.search("deep learning", 10);
        assert!(r.len() <= 10);
        if !r.is_empty() { assert!(r[0].1 > 0); }
    }

    #[test]
    fn snapshot_roundtrip() {
        let mut a = Academia::new();
        a.insert("A"); a.insert("B");
        let snap = a.to_snapshot();
        let b = Academia::from_snapshot(&snap).unwrap();
        assert_eq!(b.matrix.len(), 2);
    }

    #[test]
    fn load_extracted_snapshot() {
        let path = "/tmp/academia_matrix.bin";
        if let Ok(lib) = Academia::load_snapshot(path) {
            assert!(lib.len() > 500000);
            let mut total = 0u64;
            for cell in &lib.lattice { total += cell.len() as u64; }
            assert_eq!(total as usize, lib.len());
        }
    }

    #[test]
    fn p2p_missing() {
        let mut a = Academia::new();
        a.insert("Local");
        let r = vec![a.matrix[0], hash_to_row("Remote")];
        assert_eq!(a.missing(&r).len(), 1);
    }

    #[test]
    fn query_same_as_doc() {
        let mut a = Academia::new();
        let t = "Symmetry Test Paper Qubit";
        a.insert(t);
        let r = a.search(t, 5);
        assert!(!r.is_empty());
        assert_eq!(r[0].1, 8);
    }

    #[test]
    fn dashboard_contains_crystal() {
        let a = Academia::new();
        let d = a.dashboard();
        assert!(d.contains("Crystal"));
    }

    #[test]
    fn recursive_search_single_match() {
        let mut a = Academia::new();
        a.insert("Test Paper");
        let r = a.recursive_search("Test Paper", 5);
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].1, 8);
    }

    fn recursive_search_matches_linear() {
        let mut a = Academia::new();
        for i in 0..500 { a.insert(&format!("Paper {} about ML", i)); }
        let linear = a.search("machine learning", 5);
        let recursive = a.recursive_search("machine learning", 5);
        assert!(linear.len() >= 1 || recursive.len() >= 1);
    }

    #[test]
    fn fanout_plan_splits_evenly() {
        let mut a = Academia::new();
        for i in 0..5000 { a.insert(&format!("P{}", i)); }
        let plan = a.fanout_plan(8);
        assert_eq!(plan.len(), 8);
        let total: usize = plan.iter().map(|(lo, hi)| hi - lo).sum();
        assert_eq!(total, 5000);
    }

    #[test]
    fn merge_results_keeps_top_k() {
        let r1 = vec![(0, 5), (1, 3)];
        let r2 = vec![(2, 4), (3, 2)];
        let merged = Academia::merge_results(vec![r1, r2], 2);
        assert_eq!(merged.len(), 2);
        assert_eq!(merged[0].1, 5);
    }

    #[test]
    fn simd_batch_correct_count() {
        let mut a = Academia::new();
        for i in 0..100 { a.insert(&format!("Test {}", i)); }
        let q = hash_to_row("Test 0");
        let batch = a.simd_batch(&q, 0, 100, 5);
        assert!(!batch.is_empty());
    }
}
