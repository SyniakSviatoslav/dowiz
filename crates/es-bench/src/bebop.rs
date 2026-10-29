//! THE BEBOP ADAPTER -- the one file R-EMBED-STORE P3 swaps for `Db`/`ReadTxn`/`WriteTxn`.
//!
//! Built ONLY from the crate's public API as of 2026-10-29 (main d838e87d): `Kv` (`put`,
//! `remove_many`, `compacted_bytes_fit`), `iodev::{Writer, FileDevice, Durability}` (W-ES-P2's
//! durable delta commit), `kv::zc::KvIn` over a `View`.
//!
//! READ PATH: the file is mmapped (MAP_SHARED) once; `open` runs `KvIn::open` over it, which IS
//! bebop's verify-on-open (the crc of the root, the four arrays and every delta record), and keeps
//! the `Checked` it mints. A read transaction is `KvIn::reopen(View, Checked)`: O(1) shape checks,
//! the delta chain walked again, no crc; values come back borrowed from the map.
//!
//! WRITE PATH: `Writer::commit_delta(ops, Durability)` -- the crate's protocol v2 (two barriers
//! under `Data`, the same writes and none under `None`), over a `FileDevice` holding
//! `<file>.lock`. When it answers `Compact`, or on `compact()`, the image is rewritten whole
//! (tmp, fsync, rename, dir fsync -- dowiz's `Catalog::compact` bytes) and the writer is opened
//! again on the new inode, which reads the file back and runs `recover`: that read is part of
//! today's compaction cost and is counted. The decoded `Kv` (the truth `append_delta` stages
//! against) and the writer are loaded on the first write, not by `open`.
use std::fs;
use std::path::{Path, PathBuf};

use bebop_store::iodev::{Durability, FileDevice, Writer};
use bebop_store::kv::delta::{Appended, Op};
use bebop_store::kv::zc::{Checked, KvIn};
use bebop_store::kv::Kv;
use bebop_store::{View, PARTTAB_CELLS};

use crate::data::{acc_of, write_atomic, Mmap};
use crate::engine::{Engine, W};

/// The largest image `compacted_bytes_fit` may produce here.
const MAX_IMAGE: usize = 1 << 30;
const FILE: &str = "kv.bebop";

pub struct Bebop {
    path: PathBuf,
    sync: bool,
    map: Mmap,
    checked: Checked,
    /// The truth (`Kv`) and the writer over the file, once the first write needs them.
    writer: Option<(Writer<FileDevice>, Kv)>,
    pub deltas: u64,
    pub compactions: u64,
}

fn s(k: &[u8]) -> &str {
    std::str::from_utf8(k).expect("bench keys are ASCII")
}

/// The image bytes for sorted entries, exactly as dowiz's `Catalog::compact` writes them.
pub fn image(entries: &[(Vec<u8>, Vec<u8>)]) -> Vec<u8> {
    let kv = Kv { entries: entries.iter().map(|(k, v)| (s(k).to_string(), v.clone())).collect() };
    kv.compacted_bytes_fit(MAX_IMAGE).expect("image fits")
}

/// Bebop's verify-on-open over bytes already in memory: the full crc check.
pub fn verify_bytes(bytes: &[u8]) -> bool {
    KvIn::open(View::new(bytes)).is_ok()
}

impl Bebop {
    fn device(&self) -> Writer<FileDevice> {
        let dev = FileDevice::open(self.path.to_str().unwrap()).expect("FileDevice::open");
        Writer::open(dev).expect("Writer::open (recover)")
    }

    fn writer(&mut self) -> &mut (Writer<FileDevice>, Kv) {
        if self.writer.is_none() {
            let w = self.device();
            let kv = Kv::load_checked(w.store()).expect("own image decodes");
            self.writer = Some((w, kv));
        }
        self.writer.as_mut().unwrap()
    }

