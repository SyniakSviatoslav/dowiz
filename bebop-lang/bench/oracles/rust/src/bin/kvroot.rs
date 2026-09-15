//! Fold the same (key, value) set through PRODUCTION `dowiz_core`'s InMemoryStore and print
//! its snapshot_root, so bebop's and bebop-store's roots can be set against dowiz's own.
use dowiz_core::retrieval::memory_store::{InMemoryStore, MemoryStore};
fn main() {
    let s = InMemoryStore::new();
    for (k, v) in [
        ("order/0001", "pending"),
        ("order/0002", "confirmed"),
        ("courier/alpha", "idle"),
        ("zone/north", "{\"cap\":12}"),
        ("order/0003", "delivered"),
    ] {
        s.put(k, v.as_bytes()).unwrap();
    }
    println!("dowiz_keys {}", s.keys().join(","));
    println!("dowiz_snapshot_root {}", s.snapshot_root());
    let h = u64::from_str_radix(&s.snapshot_root(), 16).unwrap();
    println!("dowiz_snapshot_root_i64 {}", h as i64);
}
