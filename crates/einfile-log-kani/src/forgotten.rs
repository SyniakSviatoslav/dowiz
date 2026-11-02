//! FORGOTTEN: `"ein-forgotten/1" ‖ n_redacted LE8 ‖ subject 32 ‖ (id 32 ‖ digest 32) × n_redacted`.
//! `verify` sums `n_redacted` as `declared`, and law 9 balances it against the tombstones, so the
//! count a reader believes must be the list it holds.
//!
//! BOUNDS: parser harnesses: EVERY byte string of length <= 183, i.e. the 55-byte head plus a list
//! of 0, 1 or 2 entries, with any 64-bit `n` in the count field, the mark not forced (the solver
//! has to find it). Round trip: 0, 1 or 2 entries with any bytes. A list of three or more was not
//! looked at.

use crate::common::{id_at, u64_at, Bytes};
use einfile_log::doc::seg::{forgotten_body, parse_forgotten, Erased, FORGOTTEN_MARK};

/// Mark 15 + count 8 + subject 32.
const HEAD: usize = FORGOTTEN_MARK.len() + 40;
const ENTRY: usize = 64;
/// Longest body checked: two entries.
const N: usize = HEAD + 2 * ENTRY;

fn any_erased() -> Erased {
    Erased { id: kani::any(), digest: kani::any() }
}

/// PROPERTIES over any bytes:
///   (a) no panic, no overflow;
///   (b) when it parses, `n` in the body equals the entries returned, and the length is exactly
///       head + 64n;
///   (c) the entries are the bytes at their offsets;
///   (d) canonical: writing back what was parsed gives the same bytes, so one declaration has one
///       encoding.
#[kani::proof]
#[kani::unwind(192)] // past 183
fn forgotten_parser_on_any_bytes() {
    let input = Bytes::<N>::any();
    let b = input.as_slice();
    let got = parse_forgotten(b);
    kani::cover!(matches!(&got, Some((_, l)) if l.is_empty()), "an empty list parses");
    kani::cover!(matches!(&got, Some((_, l)) if l.len() == 2), "a two-entry list parses");
    if let Some((subject, list)) = &got {
        let at = FORGOTTEN_MARK.len();
        assert_eq!(u64_at(b, at), list.len() as u64);
        assert_eq!(b.len(), HEAD + list.len() * ENTRY);
        assert_eq!(*subject, id_at(b, at + 8));
        for (i, e) in list.iter().enumerate() {
            assert_eq!(e.id, id_at(b, HEAD + i * ENTRY));
            assert_eq!(e.digest, id_at(b, HEAD + i * ENTRY + 32));
        }
        assert_eq!(forgotten_body(subject, list), b);
    }
}

/// PROPERTY: `parse_forgotten` accepts EXACTLY the bodies with the mark, a whole number of entries,
/// and a count equal to that number. So a wrong `n`, a torn entry or a trailing byte is `None`.
#[kani::proof]
#[kani::unwind(192)]
fn forgotten_accepts_exactly_the_well_formed() {
    let input = Bytes::<N>::any();
    let b = input.as_slice();
    let tail = b.len().saturating_sub(HEAD);
    let well_formed = b.starts_with(FORGOTTEN_MARK) && b.len() >= HEAD && tail % ENTRY == 0 && (tail / ENTRY) as u64 == u64_at(b, FORGOTTEN_MARK.len());
    assert_eq!(parse_forgotten(b).is_some(), well_formed);
}

/// PROPERTY: round trip: `parse_forgotten(forgotten_body(s, l)) == Some((s, l))` for any subject and
/// any list of up to two entries.
#[kani::proof]
#[kani::unwind(192)]
fn forgotten_round_trip() {
    let subject: [u8; 32] = kani::any();
    let all = [any_erased(), any_erased()];
    let k: usize = kani::any_where(|k: &usize| *k <= 2);
    let list = &all[..k];
    let body = forgotten_body(&subject, list);
    assert_eq!(parse_forgotten(&body), Some((subject, list.to_vec())));
}
