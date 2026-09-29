//! DG5: `fixtures/proj.store` -- a log image bebop WROTE, with a projection memo in it
//! (bench/vs_rust/std_tests/sproj.bp `f`) -- read by this crate to the numbers
//! `fixtures/proj.expected` derives. gate.sh greps the first test's name to count the
//! native reader among the four.

use bebop_store::evlog::{EvLog, Record};
use bebop_store::proj::{fn_key, lookup, proj_key, projtab, KIND_LOG, SB_ANC, SB_PROJTAB};
use bebop_store::{get_in, Store, View};
use bebop_wasm::proj::{bw_proj, proj_view, s1_holds, S1_BROKEN, STALE_MEMO};

fn fixture(name: &str) -> Vec<u8> {
    std::fs::read(format!("{}/fixtures/{name}.store", env!("CARGO_MANIFEST_DIR"))).expect("fixture")
}

/// `n=`, `root=`, `memo=` from fixtures/proj.expected, parsed.
fn expected() -> (i64, i64, i64) {
    let text = std::fs::read_to_string(format!("{}/fixtures/proj.expected", env!("CARGO_MANIFEST_DIR"))).expect("expected");
    let f = |k: &str| -> i64 { text.lines().find_map(|l| l.strip_prefix(k)).expect(k).trim().parse().expect("an integer") };
    (f("n="), f("root="), f("memo="))
}

#[test]
fn proj_fixture_reads_to_what_bebop_bin_printed() {
    let p = proj_view(&fixture("proj")).expect("the memo bebop wrote must read here");
    assert_eq!((p.n, p.fold, p.memo), expected());
    assert_eq!(p.memo, p.fold, "the memo IS the fold (S-2)");
}

/// The fixture is what DG5 says a bebop-written image is: the table named in cell 5 of the
/// live superblock, an anchor in cell 9, cells 13-14 zero.
#[test]
fn the_fixture_carries_the_table_and_the_anchor() {
    let img = fixture("proj");
    let v = View::new(&img);
    let sb = v.pick().expect("a live superblock");
    assert!(projtab(&v).is_some_and(|t| t as i64 == bebop_store::Cells::cell_at(&v, sb.at + SB_PROJTAB)));
    assert_ne!(bebop_store::Cells::cell_at(&v, sb.at + SB_ANC), 0, "bebop's commit writes the anchor");
}

/// The key bebop's `pj_key` wrote is the key this crate derives for the same code, fn and kind:
/// the node-key frame (RT §2) agrees across the two writers.
#[test]
fn bebops_memo_key_is_the_rust_key() {
    let img = fixture("proj");
    let v = View::new(&img);
    let t = projtab(&v).unwrap();
    let key = get_in(&v, t, 1);
    let p = lookup(&v, key).expect("row 0");
    assert_eq!(key, proj_key(p.code, fn_key("st_log_step"), KIND_LOG));
    assert_eq!(p.fn_key, fn_key("st_log_step"));
}

/// RT S-1, both ways: every fixture keeps cells 13-14 at zero, and a proj image whose live
/// superblock carries a non-zero cell 13 (crc re-sealed, so it is VALID) is refused.
#[test]
fn s1_holds_on_every_fixture_and_a_broken_one_is_refused() {
    for f in ["kv", "kv2", "proj"] {
        assert!(s1_holds(&fixture(f)), "{f}");
    }
    let mut img = fixture("proj");
    let at = View::new(&img).pick().unwrap().at;
    img[(at + 13) * 8] = 1;
    let crc = bebop_store::crc32(&img[at * 8..(at + 15) * 8]) as u64;
    img[(at + 15) * 8..(at + 16) * 8].copy_from_slice(&crc.to_le_bytes());
    assert!(View::new(&img).pick().is_some(), "still a valid superblock");
    assert_eq!(proj_view(&img), Err(S1_BROKEN));
}

/// A record appended after the memo (by the RUST writer, which carries the table forward)
/// leaves a memo that no longer describes the log: refused, not read as current.
#[test]
fn a_memo_behind_its_log_is_refused() {
    let mut st = Store::from_bytes(&fixture("proj"));
    let prev = EvLog::tip(&st).unwrap();
    let r = Record { id: [7; 32], prev, actor_pubkey: [0; 32], actor_seq: 9, payload: b"late".to_vec() };
    EvLog::append_tip_bytes(&mut st, &r).unwrap();
    assert_eq!(proj_view(&st.to_bytes_trimmed()), Err(STALE_MEMO));
}

#[test]
fn bw_proj_says_what_proj_view_says() {
    let img = fixture("proj");
    let mut out = [0i64; 3];
    assert_eq!(unsafe { bw_proj(img.as_ptr(), img.len(), out.as_mut_ptr()) }, bebop_wasm::abi::OK);
    assert_eq!((out[0], out[1], out[2]), expected());
    assert_eq!(unsafe { bw_proj(img.as_ptr(), img.len(), core::ptr::null_mut()) }, bebop_wasm::abi::NULL_ARG);
}
