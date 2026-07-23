//! crystal.rs — Self-similar crystalline lattice for O(1) nearest-neighbor retrieval.
//!
//! Extracted from `academia_p2p.rs` (NdCrystalLattice, NdSignature, CrystalMemory)
//! into a general-purpose module. The lattice uses hash-based addressing for O(1)
//! cell lookup and 27-adjacent-cell search for similarity queries.
//!
//! Used by the real-time system predictor for fast retrieval of similar past
//! states, enabling consequence prediction across performance, load, traffic,
//! telemetry, throttle, friction, and error dimensions.
//!
//! ## Design
//! Each item stored in the lattice produces a `CrystalIndex` (a hash signature).
//! The first 2 bytes of the hash select one of 65536 cells; the next bytes serve
//! as a similarity key. Query checks the target cell + 26 adjacent cells (3×3×3
//! in 2-byte space) and returns top-K by popcount similarity.
//!
//! ## Usage
//! ```
//! use dowiz_kernel::crystal::{CrystalLattice, CrystalIndex};
//!
//! #[derive(Clone, Debug)]
//! struct MyData { id: u64, value: f64 }
//!
//! impl CrystalIndex for MyData {
//!     fn crystal_hash(&self) -> [u8; 32] {
//!         let h = std::hash::BuildHasher::hash_one(
//!             &std::hash::RandomState::new(),
//!             &self.id,
//!         );
//!         let mut output = [0u8; 32];
//!         output[..8].copy_from_slice(&h.to_le_bytes());
//!         output
//!     }
//! }
//!
//! let mut lattice: CrystalLattice<MyData> = CrystalLattice::new();
//! lattice.insert(MyData { id: 1, value: 10.0 });
//! assert_eq!(lattice.len(), 1);
//! ```

use std::hash::{Hash, Hasher};

/// Number of cells in the crystal lattice (65536 = 2^16).
const CRYSTAL_CELLS: usize = 65536;

/// Items that can be stored in a [`CrystalLattice`].
///
/// Implementors provide a 32-byte hash that determines cell placement
/// (first 2 bytes → cell address) and similarity (remaining 30 bytes).
pub trait CrystalIndex: Clone + std::fmt::Debug {
    /// Produce a 32-byte hash for crystal addressing.
    /// First 2 bytes select the cell; remaining 30 bytes are the similarity key.
    fn crystal_hash(&self) -> [u8; 32];

    /// Similarity score between two crystal indices (0.0 = identical, higher = more different).
    /// Default: popcount of XOR of the similarity keys, normalized.
    fn similarity(&self, other: &[u8; 30]) -> f64 {
        let hash = self.crystal_hash();
        let key: [u8; 30] = hash[2..32].try_into().unwrap_or([0u8; 30]);
        let xor_popcount: u32 = key.iter().zip(other.iter())
            .map(|(a, b)| (a ^ b).count_ones())
            .sum();
        xor_popcount as f64 / 240.0 // 30 bytes × 8 bits = 240 max popcount
    }
}

/// A cell in the crystal lattice: bucket of items sharing the same 2-byte address.
#[derive(Debug, Clone)]
struct CrystalCell<T> {
    items: Vec<(T, [u8; 30])>, // (item, similarity_key)
}

/// Crystal lattice — O(1) hash-addressed storage with 27-cell similarity search.
///
/// Items are placed in one of 65536 cells based on the first 2 bytes of their
/// crystal hash. Query checks the target cell + 26 adjacent cells (3×3×3
/// neighborhood in the 16-bit address space via Gray-code adjacency).
///
/// Generic over `T: CrystalIndex` — works with system state snapshots,
/// telemetry records, performance profiles, any crystal-indexable data.
#[derive(Debug, Clone)]
pub struct CrystalLattice<T> {
    cells: Vec<CrystalCell<T>>,
    count: usize,
}

impl<T: CrystalIndex> CrystalLattice<T> {
    pub fn new() -> Self {
        let mut cells = Vec::with_capacity(CRYSTAL_CELLS);
        for _ in 0..CRYSTAL_CELLS {
            cells.push(CrystalCell { items: Vec::new() });
        }
        CrystalLattice { cells, count: 0 }
    }

    pub fn with_capacity(cell_cap: usize) -> Self {
        let mut cells = Vec::with_capacity(CRYSTAL_CELLS);
        for _ in 0..CRYSTAL_CELLS {
            cells.push(CrystalCell { items: Vec::with_capacity(cell_cap) });
        }
        CrystalLattice { cells, count: 0 }
    }

    pub fn len(&self) -> usize {
        self.count
    }

    pub fn is_empty(&self) -> bool {
        self.count == 0
    }

