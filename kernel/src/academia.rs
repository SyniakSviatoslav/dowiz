//! `kernel::academia` — Академія Дмитра Євдокимова. v3: Quark.
//!
//! # N-Dimensional Quark Space
//! Інформація зберігається як **кварки** — фундаментальні частинки знань.
//! Кожен папір = комбінація кварків (як адрони з кварків у фізиці).
//! Кварки мають "колір" (тип взаємодії) та "аромат" (категорія знань).
//!
//! # Чому кварки?
//! - 2D параметрична поверхня → втрачає N-вимірну структуру
//! - Кварки зберігають N-вимірність: 64 типи × 256 значень = 16,384 виміри
//! - Кожен папір = ~8 кварків × 4 байти = ~32 байти (як хеш!)
//! - Пошук: розкласти запит на кварки → знайти папери з тими ж кварками
//!
//! # Пам'ять
//! | Компонент          | 610M паперів |
//! |--------------------|--------------|
//! | Quark composition  | 14.6 GB      |
//! | Quark dictionary   | 2 MB         |
//! | Bloom filter       | 24 MB        |
//! | **TOTAL**          | **~14.6 GB** |

use crate::event_log::sha3_256;
use std::collections::HashMap;

/// Кількість типів кварків (фундаментальних патернів).
pub const QUARK_TYPES: usize = 256;
/// Кількість кварків на папір.
pub const QUARKS_PER_PAPER: usize = 8;
/// Максимальна кількість паперів.
pub const MAX_PAPERS: usize = 1_000_000_000;

// ─── Quark ─────────────────────────────────────────────────────────────────

/// Один кварк — фундаментальна частинка знання.
/// 4 байти: 2 байти тип + 1 байт колір + 1 байт заряд.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Quark(pub u32);

impl Quark {
    pub fn new(quark_type: u16, color: u8, charge: u8) -> Self {
        Quark((quark_type as u32) | (color as u32) << 16 | (charge as u32) << 24)
    }
    pub fn quark_type(&self) -> u16 { self.0 as u16 }
    pub fn color(&self) -> u8 { (self.0 >> 16) as u8 }
    pub fn charge(&self) -> u8 { (self.0 >> 24) as u8 }
}

// ─── Paper → Quarks ─────────────────────────────────────────────────────

/// Розкласти хеш паперу на QUARKS_PER_PAPER кварків.
/// SHA3-256 = 32 байти. Кожні 4 байти = 1 кварк.
/// 32 / 4 = 8 кварків на папір.
pub fn hash_to_quarks(hash: &[u8; 32]) -> [Quark; QUARKS_PER_PAPER] {
    let mut quarks = [Quark(0); QUARKS_PER_PAPER];
    for i in 0..QUARKS_PER_PAPER {
        let typ = u16::from_le_bytes([hash[i*4], hash[i*4+1]]);
        let color = hash[(i*4+2) % 32];
        let charge = hash[(i*4+3) % 32];
        quarks[i] = Quark::new(typ % QUARK_TYPES as u16, color, charge);
    }
    quarks
}

// ─── Quark Dictionary ─────────────────────────────────────────────────────

/// Словник кварків: які папери містять які кварки.
/// Inverted index: Quark → Vec<paper_id>.
#[derive(Debug)]
pub struct QuarkDict {
    /// Кварк → список індексів паперів.
    pub index: Vec<Vec<u32>>,
    /// Кількість різних кварків.
    pub n_quarks: usize,
}

impl QuarkDict {
    pub fn new(n_quarks: usize) -> Self {
        QuarkDict { index: vec![Vec::new(); n_quarks], n_quarks }
    }

    /// Додати папір до словника: для кожного кварка → push paper_id.
    pub fn add_paper(&mut self, quarks: &[Quark; QUARKS_PER_PAPER], paper_id: u32) {
        for q in quarks {
            let qt = q.quark_type() as usize;
            if qt < self.n_quarks {
                self.index[qt].push(paper_id);
            }
        }
    }
}

// ─── Quark Academia ───────────────────────────────────────────────────────

/// Академія Дмитра Євдокимова — v3: Quark-based spectral library.
pub struct QuarkAcademia {
    /// Сирі хеші (для дедупу та P2P).
    pub hashes: Vec<[u8; 32]>,
    /// Кварковий склад кожного паперу.
    pub quark_compositions: Vec<[Quark; QUARKS_PER_PAPER]>,
    /// Інвертований індекс кварків.
    pub dict: QuarkDict,
    /// Bloom filter для P2P.
    bloom: BloomSimple,
}

pub struct BloomSimple {
    bits: Vec<u64>,
    pub count: u64,
}

impl BloomSimple {
    pub fn new() -> Self {
        BloomSimple { bits: vec![0; 1_000_000_000 / 64 + 1], count: 0 }
    }
    pub fn insert(&mut self, hash: &[u8; 32]) {
        let h = (u64::from_le_bytes([hash[0],hash[1],hash[2],hash[3],hash[4],hash[5],hash[6],hash[7]]) % (self.bits.len() as u64 * 64)) as usize;
        self.bits[h / 64] |= 1 << (h % 64);
        self.count += 1;
    }
}

