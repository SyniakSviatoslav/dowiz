//! W-ES-P9: where an explicit v4 compaction spends its time, phase by phase, on the es-bench
//! data set after the put row. Not a gate: a profile whose lines are quoted in the notes.
//!
//!   cargo run --release --example p9prof -- <N> [VS]
use std::time::Instant;

use bebop_store::iodev::Db;
use bebop_store::kv::tree::{stage_build, tree_compacted, FANOUT};
use bebop_store::kv::zc::KvIn;
use bebop_store::kv::Staged;
use bebop_store::{Batch, Cells, Durability, Options, Store, View};
use es_bench::data::{entries, key, perm, val, write_atomic};
use es_bench::work::BATCH;

fn ms(t: Instant) -> f64 {
    t.elapsed().as_nanos() as f64 / 1e6
}

fn one_build(e: &[(Vec<u8>, Vec<u8>)], cap: usize, tag: &str) -> Vec<u8> {
    let t = Instant::now();
    let mut st = Store::create_bytes(cap).unwrap();
    let t_alloc = ms(t);
    let (tx, root) = match stage_build(&mut st, e.iter().map(|(k, v)| (k.as_slice(), v.as_slice())), FANOUT) {
        Ok(x) => x,
        Err(err) => {
            println!("PROF|{tag}.one_build|cap={cap}|create_bytes {t_alloc:.2} ms|REFUSED after {:.2} ms: {err:?}", ms(t));
            return Vec::new();
        }
    };
    st.commit_bytes(&tx, root);
    let t_build = ms(t);
    let b = st.to_bytes_trimmed();
    println!("PROF|{tag}.one_build|cap={cap}|create_bytes {t_alloc:.2} ms|+stage_build+commit {t_build:.2} ms|+trimmed {:.2} ms|bytes={}", ms(t), b.len());
    b
}

