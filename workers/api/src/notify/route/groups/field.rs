//! The owner patch's small field rules, split from `groups.rs` for the
//! 300-line ratchet.
use serde::Deserialize;

/// A field that is present is `Some`, even when it is `null` (= clear it).
pub(super) fn present<'de, D: serde::Deserializer<'de>, T: Deserialize<'de>>(d: D) -> Result<Option<Option<T>>, D::Error> {
    Option::<T>::deserialize(d).map(Some)
}

/// A minute of the day.
pub(super) fn minute_ok(m: i64) -> bool {
    (0..1440).contains(&m)
}
