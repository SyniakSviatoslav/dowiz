//! Write entries into a bebop store from Rust, then print what Rust reads back.
use bebop_store::{kv::Kv, Store};
fn main() {
    let path = std::env::args().nth(1).unwrap_or_else(|| "kv.store".into());
    let mut st = Store::open(&path).expect("open");
    let mut kv = Kv::load(&st).expect("load kv root");
    for (k, v) in [
        ("order/0001", "pending"),
        ("order/0002", "confirmed"),
        ("courier/alpha", "idle"),
        ("zone/north", "{\"cap\":12}"),
        ("order/0003", "delivered"),
    ] {
        kv.put(k, v.as_bytes());
    }
    let gen = kv.commit_into(&mut st, &path).expect("commit");
    let st2 = Store::open(&path).expect("reopen");
    let kv2 = Kv::load(&st2).expect("reload");
    println!("committed_gen {}", gen);
    println!("n {}", kv2.entries.len());
    println!("keys {}", kv2.keys().join(","));
    println!("get(order/0002) {}", String::from_utf8_lossy(&kv2.get("order/0002").unwrap()));
    println!("snapshot_root {}", kv2.snapshot_root());
    println!("snapshot_root_i64 {}", kv2.snapshot_root_u64() as i64);
}
