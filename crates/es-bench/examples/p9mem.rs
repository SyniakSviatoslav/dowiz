//! W-ES-P9: what one v4 compaction costs in RAM at N, on its own (design risk 3 / P13): the
//! bulk load of the es-bench entries through `tree_compacted`, then `Store::from_bytes` of the
//! result -- the writer's image after `compact_into`, padded to the capacity the image names.
//! Run under `ulimit -v` and `runmeas.py` for the maxrss; N stepwise (1e5, then 1e6).
//!
//!   cargo run --release --example p9mem -- <N> [VS]
use std::time::Instant;

use bebop_store::kv::tree::{tree_compacted, FANOUT};
use bebop_store::{Cells, Store, View};
use es_bench::data::entries;

fn ms(t: Instant) -> f64 {
    t.elapsed().as_nanos() as f64 / 1e6
}

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let n: u64 = a.get(1).and_then(|s| s.parse().ok()).unwrap_or(100_000);
    let vs: usize = a.get(2).and_then(|s| s.parse().ok()).unwrap_or(256);
    let e = entries(n, vs);
    let data: usize = e.iter().map(|(k, v)| k.len() + v.len()).sum();
    println!("# p9mem N={n} VS={vs} entry bytes={data}");
    let t = Instant::now();
    let b = tree_compacted(e.iter().map(|(k, v)| (k.as_slice(), v.as_slice())), 1 << 30, FANOUT, None).unwrap();
    let sb = View::new(&b).pick().unwrap();
    let cap = (View::new(&b).cell_at(sb.at + 12) as usize + bebop_store::ARENA) * 8;
    println!("MEM|tree_compacted|{:.1} ms|trimmed bytes={}|used={}|capacity={} ({:.2}x used)", ms(t), b.len(), sb.arena_used * 8, cap, cap as f64 / (sb.arena_used * 8) as f64);
    drop(e);
    let t = Instant::now();
    let st = Store::from_bytes(&b);
    println!("MEM|Store::from_bytes|{:.1} ms|writer cells bytes={} ({})", ms(t), st.cells.len() * 8, if st.cells.len() * 8 == cap { "padded to the capacity" } else { "NOT padded: capacity is past MAX_PAD_CELLS, grows on alloc" });
}
