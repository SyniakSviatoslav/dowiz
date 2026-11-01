//! The key-op bodies:
//!   REVOKE  `"bebop-revoke/2" ‖ actor_id 32 ‖ after_seq LE8 ‖ tip 32`            86 bytes
//!   ADMIT   `"bebop-admit/1"  ‖ actor_id 32 ‖ alg LE8 ‖ valid_from LE8 ‖ pk_len LE8 ‖ pk`
//!
//! ADMIT was not in the lane's list. It is here because it is the one body of this family whose
//! length comes out of its own bytes through `idx`, and `settle.rs` runs it on every record.
//!
//! BOUNDS:
//!   REVOKE parser: EVERY byte string of length <= 94 (the body and 8 past it). Round trip: every
//!   value of every field, negative `after_seq` included.
//!   ADMIT parser: EVERY byte string of length <= 81, so a key of at most 12 bytes. `pk_len` is any
//!   i64, which covers the negative and the past-`MAX_PK_BYTES` refusals, but a pk of real size
//!   (1952 bytes) and `pk_len` exactly `MAX_PK_BYTES` inside a body that holds it are NOT reached.
//!   ADMIT round trip: pk of at most 8 bytes.
//!
//! As for TRUNCATE, both parsers are EXACT (W-MG-TIDY2): a byte past the end is a refusal.

use crate::common::{i64_at, id_at, Bytes};
use bebop_log::doc::keys::{admit_body, parse_admit, parse_revoke, revoke_body, ADMIT_MARK, MAX_PK_BYTES, REVOKE_MARK};

const REVOKE_LEN: usize = REVOKE_MARK.len() + 72;
const REVOKE_N: usize = REVOKE_LEN + 8;
const ADMIT_HEAD: usize = ADMIT_MARK.len() + 56;
const ADMIT_N: usize = ADMIT_HEAD + 12;

/// PROPERTIES over any bytes: no panic, no overflow; it parses exactly when the mark is there and
/// the body is 86 bytes; each field is the bytes at its offset; the writer gives the same bytes back.
#[kani::proof]
#[kani::unwind(102)] // past 94
fn revoke_parser_on_any_bytes() {
    let input = Bytes::<REVOKE_N>::any();
    let b = input.as_slice();
    let got = parse_revoke(b);
    assert_eq!(got.is_some(), b.starts_with(REVOKE_MARK) && b.len() == REVOKE_LEN);
    kani::cover!(got.is_some(), "a REVOKE parses");
    if let Some((actor, after_seq, tip)) = got {
        let at = REVOKE_MARK.len();
        assert_eq!(actor, id_at(b, at));
        assert_eq!(after_seq, i64_at(b, at + 32));
        assert_eq!(tip, id_at(b, at + 40));
        assert_eq!(revoke_body(&actor, after_seq, &tip), b);
    }
}

/// PROPERTY: round trip: any actor, any `after_seq` (including 0, negatives, `i64::MIN`), any tip.
#[kani::proof]
#[kani::unwind(102)]
fn revoke_round_trip() {
    let (actor, after_seq, tip): ([u8; 32], i64, [u8; 32]) = (kani::any(), kani::any(), kani::any());
    let body = revoke_body(&actor, after_seq, &tip);
    assert_eq!(body.len(), REVOKE_LEN);
    assert_eq!(parse_revoke(&body), Some((actor, after_seq, tip)));
}

/// PROPERTY: a body with bytes past the 86th is no REVOKE, whatever its first 86 say.
#[kani::proof]
#[kani::unwind(102)]
fn revoke_refuses_trailing_bytes() {
    let input = Bytes::<REVOKE_N>::any();
    let b = input.as_slice();
    kani::assume(b.len() > REVOKE_LEN);
    kani::cover!(parse_revoke(&b[..REVOKE_LEN]).is_some(), "the 86 bytes alone are a REVOKE");
    assert_eq!(parse_revoke(b), None);
}

/// PROPERTIES over any bytes: no panic, no overflow; it parses exactly when the mark is there,
/// `pk_len` is in `0..=MAX_PK_BYTES`, and the body holds exactly that many bytes after the head; the
/// key returned has `pk_len` bytes, and they are all the bytes after the head.
#[kani::proof]
#[kani::unwind(90)] // past 81
fn admit_parser_on_any_bytes() {
    let input = Bytes::<ADMIT_N>::any();
    let b = input.as_slice();
    let got = parse_admit(b);
    let pk_len = if b.len() >= ADMIT_HEAD { i64_at(b, ADMIT_MARK.len() + 48) } else { -1 };
    let room = b.len().saturating_sub(ADMIT_HEAD) as i64;
    let fits = (0..=MAX_PK_BYTES as i64).contains(&pk_len) && pk_len == room;
    assert_eq!(got.is_some(), b.starts_with(ADMIT_MARK) && b.len() >= ADMIT_HEAD && fits);
    kani::cover!(matches!(&got, Some((.., pk)) if !pk.is_empty()), "an ADMIT with a key parses");
    if let Some((actor, alg, valid_from, pk)) = got {
        let at = ADMIT_MARK.len();
        assert_eq!(actor, id_at(b, at));
        assert_eq!(alg, i64_at(b, at + 32));
        assert_eq!(valid_from, i64_at(b, at + 40));
        assert_eq!(pk.len() as i64, pk_len);
        assert_eq!(&pk[..], &b[ADMIT_HEAD..]);
    }
}

/// PROPERTY: round trip: any actor, alg, valid_from, and any key of at most 8 bytes.
#[kani::proof]
#[kani::unwind(90)]
fn admit_round_trip() {
    let (actor, alg, valid_from): ([u8; 32], i64, i64) = (kani::any(), kani::any(), kani::any());
    let pk = Bytes::<8>::any();
    let body = admit_body(&actor, alg, valid_from, pk.as_slice());
    assert_eq!(parse_admit(&body), Some((actor, alg, valid_from, pk.as_slice().to_vec())));
}
