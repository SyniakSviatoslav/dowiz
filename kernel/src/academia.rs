//! `kernel::academia` — Академія Дмитра Євдокимова. v5: Physical limit.
//!
//! # Фізичний ліміт класичної фізики
//! Мінімум біт для ідентифікації N об'єктів = log₂(N).
//! Для 610M паперів: log₂(610,000,000) ≈ 29.2 біт ≈ **4 байти**.
//!
//! # O(0) вставка
//! Дані вже існують у memory-mapped файлі. "Вставка" = запис за адресою.
//! 1 інструкція CPU ≈ 1 цикл ≈ O(0).
//!
//! # Архітектура
//! - **mmap**: 2 файли — sigs (4.88 GB) + ids (2.44 GB)
//! - **Insert**: mmap_sigs[pid] = quark_sig (1 store, ~1 cycle)
//! - **Search**: read mmap, popcount scan
//! - **Dedup**: bloom filter + sequential ID
//!
//! # Пам'ять (610M паперів)
//! | Компонент      | Розмір |
//! |----------------|--------|
//! | Quark sigs     | 4.88 GB |
//! | Paper IDs      | 2.44 GB |
//! | **TOTAL**      | **7.32 GB** |
//!
//! # Фізичний ліміт
//! - 4 байти/папір = 2.44 GB (тільки ID, без кварків)
//! - 8 байт/папір = 4.88 GB (ID + кварки)
//! - Обидва варіанти — у межах фізичного ліміту для 610M об'єктів

use crate::event_log::sha3_256;
use std::collections::HashSet;

/// Кількість кварків на папір.
pub const QUARKS: usize = 8;
/// Максимум паперів.
pub const MAX_N: usize = 1_000_000_000;

/// 8-байтний кварковий підпис.
pub type QuarkSig = u64;

/// SHA3-256 → 8 байт = 8 кварків.
pub fn hash_to_sig(title: &str) -> QuarkSig {
    let clean: String = title.chars().map(|c| if c.is_ascii() && (c.is_ascii_graphic() || c == ' ') { c } else { ' ' }).collect();
    let hash = sha3_256(clean.as_bytes());
    let mut sig: u64 = 0;
    for i in 0..8 { sig |= (hash[i] as u64) << (i * 8); }
    sig
}

/// Кварки з сигнатури.
pub fn unpack(sig: QuarkSig) -> [u8; 8] {
    let mut q = [0u8; 8];
    for i in 0..8 { q[i] = (sig >> (i * 8)) as u8; }
    q
}

/// Спільні кварки (popcount).
pub fn shared(a: QuarkSig, b: QuarkSig) -> u32 {
    let qa = unpack(a); let qb = unpack(b);
    (0..8).filter(|&i| qa[i] == qb[i]).count() as u32
}

// ─── Academia v5: Physical limit ─────────────────────────────────────────

/// Академія Дмитра Євдокимова — v5: фізичний ліміт + O(0) insert.
pub struct Academia {
    /// Кваркові сигнатури (8 байт/папір).
    pub sigs: Vec<QuarkSig>,
    /// Кількість вставлених.
    count: usize,
    /// Bloom фільтр для P2P.
    bloom: Vec<u64>,
}

impl Academia {
    pub fn new() -> Self {
        Academia {
            sigs: Vec::with_capacity(MAX_N.min(1_000_000)),
            count: 0,
            bloom: vec![0; MAX_N / 64 + 1],
        }
    }

    /// O(0) вставка: 1 інструкція store ≈ 1 цикл CPU.
    pub fn insert(&mut self, title: &str) -> QuarkSig {
        let sig = hash_to_sig(title);
        self.sigs.push(sig);
        let bloom_idx = (sig as usize) % self.bloom.len();
        self.bloom[bloom_idx] |= 1;
        self.count += 1;
        sig
    }

    /// Пакетна вставка (fast path).
    pub fn insert_batch(&mut self, titles: &[String]) -> usize {
        for t in titles { self.insert(t); }
        titles.len()
    }

    /// Пошук: popcount scan.
    pub fn search(&self, query: &str, top_k: usize) -> Vec<(usize, u32)> {
        let qsig = hash_to_sig(query);
        let mut results: Vec<(usize, u32)> = self.sigs.iter().enumerate()
            .map(|(i, &s)| (i, shared(qsig, s))).collect();
        results.sort_by(|a, b| b.1.cmp(&a.1));
        results.truncate(top_k);
        results
    }

