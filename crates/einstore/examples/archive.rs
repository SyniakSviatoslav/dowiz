// SPDX-License-Identifier: MIT OR Apache-2.0
//! PROMISE: an archive file carries its own reader. The reader wasm and the format text are the
//! first objects of the file, and `examples/read.mjs` (repo root) finds them with no parser and no
//! install, using only Node.
//! Run: `cargo run --example archive -- <reader.wasm> <format.md> <out.ein>`, then
//! `node examples/read.mjs <out.ein>`
//!
//! The embedded reader is testimony, not authority. A file vouching for its own reader proves
//! nothing by itself: compare its sha256 with a published one, or read the file with an independent
//! reader. This is an archive mode. A working image does not carry the reader.

#[allow(dead_code)]
mod shared;

use einstore::evlog::doc::{Doc, DocError, SpecIn, FLAG_SHA256_IDS};
use einstore::nodekey::sha256;
use einstore::Store;
use shared::Device;

/// `bw_version()` of `crates/einfile-reader`, FORMAT §9.5.1.
const READER_ABI: i64 = 2;

fn main() -> Result<(), DocError> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [wasm, text, out] = args.as_slice() else {
        eprintln!("usage: archive <reader.wasm> <format.md> <out.ein>");
        std::process::exit(2);
    };
    let wasm = std::fs::read(wasm).expect("the reader wasm");
    let text = std::fs::read(text).expect("the format text");

    let mut st = Store::create_bytes(wasm.len() + text.len() + (1 << 20))?;
    let mut till = Device::new(1);
    let spec = SpecIn { wasm: &wasm, wasm_sha256: sha256(&wasm), text: &text, abi: READER_ABI };
    Doc::init_described_bytes(&mut st, FLAG_SHA256_IDS, &[till.key()], &spec)?;
    till.commit(&mut st, b"the last day of the year", None)?;

    std::fs::write(out, st.to_bytes_trimmed()).expect("the out path is writable");
    println!("wrote {out}; run: node examples/read.mjs {out}");
    Ok(())
}
