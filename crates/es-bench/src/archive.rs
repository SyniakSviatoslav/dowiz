//! What an archive embeds: the COMMITTED reader, out of `tools/verify/bebop-verify.mjs` under the
//! digest that file states, and the format text. Both are read at run time and a missing or
//! mismatched one stops the run by name: a smaller stand-in would print a smaller archive.
use std::fs;

use bebop_store::nodekey::sha256;

/// `bw_version()` of `crates/bebop-wasm-reader`, FORMAT §9.5.1.
pub const READER_ABI: i64 = 2;
const VERIFY_MJS: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../tools/verify/bebop-verify.mjs");
const FORMAT_TEXT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../docs/FORMAT-BEBOP-IMAGE-v2.en.md");

fn base64(s: &str) -> Option<Vec<u8>> {
    let mut out = Vec::with_capacity(s.len() / 4 * 3);
    let (mut acc, mut bits) = (0u32, 0u32);
    for c in s.bytes().filter(|&c| c != b'=') {
        let v = match c {
            b'A'..=b'Z' => c - b'A',
            b'a'..=b'z' => c - b'a' + 26,
            b'0'..=b'9' => c - b'0' + 52,
            b'+' => 62,
            b'/' => 63,
            _ => return None,
        };
        acc = (acc << 6) | v as u32;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((acc >> bits) as u8);
            acc &= (1 << bits) - 1;
        }
    }
    Some(out)
}

fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

fn constant<'a>(text: &'a str, name: &str) -> Option<&'a str> {
    let head = format!("const {name} = \"");
    text.lines().find_map(|l| l.strip_prefix(head.as_str())?.strip_suffix("\";"))
}

/// The reader inside the text of bebop-verify.mjs: `Err` names what is wrong with it.
pub fn reader_in(mjs: &str) -> Result<Vec<u8>, String> {
    let b64 = constant(mjs, "WASM_B64").ok_or("no WASM_B64 line")?;
    let want = constant(mjs, "WASM_SHA256").ok_or("no WASM_SHA256 line")?;
    let wasm = base64(b64).ok_or("WASM_B64 is not base64")?;
    let got = hex(&sha256(&wasm));
    if got != want {
        return Err(format!("the embedded wasm hashes to {got}, the file states {want}"));
    }
    Ok(wasm)
}

/// (reader wasm, format text) from this repository.
pub fn inputs() -> (Vec<u8>, Vec<u8>) {
    let mjs = fs::read_to_string(VERIFY_MJS).unwrap_or_else(|e| panic!("archive row: {VERIFY_MJS}: {e}"));
    let wasm = reader_in(&mjs).unwrap_or_else(|e| panic!("archive row: {VERIFY_MJS}: {e}"));
    let text = fs::read(FORMAT_TEXT).unwrap_or_else(|e| panic!("archive row: {FORMAT_TEXT}: {e}"));
    (wasm, text)
}

/// FORMAT §9.5's cost of the SPEC and its two blobs, in bytes.
pub fn formula(wasm: usize, text: usize) -> u64 {
    8 * ((2 + 8) + (2 + 1 + wasm.div_ceil(8)) + (2 + 1 + text.div_ceil(8))) as u64
}
