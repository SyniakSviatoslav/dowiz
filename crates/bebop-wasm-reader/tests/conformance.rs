// SPDX-License-Identifier: MIT OR Apache-2.0
//! This crate answers what conformance.sh's five readers answer, on every one of its fixtures,
//! and the ABI round-trips natively (the exact functions node calls).
//!
//! The fixtures and their `.expected` are `crates/bebop-wasm/fixtures` -- READ, never copied:
//! a number here that is not in that directory is a number nobody else has agreed to.

use bebop_wasm_reader::abi::*;
use bebop_wasm_reader::status::{self, TABLE};
use bebop_wasm_reader::{open, Kind, Refusal};

/// conformance.sh's FIXTURES line, verbatim (W-ES-P10).
const FIXTURES: &str = "kv kv2 kv3 ptaway kv4stub kv3max kv3c torn ptbad empty kv0 kv4small kv4deep kv4del kv4zero kv4badn kv4badv kv4badf kv4badd kv4stale kv4old";

fn fixtures_dir() -> String {
    format!("{}/../bebop-wasm/fixtures", env!("CARGO_MANIFEST_DIR"))
}
fn fixture(name: &str) -> Vec<u8> {
    std::fs::read(format!("{}/{name}.store", fixtures_dir())).expect("fixture")
}
/// `(status, n, root)` as `.expected` states it: `status=` alone for a refusal fixture.
fn expected(name: &str) -> (i32, i64, i64) {
    let text = std::fs::read_to_string(format!("{}/{name}.expected", fixtures_dir())).expect("expected");
    let field = |k: &str| text.lines().find_map(|l| l.strip_prefix(k)).map(|v| v.trim().parse::<i64>().expect("int"));
    match field("status=") {
        Some(s) => (s as i32, 0, 0),
        None => (0, field("n=").expect("n="), field("root=").expect("root=")),
    }
}
fn answer(bytes: &[u8]) -> (i32, i64, i64) {
    match open(bytes) {
        Ok(d) => (0, d.len() as i64, d.fold),
        Err(r) => (status_of(&r), 0, 0),
    }
}

/// The gate: 21 KV fixtures, each to the line its `.expected` holds.
#[test]
fn every_conformance_fixture_answers_its_expected_line() {
    for name in FIXTURES.split(' ') {
        let want = expected(name);
        let got = answer(&fixture(name));
        assert_eq!(got, want, "{name}.store: want {want:?}, this reader says {got:?}");
    }
}

/// The log fixture (LV-INC-A proj4.log.store, the Rust writer's 9-record log): `n` and the LOG
/// fold. `proj4.log.expected`'s root is the kind-3 LogSummary fold, not the log's; the log fold
/// of that chain is the number its header comment states, 6344481284825166345, which is also
/// `proj.expected`'s root (the same 9 records, written by bebop) -- read from there.
#[test]
fn the_log_fixture_reads_to_its_expected_fold() {
    let d = open(&fixture("proj4.log")).expect("a log image reads");
    assert_eq!(d.kind, Kind::Log);
    let (_, n, root) = expected("proj");
    assert_eq!((d.len() as i64, d.fold), (n, root));
    assert!(d.records.windows(2).all(|w| w[0].actor_seq < w[1].actor_seq), "records come oldest first");
}