/// The publish as `FileDevice::publish` does it, each step timed: write whole, fdatasync, rename, dir fsync.
fn publish_steps(path: &std::path::Path, bytes: &[u8], tag: &str) {
    use std::io::Write;
    let tmp = path.with_extension("compact");
    let t = Instant::now();
    let mut f = std::fs::File::create(&tmp).unwrap();
    f.write_all(bytes).unwrap();
    let t_w = ms(t);
    f.sync_data().unwrap();
    let t_s = ms(t);
    std::fs::rename(&tmp, path).unwrap();
    let t_r = ms(t);
    es_bench::data::dir_sync(path);
    println!("PROF|{tag}.publish|write {t_w:.2} ms|+fdatasync {t_s:.2} ms|+rename {t_r:.2} ms|+dir fsync {:.2} ms|bytes={}", ms(t), bytes.len());
}

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let n: u64 = a.get(1).and_then(|s| s.parse().ok()).unwrap_or(10_000);
    let vs: usize = a.get(2).and_then(|s| s.parse().ok()).unwrap_or(256);
    let dir = std::path::PathBuf::from(std::env::var("ES_BENCH_DIR").unwrap_or_else(|_| "es-bench-data".into())).join("p9prof");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("kv4.bebop");
    let e = entries(n, vs);
    println!("# p9prof N={n} VS={vs}");

    // LOAD: tree_compacted as es-bench's bebop4 load does it, then one build at the cap it landed on.
    let t = Instant::now();
    let b = tree_compacted(e.iter().map(|(k, v)| (k.as_slice(), v.as_slice())), 1 << 30, FANOUT, None).unwrap();
    println!("PROF|load.tree_compacted|{:.2} ms|bytes={}", ms(t), b.len());
    let cap = ((Store::from_bytes(&b).pick().unwrap().arena_used.max(0) as usize) * 8).next_power_of_two();
    let b1 = one_build(&e, cap, "load");
    assert_eq!(b1.len(), b.len());
    let t = Instant::now();
    write_atomic(&path, &b, true);
    println!("PROF|load.write_atomic(sync)|{:.2} ms", ms(t));
    let t = Instant::now();
    let db = Db::open_path(path.to_str().unwrap(), Options::default().max_image_bytes(1 << 30)).unwrap();
    println!("PROF|open_path|{:.2} ms", ms(t));

    // PUT ROW: es-bench's put row, each commit timed.
    let k = (n as usize).min(10_000);
    let order = perm(n, 100);
    let (mut total, mut max, mut commits) = (0.0f64, 0.0f64, 0usize);
    let mut pauses = Vec::new();
    for ch in order[..k].chunks(BATCH) {
        let kv: Vec<(Vec<u8>, Vec<u8>)> = ch.iter().map(|&i| (key(i), val(i, vs, 1))).collect();
        let mut w = db.begin_write().unwrap();
        for (kk, vv) in &kv {
            w.put(kk, vv).unwrap();
        }
        let before = db.compactions();
        let t = Instant::now();
        w.commit(Durability::None).unwrap();
        let d = ms(t);
        if db.compactions() > before {
            pauses.push(format!("{d:.1}"));
        }
        total += d;
        max = max.max(d);
        commits += 1;
    }
    println!("PROF|put|{k} puts in {commits} commits|total {total:.1} ms = {:.2} us/put|max commit {max:.2} ms|compactions={} pauses ms [{}]", total * 1000.0 / k as f64, db.compactions(), pauses.join(" "));

    // THE DIRTY IMAGE and the phases of compact_into, each on its own.
    let bytes = db.snapshot_bytes();
    let sb = View::new(&bytes).pick().unwrap();
    println!("PROF|dirty|bytes={}|live_cells={} superseded_cells={} arena_used={} capacity={}", bytes.len(), sb.live_cells, sb.superseded_cells, sb.arena_used, View::new(&bytes).cell_at(sb.at + 12));
    let t = Instant::now();
    let kv = KvIn::open_lazy(View::new(&bytes)).unwrap();
    println!("PROF|A0.open_lazy(View)|{:.2} ms", ms(t));
    let t = Instant::now();
    let ents = Staged::new(&kv, Batch::empty()).entries().unwrap();
    println!("PROF|A.Staged::entries|{:.2} ms|n={}", ms(t), ents.len());
    let t = Instant::now();
    let nb = tree_compacted(ents.iter().map(|(k, v)| (k.as_bytes(), v.as_bytes())), 1 << 30, FANOUT, Some(sb.generation)).unwrap();
    println!("PROF|B.tree_compacted|{:.2} ms|bytes={}", ms(t), nb.len());
    let owned: Vec<(Vec<u8>, Vec<u8>)> = ents.iter().map(|(k, v)| (k.as_bytes().to_vec(), v.as_bytes().to_vec())).collect();
    let cap = ((Store::from_bytes(&nb).pick().unwrap().arena_used.max(0) as usize) * 8).next_power_of_two();
    let _ = one_build(&owned, cap / 2, "B.attempt1(cap_floor)");
    let _ = one_build(&owned, cap, "B.attempt2");
    let _ = one_build(&owned, cap * 2, "B.attempt3");
    let t = Instant::now();
    let img = Store::from_bytes(&nb);
    println!("PROF|C.Store::from_bytes|{:.2} ms|cells={}", ms(t), img.cells.len());
    let t = Instant::now();
    let kv2 = KvIn::open_lazy(View::new(&nb)).unwrap();
    println!("PROF|D.open_lazy(fresh)|{:.2} ms|len={}", ms(t), kv2.len());
    publish_steps(&dir.join("kv4.bebop.probe"), &nb, "E");
    publish_steps(&dir.join("kv4.bebop.probe"), &nb, "E2");
    // The put row's commits were UNSYNCED: their appends sit dirty in the page cache of the old
    // file until something forces them out. Is that the pause? Sync them first, timed, then compact.
    let t = Instant::now();
    std::fs::OpenOptions::new().write(true).open(&path).unwrap().sync_data().unwrap();
    println!("PROF|E3.fdatasync of the OLD file (the put row's unsynced appends, {} bytes)|{:.2} ms", std::fs::metadata(&path).unwrap().len(), ms(t));
    let t = Instant::now();
    db.compact_fresh().unwrap();
    println!("PROF|F.Db::compact_fresh (old file synced first)|{:.2} ms|compactions={}", ms(t), db.compactions());
    let t = Instant::now();
    db.compact_fresh().unwrap();
    println!("PROF|F2.Db::compact_fresh again (clean image)|{:.2} ms", ms(t));
    let _ = std::fs::remove_dir_all(&dir);
}
