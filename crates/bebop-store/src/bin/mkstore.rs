//! Create a bebop store file from scratch IN RUST -- superblock, arena, objects, PartTab,
//! commit -- so it can be handed to bebop as proof the format works in both directions.
//! bebop has never touched the resulting file.
use bebop_store::{kv::Kv, Store};
fn main() {
    let p = std::env::args().nth(1).unwrap_or_else(|| "kv.store".into());
    let mut st = Store::create(&p, 64 << 20).expect("create");
    Kv::init(&mut st, &p).expect("init");
    let st = Store::open(&p).unwrap();
    let mut kv = Kv::load(&st).unwrap();
    for (k, v) in [
        ("order/0001", "pending"), ("order/0002", "confirmed"),
        ("courier/alpha", "idle"), ("zone/north", "{\"cap\":12}"),
        ("order/0003", "delivered"),
    ] { kv.put(k, v.as_bytes()); }
    let mut st = Store::open(&p).unwrap();
    let g = kv.commit_into(&mut st, &p).unwrap();
    println!("created_gen {}  snapshot_root {}", g, kv.snapshot_root());
}