/// THE ABI, natively: open, kind, len, root, get, scan (its frames fold to the root), the log
/// calls, status text, verify -- every return value of abi.rs exercised, in one test because the
/// slot is one.
#[test]
fn the_abi_round_trips_natively() {
    unsafe {
        assert_eq!(bw_version(), ABI_VERSION);
        assert_eq!(bw_kind(), 0);
        assert_eq!(bw_len(), -(status::NOT_OPEN as i64));
        assert_eq!(bw_verify(2), status::NOT_OPEN);
        let mut root = 0i64;
        assert_eq!(bw_root(&mut root), status::NOT_OPEN);
        assert_eq!(bw_open(core::ptr::null(), 0), status::NULL_ARG);

        // kv3: a v3 image, five entries, `order/0002` -> "cancelled" (kv3.expected's comment).
        let img = fixture("kv3");
        assert_eq!(bw_open(img.as_ptr(), img.len()), status::OK);
        assert_eq!(bw_kind(), 3);
        assert_eq!(bw_len(), 5);
        assert_eq!(bw_root(&mut root), status::OK);
        assert_eq!(root, expected("kv3").2);
        let mut buf = vec![0u8; 4096];
        let n = bw_get(b"order/0002".as_ptr(), 10, buf.as_mut_ptr(), buf.len());
        assert_eq!(&buf[..n as usize], b"cancelled");
        assert_eq!(bw_get(b"courier/alpha".as_ptr(), 13, buf.as_mut_ptr(), buf.len()), -(status::NOT_FOUND as i64));
        // too small: the length comes back and nothing is written
        buf[0] = 0xAA;
        assert_eq!(bw_get(b"order/0002".as_ptr(), 10, buf.as_mut_ptr(), 3), 9);
        assert_eq!(buf[0], 0xAA);
        assert_eq!(bw_log_len(), -(status::NOT_A_LOG as i64));
        assert_eq!(bw_log_at(0, buf.as_mut_ptr(), buf.len()), -(status::NOT_A_LOG as i64));
        // the scan: frames in key order, hashed exactly as the fold is
        let (mut h, mut i, mut keys) = (0xcbf2_9ce4_8422_2325u64, 0i64, Vec::new());
        loop {
            let n = bw_scan_next(i, buf.as_mut_ptr(), buf.len());
            assert!(n >= 0, "scan refused at {i}: {n}");
            if n == 0 {
                break;
            }
            let f = &buf[..n as usize];
            let kl = u64::from_le_bytes(f[..8].try_into().unwrap()) as usize;
            keys.push(f[8..8 + kl].to_vec());
            for &b in f {
                h ^= b as u64;
                h = h.wrapping_mul(0x100_0000_01b3);
            }
            i += 1;
        }
        assert_eq!(i, 5);
        assert_eq!(h as i64, root, "the scan's frames fold to the root");
        assert!(keys.windows(2).all(|w| w[0] < w[1]), "key order");
        assert_eq!(bw_verify(0), status::OK);
        assert_eq!(bw_verify(2), status::OK);

        // a refusal closes the slot
        let bad = fixture("kv4old");
        assert_eq!(bw_open(bad.as_ptr(), bad.len()), status::BAD_CRC);
        assert_eq!(bw_kind(), 0);

        // the log: records oldest first, the frame layout, past the end
        let log = fixture("proj4.log");
        assert_eq!(bw_open(log.as_ptr(), log.len()), status::OK);
        assert_eq!(bw_kind(), KIND_LOG);
        assert_eq!(bw_log_len(), 9);
        assert_eq!(bw_len(), 9);
        assert_eq!(bw_get(b"x".as_ptr(), 1, buf.as_mut_ptr(), buf.len()), -(status::NOT_A_KV as i64));
        assert_eq!(bw_scan_next(0, buf.as_mut_ptr(), buf.len()), -(status::NOT_A_KV as i64));
        let n = bw_log_at(0, buf.as_mut_ptr(), buf.len()) as usize;
        let plen = u64::from_le_bytes(buf[8..16].try_into().unwrap()) as usize;
        assert_eq!(n, 112 + plen, "seq8 || plen8 || id32 || prev32 || pk32 || payload");
        assert_eq!(bw_log_at(9, buf.as_mut_ptr(), buf.len()), -(status::OUT_OF_RANGE as i64));
        assert_eq!(bw_log_at(-1, buf.as_mut_ptr(), buf.len()), -(status::OUT_OF_RANGE as i64));

        // status text: every row of the table, and nothing for a number the reader never gives
        for (code, line) in TABLE {
            let n = bw_status_text(*code, buf.as_mut_ptr(), buf.len()) as usize;
            assert_eq!(std::str::from_utf8(&buf[..n]).unwrap(), *line);
        }
        assert_eq!(bw_status_text(99, buf.as_mut_ptr(), buf.len()), 0);
        assert_eq!(bw_status_text(0, core::ptr::null_mut(), 0), -(status::NULL_ARG as i64));
        bw_close();
        assert_eq!(bw_kind(), 0);
    }
}

