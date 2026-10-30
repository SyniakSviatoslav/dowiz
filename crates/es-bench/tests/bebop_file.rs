//! The bench's bebop adapter persists deltas ITSELF (the crate has no public durable delta
//! commit), so a number from it is only worth something if the file it leaves is the truth.
//! These read the file back through the crate's own checked loader after every kind of commit
//! the workloads issue -- puts, new keys, deletes via `remove_many`, the automatic compaction
//! `append_delta` asks for, and an explicit `compact()` -- synced and unsynced.
use bebop_store::kv::Kv;
use bebop_store::Store;
use es_bench::bebop::Bebop;
use es_bench::data::{acc_of, entries, key, val};
use es_bench::engine::{Engine, Sync, W};

fn on_disk(dir: &std::path::Path) -> Vec<(String, Vec<u8>)> {
    let st = Store::from_bytes(&std::fs::read(dir.join("kv.bebop")).unwrap());
    Kv::load_checked(&st).expect("the file the adapter wrote loads checked").entries
}

fn exercise(durable: bool) {
    let dir = std::env::temp_dir().join(format!("es-bench-file-{}-{durable}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    drop(Bebop::load(&dir, &entries(500, 64)));
    let mut db = Bebop::open(&dir, Sync::durable(durable));
    for round in 0..40u64 {
        let ks: Vec<Vec<u8>> = (0..8).map(|j| key((round * 37 + j * 11) % 700)).collect();
        let vs: Vec<Vec<u8>> = (0..8).map(|j| val(round * 8 + j, 64, round)).collect();
        let mut ops: Vec<W> = ks.iter().zip(&vs).map(|(k, v)| W::Put(k, v)).collect();
        let gone = key((round * 13) % 500);
        if round % 3 == 0 {
            ops.push(W::Del(&gone));
        }
        db.commit(&ops);
        let truth = db.truth();
        assert_eq!(on_disk(&dir), truth, "round {round}: file != adapter truth ({})", db.note());
        // and the adapter's own read path, freshly opened (verify-on-open) and on the live handle
        let fresh = Bebop::open(&dir, Sync::durable(durable));
        let want = truth.iter().map(|(_, v)| acc_of(v)).fold(0u64, u64::wrapping_add);
        let all: Vec<Vec<u8>> = truth.iter().map(|(k, _)| k.as_bytes().to_vec()).collect();
        assert_eq!(fresh.get_many(&all), want, "round {round}: fresh open reads other values");
        assert_eq!(db.get_many(&all), want, "round {round}: live handle reads other values");
    }
    assert!(db.deltas > 0 && db.compactions > 0, "both write paths must have run: {}", db.note());
    db.compact();
    assert_eq!(on_disk(&dir), db.truth(), "after compact()");
    assert_eq!(db.verify(), Some(true));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn adapter_file_is_the_truth_unsynced() {
    exercise(false);
}

#[test]
fn adapter_file_is_the_truth_synced() {
    exercise(true);
}
