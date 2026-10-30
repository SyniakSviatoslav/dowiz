//! redb adapter. One table `kv: &[u8] -> &[u8]`. Durable = `Durability::Immediate` (redb's
//! default: one commit = its two-phase fsync), unsynced = `Durability::None`, normal = `None`
//! with an `Immediate` every 64th commit.
//! Reads: one `begin_read` per batch. Compaction: `Database::compact()` (in place, needs no
//! live transaction). Verify: `check_integrity()` (redb's own full check).
use std::fs;
use std::path::Path;

use redb::{Database, Durability, ReadableDatabase, TableDefinition};

use crate::data::acc_of;
use crate::engine::{Engine, Sync, W};

const T: TableDefinition<&[u8], &[u8]> = TableDefinition::new("kv");
const FILE: &str = "kv.redb";

pub struct Redb {
    db: Database,
    sync: Sync,
    commits: u64,
}

impl Engine for Redb {
    const NAME: &'static str = "redb";

    fn load(dir: &Path, entries: &[(Vec<u8>, Vec<u8>)]) -> Self {
        fs::create_dir_all(dir).unwrap();
        let db = Database::create(dir.join(FILE)).unwrap();
        let w = db.begin_write().unwrap();
        {
            let mut t = w.open_table(T).unwrap();
            for (k, v) in entries {
                t.insert(k.as_slice(), v.as_slice()).unwrap();
            }
        }
        w.commit().unwrap();
        drop(db);
        Self::open(dir, Sync::Full)
    }

    fn open(dir: &Path, sync: Sync) -> Self {
        Redb { db: Database::open(dir.join(FILE)).unwrap(), sync, commits: 0 }
    }

    fn get_many(&self, keys: &[Vec<u8>]) -> u64 {
        let r = self.db.begin_read().unwrap();
        let t = r.open_table(T).unwrap();
        keys.iter().map(|k| acc_of(t.get(k.as_slice()).unwrap().expect("present").value())).fold(0, u64::wrapping_add)
    }

    fn prefix_many(&self, prefixes: &[Vec<u8>]) -> u64 {
        let r = self.db.begin_read().unwrap();
        let t = r.open_table(T).unwrap();
        let mut acc = 0u64;
        for p in prefixes {
            let mut hi = p.clone();
            *hi.last_mut().unwrap() += 1;
            for item in t.range(p.as_slice()..hi.as_slice()).unwrap() {
                let (k, v) = item.unwrap();
                acc = acc.wrapping_add(v.value().len() as u64 + k.value()[14] as u64);
            }
        }
        acc
    }

    fn commit(&mut self, ops: &[W]) {
        let mut w = self.db.begin_write().unwrap();
        // redb 4 has no NORMAL: `Normal` is `None` with an `Immediate` every 64th commit, the
        // checkpoint cadence bebop's `Durability::Normal` uses.
        self.commits += 1;
        let d = match self.sync {
            Sync::Full => Durability::Immediate,
            Sync::Normal if self.commits % 64 == 0 => Durability::Immediate,
            _ => Durability::None,
        };
        w.set_durability(d).unwrap();
        {
            let mut t = w.open_table(T).unwrap();
            for op in ops {
                match *op {
                    W::Put(k, v) => {
                        t.insert(k, v).unwrap();
                    }
                    W::Del(k) => {
                        t.remove(k).unwrap();
                    }
                }
            }
        }
        w.commit().unwrap();
    }

    fn compact(&mut self) -> String {
        // redb refuses to compact over a non-durable commit; make the last one durable first.
        let mut w = self.db.begin_write().unwrap();
        w.set_durability(Durability::Immediate).unwrap();
        w.commit().unwrap();
        format!("Database::compact() = {}", self.db.compact().unwrap())
    }

    fn verify(&mut self) -> Option<bool> {
        Some(self.db.check_integrity().unwrap())
    }
}
