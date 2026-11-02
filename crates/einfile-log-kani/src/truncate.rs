//! TRUNCATE: `"ein-truncate/1" ‖ root 32 ‖ prev_root 32 ‖ first LE8 ‖ last LE8 ‖ redacted LE8 ‖
//! declared LE8 ‖ seg_hash 32`, 142 bytes. The body that lets records leave the file.
//!
//! BOUNDS: EVERY byte string of length <= 150: the 142 of the body and 8 past it, the mark not
//! forced. The body has a fixed size and no loop over its contents, so the extra 8 are there only to
//! show what the parser does past the end. Round trip: every value of every field.
//!
//! EXACT, like FORGOTTEN and MERGE (W-MG-TIDY2): a byte past the end is a refusal. Same for REVOKE
//! and ADMIT (`keys.rs`).

use crate::common::{id_at, u64_at, Bytes};
use einfile_log::doc::seg::{parse_truncate, Segment, TRUNCATE_MARK};

/// Mark 14 + three ids and four counters.
const LEN: usize = TRUNCATE_MARK.len() + 128;
const N: usize = LEN + 8;

fn any_segment() -> Segment {
    Segment {
        first: kani::any(),
        last: kani::any(),
        root: kani::any(),
        prev: None,
        prev_root: kani::any(),
        blob: None,
        redacted: kani::any(),
        declared: kani::any(),
        frontier: Vec::new(),
        seg_hash: kani::any(),
    }
}

/// PROPERTIES over any bytes:
///   (a) no panic, no overflow;
///   (b) it parses exactly when the mark is there and the body is 142 bytes;
///   (c) each field is the bytes at its documented offset.
#[kani::proof]
#[kani::unwind(160)] // past 150
fn truncate_parser_on_any_bytes() {
    let input = Bytes::<N>::any();
    let b = input.as_slice();
    let got = parse_truncate(b);
    assert_eq!(got.is_some(), b.starts_with(TRUNCATE_MARK) && b.len() == LEN);
    kani::cover!(got.is_some(), "a TRUNCATE parses");
    if let Some(t) = got {
        let at = TRUNCATE_MARK.len();
        assert_eq!(t.root, id_at(b, at));
        assert_eq!(t.prev_root, id_at(b, at + 32));
        assert_eq!(t.first, u64_at(b, at + 64));
        assert_eq!(t.last, u64_at(b, at + 72));
        assert_eq!(t.redacted, u64_at(b, at + 80));
        assert_eq!(t.declared, u64_at(b, at + 88));
        assert_eq!(t.seg_hash, id_at(b, at + 96));
    }
}

/// PROPERTY: round trip with the writer. `truncate_body` is exactly 142 bytes; it parses to the
/// seven fields it was written from; the segment is named by what it wrote. `first`, `last`,
/// `redacted` and `declared` are unconstrained, so `first > last` is in: the body does not judge.
#[kani::proof]
#[kani::unwind(160)]
fn truncate_round_trip() {
    let s = any_segment();
    let body = s.truncate_body();
    assert_eq!(body.len(), LEN);
    let t = parse_truncate(&body).expect("the writer's body must parse");
    assert_eq!(t.root, s.root);
    assert_eq!(t.prev_root, s.prev_root);
    assert_eq!((t.first, t.last, t.redacted, t.declared), (s.first, s.last, s.redacted, s.declared));
    assert_eq!(t.seg_hash, s.seg_hash);
    assert!(s.is_named_by(&t));
}

/// PROPERTY: a body with bytes past the 142nd is no TRUNCATE, whatever its first 142 say.
#[kani::proof]
#[kani::unwind(160)]
fn truncate_refuses_trailing_bytes() {
    let input = Bytes::<N>::any();
    let b = input.as_slice();
    kani::assume(b.len() > LEN);
    kani::cover!(parse_truncate(&b[..LEN]).is_some(), "the 142 bytes alone are a TRUNCATE");
    assert_eq!(parse_truncate(b), None);
}
