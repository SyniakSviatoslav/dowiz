//! A minted id, cut to fit. The hub keeps an id to 64 BYTES (`supplies::check`,
//! and every reader of a dish id); a Cyrillic letter is two, so a cut counted in
//! letters let a long Ukrainian name past 64 and the whole write was refused.

/// A minted supply id's base in bytes: the tail ("-99") must still fit in 64.
pub(crate) const ID_BASE_BYTES: usize = 60;

/// `s` cut to `max` bytes on a letter's edge, with no dash left at the cut. PURE.
pub(crate) fn fit(s: &str, max: usize) -> String {
    let mut end = 0;
    for (i, c) in s.char_indices() {
        if i + c.len_utf8() > max {
            break;
        }
        end = i + c.len_utf8();
    }
    s[..end].trim_end_matches('-').to_string()
}
