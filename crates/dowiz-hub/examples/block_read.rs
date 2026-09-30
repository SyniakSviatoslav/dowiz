//! Row DG9's NATIVE reader (SPEC-DATALOG-AND-CODEC §B.6): DG7's own `decode` and `encode`
//! (`dowiz_hub::block`), run on a FILE, printing what it decoded -- so `gate.sh blocks`
//! compares printed facts, never a test name found in a log.
//!
//!   block_read <file>... | block_read @<list>   -- one line per block:
//!     block <schema> n=<n> nnz=<nnz> vals=<fnv64 hex> rt=<ok|diff> k256=<sha256 hex>
//!     block refused=<code> col=<column|->
//!   block_read --schemas                        -- schema <name> k64=<hex> <string>
//!
//! `vals` is FNV-1a 64 over the DECODED columns in schema order: per column its element
//! count, then every element, each as 8 little-endian bytes (u32 and byte columns
//! zero-extended, i32 sign-extended). `rt` is `encode(decode(b)) == b`, byte for byte, and
//! `k256` is the sha256 of the RE-ENCODED bytes -- both are facts a reader that never decoded
//! could not print. The four readers (this, crates/bebop-wasm/src/block.rs under node,
//! oracle.py --block, bebop-lang/selfhost/std/block.bp) print the same line or disagree.

use dowiz_hub::block::decode::decode;
use dowiz_hub::block::encode::encode;
use dowiz_hub::block::{schema, Col, Refusal};
use sha2::{Digest, Sha256};

const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

fn fnv(mut h: u64, v: u64) -> u64 {
    for b in v.to_le_bytes() {
        h = (h ^ u64::from(b)).wrapping_mul(FNV_PRIME);
    }
    h
}

fn vals(cols: &[Col]) -> u64 {
    let mut h = FNV_OFFSET;
    for c in cols {
        let e: Vec<u64> = match c {
            Col::I64(v) => v.iter().map(|x| *x as u64).collect(),
            Col::I32(v) => v.iter().map(|x| i64::from(*x) as u64).collect(),
            Col::Bytes(v) => v.iter().map(|x| u64::from(*x)).collect(),
            Col::U32(v) => v.iter().map(|x| u64::from(*x)).collect(),
        };
        h = fnv(h, e.len() as u64);
        h = e.iter().fold(h, |h, x| fnv(h, *x));
    }
    h
}

/// The column a refusal names, or `-`.
fn col_of(r: &Refusal) -> &'static str {
    match r {
        Refusal::BadType { col }
        | Refusal::BadUnit { col }
        | Refusal::BadReserved { col }
        | Refusal::BadAlignment { col }
        | Refusal::BadOffsets { col }
        | Refusal::BadLength { col }
        | Refusal::BadPadding { col }
        | Refusal::NotUtf8 { col }
        | Refusal::Overflow { col }
        | Refusal::BadRate { col } => col,
        _ => "-",
    }
}

fn line(b: &[u8]) -> String {
    let block = match decode(b) {
        Ok(x) => x,
        Err(r) => return format!("block refused={} col={}", r.code(), col_of(&r)),
    };
    let again = match encode(&block) {
        Ok(x) => x,
        Err(r) => return format!("block reencode-refused={} col={}", r.code(), col_of(&r)),
    };
    let k256: String = Sha256::digest(&again).iter().map(|x| format!("{x:02x}")).collect();
    let rt = if again == b { "ok" } else { "diff" };
    format!("block {} n={} nnz={} vals={:016x} rt={rt} k256={k256}", block.schema.name, block.n, block.nnz, vals(&block.cols))
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.first().map(String::as_str) == Some("--schemas") {
        for s in schema::TABLE {
            println!("schema {} k64={:016x} {}", s.name, schema::k64(s.string.as_bytes()), s.string);
        }
        return;
    }
    let paths: Vec<String> = match args.first().and_then(|a| a.strip_prefix('@')) {
        Some(list) => std::fs::read_to_string(list).unwrap_or_else(|e| panic!("list {list}: {e}")).lines().map(String::from).collect(),
        None => args,
    };
    if paths.is_empty() {
        eprintln!("usage: block_read <file>... | block_read @<list> | block_read --schemas");
        std::process::exit(2);
    }
    for p in paths {
        match std::fs::read(&p) {
            Ok(b) => println!("{}", line(&b)),
            Err(e) => println!("block unreadable={e}"),
        }
    }
}
