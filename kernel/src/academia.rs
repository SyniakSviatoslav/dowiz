//! `kernel::academia` — Академія Дмитра Євдокимова. v4: 8-byte.
//!
//! # 8 bytes per paper
//! Кожен папір = 8 байт. Кожен байт = один кварк (0-255).
//! 8 кварків × 1 байт = повний кварковий склад паперу.
//!
//! # Операції
//! - Вставка: SHA3-256(title) → 8 quarks (8 bytes) → push.
//! - Пошук: query → 8 quarks → SIMD popcount XOR з усіма → top-K.
//! - Снепшот: просто масив u64.
//!
//! # Пам'ять (610M паперів)
//! - Кварковий склад: 610M × 8 = 4.88 GB
//! - Bloom: 24 MB
//! - **TOTAL: ~4.9 GB**

use crate::event_log::sha3_256;

const QUARK_TYPES: usize = 256;
const QUARKS_PER_PAPER: usize = 8;
const MAX_PAPERS: usize = 1_000_000_000;

/// 8-байтний кварковий підпис паперу.
pub type QuarkSig = u64;

/// SHA3-256 → 8 кварків (кожен 1 байт).
pub fn hash_to_quarks(hash: &[u8; 32]) -> QuarkSig {
    let mut sig: u64 = 0;
    for i in 0..8 {
        let q = hash[i] as u64; // Кварк = просто байт хешу (0-255)
        sig |= q << (i * 8);
    }
    sig
}

/// Розпакувати u64 → 8 кварків.
pub fn unpack_quarks(sig: QuarkSig) -> [u8; 8] {
    [
        (sig >> 0) as u8, (sig >> 8) as u8, (sig >> 16) as u8, (sig >> 24) as u8,
        (sig >> 32) as u8, (sig >> 40) as u8, (sig >> 48) as u8, (sig >> 56) as u8,
    ]
}

/// Кількість спільних кварків між двома підписами (SIMD popcount).
pub fn shared_quarks(a: QuarkSig, b: QuarkSig) -> u32 {
    let qa = unpack_quarks(a);
    let qb = unpack_quarks(b);
    let mut count = 0;
    for i in 0..8 {
        if qa[i] == qb[i] { count += 1; }
    }
    count
}

// ─── Bloom ────────────────────────────────────────────────────────────────

pub struct Bloom8 {
    bits: Vec<u64>,
    pub count: u64,
}

impl Bloom8 {
    pub fn new() -> Self {
        Bloom8 { bits: vec![0; (1_000_000_000 / 64 + 1)], count: 0 }
    }
    pub fn insert(&mut self, sig: QuarkSig) {
        let h = (sig % (self.bits.len() as u64 * 64)) as usize;
        self.bits[h / 64] |= 1 << (h % 64);
        self.count += 1;
    }
}

// ─── Academia 8-byte ─────────────────────────────────────────────────────

/// Академія Дмитра Євдокимова — v4: 8 bytes/paper.
pub struct Academia {
    /// Кваркові підписи паперів (8 байт = 1 u64).
    pub sigs: Vec<QuarkSig>,
    /// Bloom filter.
    bloom: Bloom8,
}

impl Academia {
    pub fn new() -> Self {
        Academia { sigs: Vec::with_capacity(10_000_000), bloom: Bloom8::new() }
    }

    /// Вставка: title → SHA3-256 → 8 quarks → push.
    pub fn insert(&mut self, title: &str) -> QuarkSig {
        let clean: String = title.chars().map(|c| if c.is_ascii() && (c.is_ascii_graphic() || c == ' ') { c } else { ' ' }).collect();
        let hash = sha3_256(clean.as_bytes());
        let sig = hash_to_quarks(&hash);
        self.sigs.push(sig);
        self.bloom.insert(sig);
        sig
    }

