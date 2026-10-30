//! THE v4 ADAPTER (W-ES-P8C): bebop through the library API -- `Db<FileDevice>`, `ReadTxn`,
//! `WriteTxn` -- over a v4 (copy-on-write B+tree) image. Engine name `bebop4`; `bebop` stays the
//! v3 adapter so the two are measured side by side.
//!
//! LOAD writes the entries as a fresh v4 image (`tree_compacted`, the bulk load) atomically and
//! opens it. OPEN is `Db::open_path` (`Verify::Anchor`: recovery proves the last commit; the full
//! check is the `verify` row). A COMMIT is one `WriteTxn`: the batch path-copied into the tree
//! (`Writer::stage_tree`), flushed with protocol v2 (`Durability::None` unsynced, `Normal` =
//! P11's checkpoint every 64th commit, `Data` durable);
//! a compaction happens only when the dead cells pass the live ones (`Options::tree`).
use std::fs;
use std::path::{Path, PathBuf};

use bebop_store::iodev::{Db, Durability, FileDevice, Options, Verify};
use bebop_store::kv::tree::{tree_compacted, FANOUT};

use crate::data::{acc_of, write_atomic};
use crate::engine::{Engine, Sync, W};

const MAX_IMAGE: usize = 1 << 30;
const FILE: &str = "kv4.bebop";

pub struct Bebop4 {
    db: Db<FileDevice>,
    sync: Sync,
    commits: u64,
}

fn opts() -> Options {
    Options::default().max_image_bytes(MAX_IMAGE)
}

impl Engine for Bebop4 {
    const NAME: &'static str = "bebop4";

    fn load(dir: &Path, entries: &[(Vec<u8>, Vec<u8>)]) -> Self {
        fs::create_dir_all(dir).unwrap();
        let b = tree_compacted(entries.iter().map(|(k, v)| (k.as_slice(), v.as_slice())), MAX_IMAGE, FANOUT, None).expect("v4 image");
        write_atomic(&dir.join(FILE), &b, true);
        Self::open(dir, Sync::Full)
    }

    fn open(dir: &Path, sync: Sync) -> Self {
        let p: PathBuf = dir.join(FILE);
        let db = Db::open_path(p.to_str().unwrap(), opts()).expect("Db::open_path");
        Bebop4 { db, sync, commits: 0 }
    }

    fn get_many(&self, keys: &[Vec<u8>]) -> u64 {
        let r = self.db.begin_read();
        keys.iter().map(|k| acc_of(&r.get(k).unwrap().expect("present"))).fold(0, u64::wrapping_add)
    }

    fn prefix_many(&self, prefixes: &[Vec<u8>]) -> u64 {
        let r = self.db.begin_read();
        let mut acc = 0u64;
        for p in prefixes {
            for e in r.prefix(p).unwrap() {
                let (k, v) = e.unwrap();
                acc = acc.wrapping_add(v.len() as u64 + k[14] as u64);
            }
        }
        acc
    }

    fn commit(&mut self, ops: &[W]) {
        let d = match self.sync {
            Sync::None => Durability::None,
            Sync::Normal => Durability::Normal,
            Sync::Full => Durability::Data,
        };
        let mut w = self.db.begin_write().expect("begin_write");
        for op in ops {
            match *op {
                W::Put(k, v) => w.put(k, v).unwrap(),
                W::Del(k) => {
                    w.remove(k).unwrap();
                }
            }
        }
        w.commit(d).expect("commit");
        self.commits += 1;
    }

    fn compact(&mut self) -> String {
        self.db.compact_fresh().expect("compact_fresh");
        "v4 copy compaction into <file>.compact, rename".into()
    }

    fn verify(&mut self) -> Option<bool> {
        Some(self.db.verify(Verify::Reachable).is_ok())
    }

    fn note(&self) -> String {
        format!("commits={} compactions={}", self.commits, self.db.compactions())
    }
}
