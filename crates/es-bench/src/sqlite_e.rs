//! SQLite adapter (rusqlite 0.32.1, bundled). `CREATE TABLE kv(k BLOB PRIMARY KEY, v BLOB)
//! WITHOUT ROWID`, WAL, `locking_mode=EXCLUSIVE` (file locks taken once: under proot every
//! fcntl costs ~140 us, R-STORE-VS-SQL §2). Durable = `synchronous=FULL`, normal = `NORMAL`,
//! unsynced = `OFF`.
//! Reads: one BEGIN..COMMIT per batch. Compaction: `VACUUM`. Verify: `PRAGMA integrity_check`.
use std::fs;
use std::path::Path;

use rusqlite::{params, Connection};

use crate::data::acc_of;
use crate::engine::{Engine, Sync, W};

const FILE: &str = "kv.sqlite";

pub struct Sqlite {
    db: Connection,
}

fn conn(dir: &Path, sync: Sync) -> Connection {
    let db = Connection::open(dir.join(FILE)).unwrap();
    let m: String = db.query_row("PRAGMA locking_mode=EXCLUSIVE", [], |r| r.get(0)).unwrap();
    assert_eq!(m, "exclusive");
    let j: String = db.query_row("PRAGMA journal_mode=WAL", [], |r| r.get(0)).unwrap();
    assert_eq!(j, "wal");
    db.execute_batch(match sync {
        Sync::None => "PRAGMA synchronous=OFF;",
        Sync::Normal => "PRAGMA synchronous=NORMAL;",
        Sync::Full => "PRAGMA synchronous=FULL;",
    })
    .unwrap();
    db
}

impl Engine for Sqlite {
    const NAME: &'static str = "sqlite";

    fn load(dir: &Path, entries: &[(Vec<u8>, Vec<u8>)]) -> Self {
        fs::create_dir_all(dir).unwrap();
        let mut db = conn(dir, Sync::Full);
        db.execute_batch("CREATE TABLE kv(k BLOB PRIMARY KEY, v BLOB) WITHOUT ROWID;").unwrap();
        let tx = db.transaction().unwrap();
        {
            let mut st = tx.prepare("INSERT INTO kv(k, v) VALUES(?1, ?2)").unwrap();
            for (k, v) in entries {
                st.execute(params![k, v]).unwrap();
            }
        }
        tx.commit().unwrap();
        drop(db); // close = checkpoint
        Self::open(dir, Sync::Full)
    }

    fn open(dir: &Path, sync: Sync) -> Self {
        Sqlite { db: conn(dir, sync) }
    }

    fn get_many(&self, keys: &[Vec<u8>]) -> u64 {
        self.db.execute_batch("BEGIN").unwrap();
        let mut st = self.db.prepare_cached("SELECT v FROM kv WHERE k=?1").unwrap();
        let mut acc = 0u64;
        for k in keys {
            acc = acc.wrapping_add(st.query_row([k], |r| Ok(acc_of(r.get_ref(0)?.as_blob()?))).unwrap());
        }
        drop(st);
        self.db.execute_batch("COMMIT").unwrap();
        acc
    }

    fn prefix_many(&self, prefixes: &[Vec<u8>]) -> u64 {
        self.db.execute_batch("BEGIN").unwrap();
        let mut st = self.db.prepare_cached("SELECT k, v FROM kv WHERE k >= ?1 AND k < ?2").unwrap();
        let mut acc = 0u64;
        for p in prefixes {
            let mut hi = p.clone();
            *hi.last_mut().unwrap() += 1;
            let mut rows = st.query([p, &hi]).unwrap();
            while let Some(row) = rows.next().unwrap() {
                let k = row.get_ref(0).unwrap().as_blob().unwrap();
                acc = acc.wrapping_add(row.get_ref(1).unwrap().as_blob().unwrap().len() as u64 + k[14] as u64);
            }
        }
        drop(st);
        self.db.execute_batch("COMMIT").unwrap();
        acc
    }

    fn commit(&mut self, ops: &[W]) {
        let tx = self.db.transaction().unwrap();
        {
            let mut put = tx.prepare_cached("INSERT OR REPLACE INTO kv(k, v) VALUES(?1, ?2)").unwrap();
            let mut del = tx.prepare_cached("DELETE FROM kv WHERE k=?1").unwrap();
            for op in ops {
                match *op {
                    W::Put(k, v) => put.execute(params![k, v]).unwrap(),
                    W::Del(k) => del.execute([k]).unwrap(),
                };
            }
        }
        tx.commit().unwrap();
    }

    fn compact(&mut self) -> String {
        self.db.execute_batch("VACUUM; PRAGMA wal_checkpoint(TRUNCATE);").unwrap();
        "VACUUM + wal_checkpoint(TRUNCATE)".into()
    }

    fn verify(&mut self) -> Option<bool> {
        let r: String = self.db.query_row("PRAGMA integrity_check", [], |r| r.get(0)).unwrap();
        Some(r == "ok")
    }
}
