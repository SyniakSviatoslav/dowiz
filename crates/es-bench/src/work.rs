//! The workloads of R-EMBED-STORE §4.6, driven identically through `Engine` for every engine.
//! Every timed row is >= `reps` repetitions reported as median / p95 / min; every read row
//! folds what it read into a checksum that `main` requires to be equal across engines.
use std::fs;
use std::path::{Path, PathBuf};

use crate::data::*;
use crate::engine::{Engine, W};

pub struct Cfg {
    pub n: u64,
    pub vs: usize,
    pub reps: usize,
    pub dir: PathBuf,
    /// Rows to run (names as printed), empty = all.
    pub only: Vec<String>,
}

/// Point gets per repetition (hot), cold gets per open, prefixes per repetition, puts per write
/// repetition, commits per durable repetition, the write batch.
pub const HOT_GETS: usize = 100_000;
pub const COLD_GETS: usize = 1_000;
pub const PREFIXES: usize = 1_000;
pub const BATCH: usize = 32;
pub const DURABLE_COMMITS: usize = 100;

impl Cfg {
    fn want(&self, row: &str) -> bool {
        self.only.is_empty() || self.only.iter().any(|r| r == row)
    }
    fn writes(&self) -> usize {
        (self.n as usize).min(10_000)
    }
}

fn fresh<E: Engine>(dir: &Path, e: &[(Vec<u8>, Vec<u8>)]) -> E {
    let _ = fs::remove_dir_all(dir);
    E::load(dir, e)
}

