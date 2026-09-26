//! PURE. The words under the room's and the console's extra grammar: one
//! normaliser, one "is this word here", one number reader.
//!
//! The hub's `dowiz_hub::voice` keeps its own `norm`/`has` private; these are
//! the same rules (lowercase, punctuation out, whole words only) with one
//! difference that matters for Ukrainian: an APOSTROPHE IS DROPPED, not turned
//! into a space, so "п'ять" and "пять" are one word -- a recogniser writes
//! whichever it likes, and "п ять" would be two words neither of which is five.

/// Lowercase, apostrophes gone, every other non-letter-or-digit a single space.
pub fn norm(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut space = false;
    for c in s.to_lowercase().chars() {
        if matches!(c, '\'' | '\u{2019}' | '\u{02bc}' | '`') {
            continue;
        }
        if c.is_alphanumeric() {
            if space && !out.is_empty() {
                out.push(' ');
            }
            space = false;
            out.push(c);
        } else {
            space = true;
        }
    }
    out
}

/// The words of a normalised utterance.
pub fn words(t: &str) -> Vec<&str> {
    t.split(' ').filter(|w| !w.is_empty()).collect()
}

/// Is any of `needles` one of the words? Whole words only: "on" must not fire
/// inside "one", and a substring test would fire on half the vocabulary.
pub fn has(ws: &[&str], needles: &[&str]) -> bool {
    ws.iter().any(|w| needles.contains(w))
}

/// The biggest quantity or table a voice command may carry. A misheard
/// "twenty-two" as "2 2" is two numbers and is refused; anything above this is
/// not a round anybody says aloud and is refused rather than rung.
pub const MAX_SAID: u32 = 99;

/// Spoken numbers, one to twelve, in the three languages. Recognisers mostly
/// write digits; these are for the ones that do not.
const NUMBER_WORDS: &[(&str, u32)] = &[
    ("one", 1), ("two", 2), ("three", 3), ("four", 4), ("five", 5), ("six", 6),
    ("seven", 7), ("eight", 8), ("nine", 9), ("ten", 10), ("eleven", 11), ("twelve", 12),
    ("один", 1), ("одна", 1), ("одну", 1), ("одне", 1), ("два", 2), ("дві", 2),
    ("три", 3), ("чотири", 4), ("пять", 5), ("шість", 6), ("сім", 7), ("вісім", 8),
    ("девять", 9), ("десять", 10), ("одинадцять", 11), ("дванадцять", 12),
    ("një", 1), ("nje", 1), ("dy", 2), ("tre", 3), ("tri", 3), ("katër", 4),
    ("kater", 4), ("pesë", 5), ("pese", 5), ("gjashtë", 6), ("gjashte", 6),
    ("shtatë", 7), ("shtate", 7), ("tetë", 8), ("tete", 8), ("nëntë", 9),
    ("nente", 9), ("dhjetë", 10), ("dhjete", 10),
];

/// A word read as a number: digits, a number word, or a quantity mark glued
/// to digits ("x2", "2x", Cyrillic "х2"). `None` for everything else.
pub fn number(w: &str) -> Option<u32> {
    let core = w
        .strip_prefix('x')
        .or_else(|| w.strip_prefix('х'))
        .or_else(|| w.strip_suffix('x'))
        .or_else(|| w.strip_suffix('х'))
        .unwrap_or(w);
    if !core.is_empty() && core.chars().all(|c| c.is_ascii_digit()) {
        return core.parse().ok().filter(|n| (1..=MAX_SAID).contains(n));
    }
    NUMBER_WORDS.iter().find(|(s, _)| *s == w).map(|(_, n)| *n)
}

#[cfg(test)]
mod tests;