/// Every `Refusal` has a row in the table, so no status leaves this crate without words.
#[test]
fn the_status_table_is_total_over_refusal() {
    let all = [
        Refusal::NoSuperblock,
        Refusal::NotAKv,
        Refusal::NotALog,
        Refusal::Truncated { claimed: 1, chained: None },
        Refusal::BadCrc { obj: 1024 },
        Refusal::UnknownVersion,
        Refusal::LastCommit,
        Refusal::CutUndeclared,
    ];
    for r in all {
        assert!(status::text(status_of(&r)).is_some(), "{r:?} has no line");
    }
    for c in [status::NULL_ARG, status::NOT_OPEN, status::NOT_FOUND, status::OUT_OF_RANGE, status::NOT_A_DOC] {
        assert!(status::text(c).is_some(), "{c} has no line");
    }
}

/// The cell `conformance.sh --prove` names (bit 0 of the first value cell of kv3max's newest
/// delta record), flipped here the same way: BAD_CRC naming that record, never a fallback.
#[test]
fn the_prove_cell_moves_kv3max_to_bad_crc_naming_the_record() {
    let mut b = fixture("kv3max");
    let c = |b: &[u8], i: usize| i64::from_le_bytes(b[8 * i..8 * i + 8].try_into().unwrap());
    let sb = [0usize, 512].into_iter().filter(|&at| c(&b, at) == 3554557610294396226).max_by_key(|&at| c(&b, at + 2)).unwrap();
    let pt = c(&b, sb + 3) as usize;
    let root = c(&b, pt + 18) as usize;
    let rec = (root as i64 + c(&b, root + 2 + 6)) as usize;
    let cell = rec + 2 + 4 + (c(&b, rec + 4) as usize + 7) / 8;
    assert_eq!(answer(&b).0, 0, "clean kv3max reads");
    b[8 * cell] ^= 1;
    assert_eq!(open(&b).err(), Some(Refusal::BadCrc { obj: rec }), "cell {cell} of record {rec}");
}

/// RED for `bw_verify(2)`: an object `open` does NOT reach (a dead one of an older generation
/// in kv4del, whose deletes superseded leaves and values) with one bit flipped still OPENS and
/// is caught only by the whole-arena walk, naming it. At least one such object must exist, or
/// this test measured nothing.
#[test]
fn verify_whole_arena_catches_a_dead_object_open_does_not() {
    let clean = fixture("kv4del");
    let d = open(&clean).expect("kv4del reads");
    assert_eq!(d.verify_arena(), Ok(()), "the clean arena verifies whole");
    let c = |b: &[u8], i: usize| i64::from_le_bytes(b[8 * i..8 * i + 8].try_into().unwrap());
    let used = d.superblock().arena_used as usize;
    let (mut obj, mut dead_caught) = (1024usize, 0);
    while obj < used {
        let len = (c(&clean, obj) & 0xFFFF_FFFF) as usize;
        if len > 0 {
            let mut b = clean.clone();
            b[8 * (obj + 2)] ^= 1;
            if let Ok(d2) = open(&b) {
                assert_eq!(d2.verify_arena(), Err(Refusal::BadCrc { obj }), "object {obj} is dead and must be named");
                dead_caught += 1;
            }
        }
        obj += 2 + len;
    }
    assert!(dead_caught > 0, "kv4del has no dead object to flip: the test measured nothing");
}
