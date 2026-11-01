//! W-MG-ATK5: WHAT A MERGE OR ITS AUDIT COSTS ON A HOSTILE FILE. Closed by W-MG-MERGE2, kept as law.
mod atk5_common;
mod atk_forge;

use atk5_common::*;
use atk_forge::{forged, read, write};
use bebop_store::evlog::doc::{merge_body, Ids, MergeView, Row, Signer, STATE_COMMUTES};
use bebop_store::evlog::Record;
use dowiz_hub::doc_seal::merge::{merge_report, refusal_name, MergeError};
use dowiz_hub::doc_seal::{HubIds, SealedHub, KIND_DECL, KIND_STOCK};
use std::time::Instant;

fn rec(actor: [u8; 32], seq: u64, prev: [u8; 32], kind: u8, body: &[u8]) -> Record {
    let mut r = forged(actor, seq, prev, 0x33);
    r.payload.truncate(16);
    r.payload.push(kind);
    r.payload.extend_from_slice(body);
    r.id = HubIds.chained(&r);
    r
}

/// `n` movements of two no-key actors, `n` different items, and `m` MERGEs each of `rows` rows that
/// name nobody who wrote. No private key was used; `doc_verify` runs `merge_report` on a file that
/// does not hold as on one that does.
fn shape(n: usize, m: usize, rows: usize) -> Vec<u8> {
    let s = genesis(&[received("rice", 1)]);
    let mut side = read(&s);
    let (x, y) = ([0x71u8; 32], [0x72u8; 32]);
    let mut prev = [0u8; 32];
    for i in 0..n {
        let line = dowiz_hub::stock::encode(&served(&format!("item{i}"), 1, "o"));
        let r = rec(if i % 2 == 0 { x } else { y }, i as u64 + 1, prev, KIND_STOCK, line.as_bytes());
        prev = r.id;
        side.recs.push(r);
    }
    let noise = Row { actor_id: [0x7F; 32], seq: 1, tip: [0; 32] };
    let side_of = |who| -> Vec<Row> {
        let mut v = vec![noise; rows];
        v.push(Row { actor_id: who, seq: n as u64, tip: [0; 32] });
        v
    };
    let body = merge_body(&MergeView { ours: side_of(x), theirs: side_of(y), state: STATE_COMMUTES, seg_root: [0; 32], conflicts: Vec::new() });
    for i in 0..m {
        let r = rec([0x73; 32], i as u64 + 1, prev, KIND_DECL, &body);
        prev = r.id;
        side.recs.push(r);
    }
    write(&side)
}
fn run(f: &[u8]) -> f64 {
    let sh = SealedHub::load(f).unwrap();
    let t = Instant::now();
    let holds = sh.verify().unwrap().holds();
    assert!(!holds, "this shape holds nothing: no key signed it");
    let (lines, _) = merge_report(&sh, holds);
    let s = t.elapsed().as_secs_f64();
    println!("ATK5 audit: file={} B lines={} time={s:.3}s", f.len(), lines.len());
    assert_eq!(lines, vec!["merge recheck skipped: the file does not hold".to_string()], "a file nobody signed gets no fold");
    s
}

/// Double records, MERGEs and rows: a bounded audit is about 2x. It was cubic, on a file nobody
/// signed. The stopwatch includes `verify`, which `atk2_dos` bounds.
#[test]
fn the_audit_of_a_hostile_file_is_about_linear() {
    if std::env::var("ATK5_FULL").is_ok() {
        run(&shape(4800, 24, 2400));
    }
    let (f1, f2) = (shape(600, 3, 300), shape(1200, 6, 600));
    let (t1, t2) = (run(&f1), run(&f2));
    println!("ATK5 growth: x{:.2} file x{:.2} time", f2.len() as f64 / f1.len() as f64, t2 / t1);
    assert!(t2 / t1 < 4.0 || t2 < 0.01, "HOLE 12: x2 file took x{:.1} time ({t2:.2}s)", t2 / t1);
}

/// A KEY HOLDER'S FILE HOLDS, so it does reach the fold: M MERGEs over n movements of n items.
/// Two honest tails under every MERGE, each signed. Double the records and the MERGEs: x4 is M*n,
/// the price of asking each MERGE, and a rich `levels` scan on top of it was x8.
fn signed(n: usize, m: usize) -> Vec<u8> {
    let s = genesis(&[received("rice", 1)]);
    let many = |tag: &str| -> Vec<dowiz_hub::stock::StockEvent> { (0..n).map(|i| served(&format!("{tag}{i}"), 1, "o")).collect() };
    let (x, _) = merged(&offline(&s, 2, &a(), &many("a")), &offline(&s, 3, &b(), &many("b")), &a(), 4);
    let mut sh = opened(&x, 5);
    let v = sh.merge_lines()[0].view.clone().unwrap();
    for i in 0..m {
        sh.declare(&merge_body(&v), &a(), 600 + i as i64).unwrap();
    }
    trimmed(&sh)
}
#[test]
fn the_audit_of_a_signed_file_is_no_worse_than_merges_times_records() {
    let time = |f: &[u8]| {
        let sh = SealedHub::load(f).unwrap();
        let t = Instant::now();
        let (lines, clean) = merge_report(&sh, true);
        let s = t.elapsed().as_secs_f64();
        println!("ATK5 signed audit: file={} B lines={} clean={clean} time={s:.3}s", f.len(), lines.len());
        s
    };
    let (f1, f2) = (signed(400, 3), signed(800, 6));
    let (t1, t2) = (time(&f1), time(&f2));
    println!("ATK5 signed growth: x{:.2} file x{:.2} time", f2.len() as f64 / f1.len() as f64, t2 / t1);
    assert!(t2 / t1 < 6.5, "HOLE 12b: x2 records and MERGEs took x{:.1} time", t2 / t1);
}

/// `TailDelta` and `plus` add without a check, and each side folds alone, so the ledger's checked
/// arithmetic passes and the sum of the two levels does not fit: a panic in a debug build, in a
/// release build a wrapped level. Either way the answer is a named refusal.
#[test]
fn two_huge_deliveries_do_not_panic_the_merge() {
    let s = genesis(&[received("salt", 1)]);
    let huge = i64::MAX - 10;
    let fa = offline(&s, 2, &a(), &[received("salt", huge)]);
    let fb = offline(&s, 3, &b(), &[received("salt", huge)]);
    let got = std::panic::catch_unwind(|| opened(&fa, 4).merge(&fb, &a(), 500));
    assert!(got.is_ok(), "HOLE 11: a key holder's quantity panicked the merge");
    let e = got.unwrap().unwrap_err();
    assert!(matches!(e, MergeError::Ledger(_)), "{e:?}");
    println!("ATK5 huge: {}", refusal_name(&e));
    assert!(refusal_name(&e).starts_with("LEDGER: "));
    // THE TWIN: the largest sum that fits is an honest answer, not a refusal.
    let big = i64::MAX / 8;
    let (fa, fb) = (offline(&s, 2, &a(), &[received("salt", big)]), offline(&s, 3, &b(), &[received("salt", big)]));
    assert!(opened(&fa, 4).merge(&fb, &a(), 500).is_ok());
}
