//! CELLS, MEASURED FROM THE REAL CODE against the prototype (`scratchpad/p8proto/p8model.py`, the
//! design's §6 rows), on the prototype's workload: keys `key%012d` (15 B), 256 B values, the tree
//! grown by inserts in a fixed pseudo-random order, then 2000 single overwrites (b=1) and 150
//! commits of 32 overwrites (b=32). WRITTEN = the arena cells a commit allocates + 37 (PartTab and
//! superblock), the prototype's own definition. READ = every `cell_at` a `get` makes after `at`.
//!
//! Prototype at B=32: 1e4 get 89, put b=1 352 (p99 409), b=32 176/op;
//!                    1e5 get 103, put b=1 400 (p99 476), b=32 238/op.

use alloc::vec::Vec;
use alloc::{format, vec};
use core::cell::Cell;

use super::*;
use crate::delta::KeyOp;
use crate::zc::Image;
use crate::{Cells, Store};

/// A `Store` read through a counter: every cell a reader touches.
struct Count<'a>(&'a Store, Cell<usize>);
impl Cells for Count<'_> {
    fn cell_at(&self, i: usize) -> i64 {
        self.1.set(self.1.get() + 1);
        self.0.cell_at(i)
    }
    fn n_cells(&self) -> usize {
        self.0.n_cells()
    }
}
impl Image for Count<'_> {}

fn key(i: u64) -> Vec<u8> {
    format!("key{i:012}").into_bytes()
}

/// A fixed permutation step (xorshift64*): the workload's order, not a test generator.
struct Order(u64);
impl Order {
    fn below(&mut self, n: u64) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_F491_4F6C_DD1D) % n
    }
}

/// One commit of `ops`; returns the cells it wrote, prototype-defined.
fn written(st: &mut Store, ops: &[KeyOp], b: usize) -> usize {
    let (tx, root) = stage_tree(st, ops, b).unwrap();
    st.commit_bytes(&tx, root);
    (tx.cursor - tx.mark) as usize + 37
}

pub(super) struct Report {
    pub depth: usize,
    pub get: f64,
    pub put1: f64,
    pub put1_p99: usize,
    pub put32: f64,
}

pub(super) fn measure(n: u64, b: usize) -> Report {
    let mut st = Store::create_bytes((n as usize * 400 + (8 << 20)) * 8).unwrap();
    let val = vec![0x5Au8; 256];
    // Insert every key once, in a shuffled order (Fisher-Yates over 0..n), 64 per commit.
    let mut order: Vec<u64> = (0..n).collect();
    let mut rnd = Order(7);
    for i in (1..n as usize).rev() {
        order.swap(i, rnd.below(i as u64 + 1) as usize);
    }
    for chunk in order.chunks(64) {
        let mut ks: Vec<Vec<u8>> = chunk.iter().map(|&i| key(i)).collect();
        ks.sort();
        let ops: Vec<KeyOp> = ks.iter().map(|k| KeyOp::Put(k, &val)).collect();
        stage_tree(&mut st, &ops, b).map(|(tx, r)| st.commit_bytes(&tx, r)).unwrap();
    }
    let depth = TreeIn::open(&st).unwrap().depth();
    // Reads per get, after `at`.
    let probes: Vec<Vec<u8>> = (0..2000).map(|_| key(rnd.below(n))).collect();
    let root = st.root().unwrap();
    let mut reads = 0usize;
    for p in &probes {
        let c = Count(&st, Cell::new(0));
        let t = TreeIn::at(&c, root).unwrap();
        t.image().1.set(0);
        assert!(t.get(p).unwrap().is_some());
        reads += t.image().1.get();
    }
    // Writes: 2000 single overwrites, then 150 commits of 32.
    let val2 = vec![0xA5u8; 256];
    let mut w1: Vec<usize> = probes.iter().map(|p| written(&mut st, &[KeyOp::Put(p, &val2)], b)).collect();
    let mut w32 = 0usize;
    for _ in 0..150 {
        let mut ks: Vec<Vec<u8>> = (0..32).map(|_| key(rnd.below(n))).collect();
        ks.sort();
        ks.dedup();
        let ops: Vec<KeyOp> = ks.iter().map(|k| KeyOp::Put(k, &val)).collect();
        w32 += written(&mut st, &ops, b) * 32 / ks.len();
    }
    TreeIn::open(&st).unwrap().verify().unwrap();
    let put1 = w1.iter().sum::<usize>() as f64 / w1.len() as f64;
    w1.sort();
    Report { depth, get: reads as f64 / 2000.0, put1, put1_p99: w1[w1.len() * 99 / 100], put32: w32 as f64 / (150.0 * 32.0) }
}

fn print(n: u64, r: &Report) {
    std::println!("CELLS N={n} B=32 depth={} get={:.0} put_b1={:.0} (p99 {}) put_b32={:.0}/op", r.depth, r.get, r.put1, r.put1_p99, r.put32);
}

/// At 1e4 the real tree must stay within 25 % of the prototype's counts on every row (it may do
/// better: the prototype has no batched reads).
#[test]
fn cells_at_1e4_are_within_the_prototype_bound() {
    let r = measure(10_000, FANOUT);
    print(10_000, &r);
    assert_eq!(r.depth, 3);
    assert!(r.get <= 89.0 * 1.25, "get {}", r.get);
    assert!(r.put1 <= 352.0 * 1.25, "put b=1 {}", r.put1);
    assert!(r.put32 <= 176.0 * 1.25, "put b=32 {}", r.put32);
}

/// The 1e5 row (release, `--ignored`): ~200 MB of arena.
#[test]
#[ignore]
fn cells_at_1e5() {
    let r = measure(100_000, FANOUT);
    print(100_000, &r);
    assert!(r.get <= 103.0 * 1.25 && r.put1 <= 400.0 * 1.25 && r.put32 <= 238.0 * 1.25);
}