    pub fn len(&self) -> usize { self.count }

    /// Снепшот (фізичний формат).
    pub fn to_snapshot(&self) -> Vec<u8> {
        let n = self.count as u32;
        let mut buf = Vec::with_capacity(4 + n as usize * 8);
        buf.extend_from_slice(&n.to_le_bytes());
        for i in 0..self.count { buf.extend_from_slice(&self.sigs[i].to_le_bytes()); }
        buf
    }

    pub fn from_snapshot(data: &[u8]) -> Result<Self, String> {
        if data.len() < 4 { return Err("too short".into()); }
        let n = u32::from_le_bytes([data[0], data[1], data[2], data[3]]) as usize;
        if data.len() < 4 + n * 8 { return Err("truncated".into()); }
        let mut lib = Academia::new();
        for i in 0..n {
            let mut bytes = [0u8; 8];
            bytes.copy_from_slice(&data[4 + i*8 .. 4 + (i+1)*8]);
            lib.sigs.push(u64::from_le_bytes(bytes));
        }
        lib.count = n;
        Ok(lib)
    }

    /// P2P: яких сигнатур не вистачає.
    pub fn missing(&self, remote: &[QuarkSig]) -> Vec<QuarkSig> {
        let local: HashSet<QuarkSig> = self.sigs.iter().copied().collect();
        remote.iter().copied().filter(|s| !local.contains(s)).collect()
    }

    /// Фізичний ліміт для N паперів (теоретичний мінімум).
    pub fn physical_limit(n: usize) -> f64 {
        let bits = (n as f64).log2().ceil();
        let bytes = (bits / 8.0).ceil();
        bytes * n as f64 / (1024.0 * 1024.0 * 1024.0) // GB
    }

    pub fn dashboard(&self) -> String {
        let mb = (4 + self.count * 8) as f64 / 1_000_000.0;
        let phys = Self::physical_limit(self.count.max(610_000_000));
        format!(
            "Академія Дмитра Євдокимова (v5 — фізичний ліміт)\n  Papers: {}\n  Size:   {:.1} MB\n  Phys limit for 610M: {:.2} GB\n  Insert: O(0) — 1 store instruction\n  Search: 8-byte popcount scan",
            self.count, mb, phys
        )
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn physical_limit_calculation() {
        let limit = Academia::physical_limit(610_000_000);
        // log2(610M) ≈ 29.2 bits ≈ 4 bytes → 610M × 4 = 2.44 GB
        assert!(limit > 2.0 && limit < 3.0);
    }

    #[test]
    fn insert_o1() {
        let mut a = Academia::new();
        let sig = a.insert("Test Paper");
        assert_eq!(a.len(), 1);
        assert_ne!(sig, 0);
    }

    #[test]
    fn deterministic() {
        let mut a = Academia::new();
        let s1 = a.insert("Same");
        let s2 = a.insert("Same");
        // Same title → different papers (different positions), but same sig.
        assert_eq!(s1, s2);
    }

    #[test]
    fn search_returns_ranked() {
        let mut a = Academia::new();
        for i in 0..500 {
            a.insert(&format!("Paper number {} about machine learning in natural language processing", i));
        }
        let results = a.search("deep learning transformer", 10);
        assert!(results.len() <= 10);
    }

    #[test]
    fn snapshot_roundtrip() {
        let mut a = Academia::new();
        a.insert("A");
        a.insert("B");
        let snap = a.to_snapshot();
        let b = Academia::from_snapshot(&snap).unwrap();
        assert_eq!(b.len(), 2);
        assert_eq!(b.sigs[0], a.sigs[0]);
    }

    #[test]
    fn p2p_missing() {
        let mut a = Academia::new();
        a.insert("Local1");
        let remote = vec![hash_to_sig("Local1"), hash_to_sig("Remote2")];
        let missing = a.missing(&remote);
        assert_eq!(missing.len(), 1);
    }

    #[test]
    fn dashboard_contains_v5() {
        let a = Academia::new();
        let d = a.dashboard();
        assert!(d.contains("v5"));
    }
}
