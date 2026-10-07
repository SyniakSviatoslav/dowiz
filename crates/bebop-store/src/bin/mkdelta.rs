//! Make `crates/bebop-wasm/fixtures/kv3.store` (W-DELTA): `kv2.store`'s five entries, then three
//! delta writes through `kv::delta::append_delta` -- a changed value, a new key, a removal -- each
//! its own generation. Prints what this crate reads back; oracle.py re-derives it from the bytes.
//!   mkdelta <kv2.store> <out kv3.store>
use bebop_store::kv::delta::{append_delta_with, Appended, Op, MAX_DELTAS};
use bebop_store::{kv::Kv, Store};

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let (src, out) = (a.get(1).expect("kv2.store path"), a.get(2).expect("output path"));
    let mut st = Store::from_bytes(&std::fs::read(src).expect("read kv2.store"));
    let steps: [&[Op]; 3] = [
        &[Op::Put("order/0002", b"cancelled")],
        &[Op::Put("zone/south", b"{\"cap\":7}")],
        &[Op::Remove("courier/alpha")],
    ];
    for ops in steps {
        // dead_div 0: the dead-cell trigger is policy, and a five-entry image trips it at once;
        // the fixture pins the FORMAT, so it is a chain.
        match append_delta_with(&mut st, ops, MAX_DELTAS, 0).expect("append") {
            Appended::Delta(g) => println!("delta gen {g}"),
            Appended::Compact(w) => panic!("the fixture must be a chain, the writer asked to compact: {w:?}"),
        }
    }
    let bytes = st.to_bytes_trimmed();
    std::fs::write(out, &bytes).expect("write");
    let kv = Kv::load(&Store::from_bytes(&bytes)).expect("reload");
    println!("bytes {}", bytes.len());
    println!("version {}", Kv::version(&Store::from_bytes(&bytes)));
    println!("n {}", kv.entries.len());
    println!("keys {}", kv.keys().join(","));
    println!("snapshot_root_i64 {}", kv.snapshot_root_u64() as i64);
}