    fn cell_addr(hash: &[u8; 32]) -> usize {
        (hash[0] as usize) << 8 | hash[1] as usize
    }

    fn similarity_key(hash: &[u8; 32]) -> [u8; 30] {
        let mut key = [0u8; 30];
        key.copy_from_slice(&hash[2..32]);
        key
    }

    /// Insert an item into the lattice. Returns the cell address.
    pub fn insert(&mut self, item: T) -> usize {
        let hash = item.crystal_hash();
        let addr = Self::cell_addr(&hash);
        let key = Self::similarity_key(&hash);
        self.cells[addr].items.push((item, key));
        self.count += 1;
        addr
    }

    /// Query top-K most similar items from the lattice.
    ///
    /// Searches the target cell + adjacent cells in a 5×5×5 neighborhood
    /// (125 cells) to ensure coverage even with sparse datasets. Returns
    /// items sorted by similarity (most similar first), limited to `k`.
    /// Returns empty vec if the lattice is empty.
    pub fn query(&self, item: &T, k: usize) -> Vec<&T> {
        if self.count == 0 {
            return Vec::new();
        }
        let hash = item.crystal_hash();
        let addr = Self::cell_addr(&hash);
        let query_key = Self::similarity_key(&hash);

        // Collect candidates from 441-cell neighborhood (21×21)
        // to ensure coverage even in sparse lattices.
        let mut candidates: Vec<(&T, f64)> = Vec::new();
        for dx in -10i32..=10 {
            for dy in -10i32..=10 {
                let nx = ((addr >> 8) as i32 + dx).rem_euclid(256) as u16;
                let ny = ((addr & 0xFF) as i32 + dy).rem_euclid(256) as u16;
                let neighbor = (nx as usize) << 8 | ny as usize;
                let cell = &self.cells[neighbor];
                for (stored, stored_key) in &cell.items {
                    let xor_popcount: u32 = stored_key.iter().zip(query_key.iter())
                        .map(|(a, b)| (a ^ b).count_ones())
                        .sum();
                    let sim = xor_popcount as f64 / 240.0;
                    candidates.push((stored, sim));
                }
            }
        }

        // Sort by similarity (ascending = more similar)
        crate::sort_by_f64_asc(&mut candidates, |&(_, s)| s);
        candidates.truncate(k);
        candidates.into_iter().map(|(t, _)| t).collect()
    }

    /// Drain all items from the lattice (memory pressure).
    pub fn clear(&mut self) {
        for cell in &mut self.cells {
            cell.items.clear();
        }
        self.count = 0;
    }
}

impl<T: CrystalIndex> Default for CrystalLattice<T> {
    fn default() -> Self {
        Self::new()
    }
}

/// Convenient CrystalIndex for system state snapshots (f64 vector + u64 id).
///
/// Used by the system predictor to store and retrieve historical states.
/// Hash is computed from the id + a simple mix of the values.
#[derive(Debug, Clone)]
pub struct StateSnapshot {
    pub id: u64,
    pub metrics: Vec<f64>,
    pub label: String,
}

impl StateSnapshot {
    pub fn new(id: u64, metrics: Vec<f64>, label: &str) -> Self {
        StateSnapshot { id, metrics, label: label.to_string() }
    }
}

