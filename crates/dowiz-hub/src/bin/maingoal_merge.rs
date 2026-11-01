//! `maingoal_merge` (W-MG-M2): THE TWO OFFLINE DEVICES of `tools/gates/maingoal.sh`. Nothing here is a
//! test double and there is no server: each step is one device with its own key, its own file and
//! the file it was handed.
//!
//!     maingoal_merge write <out dir> --seed <hex 32 B>
//!     maingoal_merge merge <ours> <theirs> <out> --seed <hex 32 B> --who a|b --ms <claimed ms>
//!
//! `write` lays the files the gate merges, each pair from ONE genesis (device A receives, admits B,
//! the two part): calm-a/-b commute; salmon-a/-b sell the last salmon on both; fork-a/-b are A on
//! two copies; revoke-a/-b hold a REVOKE of a device that wrote past it; forget-a/-b an erasure on
//! one side; cut-a/-b a cut on one side; other a stranger's genesis; lie a MERGE, really signed,
//! over a record no file holds; hidden a MERGE, really signed, that leaves out the conflict.
//! Also merge.actor: A's id, hex, which is all an auditor is given.
//!
//! `merge` prints `merged records=<n> conflicts=<n>` / `adopted records=<n>` / `nothing`, and
//! writes <out> (the unchanged file on `nothing`); rc 0. rc 4 with `REFUSED <NAME>: ...` on stderr and
//! no <out>. rc 2 bad arguments, rc 1 a file that cannot be read.

use bebop_store::evlog::doc::keys::KeyOp;
use bebop_store::evlog::doc::{merge_body, MergeView, Row, STATE_COMMUTES};
use bebop_store::nodekey::sha256;
use dowiz_hub::crypto::unhex;
use dowiz_hub::doc_seal::merge::{refusal_name, Merged};
use dowiz_hub::doc_seal::{hex32, DeviceKey, SealedHub};
use dowiz_hub::stock::StockEvent;
use dowiz_hub::EventKind;

const T0: i64 = 1_761_900_000_000;
/// The person of the erasure; `tools/gates/maingoal.sh` greps for this exact text.
const PHONE: &str = "+355692222222";

fn tagged(tag: &str, seed: &[u8; 32]) -> [u8; 32] {
    let mut m = tag.as_bytes().to_vec();
    m.extend_from_slice(seed);
    sha256(&m)
}
fn device(who: &str, seed: &[u8; 32]) -> DeviceKey {
    DeviceKey::from_seed(&tagged(&format!("maingoal/merge-{who}"), seed))
}
fn got(item: &str, qty: i64) -> StockEvent {
    StockEvent::Received { item: item.to_string(), qty }
}
fn sold(item: &str, qty: i64, order: &str) -> StockEvent {
    StockEvent::Served { item: item.to_string(), qty, order_id: order.to_string() }
}

type R<T> = Result<T, String>;
fn er<E: std::fmt::Debug>(e: E) -> String {
    format!("{e:?}")
}

fn opened(bytes: &[u8], tag: &str, seed: &[u8; 32]) -> R<SealedHub> {
    let mut sh = SealedHub::load(bytes).map_err(er)?;
    sh.set_entropy(tagged(tag, seed));
    Ok(sh)
}

/// The shared past of every pair.
fn genesis(a: &DeviceKey, b: &DeviceKey, seed: &[u8; 32]) -> R<Vec<u8>> {
    let mut sh = SealedHub::create(a).map_err(er)?;
    sh.set_entropy(tagged("maingoal/merge-genesis", seed));
    sh.sync_stock(&[got("salmon", 1), got("rice", 10)], a, T0).map_err(er)?;
    sh.commit_keyop(&KeyOp::Admit(b.key()), a, T0 + 1000).map_err(er)?;
    Ok(sh.to_bytes_trimmed())
}

