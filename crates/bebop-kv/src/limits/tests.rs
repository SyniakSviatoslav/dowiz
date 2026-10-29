//! T-ES-16 (the pure half): every limit accepted AT its edge and refused one past it, named.
//! No 1 GiB buffer: the check is over a length. RED mutants (W-ES-P3 hand-back): M16a `>` -> `>=`
//! refuses the edge; the put-side half (M16b) is bebop-io's `db/tests.rs`.
use super::*;

#[test]
fn t_es_16_key_65535_is_accepted_and_65536_is_refused() {
    assert_eq!(Limits::FORMAT.check_key(65_535), Ok(()));
    assert_eq!(Limits::FORMAT.check_key(65_536), Err(LimitError::KeyTooLong { len: 65_536, max: 65_535 }));
}

#[test]
fn t_es_16_value_one_gib_is_accepted_and_one_more_byte_is_refused() {
    assert_eq!(Limits::FORMAT.check_value(1 << 30), Ok(()));
    assert_eq!(Limits::FORMAT.check_value((1 << 30) + 1), Err(LimitError::ValueTooLong { len: (1 << 30) + 1, max: 1 << 30 }));
}

#[test]
fn t_es_16_a_holder_can_lower_the_limits_but_never_raise_them() {
    let l = Limits { key: usize::MAX, value: 7 }.lowered();
    assert_eq!(l, Limits { key: 65_535, value: 7 });
    assert_eq!(l.check_value(7), Ok(()));
    assert_eq!(l.check_value(8), Err(LimitError::ValueTooLong { len: 8, max: 7 }));
}
