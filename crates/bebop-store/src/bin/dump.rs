use bebop_store::{kv::Kv, Store};
fn main() {
    let path = std::env::args().nth(1).unwrap_or_else(|| "kv.store".into());
    let st = Store::open(&path).expect("open");
    let kv = Kv::load(&st).expect("load");
    for (k, v) in &kv.entries {
        println!("key[{}]={:?}  val[{}]={:?}", k.len(), k, v.len(), String::from_utf8_lossy(v));
    }
    println!("root {}", kv.snapshot_root());
}