fn offline(split: &[u8], tag: &str, seed: &[u8; 32], key: &DeviceKey, moves: &[StockEvent]) -> R<Vec<u8>> {
    let mut sh = opened(split, tag, seed)?;
    sh.sync_stock(moves, key, T0 + 2000).map_err(er)?;
    Ok(sh.to_bytes_trimmed())
}

fn write(dir: &str, name: &str, bytes: &[u8]) -> R<()> {
    let path = format!("{dir}/{name}");
    std::fs::write(&path, bytes).map_err(|e| format!("{path}: {e}"))?;
    println!("device: wrote {name} ({} bytes)", bytes.len());
    Ok(())
}

/// One conflict's files, and the two MERGEs that lie.
fn write_merges(dir: &str, seed: &[u8; 32], a: &DeviceKey, b: &DeviceKey, split: &[u8]) -> R<()> {
    let salmon_a = offline(split, "sa", seed, a, &[sold("salmon", 1, "o1"), sold("rice", 2, "o1")])?;
    let salmon_b = offline(split, "sb", seed, b, &[sold("salmon", 1, "o2")])?;
    write(dir, "salmon-a.sealed", &salmon_a)?;
    write(dir, "salmon-b.sealed", &salmon_b)?;
    let rows = |sh: &SealedHub| -> R<Vec<Row>> { Ok(sh.verify().map_err(er)?.heads.iter().map(|h| Row { actor_id: h.actor_id, seq: h.seq, tip: h.tip }).collect()) };
    let mut lie = opened(&salmon_a, "lie", seed)?;
    let theirs = vec![Row { actor_id: b.id(), seq: 9, tip: [0x77; 32] }];
    let ours = rows(&lie)?;
    lie.declare(&merge_body(&MergeView { ours, theirs, state: STATE_COMMUTES, seg_root: [0; 32], conflicts: Vec::new() }), a, T0 + 3000).map_err(er)?;
    write(dir, "lie.sealed", &lie.to_bytes_trimmed())?;
    let mut hidden = opened(&salmon_a, "hidden", seed)?;
    let (ours, other) = (rows(&hidden)?, opened(&salmon_b, "hidden2", seed)?);
    hidden.merge(&salmon_b, a, T0 + 3000).map_err(er)?;
    let theirs = rows(&other)?;
    hidden.declare(&merge_body(&MergeView { ours, theirs, state: STATE_COMMUTES, seg_root: [0; 32], conflicts: Vec::new() }), a, T0 + 4000).map_err(er)?;
    write(dir, "hidden.sealed", &hidden.to_bytes_trimmed())
}