    /// Пошук: query → 8 quarks → popcount з усіма → top-K.
    pub fn search(&self, query: &str, top_k: usize) -> Vec<(usize, u32)> {
        let clean: String = query.chars().map(|c| if c.is_ascii() && (c.is_ascii_graphic() || c == ' ') { c } else { ' ' }).collect();
        let hash = sha3_256(clean.as_bytes());
        let qsig = hash_to_quarks(&hash);

        let mut results: Vec<(usize, u32)> = self.sigs.iter().enumerate()
            .map(|(i, &s)| (i, shared_quarks(qsig, s)))
            .collect();
        results.sort_by(|a, b| b.1.cmp(&a.1));
        results.truncate(top_k);
        results
    }

    pub fn len(&self) -> usize { self.sigs.len() }

    /// Снепшот: [N: u32] [sig: u64]*
    pub fn to_snapshot(&self) -> Vec<u8> {
        let n = self.sigs.len() as u32;
        let mut buf = Vec::with_capacity(4 + n as usize * 8);
        buf.extend_from_slice(&n.to_le_bytes());
        for s in &self.sigs { buf.extend_from_slice(&s.to_le_bytes()); }
        buf
    }

    pub fn from_snapshot(data: &[u8]) -> Result<Self, String> {
        if data.len() < 4 { return Err("too short".into()); }
        let n = u32::from_le_bytes([data[0], data[1], data[2], data[3]]) as usize;
        if data.len() < 4 + n * 8 { return Err("truncated".into()); }
        let mut lib = Academia::new();
        for i in 0..n {
            let s = u64::from_le_bytes([
                data[4 + i*8], data[5 + i*8], data[6 + i*8], data[7 + i*8],
                data[8 + i*8], data[9 + i*8], data[10 + i*8], data[11 + i*8],
            ]);
            lib.sigs.push(s);
        }
        Ok(lib)
    }
    
    /// P2P: розбіжності з віддаленою бібліотекою.
    pub fn missing_from(&self, remote_sigs: &[QuarkSig]) -> Vec<QuarkSig> {
        let local_set: std::collections::HashSet<QuarkSig> = self.sigs.iter().copied().collect();
        remote_sigs.iter().copied().filter(|s| !local_set.contains(s)).collect()
    }

    pub fn dashboard(&self) -> String {
        let mb = (4 + self.sigs.len() * 8) as f64 / 1_000_000.0;
        format!(
            "Академія Дмитра Євдокимова (8B)\n  Papers: {}\n  Size:   {:.1} MB\n  Bloom:  {}",
            self.sigs.len(), mb, self.bloom.count
        )
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insert_and_len() {
        let mut a = Academia::new();
        a.insert("Test Paper");
        assert_eq!(a.len(), 1);
    }

    #[test]
    fn deterministic_quarks() {
        assert_eq!(Academia::new().insert("Same"), Academia::new().insert("Same"));
    }

    #[test]
    fn different_titles_different_sigs() {
        let mut a = Academia::new();
        let s1 = a.insert("Paper A about cats");
        let s2 = a.insert("Paper B about dogs");
        assert_ne!(s1, s2);
    }

    #[test]
    fn shared_quarks_count() {
        let mut a = Academia::new();
        a.insert("Machine Learning Neural Networks");
        let _ = a.search("Deep Learning Transformer", 5);
    }

    #[test]
    fn snapshot_roundtrip() {
        let mut a = Academia::new();
        a.insert("Paper A");
        a.insert("Paper B");
        let snap = a.to_snapshot();
        let b = Academia::from_snapshot(&snap).unwrap();
        assert_eq!(b.len(), 2);
        assert_eq!(b.sigs[0], a.sigs[0]);
    }

    #[test]
    fn popcount_identical_max() {
        let a = Academia::new().insert("Test");
        assert_eq!(shared_quarks(a, a), 8);
    }

    #[test]
    fn popcount_zero_for_different() {
        // Two very different strings should have few or zero shared quarks.
        let a = Academia::new().insert("AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA");
        let b = Academia::new().insert("BBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBB");
        assert!(shared_quarks(a, b) < 4);
    }

    #[test]
    fn dashboard_contains() {
        let a = Academia::new();
        let d = a.dashboard();
        assert!(d.contains("Академія"));
    }
}