/// One engine, every row. Returns (row, checksum) for the cross-engine check.
pub fn run<E: Engine>(c: &Cfg) -> Vec<(String, u64)> {
    let name = E::NAME;
    let dir = c.dir.join(name.replace(['(', ')'], "_"));
    let e = entries(c.n, c.vs);
    let tag = format!("N={} VS={}", c.n, c.vs);
    let mut sums = vec![];

    if c.want("load") {
        let mut v = vec![];
        for _ in 0..c.reps {
            let _ = fs::remove_dir_all(&dir);
            let t0 = t();
            drop(E::load(&dir, &e));
            v.push(us(t0) / 1000.0);
        }
        res("load", name, "ms", stats(v), &format!("{tag}; sorted input, one txn, synced; includes open; file bytes={}", dir_bytes(&dir)));
    } else {
        drop(fresh::<E>(&dir, &e));
    }

    if c.want("open") || c.want("verify") {
        let mut vo = vec![];
        let mut vv = vec![];
        let mut has = None;
        for _ in 0..c.reps {
            let t0 = t();
            let mut db = E::open(&dir, false);
            vo.push(us(t0));
            let t1 = t();
            has = db.verify();
            vv.push(us(t1) / 1000.0);
            assert_ne!(has, Some(false), "{name}: verify failed on its own image");
        }
        res("open", name, "us", stats(vo), &format!("{tag}; warm page cache; bebop's open = KvIn::open = full crc"));
        match has {
            Some(_) => res("verify", name, "ms", stats(vv), &format!("{tag}; the engine's full integrity check")),
            None => one("verify", name, "-", 0.0, "the engine offers no integrity check"),
        }
    }

    let db = E::open(&dir, false);
    if c.want("get_hot") {
        let ks = lookups(c.n, HOT_GETS, 7);
        let acc = db.get_many(&ks); // warm
        let mut v = vec![];
        for _ in 0..c.reps {
            let t0 = t();
            assert_eq!(db.get_many(&ks), acc);
            v.push(us(t0) * 1000.0 / ks.len() as f64);
        }
        res("get_hot", name, "ns/get", stats(v), &format!("{tag}; {} random gets in ONE read txn; acc={acc}", ks.len()));
        sums.push(("get_hot".into(), acc));
    }
    if c.want("prefix") {
        let ps = prefixes(c.n, PREFIXES, 11);
        let acc = db.prefix_many(&ps);
        let mut v = vec![];
        for _ in 0..c.reps {
            let t0 = t();
            assert_eq!(db.prefix_many(&ps), acc);
            v.push(us(t0) / ps.len() as f64);
        }
        res("prefix", name, "us/scan", stats(v), &format!("{tag}; {} 13-byte prefixes (<=100 keys each) in ONE read txn; acc={acc}", ps.len()));
        sums.push(("prefix".into(), acc));
    }
    drop(db);

    if c.want("get_cold") {
        let ks = lookups(c.n, COLD_GETS, 13);
        let mut v = vec![];
        let mut evicted = true;
        let mut acc = 0;
        for _ in 0..c.reps {
            evicted &= evict_dir(&dir);
            let t0 = t();
            let db = E::open(&dir, false);
            acc = db.get_many(&ks);
            v.push(us(t0) / 1000.0);
        }
        res("get_cold", name, "ms", stats(v), &format!("{tag}; fadvise(DONTNEED) then open + {} gets; fadvise_ok={evicted}; acc={acc}", ks.len()));
        sums.push(("get_cold".into(), acc));
    }

    let k = c.writes();
    let order = perm(c.n, 100);
    if c.want("put") || c.want("compact") {
        let vals: Vec<Vec<u8>> = order[..k].iter().map(|&i| val(i, c.vs, 1)).collect();
        let keys: Vec<Vec<u8>> = order[..k].iter().map(|&i| key(i)).collect();
        let (mut v, mut vc, mut note, mut cnote) = (vec![], vec![], String::new(), String::new());
        for _ in 0..c.reps {
            drop(fresh::<E>(&dir, &e));
            let mut db = E::open(&dir, false);
            let t0 = t();
            for (ks, vs) in keys.chunks(BATCH).zip(vals.chunks(BATCH)) {
                let ops: Vec<W> = ks.iter().zip(vs).map(|(a, b)| W::Put(a, b)).collect();
                db.commit(&ops);
            }
            v.push(us(t0) / k as f64);
            note = db.note();
            let before = dir_bytes(&dir);
            let t1 = t();
            let what = db.compact();
            vc.push(us(t1) / 1000.0);
            cnote = format!("{what}; bytes {before} -> {}", dir_bytes(&dir));
        }
        res("put", name, "us/put", stats(v), &format!("{tag}; {k} overwrites, {BATCH} per commit, UNSYNCED; {note}"));
        res("compact", name, "ms", stats(vc), &format!("{tag}; after the put row; {cnote}"));
    }
    if c.want("put_durable") {
        let mut v = vec![];
        let mut note = String::new();
        for r in 0..c.reps {
            drop(fresh::<E>(&dir, &e));
            let mut db = E::open(&dir, true);
            for j in 0..DURABLE_COMMITS {
                let i = order[j];
                let (kk, vv) = (key(i), val(i, c.vs, 2 + r as u64));
                let t0 = t();
                db.commit(&[W::Put(&kk, &vv)]);
                v.push(us(t0) / 1000.0);
            }
            note = db.note();
        }
        res("put_durable", name, "ms/commit", stats(v), &format!("{tag}; 1 put per commit, synced (proot: fsync is not representative); {note}"));
    }
    if c.want("delete") {
        let keys: Vec<Vec<u8>> = order[..k].iter().map(|&i| key(i)).collect();
        let mut v = vec![];
        let mut note = String::new();
        for _ in 0..c.reps {
            drop(fresh::<E>(&dir, &e));
            let mut db = E::open(&dir, false);
            let t0 = t();
            for ks in keys.chunks(BATCH) {
                let ops: Vec<W> = ks.iter().map(|a| W::Del(a)).collect();
                db.commit(&ops);
            }
            v.push(us(t0) / k as f64);
            note = db.note();
            let rest: Vec<Vec<u8>> = order[k..].iter().take(1000).map(|&i| key(i)).collect();
            if !rest.is_empty() {
                db.get_many(&rest); // survivors still read (panics on a lost key)
            }
        }
        res("delete", name, "us/delete", stats(v), &format!("{tag}; {k} deletes, {BATCH} per commit, UNSYNCED; {note}"));
    }
    let _ = fs::remove_dir_all(&dir);
    sums
}
