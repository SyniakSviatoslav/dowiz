//! MERGE, the one body with variable-length parts inside it:
//!
//! ```text
//! "ein-merge/1" ‖ n_ours LE8 ‖ rows ‖ n_theirs LE8 ‖ rows ‖ state u8 ‖ seg_root 32
//!                 ‖ dup_declared LE8 ‖ held_n LE8 ‖ n_conf LE8 ‖ (len LE8 ‖ bytes) × n_conf
//! ```
//! A row is 72 bytes. `X` below is where `state` lands: 11 + 16 + 72 × rows.
//!
//! BOUNDS:
//!   `merge_parser_no_rows`: EVERY byte string of length <= 108, enough for the 84-byte minimum and
//!   24 more: no row fits, up to three conflicts do. The cheap one; run it first.
//!   `merge_parser_one_row`: EVERY byte string of length <= 168: at most one row in all, with 12
//!   bytes left for conflicts. Includes the former; kept apart because this one may be expensive.
//!   Neither reaches two rows, or a row on each side (228 bytes).
//!   `merge_round_trip`: up to one row per side, so the two-row case IS reached from the writer's
//!   side; at most one conflict of at most 2 bytes.
//!   Counts are any u64 in the parser harnesses, so the "more rows than bytes" refusal is covered.
//!
//! NOT REACHED: `walk_merge` is `pub(crate)`, so `row` returning false (the `merges_bad` stop) is
//! not exercised: this crate only has `parse_merge`, whose callback always says go on.

use crate::common::{id_at, u64_at, Bytes};
use einfile_log::doc::merge_body::{merge_body, parse_merge, MergeView, Row, MERGE_MARK, STATE_COMMUTES, STATE_FAST_FORWARD, STATE_OBSTRUCTED};

const ROW: usize = 72;
/// Everything but the rows and the conflicts: mark, two counts, state, root, two zero words, count.
const MIN: usize = MERGE_MARK.len() + 16 + 1 + 32 + 16 + 8;
const NO_ROWS_N: usize = MIN + 24;
const ONE_ROW_N: usize = MIN + ROW + 12;

fn any_row() -> Row {
    Row { actor_id: kani::any(), seq: kani::any(), tip: kani::any() }
}

fn any_rows() -> Vec<Row> {
    if kani::any() {
        vec![any_row()]
    } else {
        Vec::new()
    }
}

/// The same four-part check for both parser harnesses, over any bytes:
///   (a) no panic, no overflow, and no huge walk: a count is judged before a row is read;
///   (b) both row counts and the conflict count in the body equal what came back;
///   (c) the length is EXACTLY what those parts add up to: the header's "EXACT";
///   (d) the reserved words are zero, `state` is 1..=3, and state 3 holds if and only if a conflict
///       is named;
///   (e) canonical: `merge_body` of what was parsed is the same bytes.
/// (c), (d) and (e) together mean a torn body, a trailing byte, a bad state or a set reserved bit
/// within the bound is `None`, so no separate refusal harness is needed.
fn check_merge(b: &[u8]) {
    let got = parse_merge(b);
    kani::cover!(got.is_some(), "a MERGE parses");
    kani::cover!(got.as_ref().is_some_and(|v| !v.conflicts.is_empty()), "a MERGE with a conflict parses");
    let Some(v) = got else { return };
    let m = MERGE_MARK.len();
    let (o, t) = (v.ours.len(), v.theirs.len());
    assert_eq!(u64_at(b, m), o as u64);
    assert_eq!(u64_at(b, m + 8 + o * ROW), t as u64);
    let x = m + 16 + (o + t) * ROW;
    assert_eq!(b[x], v.state);
    assert_eq!(id_at(b, x + 1), v.seg_root);
    assert_eq!(u64_at(b, x + 33), 0);
    assert_eq!(u64_at(b, x + 41), 0);
    assert_eq!(u64_at(b, x + 49), v.conflicts.len() as u64);
    let conflicts: usize = v.conflicts.iter().map(|c| 8 + c.len()).sum();
    assert_eq!(b.len(), x + 57 + conflicts);
    assert!(v.state >= STATE_FAST_FORWARD && v.state <= STATE_OBSTRUCTED);
    assert_eq!(v.state == STATE_OBSTRUCTED, !v.conflicts.is_empty());
    assert_eq!(merge_body(&v), b);
}

/// PROPERTIES: `check_merge`, every byte string up to 108 bytes.
#[kani::proof]
#[kani::unwind(118)] // past 108
fn merge_parser_no_rows() {
    let input = Bytes::<NO_ROWS_N>::any();
    check_merge(input.as_slice());
}

/// PROPERTIES: `check_merge`, every byte string up to 168 bytes.
#[kani::proof]
#[kani::unwind(178)] // past 168
fn merge_parser_one_row() {
    let input = Bytes::<ONE_ROW_N>::any();
    let b = input.as_slice();
    kani::cover!(parse_merge(b).is_some_and(|v| v.ours.len() + v.theirs.len() == 1), "a MERGE with a row parses");
    check_merge(b);
}

/// PROPERTY: round trip: `parse_merge(merge_body(v)) == Some(v)` for every view the header allows,
/// i.e. state 1 or 2 with no conflict, state 3 with one. FastForward is included: it is never
/// written, and the header says it parses.
#[kani::proof]
#[kani::unwind(254)] // past the longest body: 84 + 144 + 10
fn merge_round_trip() {
    let c = Bytes::<2>::any();
    let conflicts = if kani::any() { vec![c.as_slice().to_vec()] } else { Vec::new() };
    let state = if conflicts.is_empty() {
        if kani::any() { STATE_COMMUTES } else { STATE_FAST_FORWARD }
    } else {
        STATE_OBSTRUCTED
    };
    let v = MergeView { ours: any_rows(), theirs: any_rows(), state, seg_root: kani::any(), conflicts };
    assert_eq!(parse_merge(&merge_body(&v)), Some(v));
}