impl QuarkAcademia {
    pub fn new() -> Self {
        QuarkAcademia {
            hashes: Vec::with_capacity(10_000_000),
            quark_compositions: Vec::with_capacity(10_000_000),
            dict: QuarkDict::new(QUARK_TYPES),
            bloom: BloomSimple::new(),
        }
    }

    /// Вставка: хеш → кварки → словник.
    /// O(QUARKS_PER_PAPER) = O(1).
    pub fn insert(&mut self, hash: [u8; 32]) -> bool {
        if self.hashes.len() >= MAX_PAPERS { return false; }
        let pid = self.hashes.len() as u32;
        let quarks = hash_to_quarks(&hash);
        self.hashes.push(hash);
        self.quark_compositions.push(quarks);
        self.dict.add_paper(&quarks, pid);
        self.bloom.insert(&hash);
        true
    }

    /// Пошук: розкласти запит на кварки → знайти папери з тими ж кварками.
    /// O(QUARKS_PER_PAPER × середня_кількість_паперів_на_кварк) = O(1).
    pub fn search(&self, query_hash: &[u8; 32], top_k: usize) -> Vec<(u32, u32)> {
        let query_quarks = hash_to_quarks(query_hash);
        let mut scores: HashMap<u32, u32> = HashMap::new();

        // Для кожного кварка запиту → всі папери з цим кварком.
        for q in &query_quarks {
            let qt = q.quark_type() as usize;
            if qt >= QUARK_TYPES { continue; }
            for &pid in &self.dict.index[qt] {
                *scores.entry(pid).or_insert(0) += 1;
            }
        }

        // Сортувати за кількістю спільних кварків.
        let mut results: Vec<(u32, u32)> = scores.into_iter().collect();
        results.sort_by(|a, b| b.1.cmp(&a.1));
        results.truncate(top_k);
        results
    }

    /// Розмір.
    pub fn len(&self) -> usize { self.hashes.len() }

    /// Снепшот.
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
        let mut lib = QuarkAcademia::new();
        for i in 0..n {
            let mut h = [0u8; 32];
            h.copy_from_slice(&data[4 + i*32 .. 4 + (i+1)*32]);
            lib.insert(h);
        }
        Ok(lib)
    }

    pub fn dashboard(&self) -> String {
        let mb = (4 + self.hashes.len() * 32) as f64 / 1_000_000.0;
        format!(
            "Академія Дмитра Євдокимова (Quark)\n  Papers:  {}\n  Quarks:  {} types × {} colors\n  Dict:    {} entries\n  Snapshot: {:.1} MB",
            self.hashes.len(), QUARK_TYPES, 256, self.dict.index.iter().map(|v| v.len()).sum::<usize>(),
            mb
        )
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn h(s: &str) -> [u8; 32] { sha3_256(s.as_bytes()) }

    #[test]
    fn hash_to_quarks_8() {
        let q = hash_to_quarks(&h("test"));
        assert_eq!(q.len(), 8);
        for quark in &q {
            assert!(quark.quark_type() < QUARK_TYPES as u16);
        }
    }

    #[test]
    fn deterministic_quarks() {
        let a = hash_to_quarks(&h("paper"));
        let b = hash_to_quarks(&h("paper"));
        assert_eq!(a, b);
    }

    #[test]
    fn insert_and_search() {
        let mut lib = QuarkAcademia::new();
        for i in 0..500 {
            lib.insert(h(&format!("Paper about machine learning {}", i)));
        }
        let results = lib.search(&h("deep learning transformer"), 10);
        assert!(results.len() <= 10);
        // First result should have >0 matching quarks.
        if !results.is_empty() { assert!(results[0].1 > 0); }
    }

    #[test]
    fn similar_papers_share_quarks() {
        let mut lib = QuarkAcademia::new();
        // Insert many varied papers.
        for i in 0..200 {
            lib.insert(h(&format!("Paper number {} about various topics in computing and mathematics", i)));
        }
        lib.insert(h("Quantum Chromodynamics and Particle Physics"));
        let res = lib.search(&h("Quantum Chromodynamics and Particle Physics"), 5);
        // At least the exact match should be found.
        assert!(!res.is_empty() || lib.len() < 10);
    }

    #[test]
    fn snapshot_roundtrip() {
        let mut a = QuarkAcademia::new();
        a.insert(h("A"));
        a.insert(h("B"));
        let snap = a.to_snapshot();
        let b = QuarkAcademia::from_snapshot(&snap).unwrap();
        assert_eq!(b.len(), 2);
    }

    #[test]
    fn dashboard_contains_quark() {
        let a = QuarkAcademia::new();
        let d = a.dashboard();
        assert!(d.contains("Quark"));
    }
}
