//! LMDB adapter through heed. One named database of raw bytes. Durable = default env flags
//! (fsync at every commit), unsynced = normal = `NO_SYNC` (ACI kept without `WRITEMAP`). Reads: one `read_txn` per batch, values
//! borrowed from the map. Compaction: LMDB has none in place (freed pages are reused, the file
//! never shrinks); the only way to give space back is `copy_to_path(.., Compact)` and swapping
//! the file, which is what `compact` does and says. Verify: LMDB offers no integrity check.
use std::fs;
use std::path::{Path, PathBuf};

use heed::types::Bytes;
use heed::{CompactionOption, Database, Env, EnvFlags, EnvOpenOptions};

use crate::data::acc_of;
use crate::engine::{Engine, Sync, W};

/// A reservation, not memory. 1 GiB: 1e6 entries of 16 + 256 bytes are ~0.3 GB with LMDB's
/// page overhead, and 4 GiB refused to map under the `ulimit -v 2500000` every scaling run on
/// this box carries (W-ES-P8E: `Io(Os { code: 12, kind: OutOfMemory })` at env open).
const MAP: usize = 1 << 30;

pub struct Lmdb {
    dir: PathBuf,
    /// `None` only inside `compact`, between closing the old file and opening the new one.
    env: Option<Env>,
    db: Database<Bytes, Bytes>,
    sync: Sync,
}

fn env_open(dir: &Path, sync: Sync) -> Env {
    let mut o = EnvOpenOptions::new();
    o.map_size(MAP).max_dbs(1);
    // `NO_SYNC` without `WRITEMAP` keeps ACI and loses only D (the LMDB docs): it is both the
    // unsynced and the NORMAL mode here.
    if sync != Sync::Full {
        unsafe { o.flags(EnvFlags::NO_SYNC) };
    }
    unsafe { o.open(dir) }.unwrap()
}

impl Lmdb {
    fn env(&self) -> &Env {
        self.env.as_ref().unwrap()
    }
}

impl Engine for Lmdb {
    const NAME: &'static str = "lmdb(heed)";

    fn load(dir: &Path, entries: &[(Vec<u8>, Vec<u8>)]) -> Self {
        fs::create_dir_all(dir).unwrap();
        let env = env_open(dir, Sync::Full);
        let mut w = env.write_txn().unwrap();
        let db: Database<Bytes, Bytes> = env.create_database(&mut w, Some("kv")).unwrap();
        for (k, v) in entries {
            db.put(&mut w, k, v).unwrap();
        }
        w.commit().unwrap();
        env.prepare_for_closing().wait();
        Self::open(dir, Sync::Full)
    }

    fn open(dir: &Path, sync: Sync) -> Self {
        let env = env_open(dir, sync);
        let r = env.read_txn().unwrap();
        let db = env.open_database(&r, Some("kv")).unwrap().expect("kv db");
        r.commit().unwrap();
        Lmdb { dir: dir.to_path_buf(), env: Some(env), db, sync }
    }

    fn get_many(&self, keys: &[Vec<u8>]) -> u64 {
        let r = self.env().read_txn().unwrap();
        keys.iter().map(|k| acc_of(self.db.get(&r, k).unwrap().expect("present"))).fold(0, u64::wrapping_add)
    }

    fn prefix_many(&self, prefixes: &[Vec<u8>]) -> u64 {
        let r = self.env().read_txn().unwrap();
        let mut acc = 0u64;
        for p in prefixes {
            for item in self.db.prefix_iter(&r, p).unwrap() {
                let (k, v) = item.unwrap();
                acc = acc.wrapping_add(v.len() as u64 + k[14] as u64);
            }
        }
        acc
    }

    fn commit(&mut self, ops: &[W]) {
        let mut w = self.env().write_txn().unwrap();
        for op in ops {
            match *op {
                W::Put(k, v) => self.db.put(&mut w, k, v).unwrap(),
                W::Del(k) => {
                    self.db.delete(&mut w, k).unwrap();
                }
            }
        }
        w.commit().unwrap();
    }

    fn compact(&mut self) -> String {
        let tmp = self.dir.join("compact.mdb");
        let _ = fs::remove_file(&tmp);
        self.env().copy_to_path(&tmp, CompactionOption::Enabled).unwrap();
        self.env.take().unwrap().prepare_for_closing().wait();
        fs::rename(&tmp, self.dir.join("data.mdb")).unwrap();
        *self = Self::open(&self.dir.clone(), self.sync);
        "copy_to_path(Compact) + close + rename over data.mdb + reopen".into()
    }

    fn verify(&mut self) -> Option<bool> {
        None
    }
}