    fn rewrite(&mut self) {
        let (w, kv) = self.writer.take().unwrap();
        drop(w); // its fd names the old inode, and its lock must go before the next open
        let bytes = kv.compacted_bytes_fit(MAX_IMAGE).expect("fits");
        write_atomic(&self.path, &bytes, self.sync);
        self.writer = Some((self.device(), kv));
        self.map = Mmap::open(&self.path); // a new inode: the old map still shows the old file
        self.compactions += 1;
    }

    /// The truth after this commit, for tests: every entry the writer holds.
    pub fn truth(&mut self) -> Vec<(String, Vec<u8>)> {
        self.writer().1.entries.clone()
    }
}

impl Engine for Bebop {
    const NAME: &'static str = "bebop";

    fn load(dir: &Path, entries: &[(Vec<u8>, Vec<u8>)]) -> Self {
        fs::create_dir_all(dir).unwrap();
        write_atomic(&dir.join(FILE), &image(entries), true);
        Self::open(dir, true)
    }

    fn open(dir: &Path, durable: bool) -> Self {
        let path = dir.join(FILE);
        let map = Mmap::open(&path);
        let checked = KvIn::open(View::new(map.bytes())).expect("verify on open").checked();
        Bebop { path, sync: durable, map, checked, writer: None, deltas: 0, compactions: 0 }
    }

    fn get_many(&self, keys: &[Vec<u8>]) -> u64 {
        let kv = KvIn::reopen(View::new(self.map.bytes()), self.checked).unwrap();
        keys.iter().map(|k| acc_of(&kv.get(k).unwrap().expect("present"))).fold(0, u64::wrapping_add)
    }

    fn prefix_many(&self, prefixes: &[Vec<u8>]) -> u64 {
        let kv = KvIn::reopen(View::new(self.map.bytes()), self.checked).unwrap();
        let mut acc = 0u64;
        for p in prefixes {
            for i in kv.prefix_range(p).unwrap() {
                acc = acc.wrapping_add(kv.value(i).unwrap().len() as u64 + kv.key(i).unwrap()[14] as u64);
            }
        }
        acc
    }

    fn commit(&mut self, ops: &[W]) {
        let d = if self.sync { Durability::Data } else { Durability::None };
        let bops: Vec<Op> = ops.iter().map(|o| match *o {
            W::Put(k, v) => Op::Put(s(k), v),
            W::Del(k) => Op::Remove(s(k)),
        }).collect();
        let (w, kv) = self.writer();
        // In order: a run of deletes goes through `remove_many` (one pass), flushed before
        // the next put so a Del-then-Put of one key ends with the put, as the delta does.
        let mut dels = Vec::new();
        for op in ops {
            match *op {
                W::Put(k, v) => {
                    kv.remove_many(dels.drain(..));
                    kv.put(s(k), v)
                }
                W::Del(k) => dels.push(s(k)),
            }
        }
        kv.remove_many(dels.drain(..));
        // `Some(end)`: a delta was written and the image now reaches cell `end`.
        let grew = match w.commit_delta(&bops, d).expect("commit_delta") {
            Appended::Delta(_) => {
                let st = w.store();
                let sb = st.pick().unwrap();
                let pt = st.cells[sb.at + 3] as usize;
                Some((sb.arena_used as usize).max(pt + PARTTAB_CELLS as usize + 2).max(sb.at + 16))
            }
            Appended::Compact(_) => None,
        };
        match grew {
            Some(end) => {
                self.deltas += 1;
                // A trimmed image grows when a delta lands past its end: the map must follow, or
                // the live handle reads a superblock naming cells it cannot see (bebop_file.rs
                // caught exactly this: `KvIn::reopen` -> NotKv on the second commit).
                if end * 8 > self.map.len() {
                    self.map = Mmap::open(&self.path);
                }
            }
            None => self.rewrite(),
        }
    }

    fn compact(&mut self) -> String {
        self.writer();
        self.rewrite();
        "whole-image rewrite (tmp+fsync+rename+dir fsync)".into()
    }

    fn verify(&mut self) -> Option<bool> {
        Some(verify_bytes(self.map.bytes()))
    }

    fn note(&self) -> String {
        format!("deltas={} compactions={}", self.deltas, self.compactions)
    }
}
