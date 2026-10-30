// SPDX-License-Identifier: MIT OR Apache-2.0
//! THE CONFORMANCE FIXTURES, WRITTEN BY THE RUST WRITER, AND THE NATIVE READER THAT JUDGES THEM
//! (W-ES-P4, R-EMBED-STORE §4.4 T-ES-30/32, §5 row P4).
//!
//! `bebop-conform write <fixtures dir>` reads `<dir>/kv2.store` and writes, next to it:
//!   kv3max.store  kv2's five entries + MAX_DELTAS (32) one-op delta commits (dead-cell trigger off,
//!                 as mkdelta: the fixture pins the FORMAT) -- a v3 root with D = 32, the longest
//!                 chain the writer builds; a 33rd append is asserted to answer Compact(Deltas).
//!   kv3c.store    the same entries right after the compaction that 33rd append asks for
//!                 (`compacted_from_fit`, after_gen = kv3max's generation): v2, generation + 1,
//!                 and the SAME n and fold as kv3max.
//!   torn.store    kv3max's last commit cut mid-write in the superblock (W-ES-P2's sim crash model,
//!                 `Model::Power(Tear::Cells)`, window 3: the arena range and the PartTab barriers
//!                 are durable, the 512-byte sector holding the new superblock is torn with its first
//!                 8 cells new and the rest old). The new superblock's crc15 fails, so every reader
//!                 falls back to generation k: the 31-delta image's n and fold.
//!   ptbad.store   kv3c (a v2 image written by Rust, anchored) with ONE bit flipped in its live
//!                 PartTab: payload cell 1 (the copied superblock's version cell), bit 0. The
//!                 superblock is whole, so FORMAT §3.1 allows no fallback: NO_SUPERBLOCK (1).
//!   empty.store   `Store::create_bytes(16 KiB)` trimmed: superblock A, generation 0, no PartTab,
//!                 no KV root: NOT_A_KV (2).
//!   kv0.store     an empty KV (`Kv::init_bytes`): n = 0, fold = the FNV-1a offset basis.
//!   kv4*.store    KV v4, the COW B+tree (W-ES-P8D): see `kv4.rs`.
//!
//! `bebop-conform read <image>...` prints, per image, `kv status=<s> n=<n> root=<fold>` in
//! harness.mjs's shape, the status from docs/STATUS-CODES.md:
//!   pick_checked_in refuses -> 1, then the ANCHOR (W-ES-P10, FORMAT §7: a torn last commit's
//!   failing object -> 10, every object whole -> 41), no KV root -> 2, KvError::NotKv -> 2,
//!   BadCrc -> 10, UnknownVersion -> 12, Tree(BadCrc) -> 10, any other Tree refusal -> 2.
mod kv4;

use bebop_image::format::pick_checked_in;
use bebop_image::verify::{last_commit_in, LastCommit};
use bebop_image::Store;
use bebop_kv::delta::{append_delta_with, Appended, Op, Why, MAX_DELTAS};
use bebop_kv::verify::KvError;
use bebop_kv::Kv;

fn read(bytes: &[u8]) -> (i32, i64, i64) {
    let st = Store::from_bytes(bytes);
    let Ok(sb) = pick_checked_in(&st) else {
        return (1, 0, 0);
    };
    // W-ES-P10 (FORMAT §7 phase P2): the anchor is asked BEFORE the KV rules, the same rule on
    // every reader of bytes -- a torn last commit names its failing object (10) or itself (41).
    match last_commit_in(&st, &sb) {
        Ok(_) => {}
        Err(LastCommit::Object(_)) => return (10, 0, 0),
        Err(LastCommit::Anchor(_)) => return (41, 0, 0),
    }
    if st.root().is_none() {
        return (2, 0, 0);
    }
    match Kv::load_checked(&st) {
        Ok(kv) => (0, kv.entries.len() as i64, kv.snapshot_root_u64() as i64),
        Err(KvError::NotKv) => (2, 0, 0),
        Err(KvError::BadCrc(_)) => (10, 0, 0),
        Err(KvError::UnknownVersion { .. }) => (12, 0, 0),
        // KV v4 (W-ES-P8D): a reachable object's crc -> 10; any structural rule of FORMAT §9.1 v4
        // (forward ref, depth, order, count, a length) -> NOT_A_KV, as a v3 claim that does not hold.
        Err(KvError::Tree(bebop_kv::tree::TreeError::BadCrc(_))) => (10, 0, 0),
        Err(KvError::Tree(_)) => (2, 0, 0),
    }
}

/// Op i of the 32: puts of new keys (values of 0, 1, 7, 8, 9 and 40 bytes, keys of 6 to 17
/// bytes), overwrites of kv2's keys and of earlier new ones, removes of base and new keys, and a
/// remove of a key that is not there.
fn op(i: usize, keys: &mut Vec<String>, vals: &mut Vec<Vec<u8>>) -> (bool, usize) {
    let (k, v): (String, Option<Vec<u8>>) = match i {
        3 => ("order/0002".into(), Some(b"cancelled".to_vec())),
        7 => ("courier/alpha".into(), None),
        11 => ("item/04".into(), None),
        15 => ("nothing/here".into(), None),
        19 => ("item/08".into(), Some(Vec::new())),
        23 => ("zone/north".into(), Some(b"{\"cap\":12,\"open\":true}".to_vec())),
        27 => ("order/0001".into(), None),
        31 => ("item/00".into(), Some(b"last".to_vec())),
        _ => {
            let k = if i % 5 == 0 { format!("item/{i:02}/long-key") } else { format!("item/{i:02}") };
            let n = [0usize, 1, 7, 8, 9, 40][i % 6];
            (k, Some((0..n).map(|j| (i * 31 + j * 7) as u8).collect()))
        }
    };
    keys.push(k);
    match v {
        Some(v) => {
            vals.push(v);
            (true, keys.len() - 1)
        }
        None => {
            vals.push(Vec::new());
            (false, keys.len() - 1)
        }
    }
}