/// The pairs M2-min refuses, by name.
fn write_refusals(dir: &str, seed: &[u8; 32], a: &DeviceKey, b: &DeviceKey, split: &[u8]) -> R<()> {
    write(dir, "fork-a.sealed", &offline(split, "fa", seed, a, &[sold("rice", 1, "x")])?)?;
    write(dir, "fork-b.sealed", &offline(split, "fb", seed, a, &[sold("rice", 1, "y")])?)?;
    let tail = offline(split, "rb", seed, b, &[got("tuna", 1), got("tuna", 1)])?;
    write(dir, "revoke-b.sealed", &tail)?;
    let mut owner = opened(split, "ra", seed)?;
    owner.commit_keyop(&KeyOp::Revoke { actor_id: b.id(), after_seq: 0, tip: [0; 32] }, a, T0 + 3000).map_err(er)?;
    write(dir, "revoke-a.sealed", &owner.to_bytes_trimmed())?;

    let mut both = opened(split, "pa", seed)?;
    let json = format!(r#"{{"status":"new","contact":{{"phone":"{PHONE}"}}}}"#);
    both.append(EventKind::Placed, "p1", &json, a, T0 + 2000).map_err(er)?;
    let shared = both.to_bytes_trimmed();
    write(dir, "forget-b.sealed", &offline(&shared, "pb", seed, b, &[got("tuna", 1)])?)?;
    both.forget(|e| e.order_json.contains(PHONE).then(|| e.order_json.replace(PHONE, "")), &sha256(PHONE.as_bytes()), a, T0 + 3000).map_err(er)?;
    write(dir, "forget-a.sealed", &both.to_bytes_trimmed())?;

    let mut long = opened(split, "ca", seed)?;
    long.sync_stock(&[got("salt", 1)], a, T0 + 2000).map_err(er)?;
    long.sync_stock(&[got("oil", 1)], a, T0 + 2500).map_err(er)?;
    let shared = long.to_bytes_trimmed();
    write(dir, "cut-b.sealed", &offline(&shared, "cb", seed, b, &[got("tuna", 1)])?)?;
    long.cut(3, a, T0 + 3000).map_err(er)?;
    write(dir, "cut-a.sealed", &long.to_bytes_trimmed())
}

fn write_all(dir: &str, seed: &[u8; 32]) -> R<()> {
    let (a, b, c) = (device("a", seed), device("b", seed), device("stranger", seed));
    let split = genesis(&a, &b, seed)?;
    write(dir, "calm-a.sealed", &offline(&split, "ca1", seed, &a, &[sold("rice", 2, "o1"), sold("rice", 1, "o2")])?)?;
    write(dir, "calm-b.sealed", &offline(&split, "cb1", seed, &b, &[got("tuna", 4)])?)?;
    write_merges(dir, seed, &a, &b, &split)?;
    write_refusals(dir, seed, &a, &b, &split)?;
    write(dir, "other.sealed", &genesis(&c, &b, seed)?)?;
    write(dir, "merge.actor", hex32(&a.id()).as_bytes())
}

fn merge(ours: &str, theirs: &str, out: &str, seed: &[u8; 32], who: &str, ms: i64) -> R<i32> {
    let read = |p: &str| std::fs::read(p).map_err(|e| format!("{p}: {e}"));
    let key = device(who, seed);
    let mut sh = opened(&read(ours)?, &format!("merge-{who}-{ms}"), seed)?;
    let answer = sh.merge(&read(theirs)?, &key, ms);
    match answer {
        Err(e) => {
            eprintln!("REFUSED {}", refusal_name(&e));
            return Ok(4);
        }
        Ok(Merged::Nothing) => println!("nothing"),
        Ok(Merged::Adopted { records }) => println!("adopted records={records}"),
        Ok(Merged::Sealed { records, conflicts }) => println!("merged records={records} conflicts={}", conflicts.len()),
    }
    std::fs::write(out, sh.to_bytes_trimmed()).map_err(|e| format!("{out}: {e}"))?;
    Ok(0)
}

fn opt<'a>(argv: &'a [String], name: &str) -> Option<&'a str> {
    argv.iter().position(|a| a == name).and_then(|i| argv.get(i + 1)).map(String::as_str)
}

fn main() {
    let argv: Vec<String> = std::env::args().skip(1).collect();
    let usage = "usage: maingoal_merge write <dir> --seed <hex> | merge <ours> <theirs> <out> --seed <hex> --who a|b --ms <ms>";
    let Some(seed) = opt(&argv, "--seed").and_then(unhex).and_then(|b| <[u8; 32]>::try_from(b).ok()) else {
        eprintln!("maingoal_merge: {usage}");
        std::process::exit(2)
    };
    let run = match argv.iter().map(String::as_str).collect::<Vec<_>>().as_slice() {
        ["write", dir, ..] => write_all(dir, &seed).map(|_| 0),
        ["merge", ours, theirs, out, ..] => match (opt(&argv, "--who"), opt(&argv, "--ms").and_then(|m| m.parse().ok())) {
            (Some(who @ ("a" | "b")), Some(ms)) => merge(ours, theirs, out, &seed, who, ms),
            _ => Err(usage.to_string()),
        },
        _ => Err(usage.to_string()),
    };
    match run {
        Ok(rc) => std::process::exit(rc),
        Err(e) => {
            eprintln!("maingoal_merge: refused: {e}");
            std::process::exit(1)
        }
    }
}
