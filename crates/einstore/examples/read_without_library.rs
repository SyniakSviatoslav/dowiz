// SPDX-License-Identifier: MIT OR Apache-2.0
//! PROMISE: the file reads without this library. This writes two files and names the commands that
//! read them with nothing installed. `tools/verify/ein-verify.mjs` needs only Node and
//! `tools/verify/ein-verify.py` only Python's standard library.
//! Run: `cargo run --example read_without_library -- <out-dir>`
//!
//! Those verifiers check structure and crc. They carry no signature algorithm, so they count seals
//! and do not check them, and they say so in their output (`checked=0`).

#[allow(dead_code)]
mod shared;

use einstore::evlog::doc::{Doc, FLAG_SHA256_IDS};
use einstore::{Db, Durability, Options, Store};
use shared::Device;

fn main() {
    let dir = std::env::args().nth(1).unwrap_or_else(|| std::env::temp_dir().to_string_lossy().into_owned());

    let db = Db::create_detached(Options::detached()).expect("an empty image");
    let mut w = db.begin_write().expect("the only writer");
    w.put(b"order/1", b"new").expect("a short key");
    w.put(b"order/2", b"paid").expect("a short key");
    w.commit(Durability::None).expect("the commit"); // the bytes are written below
    let kv = format!("{dir}/example-kv.ein");
    std::fs::write(&kv, db.snapshot_bytes()).expect("the directory is writable");

    let mut st = Store::create_bytes(1 << 20).expect("1 MiB of memory");
    let mut till = Device::new(1);
    Doc::init_bytes(&mut st, FLAG_SHA256_IDS, &[till.key()]).expect("a fresh store");
    till.commit(&mut st, b"sold 2 espresso", None).expect("the commit");
    till.commit(&mut st, b"sold 1 cake", None).expect("the commit");
    let doc = format!("{dir}/example-doc.ein");
    std::fs::write(&doc, st.to_bytes_trimmed()).expect("the directory is writable");

    println!("node tools/verify/ein-verify.mjs {kv} --dump");
    println!("python3 tools/verify/ein-verify.py {kv} --dump");
    println!("node tools/verify/ein-verify.mjs {doc} --dump --verify");
    println!("python3 tools/verify/ein-verify.py {doc} --dump --verify");
}