fn append(st: &mut Store, k: &str, put: bool, v: &[u8]) -> i64 {
    let o = if put { Op::Put(k, v) } else { Op::Remove(k) };
    match append_delta_with(st, &[o], MAX_DELTAS, 0).expect("append") {
        Appended::Delta(g) => g,
        Appended::Compact(w) => panic!("the fixture must be a chain; the writer asked to compact: {w:?}"),
    }
}

fn say(name: &str, bytes: &[u8]) {
    let (s, n, r) = read(bytes);
    let gen = Store::from_bytes(bytes).pick().map(|sb| sb.generation).unwrap_or(-1);
    println!("{name}: {} bytes, generation {gen}, kv status={s} n={n} root={r}", bytes.len());
}

fn write(dir: &str) {
    let p = |f: &str| format!("{dir}/{f}");
    let mut st = Store::from_bytes(&std::fs::read(p("kv2.store")).expect("read kv2.store"));
    let (mut keys, mut vals) = (Vec::new(), Vec::new());
    let mut pre: Option<Vec<i64>> = None;
    for i in 0..MAX_DELTAS {
        if i == MAX_DELTAS - 1 {
            pre = Some(st.cells.clone());
        }
        let (put, at) = op(i, &mut keys, &mut vals);
        append(&mut st, &keys[at].clone(), put, &vals[at].clone());
    }
    let max = st.to_bytes_trimmed();
    // the 33rd append must ask for the compaction kv3c is
    let mut probe = Store::from_bytes(&max);
    match append_delta_with(&mut probe, &[Op::Put("one/more", b"x")], MAX_DELTAS, 0).expect("probe") {
        Appended::Compact(Why::Deltas) => {}
        other => panic!("a 33rd append answered {other:?}, not Compact(Deltas)"),
    }
    let sb = Store::from_bytes(&max).pick().expect("kv3max picks");
    std::fs::write(p("kv3max.store"), &max).expect("write kv3max");
    say("kv3max", &max);

    // the compaction
    let kv = Kv::load_checked(&Store::from_bytes(&max)).expect("kv3max reads");
    let ents: Vec<(Vec<u8>, Vec<u8>)> = kv.entries.iter().map(|(k, v)| (k.as_bytes().to_vec(), v.clone())).collect();
    let it = ents.iter().map(|(k, v)| (k.as_slice(), v.as_slice()));
    let c = bebop_kv::compacted_from_fit(it, 64 << 20, Some(sb.generation)).expect("compact");
    std::fs::write(p("kv3c.store"), &c).expect("write kv3c");
    say("kv3c", &c);

    // the torn superblock: kv3max's bytes with cells 8..15 of the NEW superblock from before it
    let pre = pre.expect("pre");
    let mut torn = Store::from_bytes(&max);
    for c in 8..16 {
        torn.cells[sb.at + c] = pre[sb.at + c];
    }
    let t = torn.to_bytes();
    let t = &t[..max.len()];
    let tsb = Store::from_bytes(t).pick().expect("torn picks");
    assert_ne!(tsb.at, sb.at, "the torn superblock must not be picked");
    assert_eq!(tsb.generation, sb.generation - 1);
    std::fs::write(p("torn.store"), t).expect("write torn");
    say("torn", t);
    let mut prev = Store { cells: pre };
    say("torn-want (the 31-delta image)", &prev.to_bytes_trimmed());
    prev.cells.clear();

    // the bad PartTab
    let mut bad = Store::from_bytes(&c);
    let live = bad.pick().expect("kv3c picks");
    let pt = bad.cells[live.at + 3] as usize;
    bad.cells[pt + 2 + 1] ^= 1;
    let b = bad.to_bytes();
    let b = &b[..c.len()];
    std::fs::write(p("ptbad.store"), b).expect("write ptbad");
    println!("ptbad: live superblock {} PartTab {} flipped cell {} bit 0", live.at, pt, pt + 3);
    say("ptbad", b);

    // empty
    let e = Store::create_bytes(16 * 1024).expect("create").to_bytes_trimmed();
    std::fs::write(p("empty.store"), &e).expect("write empty");
    say("empty", &e);

    // an empty KV
    let mut z = Store::create_bytes(16 * 1024).expect("create");
    Kv::init_bytes(&mut z).expect("init");
    let z = z.to_bytes_trimmed();
    std::fs::write(p("kv0.store"), &z).expect("write kv0");
    say("kv0", &z);

    kv4::write(dir, &max, &say);
}

fn main() {
    let a: Vec<String> = std::env::args().collect();
    match a.get(1).map(String::as_str) {
        Some("write") => write(a.get(2).expect("fixtures dir")),
        Some("read") => {
            for f in &a[2..] {
                let (s, n, r) = read(&std::fs::read(f).expect("read image"));
                println!("kv status={s} n={n} root={r}");
            }
        }
        // `why`: the library's refusal BY NAME (a status hides which rule), one line per image.
        Some("why") => {
            for f in &a[2..] {
                let st = Store::from_bytes(&std::fs::read(f).expect("read image"));
                println!("{f}: {:?}", Kv::load_checked(&st).map(|kv| kv.entries.len()));
            }
        }
        _ => {
            eprintln!("usage: bebop-conform write <fixtures dir> | read <image>... | why <image>...");
            std::process::exit(2);
        }
    }
}