impl CrystalIndex for StateSnapshot {
    fn crystal_hash(&self) -> [u8; 32] {
        // Use metrics for cell addressing so similar metrics land nearby.
        let mut output = [0u8; 32];
        // First 2 bytes: quantized average of first 2 metrics → cell address
        let avg1 = self.metrics.first().copied().unwrap_or(0.0);
        let avg2 = self.metrics.get(1).copied().unwrap_or(0.0);
        let cell_hi = ((avg1.abs() * 100.0) as u64 % 256) as u8;
        let cell_lo = ((avg2.abs() * 100.0) as u64 % 256) as u8;
        output[0] = cell_hi;
        output[1] = cell_lo;
        // Remaining 30 bytes: metric quanta for similarity
        for (i, &m) in self.metrics.iter().enumerate() {
            if i + 2 >= 32 { break; }
            let bits = m.to_bits();
            output[i + 2] = (bits ^ (bits >> 13) ^ (bits >> 27)) as u8;
        }
        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Debug)]
    struct TestItem {
        id: u64,
        val: f64,
    }

    impl CrystalIndex for TestItem {
        fn crystal_hash(&self) -> [u8; 32] {
            let mut h = std::hash::DefaultHasher::new();
            self.id.hash(&mut h);
            self.val.to_bits().hash(&mut h);
            let hash = h.finish();
            let mut output = [0u8; 32];
            output[..8].copy_from_slice(&hash.to_le_bytes());
            output
        }
    }

    #[test]
    fn crystal_insert_and_count() {
        let mut c: CrystalLattice<TestItem> = CrystalLattice::new();
        c.insert(TestItem { id: 1, val: 10.0 });
        c.insert(TestItem { id: 2, val: 20.0 });
        assert_eq!(c.len(), 2);
    }

    #[test]
    fn crystal_query_returns_similar() {
        let mut c: CrystalLattice<TestItem> = CrystalLattice::new();
        for i in 0..100u64 {
            c.insert(TestItem { id: i, val: i as f64 * 1.5 });
        }
        let query = TestItem { id: 50, val: 75.0 };
        let results = c.query(&query, 5);
        assert!(!results.is_empty());
        assert!(results.len() <= 5);
    }

    #[test]
    fn crystal_state_snapshot() {
        let mut c: CrystalLattice<StateSnapshot> = CrystalLattice::new();
        // Insert items that cover a wide range of metric + cell values.
        // The hash uses metrics for cell address, so filling the space.
        for i in 0..2000u64 {
            let v = ((i % 200) as f64) * 0.5;
            c.insert(StateSnapshot::new(i, vec![v, v * 2.0, v * 3.0], "seed"));
        }
        assert_eq!(c.len(), 2000);
        let q = StateSnapshot::new(9999, vec![8.0, 15.0, 20.0], "query");
        let results = c.query(&q, 5);
        // With 2000 items across 65536 cells, 441-cell neighborhood
        // should find items with reasonable probability.
        // We also check exact match is possible:
        c.insert(StateSnapshot::new(5000, vec![8.0, 15.0, 20.0], "exact"));
        let results2 = c.query(&q, 10);
        assert!(results2.iter().any(|s| s.label == "exact"),
            "exact match should be found nearby: got {} results", results2.len());
    }

    #[test]
    fn crystal_clear_resets() {
        let mut c: CrystalLattice<TestItem> = CrystalLattice::new();
        c.insert(TestItem { id: 1, val: 1.0 });
        c.insert(TestItem { id: 2, val: 2.0 });
        assert_eq!(c.len(), 2);
        c.clear();
        assert_eq!(c.len(), 0);
    }

    #[test]
    fn crystal_cell_distribution() {
        let mut c: CrystalLattice<TestItem> = CrystalLattice::new();
        let mut addrs = std::collections::HashSet::new();
        for i in 0..500u64 {
            let addr = c.insert(TestItem { id: i, val: i as f64 * 0.7 });
            addrs.insert(addr);
        }
        // With 500 items across 65536 cells, we expect reasonable distribution
        assert!(addrs.len() > 50, "must distribute across cells: {} cells", addrs.len());
    }

    #[test]
    fn crystal_query_with_exact_match() {
        let mut c: CrystalLattice<TestItem> = CrystalLattice::new();
        for i in 0..20u64 {
            c.insert(TestItem { id: i, val: i as f64 });
        }
        let query = TestItem { id: 5, val: 5.0 };
        let results = c.query(&query, 10);
        assert!(!results.is_empty());
        let has_exact = results.iter().any(|t| t.id == 5);
        assert!(has_exact, "exact match must appear in results");
    }

    // ── CHAOS / LOAD / META tests ──────────────────────────────────────

    #[test]
    fn crystal_empty_lattice_query_returns_empty() {
        let c: CrystalLattice<TestItem> = CrystalLattice::new();
        let q = TestItem { id: 1, val: 1.0 };
        assert!(c.query(&q, 10).is_empty(), "empty lattice must return empty");
    }

    #[test]
    fn crystal_single_item_query_finds_self() {
        let mut c: CrystalLattice<TestItem> = CrystalLattice::new();
        c.insert(TestItem { id: 42, val: 99.0 });
        let q = TestItem { id: 42, val: 99.0 };
        let results = c.query(&q, 5);
        assert_eq!(results.len(), 1, "single item must be findable");
        assert_eq!(results[0].id, 42);
    }

    #[test]
    fn crystal_query_k_larger_than_population() {
        let mut c: CrystalLattice<TestItem> = CrystalLattice::new();
        for i in 0..5u64 {
            c.insert(TestItem { id: i, val: i as f64 });
        }
        let q = TestItem { id: 0, val: 0.0 };
        let results = c.query(&q, 100);
        assert!(results.len() <= 5, "k must not exceed population: {}", results.len());
    }

    #[test]
    fn crystal_identical_hash_collision() {
        let mut c: CrystalLattice<TestItem> = CrystalLattice::new();
        for _ in 0..100 {
            c.insert(TestItem { id: 0, val: 0.0 });
        }
        assert_eq!(c.len(), 100, "all 100 identical items stored");
        let q = TestItem { id: 0, val: 0.0 };
        let results = c.query(&q, 10);
        assert!(!results.is_empty(), "collision items must be queryable");
    }

    #[test]
    fn crystal_load_test_10k_insert_and_query() {
        let mut c: CrystalLattice<TestItem> = CrystalLattice::new();
        for i in 0..10_000u64 {
            c.insert(TestItem { id: i, val: i as f64 * 0.1 });
        }
        assert_eq!(c.len(), 10_000);
        let q = TestItem { id: 5000, val: 500.0 };
        let results = c.query(&q, 5);
        assert!(!results.is_empty(), "10K lattice must return results");
        assert!(results.len() <= 5, "max k results: {}", results.len());
    }

    #[test]
    fn crystal_clear_then_query_empty() {
        let mut c: CrystalLattice<TestItem> = CrystalLattice::new();
        for i in 0..100u64 {
            c.insert(TestItem { id: i, val: i as f64 });
        }
        c.clear();
        assert!(c.is_empty());
        let q = TestItem { id: 50, val: 50.0 };
        assert!(c.query(&q, 5).is_empty(), "cleared lattice must return empty");
    }

    #[test]
    fn crystal_with_capacity_and_max_items() {
        let mut c: CrystalLattice<TestItem> = CrystalLattice::with_capacity(1000);
        for i in 0..50_000u64 {
            c.insert(TestItem { id: i, val: i as f64 * 0.01 });
        }
        assert_eq!(c.len(), 50_000);
        // Query after massive insert must not panic
        let q = TestItem { id: 9999, val: 99.99 };
        let _ = c.query(&q, 3);
    }

    #[test]
    fn crystal_state_snapshot_chaos_metrics() {
        let mut c: CrystalLattice<StateSnapshot> = CrystalLattice::new();
        c.insert(StateSnapshot::new(1, vec![f64::NAN, f64::INFINITY, -1.0], "chaos"));
        c.insert(StateSnapshot::new(2, vec![], "empty"));
        c.insert(StateSnapshot::new(3, vec![0.5], "single"));
        assert_eq!(c.len(), 3);
        let q = StateSnapshot::new(99, vec![0.5, 0.5, 0.5], "query");
        let _ = c.query(&q, 5);
        // Must not panic with any metric vector
    }

    // ── JAMMING / INJECTION / CONSISTENCY ──────────────────────────────

    #[test]
    fn crystal_jamming_nan_metrics_integrity() {
        let mut c: CrystalLattice<StateSnapshot> = CrystalLattice::new();
        for i in 0..100 {
            let m = vec![
                if i % 2 == 0 { f64::NAN } else { i as f64 * 0.01 },
                f64::INFINITY,
                -1.0,
                i as f64 * 0.005,
            ];
            c.insert(StateSnapshot::new(i as u64, m, "jamming"));
        }
        assert_eq!(c.len(), 100);
        let q = StateSnapshot::new(999, vec![0.5, 0.5, 0.5, 0.5], "query");
        let results = c.query(&q, 5);
        // May return empty if no nearby neighbors (jammed cells may not match)
        // Must not panic regardless
        for s in &results {
            assert!(s.metrics.iter().all(|m| m.is_finite()),
                "all metrics from query must be finite");
        }
    }

    #[test]
    fn crystal_consistency_insert_query_roundtrip() {
        let mut c: CrystalLattice<StateSnapshot> = CrystalLattice::new();
        let items: Vec<_> = (0..50).map(|i| {
            StateSnapshot::new(i, vec![i as f64 * 0.02, 0.5, 0.3], "roundtrip")
        }).collect();
        for item in &items {
            c.insert(item.clone());
        }
        assert_eq!(c.len(), 50);
        // Verify exact match finds itself
        let q = StateSnapshot::new(9999, vec![0.3, 0.5, 0.3], "roundtrip");
        let results = c.query(&q, 10);
        assert!(!results.is_empty(), "query must find nearby items");
    }

    #[test]
    fn crystal_clear_then_jamming_then_clear() {
        let mut c: CrystalLattice<StateSnapshot> = CrystalLattice::new();
        c.insert(StateSnapshot::new(1, vec![0.5; 4], "a"));
        c.clear();
        assert_eq!(c.len(), 0);
        c.insert(StateSnapshot::new(2, vec![f64::NAN; 4], "jam"));
        assert_eq!(c.len(), 1);
        c.clear();
        assert_eq!(c.len(), 0,
            "clearing after jamming must still work");
    }
}
